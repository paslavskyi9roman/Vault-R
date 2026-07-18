//! Secret health: what is already wrong with the secrets in the vault.
//!
//! Everything here is read-only analysis. It answers questions the user did
//! not ask — which values are placeholders that will fail in production, which
//! have not been touched in months, which are past a rotation policy they set
//! themselves, and which identical values are duplicated across environments
//! without being linked.
//!
//! **A report never contains a secret value.** Rows carry keys, locations and
//! ages; duplicate groups carry locations and variable ids. This is the same
//! rule the [leak guard](crate::gitguard) follows and for the same reason: a
//! health report is something a user will screenshot.

use crate::db::Vault;
use crate::error::Result;
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How long a value may go untouched before it is reported as stale. A
/// generic backstop for variables with no explicit `rotate_after_days`
/// policy; long enough that a quarterly-rotated credential does not nag.
pub const STALE_AFTER_DAYS: i64 = 90;

/// Values shorter than this are too generic to reason about: a port number, a
/// log level or a boolean is not a secret, and treating one as a duplicate or
/// a leak candidate produces noise that trains users to ignore the panel.
const MIN_MEANINGFUL_LENGTH: usize = 8;

/// Values that are obviously not real credentials. Compared case-insensitively
/// against the trimmed value. Deliberately a curated list rather than anything
/// clever: a false "this is a placeholder" on a real secret is worse than a
/// miss, because it tells the user a live credential is safe to ignore.
const PLACEHOLDER_VALUES: &[&str] = &[
    "changeme", "change_me", "change-me", "change me", "todo", "tbd", "fixme", "xxx", "xxxx",
    "xxxxx", "placeholder", "yourkey", "your-key", "your_key", "yourapikey", "your-api-key",
    "your_api_key", "yoursecret", "your-secret", "your_secret", "secret", "password", "passwd",
    "pass", "123", "1234", "12345", "123456", "1234567", "12345678", "test", "testing", "example",
    "sample", "dummy", "foo", "bar", "baz", "abc", "abc123", "none", "null", "nil", "undefined",
    "na", "n/a", "insert", "key", "value", "string", "replace", "replaceme", "unset",
];

/// Whether `value` looks like something a developer left behind rather than a
/// real credential. Empty values are *not* placeholders — they get their own,
/// more definite issue.
pub fn is_placeholder_value(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_lowercase();
    if PLACEHOLDER_VALUES.contains(&lower.as_str()) {
        return true;
    }
    // `<your api key here>`: anything angle-bracketed is a fill-in-the-blank.
    if lower.starts_with('<') && lower.ends_with('>') {
        return true;
    }
    if lower.starts_with("your") || lower.starts_with("changeme") || lower.starts_with("replace") {
        return true;
    }
    // A run of one character (`xxxxxxxx`, `00000000`) is never a real secret.
    let mut chars = lower.chars();
    if let Some(first) = chars.next() {
        if lower.chars().count() >= 3 && chars.all(|c| c == first) {
            return true;
        }
    }
    false
}

/// Length past which an all-lowercase value stops looking like a
/// configuration word and starts looking like a real (if unusually plain)
/// credential — a generated passphrase, say.
const WORD_LIKE_MAX_LENGTH: usize = 20;

/// Whether a value is just a lowercase word: `production`, `development`,
/// `localhost`, `postgres`. Real credentials essentially always carry a digit,
/// a capital or punctuation, and a value that does not is far more likely to
/// be a setting than a secret. Without this, `NODE_ENV=production` matches
/// every source file that mentions the word, which is exactly the noise that
/// gets a scanner switched off for good.
fn is_word_like(value: &str) -> bool {
    value.len() < WORD_LIKE_MAX_LENGTH && value.chars().all(|c| c.is_ascii_lowercase())
}

/// Whether a value is substantial enough to be worth comparing against other
/// values or searching source files for. Shared with the leak guard so both
/// features stay quiet about the same trivia.
pub(crate) fn is_meaningful_value(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < MIN_MEANINGFUL_LENGTH
        || is_placeholder_value(trimmed)
        || is_word_like(trimmed)
    {
        return false;
    }
    // Four distinct characters rules out padding and repeated runs that got
    // past the placeholder list.
    let mut distinct: Vec<char> = trimmed.chars().collect();
    distinct.sort_unstable();
    distinct.dedup();
    distinct.len() >= 4
}

