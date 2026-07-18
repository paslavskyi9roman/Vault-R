//! Git leak guard: is anything in the vault already committed?
//!
//! Phase 2's `projects` table maps directories to environments, so Vault-R
//! knows where the user's code lives. That makes a question answerable that no
//! `.env` manager normally answers: *is any of this already in git?*
//!
//! Three findings, in descending severity:
//!
//! 1. A **tracked `.env` file** — git has it, so it is in the history.
//! 2. A **vault value appearing verbatim in a tracked file** — the key that got
//!    pasted into `docker-compose.yml` or a README.
//! 3. An **untracked `.env` file that `.gitignore` does not cover** — not a
//!    leak yet, one `git add .` away from being one.
//!
//! # A finding never contains a secret value
//!
//! The report is what the user screenshots, pastes into a chat, or forwards to
//! a colleague. Anything in it is effectively public, so findings carry key
//! names, paths and line numbers only. A leak detector that leaks while
//! reporting a leak would be a special kind of failure.
//!
//! # What the fix does and does not do
//!
//! [`apply_gitignore_patterns`] prevents the *next* commit. It does not untrack
//! a file and it cannot touch history, so a finding that is already committed
//! carries [`LeakFinding::needs_rotation`] and says so in as many words: the
//! value is compromised, rotate it, and `git rm --cached` is the user's next
//! step. Rewriting history is deliberately not offered — that is a destructive,
//! team-coordinated operation, and a button that does it will eventually eat
//! somebody's work.

use crate::db::Vault;
use crate::error::{Result, VaultError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Files larger than this are not searched. A secret pasted into a 2 MiB file
/// is possible; reading a whole source tree's worth of bundles and fixtures to
/// find it is not worth freezing the UI over.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Upper bound on tracked files searched in one scan, so a monorepo degrades
/// into a partial answer rather than a hang.
const MAX_FILES_SCANNED: usize = 5_000;

/// How much of a file to inspect for NUL bytes before deciding it is binary.
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// One problem found in one repository. Contains no secret values — see the
/// module docs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeakFinding {
    /// `trackedEnvFile`, `trackedValue` or `unignoredEnvFile`.
    pub kind: String,
    /// `critical` (already in git) or `warning` (not yet, but unprotected).
    pub severity: String,
    /// Repository-relative path, as git reports it.
    pub path: String,
    /// 1-based line number, for a value found inside a file.
    pub line: Option<i64>,
    /// The vault key whose value was found — never the value itself.
    pub key: Option<String>,
    pub repo_name: Option<String>,
    pub env_name: Option<String>,
    pub detail: String,
    /// A `.gitignore` pattern that would prevent this being committed again,
    /// when one applies.
    pub fix_pattern: Option<String>,
    /// Whether the secret must be treated as compromised. True for anything
    /// already tracked: `.gitignore` cannot undo history.
    pub needs_rotation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeakReport {
    /// The directory the caller asked about.
    pub path: String,
    /// The repository root git resolved `path` to, if it is inside one.
    pub git_root: Option<String>,
    /// Why this scan found nothing, when the reason was something other than
    /// "the repository is clean" — not a git repository, git missing, or the
    /// file cap hit.
    pub note: Option<String>,
    pub files_scanned: i64,
    pub findings: Vec<LeakFinding>,
}

impl LeakReport {
    /// Whether anything needs the user's attention — what `vault scan` exits
    /// non-zero on.
    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }

    /// The distinct `.gitignore` patterns that would address these findings,
    /// in the order they were first suggested.
    pub fn suggested_patterns(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.findings
            .iter()
            .filter_map(|f| f.fix_pattern.clone())
            .filter(|p| seen.insert(p.clone()))
            .collect()
    }
}

/// Runs git in `dir`, returning stdout on success. A non-zero exit is `Ok(None)`
/// rather than an error: "this is not a git repository" is an ordinary answer
/// to a scan, not a failure. A missing git binary *is* an error, because then
/// we cannot answer at all and must say so instead of reporting a clean repo.
fn git(dir: &Path, args: &[&str]) -> Result<Option<Vec<u8>>> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => VaultError::Git(
                "git was not found on PATH — the leak guard needs it to see what is committed"
                    .into(),
            ),
            _ => VaultError::Git(e.to_string()),
        })?;
    Ok(output.status.success().then_some(output.stdout))
}

