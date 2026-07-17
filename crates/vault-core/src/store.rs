use crate::db::Vault;
use crate::dotenv::{parse_env_text, serialize_env};
use crate::error::{Result, VaultError};
use crate::models::{
    Environment, EnvironmentSummary, GroupMember, Member, Repo, RepoSummary, SearchResult,
    Snapshot, SnapshotVariable, Variable, VariableWithUsage,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;
use uuid::Uuid;

const MAX_SNAPSHOTS_PER_ENV: i64 = 100;

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn map_unique(e: rusqlite::Error, what: &str) -> VaultError {
    if let rusqlite::Error::SqliteFailure(ref inner, _) = e {
        if inner.code == rusqlite::ErrorCode::ConstraintViolation {
            return VaultError::Duplicate(what.to_string());
        }
    }
    VaultError::Db(e)
}

fn row_to_variable(row: &rusqlite::Row) -> rusqlite::Result<Variable> {
    Ok(Variable {
        id: row.get(0)?,
        env_id: row.get(1)?,
        key: row.get(2)?,
        value: row.get(3)?,
        group_id: row.get(4)?,
    })
}

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

    pub fn delete_repo(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM repos WHERE id = ?1", params![id])?;
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

    pub fn delete_environment(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM environments WHERE id = ?1", params![id])?;
        self.persist()?;
        Ok(())
    }

    // ---------------------------------------------------------------
    // Variables
    // ---------------------------------------------------------------

    fn get_variable(&self, id: &str) -> Result<Variable> {
        self.conn
            .query_row(
                "SELECT id, env_id, key, value, group_id FROM variables WHERE id = ?1",
                params![id],
                row_to_variable,
            )
            .optional()?
            .ok_or_else(|| VaultError::Missing(format!("variable {id}")))
    }

    pub fn list_variables(&self, env_id: &str) -> Result<Vec<Variable>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, env_id, key, value, group_id FROM variables WHERE env_id = ?1 ORDER BY created_at",
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
                "SELECT id, env_id, key, value, group_id FROM variables WHERE id = ?1",
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
        let mut stmt = self.conn.prepare(
            "SELECT v.id, v.env_id, v.key, v.value, v.group_id, r.name, e.name
             FROM variables v
             JOIN environments e ON e.id = v.env_id
             JOIN repos r ON r.id = e.repo_id
             WHERE v.group_id = ?1
             ORDER BY r.name, e.name",
        )?;
        let rows = stmt.query_map(params![group_id], |row| {
            Ok(GroupMember {
                variable: row_to_variable(row)?,
                repo_name: row.get(5)?,
                env_name: row.get(6)?,
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
        let mut stmt = self.conn.prepare(
            "SELECT v.id, v.env_id, v.key, v.value, v.group_id, r.name, e.name
             FROM variables v
             JOIN environments e ON e.id = v.env_id
             JOIN repos r ON r.id = e.repo_id
             WHERE v.key = ?1 AND v.id != ?2
             ORDER BY r.name, e.name",
        )?;
        let rows = stmt.query_map(params![var.key, var_id], |row| {
            Ok(GroupMember {
                variable: row_to_variable(row)?,
                repo_name: row.get(5)?,
                env_name: row.get(6)?,
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

        let restored_vars: Vec<SnapshotVariable> = serde_json::from_str(&snap.payload)?;
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
    // Members (local-only mock of the Share modal)
    // ---------------------------------------------------------------

    pub fn list_members(&self) -> Result<Vec<Member>> {
        let mut stmt = self.conn.prepare("SELECT id, email, role FROM members ORDER BY email")?;
        let rows = stmt.query_map([], |r| {
            Ok(Member {
                id: r.get(0)?,
                email: r.get(1)?,
                role: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn add_member(&self, email: &str, role: &str) -> Result<Member> {
        let email = email.trim();
        if email.is_empty() {
            return Err(VaultError::InvalidInput("email must not be empty".into()));
        }
        let id = new_id();
        self.conn
            .execute(
                "INSERT INTO members (id, email, role) VALUES (?1, ?2, ?3)",
                params![id, email, role],
            )
            .map_err(|e| map_unique(e, "member"))?;
        self.persist()?;
        Ok(Member {
            id,
            email: email.to_string(),
            role: role.to_string(),
        })
    }

    pub fn remove_member(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM members WHERE id = ?1", params![id])?;
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
