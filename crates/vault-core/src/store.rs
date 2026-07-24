use crate::db::Vault;
use crate::dotenv::{parse_env_text, serialize_env};
use crate::error::{Result, VaultError};
use crate::models::{
    DiffRow, Environment, EnvironmentSummary, GroupMember, Project, ProjectInfo, Repo,
    RepoSummary, SearchResult, Snapshot, SnapshotVariable, SnapshotWithStats, UnlinkedMatch,
    Variable, VariableWithUsage,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

const MAX_SNAPSHOTS_PER_ENV: i64 = 100;

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// Parses a snapshot payload, mapping any deserialization failure to a generic
/// message. The payload holds secret values, and `serde_json`'s error text can
/// quote a fragment of the input near the point of failure — which must never
/// be allowed to reach a log or an error toast.
fn parse_snapshot_payload(payload: &str) -> Result<Vec<SnapshotVariable>> {
    serde_json::from_str(payload)
        .map_err(|_| VaultError::Crypto("snapshot payload is corrupt or unreadable".into()))
}

fn map_unique(e: rusqlite::Error, what: &str) -> VaultError {
    if let rusqlite::Error::SqliteFailure(ref inner, _) = e {
        if inner.code == rusqlite::ErrorCode::ConstraintViolation {
            return VaultError::Duplicate(what.to_string());
        }
    }
    VaultError::Db(e)
}

/// Compares two point-in-time variable sets by key. Rows come back sorted by
/// key so the history view has a stable order regardless of insertion order.
fn diff_variable_sets(before: &[SnapshotVariable], after: &[SnapshotVariable]) -> Vec<DiffRow> {
    let old: HashMap<&str, &str> = before.iter().map(|v| (v.key.as_str(), v.value.as_str())).collect();
    let new: HashMap<&str, &str> = after.iter().map(|v| (v.key.as_str(), v.value.as_str())).collect();

    let mut rows: Vec<DiffRow> = Vec::new();
    for (key, new_value) in &new {
        match old.get(key) {
            None => rows.push(DiffRow {
                key: (*key).to_string(),
                kind: "added".into(),
                old_value: None,
                new_value: Some((*new_value).to_string()),
            }),
            Some(old_value) if old_value != new_value => rows.push(DiffRow {
                key: (*key).to_string(),
                kind: "changed".into(),
                old_value: Some((*old_value).to_string()),
                new_value: Some((*new_value).to_string()),
            }),
            Some(_) => {}
        }
    }
    for (key, old_value) in &old {
        if !new.contains_key(key) {
            rows.push(DiffRow {
                key: (*key).to_string(),
                kind: "removed".into(),
                old_value: Some((*old_value).to_string()),
                new_value: None,
            });
        }
    }
    rows.sort_by(|a, b| a.key.cmp(&b.key));
    rows
}

/// Dissolves link groups that no longer have at least two members — which is
/// what a cascade delete of a repo or environment can leave behind — and drops
/// the now-empty `groups` rows. Returns the environments whose variables were
/// unlinked, so callers can snapshot them.
fn prune_orphan_groups(conn: &rusqlite::Connection) -> Result<Vec<String>> {
    let stale_groups: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT g.id FROM groups g
             WHERE (SELECT COUNT(*) FROM variables v WHERE v.group_id = g.id) < 2",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut v = Vec::new();
        for row in rows {
            v.push(row?);
        }
        v
    };

    let mut affected_envs: Vec<String> = Vec::new();
    for group_id in &stale_groups {
        {
            let mut stmt = conn.prepare("SELECT env_id FROM variables WHERE group_id = ?1")?;
            let rows = stmt.query_map(params![group_id], |r| r.get::<_, String>(0))?;
            for row in rows {
                let env_id = row?;
                if !affected_envs.contains(&env_id) {
                    affected_envs.push(env_id);
                }
            }
        }
        conn.execute(
            "UPDATE variables SET group_id = NULL WHERE group_id = ?1",
            params![group_id],
        )?;
        conn.execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;
    }
    Ok(affected_envs)
}

fn row_to_variable(row: &rusqlite::Row) -> rusqlite::Result<Variable> {
    Ok(Variable {
        id: row.get(0)?,
        env_id: row.get(1)?,
        key: row.get(2)?,
        value: row.get(3)?,
        group_id: row.get(4)?,
        description: row.get(5)?,
        required: row.get::<_, i64>(6)? != 0,
        rotate_after_days: row.get(7)?,
    })
}

/// The column list every [`row_to_variable`] query selects, in order. Named
/// once so a query that joins extra columns on the end (repo and environment
/// names, say) can index past it without every call site having to count.
const VARIABLE_COLUMNS: &str = "v.id, v.env_id, v.key, v.value, v.group_id, v.description, \
                                v.required, v.rotate_after_days";

impl Vault {
    // ---------------------------------------------------------------
    // Repos
    // ---------------------------------------------------------------