/// Splits git's `-z` output (NUL-separated, no quoting or escaping, which is
/// exactly why every list here asks for it).
fn split_nul(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

fn file_name_of(rel_path: &str) -> &str {
    // git always reports forward slashes, on every platform.
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

/// Whether a file name is a dotenv file: `.env`, `.env.local`, `prod.env`.
fn is_env_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == ".env" || lower.starts_with(".env.") || lower.ends_with(".env")
}

/// Whether a dotenv file is one of the ones that is *supposed* to be
/// committed. Flagging `.env.example` would train the user to dismiss the
/// scanner, which costs more than the finding is worth.
fn is_example_env_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["example", "sample", "template", "dist", "defaults", "schema"]
        .iter()
        .any(|marker| lower.contains(marker))
}

/// A `.gitignore` pattern that covers exactly this file, anchored to the
/// repository root so it cannot accidentally match a same-named file elsewhere.
fn pattern_for(rel_path: &str) -> String {
    format!("/{rel_path}")
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .take(BINARY_SNIFF_BYTES)
        .any(|b| *b == 0)
}

/// A secret to search for, reduced to what a finding is allowed to mention
/// plus the value itself (which stays inside this module).
struct SearchTarget {
    key: String,
    value: String,
    repo_name: String,
    env_name: String,
    /// How many other places in the vault hold this same value, so a finding
    /// can say the match is not unique to one environment.
    other_locations: usize,
}

/// Collapses vault values to one search target each. The same value stored in
/// five environments is one string to look for and one finding to report, not
/// five identical ones pointing at the same line.
fn dedupe_targets(candidates: Vec<(String, String, String, String)>) -> Vec<SearchTarget> {
    let mut out: Vec<SearchTarget> = Vec::new();
    for (key, value, repo_name, env_name) in candidates {
        match out.iter_mut().find(|t| t.value == value) {
            Some(existing) => existing.other_locations += 1,
            None => out.push(SearchTarget {
                key,
                value,
                repo_name,
                env_name,
                other_locations: 0,
            }),
        }
    }
    out
}

