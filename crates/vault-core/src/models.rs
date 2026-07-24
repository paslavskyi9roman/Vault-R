use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub repo_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub id: String,
    pub env_id: String,
    pub key: String,
    pub value: String,
    pub group_id: Option<String>,
    /// Free-text documentation for this variable, e.g. "get this from the
    /// Stripe dashboard". Per-environment: it is deliberately not
    /// propagated across a link group on edit, and is vault metadata, not
    /// `.env` content -- it is never emitted by export.
    pub description: Option<String>,
    /// Whether `vault check` should fail when this variable is empty.
    pub required: bool,
    /// How long this value may go untouched before the health panel calls it
    /// due for rotation. `None` -- the default -- means no policy: the panel
    /// still reports a generic staleness warning, but only an explicit policy
    /// produces a "rotation due" issue. Per-environment, like the other
    /// metadata: a link group syncs values, not policies.
    pub rotate_after_days: Option<i64>,
}

/// A [`Variable`] enriched with how many total variables share its link group,
/// for the "linked xN" pill in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableWithUsage {
    #[serde(flatten)]
    pub variable: Variable,
    pub group_usage: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSummary {
    pub id: String,
    pub repo_id: String,
    pub name: String,
    pub var_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSummary {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub envs: Vec<EnvironmentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub env_id: String,
    pub created_at: String,
    pub summary: String,
    pub payload: String,
}

/// One entry in a link group: the variable plus which repo/env it lives in,
/// used to render "same key in other repos/envs" pickers and group popovers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
    pub variable: Variable,
    pub repo_name: String,
    pub env_name: String,
}

/// One row in a snapshot's payload; env restore replaces all variables for
/// that env with a fresh set built from these.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotVariable {
    pub key: String,
    pub value: String,
    pub group_id: Option<String>,
}

/// One key's fate between two points in an environment's history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffRow {
    pub key: String,
    /// `added`, `removed` or `changed`.
    pub kind: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// A [`Snapshot`] with a summary of how much it altered, so the history list
/// can show "+2 -1 ~3" without the caller diffing every entry itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotWithStats {
    #[serde(flatten)]
    pub snapshot: Snapshot,
    pub added: i64,
    pub removed: i64,
    pub changed: i64,
}

/// A same-key, same-value pair found between two environments that is not
/// (yet) a link group -- surfaced by the compare view as a "link these?"
/// suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlinkedMatch {
    pub key: String,
    pub var_a: Variable,
    pub var_b: Variable,
}

/// A directory linked to a repo/environment via `vault link`, so future CLI
/// invocations there can omit the `<repo>/<env>` target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub path: String,
    pub env_id: String,
    pub created_at: String,
}

/// A [`Project`] joined with its repo/environment names, for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub id: String,
    pub path: String,
    pub env_id: String,
    pub repo_name: String,
    pub env_name: String,
    pub created_at: String,
}

/// One match in the command palette: a repo, an environment, or a variable
/// key, anywhere in the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub kind: String,
    pub label: String,
    pub sublabel: String,
    pub repo_id: String,
    pub env_id: Option<String>,
}
