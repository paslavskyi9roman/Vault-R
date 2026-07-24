import { invoke } from '@tauri-apps/api/core';

export interface Repo {
  id: string;
  name: string;
  sortOrder: number;
}

export interface Environment {
  id: string;
  repoId: string;
  name: string;
}

export interface EnvironmentSummary {
  id: string;
  repoId: string;
  name: string;
  varCount: number;
}

export interface RepoSummary {
  id: string;
  name: string;
  sortOrder: number;
  envs: EnvironmentSummary[];
}

export interface Variable {
  id: string;
  envId: string;
  key: string;
  value: string;
  groupId: string | null;
  description: string | null;
  required: boolean;
  rotateAfterDays: number | null;
}

export interface VariableWithUsage extends Variable {
  groupUsage: number;
}

export interface Snapshot {
  id: string;
  envId: string;
  createdAt: string;
  summary: string;
  payload: string;
}

export interface GroupMember {
  variable: Variable;
  repoName: string;
  envName: string;
}

export interface SearchResult {
  kind: 'repo' | 'environment' | 'variable';
  label: string;
  sublabel: string;
  repoId: string;
  envId: string | null;
}

export interface SnapshotWithStats extends Snapshot {
  added: number;
  removed: number;
  changed: number;
}

export interface DiffRow {
  key: string;
  kind: 'added' | 'removed' | 'changed';
  oldValue: string | null;
  newValue: string | null;
}

export interface BackupInfo {
  path: string;
  createdAt: string;
  bytes: number;
}

export interface UnlinkedMatch {
  key: string;
  varA: Variable;
  varB: Variable;
}

/// One problem the git leak guard found. Deliberately carries key names and
/// file locations but never secret values — the panel that renders these is
/// meant to be safe to screenshot.
export interface LeakFinding {
  kind: 'trackedEnvFile' | 'trackedValue' | 'unignoredEnvFile';
  severity: 'critical' | 'warning';
  path: string;
  line: number | null;
  key: string | null;
  repoName: string | null;
  envName: string | null;
  detail: string;
  fixPattern: string | null;
  /// True when the secret is already in git history, which .gitignore cannot
  /// undo — the value has to be rotated.
  needsRotation: boolean;
}

export interface LeakReport {
  path: string;
  gitRoot: string | null;
  note: string | null;
  filesScanned: number;
  findings: LeakFinding[];
}

export interface HealthIssue {
  kind: 'empty' | 'placeholder' | 'stale' | 'rotationDue';
  severity: 'critical' | 'warning';
  detail: string;
}

export interface SecretHealthRow {
  varId: string;
  key: string;
  envId: string;
  repoName: string;
  envName: string;
  updatedAt: string;
  ageDays: number;
  rotateAfterDays: number | null;
  issues: HealthIssue[];
}

export interface DuplicateLocation {
  varId: string;
  key: string;
  envId: string;
  repoName: string;
  envName: string;
}

export interface DuplicateValueGroup {
  /// Empty when the same value is stored under different key names.
  key: string;
  locations: DuplicateLocation[];
}

export interface HealthReport {
  rows: SecretHealthRow[];
  duplicates: DuplicateValueGroup[];
  totalSecrets: number;
  emptyCount: number;
  placeholderCount: number;
  staleCount: number;
  rotationDueCount: number;
}

export interface ProjectInfo {
  id: string;
  path: string;
  envId: string;
  repoName: string;
  envName: string;
  createdAt: string;
}

/// What the app can see of the vault on disk before anything is unlocked.
export interface VaultStatus {
  dir: string;
  fileName: string;
  exists: boolean;
  format: number | null;
  bytes: number;
  modifiedMs: number | null;
  backupCount: number;
}