impl Vault {
    /// Scans the git repository containing `dir` for secrets from this vault.
    ///
    /// Every value in the vault is searched for, not only the ones belonging to
    /// environments linked to this directory: "your *other* project's
    /// production key is sitting in this repo" is both more alarming and more
    /// useful than the same-project case, and each finding says which repo and
    /// environment the value came from.
    pub fn scan_directory(&self, dir: &Path) -> Result<LeakReport> {
        let path_str = dir.to_string_lossy().into_owned();
        let empty = |note: &str| LeakReport {
            path: path_str.clone(),
            git_root: None,
            note: Some(note.to_string()),
            files_scanned: 0,
            findings: Vec::new(),
        };

        if !dir.is_dir() {
            return Ok(empty("This folder no longer exists."));
        }
        let Some(root_bytes) = git(dir, &["rev-parse", "--show-toplevel"])? else {
            return Ok(empty("Not a git repository — nothing here can be committed by accident."));
        };
        let root = PathBuf::from(String::from_utf8_lossy(&root_bytes).trim());

        let tracked = git(&root, &["ls-files", "-z"])?
            .map(|out| split_nul(&out))
            .unwrap_or_default();
        // `--others --exclude-standard` is untracked-and-not-ignored: exactly
        // the set that a `git add .` would sweep up.
        let untracked_unignored = git(&root, &["ls-files", "--others", "--exclude-standard", "-z"])?
            .map(|out| split_nul(&out))
            .unwrap_or_default();

        let mut findings = Vec::new();

        // (1) Tracked dotenv files. Collected first so their contents can be
        // skipped in (2) -- one finding for the file beats one per line.
        let mut flagged_env_files: HashSet<&str> = HashSet::new();
        for rel in &tracked {
            let name = file_name_of(rel);
            if !is_env_file(name) || is_example_env_file(name) {
                continue;
            }
            flagged_env_files.insert(rel.as_str());
            findings.push(LeakFinding {
                kind: "trackedEnvFile".into(),
                severity: "critical".into(),
                path: rel.clone(),
                line: None,
                key: None,
                repo_name: None,
                env_name: None,
                detail: format!(
                    "{rel} is tracked by git, so it is in the repository's history. Adding it to \
                     .gitignore stops future commits but does not remove it: untrack it with \
                     `git rm --cached {rel}` and treat every value in it as compromised."
                ),
                fix_pattern: Some(pattern_for(rel)),
                needs_rotation: true,
            });
        }

        // (3) Unprotected dotenv files, reported before the content scan so the
        // cheap findings are present even if the scan hits its file cap.
        for rel in &untracked_unignored {
            let name = file_name_of(rel);
            if !is_env_file(name) || is_example_env_file(name) {
                continue;
            }
            findings.push(LeakFinding {
                kind: "unignoredEnvFile".into(),
                severity: "warning".into(),
                path: rel.clone(),
                line: None,
                key: None,
                repo_name: None,
                env_name: None,
                detail: format!(
                    "{rel} is not committed yet, but .gitignore does not cover it — the next \
                     `git add .` would commit it."
                ),
                fix_pattern: Some(pattern_for(rel)),
                needs_rotation: false,
            });
        }

        // (2) Vault values inside tracked files.
        let mut targets = dedupe_targets(self.leak_scan_candidates()?);
        let mut files_scanned = 0i64;
        let mut hit_cap = false;

        for rel in &tracked {
            if targets.is_empty() {
                break;
            }
            if flagged_env_files.contains(rel.as_str()) {
                continue;
            }
            if files_scanned as usize >= MAX_FILES_SCANNED {
                hit_cap = true;
                break;
            }
            let full = root.join(rel);
            let Ok(meta) = std::fs::metadata(&full) else { continue };
            if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
                continue;
            }
            let Ok(bytes) = std::fs::read(&full) else { continue };
            if looks_binary(&bytes) {
                continue;
            }
            let Ok(text) = String::from_utf8(bytes) else { continue };
            files_scanned += 1;

            // Each secret is reported once per scan: the first place it turns
            // up is enough to act on, and a value repeated through a lock file
            // should not produce hundreds of identical rows.
            let mut found_here: Vec<usize> = Vec::new();
            for (line_no, line) in text.lines().enumerate() {
                for (i, target) in targets.iter().enumerate() {
                    if found_here.contains(&i) || !line.contains(&target.value) {
                        continue;
                    }
                    found_here.push(i);
                    let shared = match target.other_locations {
                        0 => String::new(),
                        1 => " (also stored in 1 other environment)".to_string(),
                        n => format!(" (also stored in {n} other environments)"),
                    };
                    findings.push(LeakFinding {
                        kind: "trackedValue".into(),
                        severity: "critical".into(),
                        path: rel.clone(),
                        line: Some(line_no as i64 + 1),
                        key: Some(target.key.clone()),
                        repo_name: Some(target.repo_name.clone()),
                        env_name: Some(target.env_name.clone()),
                        detail: format!(
                            "The value of {} from {}/{} appears in {rel} line {}, which git \
                             tracks{shared}. It is in the repository's history: rotate this \
                             credential and remove it from the file.",
                            target.key,
                            target.repo_name,
                            target.env_name,
                            line_no + 1,
                        ),
                        // A value pasted into a source file is not fixed by
                        // ignoring that file -- the file belongs in git.
                        fix_pattern: None,
                        needs_rotation: true,
                    });
                }
            }
            found_here.sort_unstable();
            for i in found_here.into_iter().rev() {
                targets.remove(i);
            }
        }

        Ok(LeakReport {
            path: path_str,
            git_root: Some(root.to_string_lossy().into_owned()),
            note: hit_cap.then(|| {
                format!("Stopped after {MAX_FILES_SCANNED} files — this repository is large, so \
                         the value search may be incomplete.")
            }),
            files_scanned,
            findings,
        })
    }

    /// Scans every directory linked with `vault link`. A folder that is not a
    /// git repository still gets a report (with a note and no findings), so
    /// the panel can show that it was looked at rather than silently omitting
    /// it.
    pub fn scan_linked_projects(&self) -> Result<Vec<LeakReport>> {
        let mut out = Vec::new();
        for project in self.list_projects()? {
            out.push(self.scan_directory(Path::new(&project.path))?);
        }
        Ok(out)
    }
}