    pub fn create_repo(&self, name: &str) -> Result<Repo> {
        let name = name.trim();
        if name.is_empty() {
            return Err(VaultError::InvalidInput("repo name must not be empty".into()));
        }
        let id = new_id();
        let sort_order: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM repos",
            [],
            |r| r.get(0),
        )?;
        self.conn
            .execute(
                "INSERT INTO repos (id, name, sort_order, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, sort_order, now()],
            )
            .map_err(|e| map_unique(e, "repo"))?;
        self.persist()?;
        Ok(Repo {
            id,
            name: name.to_string(),
            sort_order,
        })
    }

    pub fn list_repos(&self) -> Result<Vec<Repo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, sort_order FROM repos ORDER BY sort_order")?;
        let rows = stmt.query_map([], |r| {
            Ok(Repo {
                id: r.get(0)?,
                name: r.get(1)?,
                sort_order: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn rename_repo(&self, id: &str, new_name: &str) -> Result<()> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(VaultError::InvalidInput("repo name must not be empty".into()));
        }
        let affected = self
            .conn
            .execute(
                "UPDATE repos SET name = ?1 WHERE id = ?2",
                params![new_name, id],
            )
            .map_err(|e| map_unique(e, "repo"))?;
        if affected == 0 {
            return Err(VaultError::Missing(format!("repo {id}")));
        }
        self.persist()?;
        Ok(())
    }

    /// Deletes a repo and everything under it. Environments, variables and
    /// snapshots cascade; link groups that are left with fewer than two members
    /// as a result are dissolved so the "linked xN" counts stay truthful.
    pub fn delete_repo(&self, id: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let affected = tx.execute("DELETE FROM repos WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(VaultError::Missing(format!("repo {id}")));
        }
        let dissolved_envs = prune_orphan_groups(&tx)?;
        tx.commit()?;

        for env_id in &dissolved_envs {
            self.snapshot_env_internal(env_id, "Link group dissolved")?;
        }
        self.persist()?;
        Ok(())
    }

    /// Full sidebar tree: repos with their environments and variable counts.
    pub fn list_repo_summaries(&self) -> Result<Vec<RepoSummary>> {
        let repos = self.list_repos()?;
        let mut out = Vec::with_capacity(repos.len());
        for repo in repos {
            let mut stmt = self.conn.prepare(
                "SELECT e.id, e.name, (SELECT COUNT(*) FROM variables v WHERE v.env_id = e.id)
                 FROM environments e WHERE e.repo_id = ?1 ORDER BY e.created_at",
            )?;
            let repo_id = repo.id.clone();
            let rows = stmt.query_map(params![repo.id], move |r| {
                Ok(EnvironmentSummary {
                    id: r.get(0)?,
                    repo_id: repo_id.clone(),
                    name: r.get(1)?,
                    var_count: r.get(2)?,
                })
            })?;
            let mut envs = Vec::new();
            for r in rows {
                envs.push(r?);
            }
            out.push(RepoSummary {
                id: repo.id,
                name: repo.name,
                sort_order: repo.sort_order,
                envs,
            });
        }
        Ok(out)
    }

    // ---------------------------------------------------------------
    // Environments
    // ---------------------------------------------------------------

    pub fn create_environment(&self, repo_id: &str, name: &str) -> Result<Environment> {
        let name = name.trim();
        if name.is_empty() {
            return Err(VaultError::InvalidInput(
                "environment name must not be empty".into(),
            ));
        }
        let id = new_id();
        self.conn
            .execute(
                "INSERT INTO environments (id, repo_id, name, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, repo_id, name, now()],
            )
            .map_err(|e| map_unique(e, "environment"))?;
        self.persist()?;
        Ok(Environment {
            id,
            repo_id: repo_id.to_string(),
            name: name.to_string(),
        })
    }

    pub fn list_environments(&self, repo_id: &str) -> Result<Vec<Environment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, repo_id, name FROM environments WHERE repo_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![repo_id], |r| {
            Ok(Environment {
                id: r.get(0)?,
                repo_id: r.get(1)?,
                name: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn rename_environment(&self, id: &str, new_name: &str) -> Result<()> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(VaultError::InvalidInput(
                "environment name must not be empty".into(),
            ));
        }
        let affected = self
            .conn
            .execute(
                "UPDATE environments SET name = ?1 WHERE id = ?2",
                params![new_name, id],
            )
            .map_err(|e| map_unique(e, "environment"))?;
        if affected == 0 {
            return Err(VaultError::Missing(format!("environment {id}")));
        }
        self.persist()?;
        Ok(())
    }

    /// Deletes an environment with its variables and snapshots. Link groups
    /// left with fewer than two members are dissolved (see [`Vault::delete_repo`]).
    pub fn delete_environment(&self, id: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let affected = tx.execute("DELETE FROM environments WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(VaultError::Missing(format!("environment {id}")));
        }
        let dissolved_envs = prune_orphan_groups(&tx)?;
        tx.commit()?;

        for env_id in &dissolved_envs {
            self.snapshot_env_internal(env_id, "Link group dissolved")?;
        }
        self.persist()?;
        Ok(())
    }

    /// Creates `new_name` in the same repo as `env_id` and copies every key
    /// into it. Values are copied only if `copy_values` is true; otherwise
    /// the duplicate starts with the same keys but blank values, which is
    /// the safer default when the source holds live credentials -- a
    /// duplicate should be a deliberate choice to carry secrets into a new
    /// environment, not a side effect of wanting the same key list. Copied
    /// variables are not linked to their originals; link them explicitly via
    /// [`Vault::link_variables`] if that is what's wanted.
    pub fn duplicate_environment(
        &self,
        env_id: &str,
        new_name: &str,
        copy_values: bool,
    ) -> Result<Environment> {
        let (_, source_env) = self
            .find_environment(env_id)?
            .ok_or_else(|| VaultError::Missing(format!("environment {env_id}")))?;
        let new_env = self.create_environment(&source_env.repo_id, new_name)?;

        let source_vars = self.list_variables(env_id)?;
        if !source_vars.is_empty() {
            let now_s = now();
            let tx = self.conn.unchecked_transaction()?;
            for v in &source_vars {
                let id = new_id();
                let value = if copy_values { v.value.as_str() } else { "" };
                tx.execute(
                    "INSERT INTO variables (id, env_id, key, value, group_id, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)",
                    params![id, new_env.id, v.key, value, now_s],
                )?;
            }
            tx.commit()?;

            self.snapshot_env_internal(&new_env.id, &format!("Duplicated from {}", source_env.name))?;
            self.persist()?;
        }
        Ok(new_env)
    }

    // ---------------------------------------------------------------
    // Projects: directories linked to a repo/environment
    // ---------------------------------------------------------------

    fn find_environment(&self, env_id: &str) -> Result<Option<(Repo, Environment)>> {
        self.conn
            .query_row(
                "SELECT e.id, e.repo_id, e.name, r.name, r.sort_order
                 FROM environments e JOIN repos r ON r.id = e.repo_id
                 WHERE e.id = ?1",
                params![env_id],
                |r| {
                    let env_id: String = r.get(0)?;
                    let repo_id: String = r.get(1)?;
                    let env_name: String = r.get(2)?;
                    let repo_name: String = r.get(3)?;
                    let sort_order: i64 = r.get(4)?;
                    Ok((
                        Repo {
                            id: repo_id.clone(),
                            name: repo_name,
                            sort_order,
                        },
                        Environment {
                            id: env_id,
                            repo_id,
                            name: env_name,
                        },
                    ))
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Records `path` as linked to `env_id`, so future lookups from within it
    /// (or a subdirectory of it) resolve to that environment without naming
    /// it explicitly. Relinking an already-linked path just repoints it.
    /// `path` is canonicalized before storing -- the security guarantee this
    /// depends on is that only a path that genuinely exists and was
    /// deliberately linked can ever resolve, never a value read out of a
    /// file inside the directory itself.
    pub fn link_project(&self, path: &Path, env_id: &str) -> Result<Project> {
        let canonical = path.canonicalize()?;
        let path_str = canonical.to_string_lossy().into_owned();
        let id = new_id();
        let now_s = now();
        self.conn
            .execute(
                "INSERT INTO projects (id, path, env_id, created_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(path) DO UPDATE SET env_id = excluded.env_id",
                params![id, path_str, env_id, now_s],
            )
            .map_err(|e| map_unique(e, "project"))?;
        self.persist()?;
        self.conn
            .query_row(
                "SELECT id, path, env_id, created_at FROM projects WHERE path = ?1",
                params![path_str],
                |r| {
                    Ok(Project {
                        id: r.get(0)?,
                        path: r.get(1)?,
                        env_id: r.get(2)?,
                        created_at: r.get(3)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    /// Removes the link for `path`. Falls back to comparing the raw,
    /// uncanonicalized path text if canonicalization fails (the directory no
    /// longer exists) -- a stale link should still be removable.
    pub fn unlink_project(&self, path: &Path) -> Result<()> {
        let path_str = path
            .canonicalize()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.to_string_lossy().into_owned());
        let affected = self
            .conn
            .execute("DELETE FROM projects WHERE path = ?1", params![path_str])?;
        if affected == 0 {
            return Err(VaultError::Missing(format!("linked project at {path_str}")));
        }
        self.persist()?;
        Ok(())
    }

    /// Resolves `cwd` to a linked repo/environment by walking up through its
    /// ancestors, nearest first, so a subdirectory of a linked project
    /// resolves the same way the linked directory itself does. Returns
    /// `None` if neither `cwd` nor any parent was ever linked -- a cloned
    /// repository that merely contains a marker file naming a target is not
    /// enough on its own (see the module-level docs on why the mapping lives
    /// in the vault, not a file in the directory).
    pub fn resolve_project(&self, cwd: &Path) -> Result<Option<(Repo, Environment)>> {
        let Ok(canonical) = cwd.canonicalize() else {
            return Ok(None);
        };
        let mut candidate = Some(canonical.as_path());
        while let Some(dir) = candidate {
            let dir_str = dir.to_string_lossy().into_owned();
            let env_id: Option<String> = self
                .conn
                .query_row(
                    "SELECT env_id FROM projects WHERE path = ?1",
                    params![dir_str],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(env_id) = env_id {
                if let Some(found) = self.find_environment(&env_id)? {
                    return Ok(Some(found));
                }
            }
            candidate = dir.parent();
        }
        Ok(None)
    }

    /// All linked projects, newest first. A link whose directory no longer
    /// exists on disk is pruned here rather than returned, so the list never
    /// shows folders that have since been deleted or moved.
    pub fn list_projects(&self) -> Result<Vec<ProjectInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.path, p.env_id, p.created_at, r.name, e.name
             FROM projects p
             JOIN environments e ON e.id = p.env_id
             JOIN repos r ON r.id = e.repo_id
             ORDER BY p.created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ProjectInfo {
                id: r.get(0)?,
                path: r.get(1)?,
                env_id: r.get(2)?,
                created_at: r.get(3)?,
                repo_name: r.get(4)?,
                env_name: r.get(5)?,
            })
        })?;

        let mut live = Vec::new();
        let mut stale_ids = Vec::new();
        for row in rows {
            let p = row?;
            if Path::new(&p.path).exists() {
                live.push(p);
            } else {
                stale_ids.push(p.id.clone());
            }
        }
        if !stale_ids.is_empty() {
            for id in &stale_ids {
                self.conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
            }
            self.persist()?;
        }
        Ok(live)
    }

    // ---------------------------------------------------------------
    // Environment diff and sync
    // ---------------------------------------------------------------

    /// Compares two environments' current variables by key: `added` means
    /// present only in `env_b`, `removed` means present only in `env_a`,
    /// `changed` means both have the key but with different values. Reuses
    /// [`diff_variable_sets`], the same machinery history diffing uses,
    /// by projecting each environment's live variables into the same shape.
    pub fn diff_environments(&self, env_a: &str, env_b: &str) -> Result<Vec<DiffRow>> {
        let to_snapshot_vars = |vars: Vec<Variable>| -> Vec<SnapshotVariable> {
            vars.into_iter()
                .map(|v| SnapshotVariable {
                    key: v.key,
                    value: v.value,
                    group_id: v.group_id,
                })
                .collect()
        };
        let a = to_snapshot_vars(self.list_variables(env_a)?);
        let b = to_snapshot_vars(self.list_variables(env_b)?);
        Ok(diff_variable_sets(&a, &b))
    }

    /// Copies one variable's value into `target_env_id`. If the target
    /// already has a variable under the same key, this is routed through the
    /// normal value-update path -- so if that variable belongs to a link
    /// group, the whole group still propagates. Otherwise a new, unlinked
    /// variable is created in the target.
    pub fn copy_variable_to_env(&self, var_id: &str, target_env_id: &str) -> Result<()> {
        let source = self.get_variable(var_id)?;
        let existing = self
            .list_variables(target_env_id)?
            .into_iter()
            .find(|v| v.key == source.key);
        match existing {
            Some(v) => self.update_variable_value(&v.id, &source.value),
            None => self
                .add_variable(target_env_id, &source.key, &source.value)
                .map(|_| ()),
        }
    }

    /// As [`Vault::copy_variable_to_env`], but looks the source variable up
    /// by key within `source_env_id` instead of requiring its id -- callers
    /// that only have a key (like the compare view, working from
    /// [`DiffRow`]s) do not need to look up an id first.
    pub fn copy_key_to_env(&self, source_env_id: &str, target_env_id: &str, key: &str) -> Result<()> {
        let source = self
            .list_variables(source_env_id)?
            .into_iter()
            .find(|v| v.key == key)
            .ok_or_else(|| VaultError::Missing(format!("{key} in this environment")))?;
        self.copy_variable_to_env(&source.id, target_env_id)
    }

    /// Copies every key `source_env_id` has that `target_env_id` lacks,
    /// leaving `target_env_id`'s existing values untouched. All copies land
    /// in one transaction and one history snapshot, rather than one per key.
    /// Returns how many keys were copied.
    pub fn copy_missing_to_env(&self, source_env_id: &str, target_env_id: &str) -> Result<usize> {
        let source_vars = self.list_variables(source_env_id)?;
        let existing_keys: std::collections::HashSet<String> = self
            .list_variables(target_env_id)?
            .into_iter()
            .map(|v| v.key)
            .collect();
        let missing: Vec<&Variable> = source_vars
            .iter()
            .filter(|v| !existing_keys.contains(&v.key))
            .collect();
        if missing.is_empty() {
            return Ok(0);
        }

        let now_s = now();
        let tx = self.conn.unchecked_transaction()?;
        for v in &missing {
            let id = new_id();
            tx.execute(
                "INSERT INTO variables (id, env_id, key, value, group_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)",
                params![id, target_env_id, v.key, v.value, now_s],
            )?;
        }
        tx.commit()?;

        self.snapshot_env_internal(
            target_env_id,
            &format!(
                "Copied {} missing variable{} from another environment",
                missing.len(),
                if missing.len() == 1 { "" } else { "s" }
            ),
        )?;
        self.persist()?;
        Ok(missing.len())
    }

    /// Same-key, same-value pairs between two environments that are not
    /// already part of a link group -- a discovery hook for the compare
    /// view's "link these?" prompt. A variable that is already in *any*
    /// group is excluded, not just one linked to its counterpart here.
    pub fn unlinked_identical_pairs(&self, env_a: &str, env_b: &str) -> Result<Vec<UnlinkedMatch>> {
        let vars_a = self.list_variables(env_a)?;
        let vars_b = self.list_variables(env_b)?;
        let mut out = Vec::new();
        for a in &vars_a {
            if a.group_id.is_some() {
                continue;
            }
            for b in &vars_b {
                if b.group_id.is_none() && a.key == b.key && a.value == b.value {
                    out.push(UnlinkedMatch {
                        key: a.key.clone(),
                        var_a: a.clone(),
                        var_b: b.clone(),
                    });
                }
            }
        }
        Ok(out)
    }

    // ---------------------------------------------------------------
    // Variables
    // ---------------------------------------------------------------

    fn get_variable(&self, id: &str) -> Result<Variable> {
        self.conn
            .query_row(
                "SELECT id, env_id, key, value, group_id, description, required, rotate_after_days
                 FROM variables WHERE id = ?1",
                params![id],
                row_to_variable,
            )
            .optional()?
            .ok_or_else(|| VaultError::Missing(format!("variable {id}")))
    }

    pub fn list_variables(&self, env_id: &str) -> Result<Vec<Variable>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, env_id, key, value, group_id, description, required, rotate_after_days
             FROM variables WHERE env_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![env_id], row_to_variable)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_variables_with_usage(&self, env_id: &str) -> Result<Vec<VariableWithUsage>> {
        let usage = self.group_usage_counts()?;
        let vars = self.list_variables(env_id)?;
        Ok(vars
            .into_iter()
            .map(|v| {
                let group_usage = v
                    .group_id
                    .as_ref()
                    .and_then(|g| usage.get(g))
                    .copied()
                    .unwrap_or(0);
                VariableWithUsage {
                    variable: v,
                    group_usage,
                }
            })
            .collect())
    }

    pub fn add_variable(&self, env_id: &str, key: &str, value: &str) -> Result<Variable> {
        let key = key.trim();
        if key.is_empty() {
            return Err(VaultError::InvalidInput("key must not be empty".into()));
        }
        let id = new_id();
        let now_s = now();
        self.conn
            .execute(
                "INSERT INTO variables (id, env_id, key, value, group_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)",
                params![id, env_id, key, value, now_s],
            )
            .map_err(|e| map_unique(e, "variable"))?;
        self.snapshot_env_internal(env_id, &format!("Added {key}"))?;
        self.persist()?;
        Ok(Variable {
            id,
            env_id: env_id.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            group_id: None,
            description: None,
            required: false,
            rotate_after_days: None,
        })
    }

    /// Updates a variable's value. If it belongs to a link group, every
    /// variable in that group is updated to the same value in one transaction.
    pub fn update_variable_value(&self, var_id: &str, new_value: &str) -> Result<()> {
        let var = self.get_variable(var_id)?;
        let now_s = now();
        let mut affected_envs: Vec<String> = Vec::new();

        let tx = self.conn.unchecked_transaction()?;
        if let Some(group_id) = &var.group_id {
            let ids_envs: Vec<(String, String)> = {
                let mut stmt = tx.prepare("SELECT id, env_id FROM variables WHERE group_id = ?1")?;
                let rows =
                    stmt.query_map(params![group_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
                let mut v = Vec::new();
                for row in rows {
                    v.push(row?);
                }
                v
            };
            for (id, env_id) in ids_envs {
                tx.execute(
                    "UPDATE variables SET value = ?1, updated_at = ?2 WHERE id = ?3",
                    params![new_value, now_s, id],
                )?;
                if !affected_envs.contains(&env_id) {
                    affected_envs.push(env_id);
                }
            }
        } else {
            tx.execute(
                "UPDATE variables SET value = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_value, now_s, var_id],
            )?;
            affected_envs.push(var.env_id.clone());
        }
        tx.commit()?;

        for env_id in &affected_envs {
            self.snapshot_env_internal(env_id, &format!("Updated {}", var.key))?;
        }
        self.persist()?;
        Ok(())
    }

    /// Renames a variable's key within its own environment. Link groups sync
    /// *values*, not keys, so group membership is deliberately left untouched —
    /// a renamed variable keeps syncing with its partners under the new name.
    pub fn rename_variable_key(&self, var_id: &str, new_key: &str) -> Result<()> {
        let new_key = new_key.trim();
        if new_key.is_empty() {
            return Err(VaultError::InvalidInput("key must not be empty".into()));
        }
        let var = self.get_variable(var_id)?;
        if var.key == new_key {
            return Ok(());
        }
        self.conn
            .execute(
                "UPDATE variables SET key = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_key, now(), var_id],
            )
            .map_err(|e| map_unique(e, "variable"))?;
        self.snapshot_env_internal(&var.env_id, &format!("Renamed {} to {}", var.key, new_key))?;
        self.persist()?;
        Ok(())
    }

    /// Sets a variable's documentation, whether `vault check` should treat it
    /// as required, and how long it may go untouched before the health panel
    /// calls it due for rotation. Deliberately not routed through the
    /// link-group propagation path: groups sync *values*, exactly as key
    /// renames do -- descriptions, the required flag and the rotation policy
    /// are per-environment documentation, not part of what a link keeps in
    /// sync. Not snapshotted either, since history is about value changes and
    /// restoring a snapshot does not touch metadata.
    ///
    /// A `rotate_after_days` of zero or less is rejected rather than stored:
    /// a policy that is due the instant it is set would badge every row
    /// forever, which is the same as no policy at all but noisier.
    pub fn set_variable_metadata(
        &self,
        var_id: &str,
        description: Option<&str>,
        required: bool,
        rotate_after_days: Option<i64>,
    ) -> Result<()> {
        if matches!(rotate_after_days, Some(d) if d <= 0) {
            return Err(VaultError::InvalidInput(
                "rotation interval must be at least one day".into(),
            ));
        }
        let affected = self.conn.execute(
            "UPDATE variables SET description = ?1, required = ?2, rotate_after_days = ?3
             WHERE id = ?4",
            params![description, required as i64, rotate_after_days, var_id],
        )?;
        if affected == 0 {
            return Err(VaultError::Missing(format!("variable {var_id}")));
        }
        self.persist()?;
        Ok(())
    }

    /// Variables in `env_id` marked required whose value is empty (after
    /// trimming) -- what `vault check` gates on.
    pub fn required_and_empty(&self, env_id: &str) -> Result<Vec<Variable>> {
        Ok(self
            .list_variables(env_id)?
            .into_iter()
            .filter(|v| v.required && v.value.trim().is_empty())
            .collect())
    }

    pub fn delete_variable(&self, var_id: &str) -> Result<()> {
        let var = self.get_variable(var_id)?;
        self.conn
            .execute("DELETE FROM variables WHERE id = ?1", params![var_id])?;
        if let Some(group_id) = &var.group_id {
            self.cleanup_group_if_singleton(group_id)?;
        }
        self.snapshot_env_internal(&var.env_id, &format!("Deleted {}", var.key))?;
        self.persist()?;
        Ok(())
    }

    /// Deletes every id in `var_ids` in one transaction, then snapshots each
    /// *affected environment* once -- not once per variable, which would
    /// flood the 100-snapshot-per-environment cap on anything but a small
    /// selection and destroy the very history a bulk delete might need to be
    /// undone from. Unknown ids are skipped rather than erroring, so a
    /// stale selection in the UI can't abort the rest of the batch.
    pub fn delete_variables(&self, var_ids: &[String]) -> Result<()> {
        if var_ids.is_empty() {
            return Ok(());
        }
        let mut counts: HashMap<String, i64> = HashMap::new();

        let tx = self.conn.unchecked_transaction()?;
        for id in var_ids {
            let env_id: Option<String> = tx
                .query_row("SELECT env_id FROM variables WHERE id = ?1", params![id], |r| r.get(0))
                .optional()?;
            let Some(env_id) = env_id else { continue };
            tx.execute("DELETE FROM variables WHERE id = ?1", params![id])?;
            *counts.entry(env_id).or_insert(0) += 1;
        }
        let dissolved_envs = prune_orphan_groups(&tx)?;
        tx.commit()?;

        for (env_id, count) in &counts {
            self.snapshot_env_internal(
                env_id,
                &format!("Deleted {count} variable{}", if *count == 1 { "" } else { "s" }),
            )?;
        }
        for env_id in &dissolved_envs {
            if !counts.contains_key(env_id) {
                self.snapshot_env_internal(env_id, "Link group dissolved")?;
            }
        }
        self.persist()?;
        Ok(())
    }

    /// Moves each of `var_ids` into `target_env_id`: a key that already
    /// exists there is updated in place (propagating through its link group,
    /// if any, the same as a normal value edit); otherwise a new, unlinked
    /// variable is created. The original rows are then removed. A moved
    /// variable does not carry its own link-group membership to the new
    /// row -- relocating a variable to a different environment is a
    /// structural change, not something that should silently keep syncing
    /// with wherever it used to live; re-link explicitly if that's wanted.
    /// One snapshot lands on each affected source environment and one on
    /// the target, not one per variable moved.
    pub fn move_variables(&self, var_ids: &[String], target_env_id: &str) -> Result<()> {
        if var_ids.is_empty() {
            return Ok(());
        }
        let now_s = now();
        let mut source_counts: HashMap<String, i64> = HashMap::new();
        let mut moved = 0i64;

        let tx = self.conn.unchecked_transaction()?;
        for id in var_ids {
            let row: Option<(String, String, String)> = tx
                .query_row(
                    "SELECT env_id, key, value FROM variables WHERE id = ?1",
                    params![id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let Some((source_env_id, key, value)) = row else { continue };
            if source_env_id == target_env_id {
                continue;
            }

            let existing_target_id: Option<String> = tx
                .query_row(
                    "SELECT id FROM variables WHERE env_id = ?1 AND key = ?2",
                    params![target_env_id, key],
                    |r| r.get(0),
                )
                .optional()?;
            match existing_target_id {
                Some(target_var_id) => {
                    let group_id: Option<String> = tx.query_row(
                        "SELECT group_id FROM variables WHERE id = ?1",
                        params![target_var_id],
                        |r| r.get(0),
                    )?;
                    match &group_id {
                        Some(g) => tx.execute(
                            "UPDATE variables SET value = ?1, updated_at = ?2 WHERE group_id = ?3",
                            params![value, now_s, g],
                        )?,
                        None => tx.execute(
                            "UPDATE variables SET value = ?1, updated_at = ?2 WHERE id = ?3",
                            params![value, now_s, target_var_id],
                        )?,
                    };
                }
                None => {
                    let new_var_id = new_id();
                    tx.execute(
                        "INSERT INTO variables (id, env_id, key, value, group_id, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)",
                        params![new_var_id, target_env_id, key, value, now_s],
                    )?;
                }
            }
            tx.execute("DELETE FROM variables WHERE id = ?1", params![id])?;
            *source_counts.entry(source_env_id).or_insert(0) += 1;
            moved += 1;
        }
        let dissolved_envs = prune_orphan_groups(&tx)?;
        tx.commit()?;

        for (env_id, count) in &source_counts {
            self.snapshot_env_internal(
                env_id,
                &format!("Moved {count} variable{} out", if *count == 1 { "" } else { "s" }),
            )?;
        }
        if moved > 0 {
            self.snapshot_env_internal(
                target_env_id,
                &format!("Moved {moved} variable{} in", if moved == 1 { "" } else { "s" }),
            )?;
        }
        for env_id in &dissolved_envs {
            if !source_counts.contains_key(env_id) && env_id != target_env_id {
                self.snapshot_env_internal(env_id, "Link group dissolved")?;
            }
        }
        self.persist()?;
        Ok(())
    }

    // ---------------------------------------------------------------
    // Linked groups
    // ---------------------------------------------------------------

    fn cleanup_group_if_singleton(&self, group_id: &str) -> Result<()> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM variables WHERE group_id = ?1",
            params![group_id],
            |r| r.get(0),
        )?;
        if count <= 1 {
            self.conn.execute(
                "UPDATE variables SET group_id = NULL WHERE group_id = ?1",
                params![group_id],
            )?;
            self.conn
                .execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;
        }
        Ok(())
    }

    /// Links two or more variables into one group so their values always
    /// stay in sync. If any are already in a group, that group is reused.
    /// The resulting shared value is taken from `var_ids[0]`.
    pub fn link_variables(&self, var_ids: &[String]) -> Result<String> {
        if var_ids.len() < 2 {
            return Err(VaultError::InvalidInput(
                "need at least two variables to link".into(),
            ));
        }
        let now_s = now();
        let tx = self.conn.unchecked_transaction()?;

        let primary: Variable = tx
            .query_row(
                "SELECT id, env_id, key, value, group_id, description, required, rotate_after_days
                 FROM variables WHERE id = ?1",
                params![var_ids[0]],
                row_to_variable,
            )
            .optional()?
            .ok_or_else(|| VaultError::Missing(format!("variable {}", var_ids[0])))?;

        let mut existing_group: Option<String> = None;
        for id in var_ids {
            let g: Option<String> =
                tx.query_row("SELECT group_id FROM variables WHERE id = ?1", params![id], |r| {
                    r.get(0)
                })?;
            if g.is_some() {
                existing_group = g;
                break;
            }
        }
        let group_id = match existing_group {
            Some(g) => g,
            None => {
                let gid = new_id();
                tx.execute(
                    "INSERT INTO groups (id, created_at) VALUES (?1, ?2)",
                    params![gid, now_s],
                )?;
                gid
            }
        };

        let mut affected_envs: Vec<String> = Vec::new();
        for id in var_ids {
            let env_id: String =
                tx.query_row("SELECT env_id FROM variables WHERE id = ?1", params![id], |r| {
                    r.get(0)
                })?;
            tx.execute(
                "UPDATE variables SET group_id = ?1, value = ?2, updated_at = ?3 WHERE id = ?4",
                params![group_id, primary.value, now_s, id],
            )?;
            if !affected_envs.contains(&env_id) {
                affected_envs.push(env_id);
            }
        }
        tx.commit()?;

        for env_id in &affected_envs {
            self.snapshot_env_internal(env_id, &format!("Linked {}", primary.key))?;
        }
        self.persist()?;
        Ok(group_id)
    }

    pub fn unlink_variable(&self, var_id: &str) -> Result<()> {
        let var = self.get_variable(var_id)?;
        let Some(group_id) = var.group_id else {
            return Ok(());
        };
        self.conn
            .execute("UPDATE variables SET group_id = NULL WHERE id = ?1", params![var_id])?;
        self.cleanup_group_if_singleton(&group_id)?;
        self.snapshot_env_internal(&var.env_id, &format!("Unlinked {}", var.key))?;
        self.persist()?;
        Ok(())
    }

    pub fn group_members(&self, group_id: &str) -> Result<Vec<GroupMember>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {VARIABLE_COLUMNS}, r.name, e.name
             FROM variables v
             JOIN environments e ON e.id = v.env_id
             JOIN repos r ON r.id = e.repo_id
             WHERE v.group_id = ?1
             ORDER BY r.name, e.name",
        ))?;
        let rows = stmt.query_map(params![group_id], |row| {
            Ok(GroupMember {
                variable: row_to_variable(row)?,
                repo_name: row.get(8)?,
                env_name: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn group_usage_counts(&self) -> Result<HashMap<String, i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT group_id, COUNT(*) FROM variables WHERE group_id IS NOT NULL GROUP BY group_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn linked_group_count(&self) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(DISTINCT group_id) FROM variables WHERE group_id IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .map_err(Into::into)
    }

    /// Same-key variables in other environments/repos, candidates to link
    /// against `var_id`.
    pub fn link_candidates(&self, var_id: &str) -> Result<Vec<GroupMember>> {
        let var = self.get_variable(var_id)?;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {VARIABLE_COLUMNS}, r.name, e.name
             FROM variables v
             JOIN environments e ON e.id = v.env_id
             JOIN repos r ON r.id = e.repo_id
             WHERE v.key = ?1 AND v.id != ?2
             ORDER BY r.name, e.name",
        ))?;
        let rows = stmt.query_map(params![var.key, var_id], |row| {
            Ok(GroupMember {
                variable: row_to_variable(row)?,
                repo_name: row.get(8)?,
                env_name: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Fuzzy(ish) substring search across repo names, `repo/env` pairs, and
    /// variable keys, for the command palette. Capped at 8 results.
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        'outer: for repo in self.list_repos()? {
            if repo.name.to_lowercase().contains(&q) {
                out.push(SearchResult {
                    kind: "repo".to_string(),
                    label: repo.name.clone(),
                    sublabel: "Repository".to_string(),
                    repo_id: repo.id.clone(),
                    env_id: None,
                });
                if out.len() >= 8 {
                    break 'outer;
                }
            }
            for env in self.list_environments(&repo.id)? {
                let combined = format!("{}/{}", repo.name, env.name).to_lowercase();
                if combined.contains(&q) {
                    out.push(SearchResult {
                        kind: "environment".to_string(),
                        label: format!("{}/{}", repo.name, env.name),
                        sublabel: "Environment".to_string(),
                        repo_id: repo.id.clone(),
                        env_id: Some(env.id.clone()),
                    });
                    if out.len() >= 8 {
                        break 'outer;
                    }
                }
                for v in self.list_variables(&env.id)? {
                    if v.key.to_lowercase().contains(&q) {
                        out.push(SearchResult {
                            kind: "variable".to_string(),
                            label: v.key.clone(),
                            sublabel: format!("{}/{}", repo.name, env.name),
                            repo_id: repo.id.clone(),
                            env_id: Some(env.id.clone()),
                        });
                        if out.len() >= 8 {
                            break 'outer;
                        }
                    }
                }
            }
        }
        out.truncate(8);
        Ok(out)
    }

    // ---------------------------------------------------------------
    // Import / export
    // ---------------------------------------------------------------

    pub fn import_env_text(&self, env_id: &str, text: &str) -> Result<usize> {
        let parsed = parse_env_text(text);
        if parsed.is_empty() {
            return Ok(0);
        }
        let now_s = now();
        let tx = self.conn.unchecked_transaction()?;
        for (key, value) in &parsed {
            let existing: Option<(String, Option<String>)> = tx
                .query_row(
                    "SELECT id, group_id FROM variables WHERE env_id = ?1 AND key = ?2",
                    params![env_id, key],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            match existing {
                Some((id, group_id)) => {
                    if let Some(g) = &group_id {
                        tx.execute(
                            "UPDATE variables SET value = ?1, updated_at = ?2 WHERE group_id = ?3",
                            params![value, now_s, g],
                        )?;
                    } else {
                        tx.execute(
                            "UPDATE variables SET value = ?1, updated_at = ?2 WHERE id = ?3",
                            params![value, now_s, id],
                        )?;
                    }
                }
                None => {
                    let id = new_id();
                    tx.execute(
                        "INSERT INTO variables (id, env_id, key, value, group_id, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)",
                        params![id, env_id, key, value, now_s],
                    )?;
                }
            }
        }
        tx.commit()?;

        self.snapshot_env_internal(
            env_id,
            &format!(
                "Imported {} variable{}",
                parsed.len(),
                if parsed.len() == 1 { "" } else { "s" }
            ),
        )?;
        self.persist()?;
        Ok(parsed.len())
    }

    pub fn export_env_text(&self, env_id: &str) -> Result<String> {
        let vars = self.list_variables(env_id)?;
        let pairs: Vec<(String, String)> = vars.into_iter().map(|v| (v.key, v.value)).collect();
        Ok(serialize_env(&pairs))
    }

    /// As [`Vault::export_env_text`], in one of the other formats `vault
    /// export --format` supports.
    pub fn export_env_as(&self, env_id: &str, format: crate::dotenv::ExportFormat) -> Result<String> {
        let vars = self.list_variables(env_id)?;
        let pairs: Vec<(String, String)> = vars.into_iter().map(|v| (v.key, v.value)).collect();
        crate::dotenv::export_as(&pairs, format)
    }

    // ---------------------------------------------------------------
    // Snapshots / history
    // ---------------------------------------------------------------

    fn snapshot_env_internal(&self, env_id: &str, summary: &str) -> Result<()> {
        let vars = self.list_variables(env_id)?;
        let payload_vars: Vec<SnapshotVariable> = vars
            .into_iter()
            .map(|v| SnapshotVariable {
                key: v.key,
                value: v.value,
                group_id: v.group_id,
            })
            .collect();
        let payload = serde_json::to_string(&payload_vars)?;
        let id = new_id();
        self.conn.execute(
            "INSERT INTO snapshots (id, env_id, created_at, summary, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, env_id, now(), summary, payload],
        )?;
        self.conn.execute(
            "DELETE FROM snapshots WHERE env_id = ?1 AND id NOT IN (
                SELECT id FROM snapshots WHERE env_id = ?1 ORDER BY created_at DESC LIMIT ?2
            )",
            params![env_id, MAX_SNAPSHOTS_PER_ENV],
        )?;
        Ok(())
    }

    pub fn list_snapshots(&self, env_id: &str) -> Result<Vec<Snapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, env_id, created_at, summary, payload FROM snapshots WHERE env_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![env_id], |r| {
            Ok(Snapshot {
                id: r.get(0)?,
                env_id: r.get(1)?,
                created_at: r.get(2)?,
                summary: r.get(3)?,
                payload: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn snapshot_by_id(&self, snapshot_id: &str) -> Result<Snapshot> {
        self.conn
            .query_row(
                "SELECT id, env_id, created_at, summary, payload FROM snapshots WHERE id = ?1",
                params![snapshot_id],
                |r| {
                    Ok(Snapshot {
                        id: r.get(0)?,
                        env_id: r.get(1)?,
                        created_at: r.get(2)?,
                        summary: r.get(3)?,
                        payload: r.get(4)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| VaultError::Missing(format!("snapshot {snapshot_id}")))
    }

    /// Snapshots for an environment, newest first, each annotated with what it
    /// changed relative to the snapshot before it.
    pub fn list_snapshots_with_stats(&self, env_id: &str) -> Result<Vec<SnapshotWithStats>> {
        let newest_first = self.list_snapshots(env_id)?;
        let mut out = Vec::with_capacity(newest_first.len());
        for (i, snap) in newest_first.iter().enumerate() {
            // the list is newest-first, so a snapshot's predecessor is the next element
            let before: Vec<SnapshotVariable> = match newest_first.get(i + 1) {
                Some(prev) => parse_snapshot_payload(&prev.payload)?,
                None => Vec::new(),
            };
            let after: Vec<SnapshotVariable> = parse_snapshot_payload(&snap.payload)?;
            let rows = diff_variable_sets(&before, &after);
            out.push(SnapshotWithStats {
                snapshot: snap.clone(),
                added: rows.iter().filter(|r| r.kind == "added").count() as i64,
                removed: rows.iter().filter(|r| r.kind == "removed").count() as i64,
                changed: rows.iter().filter(|r| r.kind == "changed").count() as i64,
            });
        }
        Ok(out)
    }

    /// What changed in this snapshot (`against` = `"previous"`), or what
    /// restoring it would change (`against` = `"current"`).
    pub fn diff_snapshot(&self, snapshot_id: &str, against: &str) -> Result<Vec<DiffRow>> {
        let snap = self.snapshot_by_id(snapshot_id)?;
        let target: Vec<SnapshotVariable> = parse_snapshot_payload(&snap.payload)?;

        let baseline: Vec<SnapshotVariable> = match against {
            "current" => self
                .list_variables(&snap.env_id)?
                .into_iter()
                .map(|v| SnapshotVariable {
                    key: v.key,
                    value: v.value,
                    group_id: v.group_id,
                })
                .collect(),
            "previous" => {
                let all = self.list_snapshots(&snap.env_id)?;
                let idx = all.iter().position(|s| s.id == snap.id);
                match idx.and_then(|i| all.get(i + 1)) {
                    Some(prev) => parse_snapshot_payload(&prev.payload)?,
                    None => Vec::new(),
                }
            }
            other => {
                return Err(VaultError::InvalidInput(format!(
                    "unknown diff baseline '{other}' (expected 'previous' or 'current')"
                )))
            }
        };

        Ok(diff_variable_sets(&baseline, &target))
    }

    /// Restores a single key from a snapshot, leaving the rest of the
    /// environment alone. The write goes through the normal value-update path,
    /// so a linked variable still propagates to its whole group.
    pub fn restore_variable_from_snapshot(&self, snapshot_id: &str, key: &str) -> Result<()> {
        let snap = self.snapshot_by_id(snapshot_id)?;
        let vars: Vec<SnapshotVariable> = parse_snapshot_payload(&snap.payload)?;
        let wanted = vars
            .into_iter()
            .find(|v| v.key == key)
            .ok_or_else(|| VaultError::Missing(format!("{key} in this snapshot")))?;

        let existing = self
            .list_variables(&snap.env_id)?
            .into_iter()
            .find(|v| v.key == key);
        match existing {
            Some(v) => self.update_variable_value(&v.id, &wanted.value),
            None => self.add_variable(&snap.env_id, &wanted.key, &wanted.value).map(|_| ()),
        }
    }

    /// Replaces the environment's current variables with a prior snapshot's
    /// contents. Snapshots the pre-restore state first, so a restore can
    /// itself be undone.
    pub fn restore_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snap: Snapshot = self
            .conn
            .query_row(
                "SELECT id, env_id, created_at, summary, payload FROM snapshots WHERE id = ?1",
                params![snapshot_id],
                |r| {
                    Ok(Snapshot {
                        id: r.get(0)?,
                        env_id: r.get(1)?,
                        created_at: r.get(2)?,
                        summary: r.get(3)?,
                        payload: r.get(4)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| VaultError::Missing(format!("snapshot {snapshot_id}")))?;

        self.snapshot_env_internal(&snap.env_id, "Before restore")?;

        let restored_vars: Vec<SnapshotVariable> = parse_snapshot_payload(&snap.payload)?;
        let now_s = now();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM variables WHERE env_id = ?1", params![snap.env_id])?;
        for v in &restored_vars {
            let id = new_id();
            tx.execute(
                "INSERT INTO variables (id, env_id, key, value, group_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, snap.env_id, v.key, v.value, v.group_id, now_s],
            )?;
        }
        tx.commit()?;

        self.snapshot_env_internal(&snap.env_id, &format!("Restored snapshot from {}", snap.created_at))?;
        self.persist()?;
        Ok(())
    }

    // ---------------------------------------------------------------
    // Misc key/value metadata (onboarding flag, etc.)
    // ---------------------------------------------------------------

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| r.get(0))
            .optional()
            .map_err(Into::into)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        self.persist()?;
        Ok(())
    }
}