export const api = {
  vaultExists: () => invoke<boolean>('vault_exists'),
  vaultStatus: () => invoke<VaultStatus>('vault_status'),
  vaultTryKeychain: () => invoke<boolean>('vault_try_keychain'),
  vaultCreate: (password: string, remember: boolean) =>
    invoke<void>('vault_create', { password, remember }),
  vaultUnlock: (password: string, remember: boolean) =>
    invoke<void>('vault_unlock', { password, remember }),
  vaultUnlockWithRecovery: (code: string) => invoke<void>('vault_unlock_with_recovery', { code }),
  vaultLock: () => invoke<void>('vault_lock'),
  notifyActivity: () => invoke<void>('notify_activity'),
  vaultNeedsMigration: () => invoke<boolean>('vault_needs_migration'),
  vaultChangePassword: (currentPassword: string, newPassword: string, remember: boolean) =>
    invoke<void>('vault_change_password', { currentPassword, newPassword, remember }),
  vaultResetPassword: (newPassword: string) => invoke<void>('vault_reset_password', { newPassword }),
  vaultHasRecoveryCode: () => invoke<boolean>('vault_has_recovery_code'),
  vaultGenerateRecoveryCode: () => invoke<string>('vault_generate_recovery_code'),

  saveRecoveryKit: (path: string) => invoke<void>('save_recovery_kit', { path }),
  listBackups: () => invoke<BackupInfo[]>('list_backups'),
  exportBackup: (path: string) => invoke<void>('export_backup', { path }),
  restoreBackup: (path: string) => invoke<void>('restore_backup', { path }),

  listRepoSummaries: () => invoke<RepoSummary[]>('list_repo_summaries'),
  createRepo: (name: string) => invoke<Repo>('create_repo', { name }),
  renameRepo: (id: string, newName: string) => invoke<void>('rename_repo', { id, newName }),
  deleteRepo: (id: string) => invoke<void>('delete_repo', { id }),
  createEnvironment: (repoId: string, name: string) =>
    invoke<Environment>('create_environment', { repoId, name }),
  renameEnvironment: (id: string, newName: string) =>
    invoke<void>('rename_environment', { id, newName }),
  deleteEnvironment: (id: string) => invoke<void>('delete_environment', { id }),
  duplicateEnvironment: (envId: string, newName: string, copyValues: boolean) =>
    invoke<Environment>('duplicate_environment', { envId, newName, copyValues }),

  listVariablesWithUsage: (envId: string) =>
    invoke<VariableWithUsage[]>('list_variables_with_usage', { envId }),
  addVariable: (envId: string, key: string, value: string) =>
    invoke<Variable>('add_variable', { envId, key, value }),
  updateVariableValue: (varId: string, newValue: string) =>
    invoke<void>('update_variable_value', { varId, newValue }),
  renameVariableKey: (varId: string, newKey: string) =>
    invoke<void>('rename_variable_key', { varId, newKey }),
  deleteVariable: (varId: string) => invoke<void>('delete_variable', { varId }),
  setVariableMetadata: (
    varId: string,
    description: string | null,
    required: boolean,
    rotateAfterDays: number | null,
  ) => invoke<void>('set_variable_metadata', { varId, description, required, rotateAfterDays }),
  deleteVariables: (varIds: string[]) => invoke<void>('delete_variables', { varIds }),
  moveVariables: (varIds: string[], targetEnvId: string) =>
    invoke<void>('move_variables', { varIds, targetEnvId }),

  linkCandidates: (varId: string) => invoke<GroupMember[]>('link_candidates', { varId }),
  linkVariables: (varIds: string[]) => invoke<string>('link_variables', { varIds }),
  unlinkVariable: (varId: string) => invoke<void>('unlink_variable', { varId }),
  groupMembers: (groupId: string) => invoke<GroupMember[]>('group_members', { groupId }),
  linkedGroupCount: () => invoke<number>('linked_group_count'),

  search: (query: string) => invoke<SearchResult[]>('search', { query }),

  diffEnvironments: (envA: string, envB: string) =>
    invoke<DiffRow[]>('diff_environments', { envA, envB }),
  copyKeyToEnv: (sourceEnvId: string, targetEnvId: string, key: string) =>
    invoke<void>('copy_key_to_env', { sourceEnvId, targetEnvId, key }),
  copyMissingToEnv: (sourceEnvId: string, targetEnvId: string) =>
    invoke<number>('copy_missing_to_env', { sourceEnvId, targetEnvId }),
  unlinkedIdenticalPairs: (envA: string, envB: string) =>
    invoke<UnlinkedMatch[]>('unlinked_identical_pairs', { envA, envB }),

  linkProject: (path: string, envId: string) => invoke<void>('link_project', { path, envId }),
  unlinkProject: (path: string) => invoke<void>('unlink_project', { path }),
  listProjects: () => invoke<ProjectInfo[]>('list_projects'),

  importEnvText: (envId: string, text: string) => invoke<number>('import_env_text', { envId, text }),
  exportEnvText: (envId: string) => invoke<string>('export_env_text', { envId }),
  exportEnvToFile: (envId: string, path: string) =>
    invoke<void>('export_env_to_file', { envId, path }),

  listSnapshots: (envId: string) => invoke<Snapshot[]>('list_snapshots', { envId }),
  listSnapshotsWithStats: (envId: string) =>
    invoke<SnapshotWithStats[]>('list_snapshots_with_stats', { envId }),
  diffSnapshot: (snapshotId: string, against: 'previous' | 'current') =>
    invoke<DiffRow[]>('diff_snapshot', { snapshotId, against }),
  restoreVariableFromSnapshot: (snapshotId: string, key: string) =>
    invoke<void>('restore_variable_from_snapshot', { snapshotId, key }),
  restoreSnapshot: (snapshotId: string) => invoke<void>('restore_snapshot', { snapshotId }),

  getMeta: (key: string) => invoke<string | null>('get_meta', { key }),
  setMeta: (key: string, value: string) => invoke<void>('set_meta', { key, value }),

  copySecretToClipboard: (text: string) => invoke<void>('copy_secret_to_clipboard', { text }),

  generateSecret: (kind: GeneratorKind, length: number) =>
    invoke<string>('generate_secret', { kind, length }),

  scanLinkedProjects: () => invoke<LeakReport[]>('scan_linked_projects'),
  scanDirectory: (path: string) => invoke<LeakReport>('scan_directory', { path }),
  applyGitignorePatterns: (gitRoot: string, patterns: string[]) =>
    invoke<number>('apply_gitignore_patterns', { gitRoot, patterns }),
  healthReport: () => invoke<HealthReport>('health_report'),
};

export type GeneratorKind = 'hex' | 'base64' | 'alnum' | 'words';
