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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub email: String,
    pub role: String,
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