/// Appends `patterns` to `git_root`'s `.gitignore`, skipping any it already
/// contains, and returns how many were added. Idempotent: running it twice
/// adds nothing the second time.
///
/// This prevents future commits. It does not untrack anything already in git —
/// see the module docs.
pub fn apply_gitignore_patterns(git_root: &Path, patterns: &[String]) -> Result<usize> {
    if patterns.is_empty() {
        return Ok(0);
    }
    let gitignore = git_root.join(".gitignore");
    let existing = match std::fs::read_to_string(&gitignore) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let already: HashSet<&str> = existing.lines().map(str::trim).collect();

    let mut to_add: Vec<&String> = Vec::new();
    for pattern in patterns {
        // An existing unanchored `.env` already covers the anchored `/.env`
        // we would otherwise append, so treat either spelling as coverage.
        let unanchored = pattern.trim_start_matches('/');
        if already.contains(pattern.as_str()) || already.contains(unanchored) {
            continue;
        }
        if !to_add.contains(&pattern) {
            to_add.push(pattern);
        }
    }
    if to_add.is_empty() {
        return Ok(0);
    }

    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str("# Added by Vault-R (git leak guard)\n");
    for pattern in &to_add {
        out.push_str(pattern);
        out.push('\n');
    }
    std::fs::write(&gitignore, out)?;
    Ok(to_add.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotenv_files_are_recognized_by_name() {
        for name in [".env", ".env.local", ".env.production", "prod.env", ".ENV"] {
            assert!(is_env_file(name), "{name} should be a dotenv file");
        }
        for name in ["env.ts", "environment.rb", "README.md", ".envrc"] {
            assert!(!is_env_file(name), "{name} should not be a dotenv file");
        }
    }

    #[test]
    fn committable_dotenv_templates_are_excluded() {
        for name in [".env.example", ".env.sample", ".env.template", ".env.dist"] {
            assert!(is_env_file(name));
            assert!(is_example_env_file(name), "{name} should be excluded");
        }
        assert!(!is_example_env_file(".env.local"));
    }

    #[test]
    fn a_path_yields_an_anchored_pattern() {
        assert_eq!(pattern_for(".env"), "/.env");
        assert_eq!(pattern_for("apps/api/.env"), "/apps/api/.env");
    }

    #[test]
    fn identical_values_collapse_to_one_search_target() {
        let candidates = vec![
            ("API_KEY".into(), "sk_live_abcdef123456".into(), "api".into(), "local".into()),
            ("API_KEY".into(), "sk_live_abcdef123456".into(), "api".into(), "staging".into()),
            ("OTHER".into(), "different-value-here".into(), "web".into(), "local".into()),
        ];
        let targets = dedupe_targets(candidates);
        assert_eq!(targets.len(), 2);
        let api = targets.iter().find(|t| t.key == "API_KEY").unwrap();
        assert_eq!(api.other_locations, 1);
    }

    #[test]
    fn binary_content_is_detected_by_nul_bytes() {
        assert!(looks_binary(b"MZ\x00\x00binary"));
        assert!(!looks_binary(b"KEY=value\nOTHER=thing\n"));
    }

    #[test]
    fn gitignore_patterns_are_added_once_and_preserve_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "node_modules/\ntarget/\n").unwrap();

        let patterns = vec!["/.env".to_string(), "/apps/api/.env".to_string()];
        assert_eq!(apply_gitignore_patterns(dir.path(), &patterns).unwrap(), 2);

        let text = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(text.starts_with("node_modules/\ntarget/\n"));
        assert!(text.contains("/.env"));
        assert!(text.contains("/apps/api/.env"));

        // second run is a no-op
        assert_eq!(apply_gitignore_patterns(dir.path(), &patterns).unwrap(), 0);
        assert_eq!(std::fs::read_to_string(dir.path().join(".gitignore")).unwrap(), text);
    }

    #[test]
    fn an_existing_unanchored_rule_counts_as_coverage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), ".env\n").unwrap();
        assert_eq!(
            apply_gitignore_patterns(dir.path(), &[".env".to_string()]).unwrap(),
            0
        );
        // the anchored form of a rule already present unanchored is redundant
        assert_eq!(
            apply_gitignore_patterns(dir.path(), &["/.env".to_string()]).unwrap(),
            0
        );
    }

    #[test]
    fn a_missing_gitignore_is_created() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            apply_gitignore_patterns(dir.path(), &["/.env".to_string()]).unwrap(),
            1
        );
        let text = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(text.contains("/.env"));
    }
}
