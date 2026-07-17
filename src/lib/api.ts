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

export interface Member {
  id: string;
  email: string;
  role: string;
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

export const api = {
  vaultExists: () => invoke<boolean>('vault_exists'),
  vaultTryKeychain: () => invoke<boolean>('vault_try_keychain'),
  vaultCreate: (password: string, remember: boolean) =>
    invoke<void>('vault_create', { password, remember }),
  vaultUnlock: (password: string, remember: boolean) =>
    invoke<void>('vault_unlock', { password, remember }),
  vaultLock: () => invoke<void>('vault_lock'),

  listRepoSummaries: () => invoke<RepoSummary[]>('list_repo_summaries'),
  createRepo: (name: string) => invoke<Repo>('create_repo', { name }),
  createEnvironment: (repoId: string, name: string) =>
    invoke<Environment>('create_environment', { repoId, name }),

  listVariablesWithUsage: (envId: string) =>
    invoke<VariableWithUsage[]>('list_variables_with_usage', { envId }),
  addVariable: (envId: string, key: string, value: string) =>
    invoke<Variable>('add_variable', { envId, key, value }),
  updateVariableValue: (varId: string, newValue: string) =>
    invoke<void>('update_variable_value', { varId, newValue }),
  deleteVariable: (varId: string) => invoke<void>('delete_variable', { varId }),

  linkCandidates: (varId: string) => invoke<GroupMember[]>('link_candidates', { varId }),
  linkVariables: (varIds: string[]) => invoke<string>('link_variables', { varIds }),
  unlinkVariable: (varId: string) => invoke<void>('unlink_variable', { varId }),
  groupMembers: (groupId: string) => invoke<GroupMember[]>('group_members', { groupId }),
  linkedGroupCount: () => invoke<number>('linked_group_count'),

  search: (query: string) => invoke<SearchResult[]>('search', { query }),

  importEnvText: (envId: string, text: string) => invoke<number>('import_env_text', { envId, text }),
  exportEnvText: (envId: string) => invoke<string>('export_env_text', { envId }),
  exportEnvToFile: (envId: string, path: string) =>
    invoke<void>('export_env_to_file', { envId, path }),

  listSnapshots: (envId: string) => invoke<Snapshot[]>('list_snapshots', { envId }),
  restoreSnapshot: (snapshotId: string) => invoke<void>('restore_snapshot', { snapshotId }),

  listMembers: () => invoke<Member[]>('list_members'),
  addMember: (email: string, role: string) => invoke<Member>('add_member', { email, role }),
  removeMember: (id: string) => invoke<void>('remove_member', { id }),

  getMeta: (key: string) => invoke<string | null>('get_meta', { key }),
  setMeta: (key: string, value: string) => invoke<void>('set_meta', { key, value }),

  copySecretToClipboard: (text: string) => invoke<void>('copy_secret_to_clipboard', { text }),
};