/// One thing wrong with one variable. `kind` is `empty`, `placeholder`,
/// `stale` or `rotationDue`; `severity` is `critical` or `warning`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthIssue {
    pub kind: String,
    pub severity: String,
    pub detail: String,
}

/// A variable with at least one issue, and where to find it. Carries no value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretHealthRow {
    pub var_id: String,
    pub key: String,
    pub env_id: String,
    pub repo_name: String,
    pub env_name: String,
    pub updated_at: String,
    pub age_days: i64,
    pub rotate_after_days: Option<i64>,
    pub issues: Vec<HealthIssue>,
}

/// Where one member of a duplicate-value group lives.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateLocation {
    pub var_id: String,
    pub key: String,
    pub env_id: String,
    pub repo_name: String,
    pub env_name: String,
}

/// Two or more variables holding the identical value without being linked —
/// a suggestion to link them, which is also how most users discover that
/// linked secrets exist at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateValueGroup {
    /// The shared key, when every location uses the same one; empty when the
    /// same value is stored under different names (worth linking anyway, and
    /// worth seeing).
    pub key: String,
    pub locations: Vec<DuplicateLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    /// Only variables with at least one issue; a clean vault reports none.
    pub rows: Vec<SecretHealthRow>,
    pub duplicates: Vec<DuplicateValueGroup>,
    /// Denominator for the counts, so the panel can say "3 of altogether 48".
    pub total_secrets: i64,
    pub empty_count: i64,
    pub placeholder_count: i64,
    pub stale_count: i64,
    pub rotation_due_count: i64,
}

/// One variable as the report needs it, including the value — which is used
/// to classify and to group duplicates, and is dropped before anything is
/// returned to a caller.
struct Candidate {
    var_id: String,
    key: String,
    value: String,
    env_id: String,
    repo_name: String,
    env_name: String,
    group_id: Option<String>,
    updated_at: String,
    rotate_after_days: Option<i64>,
}

fn age_in_days(updated_at: &str, now: DateTime<Utc>) -> i64 {
    match DateTime::parse_from_rfc3339(updated_at) {
        // A clock that moved backwards (or a restored backup from a machine
        // whose clock was ahead) would otherwise report a negative age and
        // read as "updated in the future"; clamp instead.
        Ok(then) => (now - then.with_timezone(&Utc)).num_days().max(0),
        Err(_) => 0,
    }
}

impl Vault {
    /// Health of every secret in the vault.
    pub fn health_report(&self) -> Result<HealthReport> {
        self.health_report_scoped(None)
    }

    /// Health of one environment's secrets. Duplicate groups are still found
    /// vault-wide — the counterpart of a duplicate is by definition somewhere
    /// else — but only groups touching this environment are reported.
    pub fn health_report_for_env(&self, env_id: &str) -> Result<HealthReport> {
        self.health_report_scoped(Some(env_id))
    }

    fn health_report_scoped(&self, env_id: Option<&str>) -> Result<HealthReport> {
        let candidates = self.health_candidates()?;
        let now = Utc::now();

        let in_scope = |c: &Candidate| env_id.is_none_or(|id| c.env_id == id);

        let mut rows = Vec::new();
        let (mut empty_count, mut placeholder_count) = (0, 0);
        let (mut stale_count, mut rotation_due_count) = (0, 0);
        let mut total_secrets = 0;

        for c in candidates.iter().filter(|c| in_scope(c)) {
            total_secrets += 1;
            let age_days = age_in_days(&c.updated_at, now);
            let mut issues = Vec::new();

            if c.value.trim().is_empty() {
                empty_count += 1;
                issues.push(HealthIssue {
                    kind: "empty".into(),
                    severity: "critical".into(),
                    detail: "This variable has no value.".into(),
                });
            } else if is_placeholder_value(&c.value) {
                placeholder_count += 1;
                issues.push(HealthIssue {
                    kind: "placeholder".into(),
                    severity: "critical".into(),
                    detail: "This looks like a placeholder rather than a real value.".into(),
                });
            }

            match c.rotate_after_days {
                Some(limit) if age_days >= limit => {
                    rotation_due_count += 1;
                    issues.push(HealthIssue {
                        kind: "rotationDue".into(),
                        severity: "critical".into(),
                        detail: format!(
                            "Rotation is due: last changed {age_days} days ago, policy is every \
                             {limit}."
                        ),
                    });
                }
                // An explicit policy replaces the generic staleness backstop:
                // a credential rotated every 180 days by design is not stale
                // at 100 days just because the default says so.
                Some(_) => {}
                None if age_days >= STALE_AFTER_DAYS && !c.value.trim().is_empty() => {
                    stale_count += 1;
                    issues.push(HealthIssue {
                        kind: "stale".into(),
                        severity: "warning".into(),
                        detail: format!("Unchanged for {age_days} days."),
                    });
                }
                None => {}
            }

            if !issues.is_empty() {
                rows.push(SecretHealthRow {
                    var_id: c.var_id.clone(),
                    key: c.key.clone(),
                    env_id: c.env_id.clone(),
                    repo_name: c.repo_name.clone(),
                    env_name: c.env_name.clone(),
                    updated_at: c.updated_at.clone(),
                    age_days,
                    rotate_after_days: c.rotate_after_days,
                    issues,
                });
            }
        }

        // Critical issues first, then oldest, so the panel opens on what
        // actually matters rather than on whatever was inserted first.
        rows.sort_by(|a, b| {
            let severity = |r: &SecretHealthRow| {
                if r.issues.iter().any(|i| i.severity == "critical") { 0 } else { 1 }
            };
            severity(a)
                .cmp(&severity(b))
                .then(b.age_days.cmp(&a.age_days))
                .then(a.key.cmp(&b.key))
        });

        let duplicates = duplicate_groups(&candidates, env_id);

        Ok(HealthReport {
            rows,
            duplicates,
            total_secrets,
            empty_count,
            placeholder_count,
            stale_count,
            rotation_due_count,
        })
    }

    fn health_candidates(&self) -> Result<Vec<Candidate>> {
        let mut stmt = self.conn.prepare(
            "SELECT v.id, v.key, v.value, v.env_id, v.group_id, v.updated_at,
                    v.rotate_after_days, r.name, e.name
             FROM variables v
             JOIN environments e ON e.id = v.env_id
             JOIN repos r ON r.id = e.repo_id
             ORDER BY r.name, e.name, v.key",
        )?;
        let rows = stmt.query_map(params![], |r| {
            Ok(Candidate {
                var_id: r.get(0)?,
                key: r.get(1)?,
                value: r.get(2)?,
                env_id: r.get(3)?,
                group_id: r.get(4)?,
                updated_at: r.get(5)?,
                rotate_after_days: r.get(6)?,
                repo_name: r.get(7)?,
                env_name: r.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Every value in the vault worth searching source files for, as
    /// `(key, value, repo, env)`. Used by the leak guard; trivial and
    /// placeholder values are filtered out here so both features agree on
    /// what counts as a secret.
    pub(crate) fn leak_scan_candidates(&self) -> Result<Vec<(String, String, String, String)>> {
        Ok(self
            .health_candidates()?
            .into_iter()
            .filter(|c| is_meaningful_value(&c.value))
            .map(|c| (c.key, c.value, c.repo_name, c.env_name))
            .collect())
    }
}

/// Groups unlinked variables that hold the identical value. Variables already
/// in a link group are excluded entirely — not just from each other's groups —
/// because the answer for them is already "yes, these are deliberately the
/// same".
fn duplicate_groups(candidates: &[Candidate], env_id: Option<&str>) -> Vec<DuplicateValueGroup> {
    let mut by_value: HashMap<&str, Vec<&Candidate>> = HashMap::new();
    for c in candidates {
        if c.group_id.is_some() || !is_meaningful_value(&c.value) {
            continue;
        }
        by_value.entry(c.value.as_str()).or_default().push(c);
    }

    let mut out: Vec<DuplicateValueGroup> = by_value
        .into_values()
        .filter(|members| members.len() >= 2)
        .filter(|members| env_id.is_none_or(|id| members.iter().any(|c| c.env_id == id)))
        .map(|members| {
            let first_key = members[0].key.as_str();
            let shared_key = members.iter().all(|c| c.key == first_key);
            DuplicateValueGroup {
                key: if shared_key { first_key.to_string() } else { String::new() },
                locations: members
                    .iter()
                    .map(|c| DuplicateLocation {
                        var_id: c.var_id.clone(),
                        key: c.key.clone(),
                        env_id: c.env_id.clone(),
                        repo_name: c.repo_name.clone(),
                        env_name: c.env_name.clone(),
                    })
                    .collect(),
            }
        })
        .collect();

    // HashMap iteration order is arbitrary; sort so the panel does not
    // reshuffle itself between two scans of an unchanged vault.
    out.sort_by(|a, b| {
        b.locations
            .len()
            .cmp(&a.locations.len())
            .then(a.key.cmp(&b.key))
            .then(a.locations[0].var_id.cmp(&b.locations[0].var_id))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obvious_placeholders_are_recognized() {
        for v in [
            "changeme",
            "CHANGEME",
            "  change_me  ",
            "your-api-key",
            "<insert key here>",
            "xxxxxxxx",
            "123456",
            "TODO",
            "replace-me-please",
        ] {
            assert!(is_placeholder_value(v), "{v} should be a placeholder");
        }
    }

    #[test]
    fn real_looking_values_are_not_placeholders() {
        for v in [
            "sk_live_51H8xQ2eZvKYlo2C",
            "postgres://user:hunter2@db.internal:5432/app",
            "a3f9c1e7b2d84f60",
            // deliberately awkward: contains a placeholder word but is not one
            "test-account-key-9f2b1c",
        ] {
            assert!(!is_placeholder_value(v), "{v} should not be a placeholder");
        }
    }

    #[test]
    fn an_empty_value_is_not_a_placeholder() {
        // Empty gets its own, more definite issue -- classifying it as a
        // placeholder too would double-count it in the panel's tallies.
        assert!(!is_placeholder_value(""));
        assert!(!is_placeholder_value("   "));
    }

    #[test]
    fn trivial_values_are_not_worth_comparing_or_searching_for() {
        for v in [
            "3000",
            "true",
            "local",
            "info",
            "",
            "aaaaaaaaaa",
            "changeme",
            // plain configuration words: searching source files for these
            // matches prose, not leaks
            "production",
            "development",
            "localhost",
            "postgres",
        ] {
            assert!(!is_meaningful_value(v), "{v} should not be meaningful");
        }
        for v in [
            "sk_live_51H8xQ2eZvKYlo2C",
            "a3f9c1e7b2d84f60",
            "postgres://user:hunter2@db.internal:5432/app",
            // all lowercase, but long enough to be a generated passphrase
            "correcthorsebatterystaple",
        ] {
            assert!(is_meaningful_value(v), "{v} should be meaningful");
        }
    }

    /// Ages a variable by rewriting `updated_at`, which is the only way to
    /// exercise staleness without waiting 90 days. Lives here rather than in
    /// the integration tests because `conn` is crate-private.
    fn backdate(vault: &Vault, var_id: &str, days: i64) {
        let when = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        vault
            .conn
            .execute(
                "UPDATE variables SET updated_at = ?1 WHERE id = ?2",
                params![when, var_id],
            )
            .unwrap();
    }

    fn seeded_vault(dir: &std::path::Path) -> (Vault, String) {
        let vault = Vault::create_in(dir, "pw").unwrap();
        let repo = vault.create_repo("api-gateway").unwrap();
        let env = vault.create_environment(&repo.id, "local").unwrap();
        (vault, env.id)
    }

    #[test]
    fn empty_and_placeholder_values_are_reported_once_each() {
        let dir = tempfile::tempdir().unwrap();
        let (vault, env_id) = seeded_vault(dir.path());
        vault.add_variable(&env_id, "BLANK", "").unwrap();
        vault.add_variable(&env_id, "STUB", "changeme").unwrap();
        vault
            .add_variable(&env_id, "REAL", "sk_live_9f2b1ce7a4d8")
            .unwrap();

        let report = vault.health_report().unwrap();
        assert_eq!(report.total_secrets, 3);
        assert_eq!(report.empty_count, 1);
        assert_eq!(report.placeholder_count, 1);
        // the healthy one is absent entirely: rows are problems, not an inventory
        assert_eq!(report.rows.len(), 2);
        assert!(report.rows.iter().all(|r| r.key != "REAL"));
    }

    #[test]
    fn staleness_uses_the_default_only_without_an_explicit_policy() {
        let dir = tempfile::tempdir().unwrap();
        let (vault, env_id) = seeded_vault(dir.path());

        let stale = vault.add_variable(&env_id, "OLD", "sk_live_9f2b1ce7a4d8").unwrap();
        backdate(&vault, &stale.id, STALE_AFTER_DAYS + 5);

        // same age, but the owner declared a 180-day cycle: not stale, not due
        let policied = vault.add_variable(&env_id, "SLOW", "sk_live_1a2b3c4d5e6f").unwrap();
        vault
            .set_variable_metadata(&policied.id, None, false, Some(180))
            .unwrap();
        backdate(&vault, &policied.id, STALE_AFTER_DAYS + 5);

        // and one whose own policy has elapsed
        let due = vault.add_variable(&env_id, "DUE", "sk_live_abcdef123456").unwrap();
        vault.set_variable_metadata(&due.id, None, false, Some(30)).unwrap();
        backdate(&vault, &due.id, 31);

        let report = vault.health_report().unwrap();
        assert_eq!(report.stale_count, 1);
        assert_eq!(report.rotation_due_count, 1);

        let by_key = |key: &str| report.rows.iter().find(|r| r.key == key).cloned();
        assert!(by_key("OLD").unwrap().issues.iter().any(|i| i.kind == "stale"));
        assert!(by_key("DUE").unwrap().issues.iter().any(|i| i.kind == "rotationDue"));
        assert!(by_key("SLOW").is_none());
    }

    #[test]
    fn duplicate_detection_ignores_linked_and_trivial_values() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        let repo = vault.create_repo("api-gateway").unwrap();
        let local = vault.create_environment(&repo.id, "local").unwrap();
        let staging = vault.create_environment(&repo.id, "staging").unwrap();

        vault.add_variable(&local.id, "SHARED", "sk_live_9f2b1ce7a4d8").unwrap();
        vault.add_variable(&staging.id, "SHARED", "sk_live_9f2b1ce7a4d8").unwrap();
        // trivial: identical, but nobody wants "3000" linked across environments
        vault.add_variable(&local.id, "PORT", "3000").unwrap();
        vault.add_variable(&staging.id, "PORT", "3000").unwrap();
        // already linked: the question is settled
        let a = vault.add_variable(&local.id, "LINKED", "sk_live_1a2b3c4d5e6f").unwrap();
        let b = vault.add_variable(&staging.id, "LINKED", "sk_live_1a2b3c4d5e6f").unwrap();
        vault.link_variables(&[a.id, b.id]).unwrap();

        let report = vault.health_report().unwrap();
        assert_eq!(report.duplicates.len(), 1);
        assert_eq!(report.duplicates[0].key, "SHARED");
        assert_eq!(report.duplicates[0].locations.len(), 2);
    }

    #[test]
    fn scoping_to_an_environment_still_shows_duplicates_whose_twin_is_elsewhere() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        let repo = vault.create_repo("api-gateway").unwrap();
        let local = vault.create_environment(&repo.id, "local").unwrap();
        let staging = vault.create_environment(&repo.id, "staging").unwrap();
        vault.add_variable(&local.id, "SHARED", "sk_live_9f2b1ce7a4d8").unwrap();
        vault.add_variable(&staging.id, "SHARED", "sk_live_9f2b1ce7a4d8").unwrap();
        vault.add_variable(&staging.id, "ONLY_B", "").unwrap();

        let report = vault.health_report_for_env(&local.id).unwrap();
        // rows are scoped: staging's empty variable is not local's problem
        assert_eq!(report.total_secrets, 1);
        assert_eq!(report.empty_count, 0);
        // but the duplicate is, even though its counterpart lives elsewhere
        assert_eq!(report.duplicates.len(), 1);
    }

    #[test]
    fn an_empty_vault_reports_nothing_rather_than_failing() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        let report = vault.health_report().unwrap();
        assert_eq!(report.total_secrets, 0);
        assert!(report.rows.is_empty());
        assert!(report.duplicates.is_empty());
    }

    #[test]
    fn a_rotation_interval_must_be_at_least_one_day() {
        let dir = tempfile::tempdir().unwrap();
        let (vault, env_id) = seeded_vault(dir.path());
        let v = vault.add_variable(&env_id, "KEY", "sk_live_9f2b1ce7a4d8").unwrap();
        assert!(vault.set_variable_metadata(&v.id, None, false, Some(0)).is_err());
        assert!(vault.set_variable_metadata(&v.id, None, false, Some(-5)).is_err());
        vault.set_variable_metadata(&v.id, None, false, None).unwrap();
    }

    #[test]
    fn age_is_measured_from_the_given_timestamp_and_never_negative() {
        let now = DateTime::parse_from_rfc3339("2026-07-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(age_in_days("2026-04-19T12:00:00Z", now), 90);
        assert_eq!(age_in_days("2026-07-18T11:00:00Z", now), 0);
        // a timestamp from the future clamps to 0 rather than going negative
        assert_eq!(age_in_days("2027-01-01T00:00:00Z", now), 0);
        // an unparseable timestamp must not be reported as ancient
        assert_eq!(age_in_days("not a date", now), 0);
    }
}
