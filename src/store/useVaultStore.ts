import { create } from 'zustand';
import { open, save } from '@tauri-apps/plugin-dialog';
import {
  api,
  type BackupInfo,
  type DiffRow,
  type DuplicateValueGroup,
  type GeneratorKind,
  type GroupMember,
  type HealthReport,
  type LeakReport,
  type Member,
  type ProjectInfo,
  type RepoSummary,
  type SearchResult,
  type SnapshotWithStats,
  type UnlinkedMatch,
  type VariableWithUsage,
  type VaultStatus,
} from '../lib/api';
import { isProtectedEnv } from '../lib/envColor';

export const DEFAULT_AUTO_LOCK_MINUTES = 15;
const AUTO_LOCK_META_KEY = 'auto_lock_minutes';

/// Where a generated secret goes when the user clicks "Generate": the
/// still-unsaved add-row, or an existing variable (which commits immediately).
export type GeneratorTarget = { type: 'add' } | { type: 'row'; varId: string };

/// The two halves of the safety panel: what git can see, and what is wrong
/// inside the vault.
export type SafetyTab = 'leaks' | 'health';

const GENERATOR_DEFAULT_LENGTH: Record<GeneratorKind, number> = {
  hex: 32,
  base64: 32,
  alnum: 24,
  words: 5,
};

let toastTimer: ReturnType<typeof setTimeout> | undefined;

/// A pending destructive action awaiting confirmation. `requireTypedName`, when
/// set, forces the user to retype that exact string before the action unlocks —
/// reserved for deletes that cascade (repos, environments).
export interface ConfirmRequest {
  title: string;
  message: string;
  confirmLabel: string;
  danger: boolean;
  requireTypedName?: string;
  onConfirm: () => Promise<void> | void;
}

interface VaultState {
  // ---- lifecycle ----
  checkingVault: boolean;
  vaultExists: boolean;
  /// Set when the startup check failed: "could not look" is not "no vault".
  initError: string | null;
  vaultStatus: VaultStatus | null;
  /// Lets the user reach the unlock form when the check says there is no vault.
  forceExisting: boolean;
  locked: boolean;
  authBusy: boolean;
  authError: string | null;

  // ---- core data ----
  repos: RepoSummary[];
  activeRepoId: string | null;
  activeEnvId: string | null;
  expandedRepos: Record<string, boolean>;
  variables: VariableWithUsage[];
  varsLoading: boolean;
  revealed: Record<string, boolean>;
  varSearch: string;
  linkedGroupCount: number;
  projects: ProjectInfo[];
  expandedVarId: string | null;
  selectedVarIds: Record<string, boolean>;
  bulkMoveTargetId: string | null;

  // ---- inline add forms ----
  addingRepo: boolean;
  newRepoName: string;
  addingEnvFor: string | null;
  newEnvName: string;
  newVarKey: string;
  newVarValue: string;

  // ---- inline rename (sidebar) ----
  renamingRepoId: string | null;
  renamingEnvId: string | null;
  renameDraft: string;

  // ---- inline duplicate environment (sidebar) ----
  duplicatingEnvId: string | null;
  duplicateNewName: string;
  duplicateCopyValues: boolean;
  duplicateBusy: boolean;

  // ---- confirmation dialog ----
  confirm: ConfirmRequest | null;
  confirmInput: string;
  confirmBusy: boolean;

  // ---- import modal ----
  importOpen: boolean;
  importText: string;

  // ---- share modal ----
  shareOpen: boolean;
  members: Member[];
  inviteEmail: string;

  // ---- history slideover ----
  historyOpen: boolean;
  historySnapshots: SnapshotWithStats[];
  expandedSnapshotId: string | null;
  snapshotDiff: DiffRow[];
  diffRevealed: Record<string, boolean>;

  // ---- settings ----
  settingsOpen: boolean;
  needsMigration: boolean;
  hasRecoveryCode: boolean;
  backups: BackupInfo[];
  autoLockMinutes: number;
  pwCurrent: string;
  pwNew: string;
  pwConfirm: string;
  pwBusy: boolean;
  pwError: string | null;
  /// Shown exactly once, right after generation — the code is not recoverable.
  recoveryCodeOnce: string | null;

  // ---- unlock screen: recovery + restore ----
  recoveryMode: boolean;
  recoveryCode: string;
  /// Set after a recovery unlock: the user must choose a new master password
  /// before they can reach the vault.
  mustResetPassword: boolean;

  /// Environments whose "these are real secrets" warning has been accepted for
  /// this unlocked session, keyed by env id.
  protectedAcknowledged: Record<string, boolean>;

  // ---- command palette ----
  cmdkOpen: boolean;
  cmdkQuery: string;
  cmdkResults: SearchResult[];

  // ---- link / unlink (design gap fill) ----
  linkModalOpen: boolean;
  linkModalVarId: string | null;
  linkModalKey: string;
  linkCandidates: GroupMember[];
  linkSelected: Record<string, boolean>;
  groupPopoverGroupId: string | null;
  groupPopoverMembers: GroupMember[];

  // ---- secret generator popover ----
  generatorOpen: boolean;
  generatorTarget: GeneratorTarget | null;
  generatorKind: GeneratorKind;
  generatorLength: number;
  generatorBusy: boolean;

  // ---- environment compare view ----
  compareOpen: boolean;
  compareEnvBId: string | null;
  compareRows: DiffRow[];
  compareRevealed: Record<string, boolean>;
  compareUnlinkedMatches: UnlinkedMatch[];
  compareBusy: boolean;

  // ---- safety panel: git leak guard + secret health ----
  safetyOpen: boolean;
  safetyTab: SafetyTab;
  leakReports: LeakReport[];
  leakScanning: boolean;
  /// Set once a scan has completed, so an empty result can be rendered as
  /// "clean" rather than as "not scanned yet".
  leakScanned: boolean;
  health: HealthReport | null;
  healthLoading: boolean;

  // ---- onboarding ----
  onboarding: boolean;
  onboardingStep: number;
  onboardingRepoName: string;
  onboardingEnvName: string;

  // ---- toast ----
  toast: string | null;

  // ---- actions ----
  init: () => Promise<void>;
  setForceExisting: (v: boolean) => void;
  createVault: (password: string, remember: boolean) => Promise<void>;
  unlockVault: (password: string, remember: boolean) => Promise<void>;
  lockVault: () => Promise<void>;
  /// Applies the locked UI state when the backend auto-lock enforcer has
  /// already locked the vault (so it must not call `vault_lock` again).
  onBackendLock: () => void;

  refreshRepos: () => Promise<void>;
  refreshVariables: () => Promise<void>;
  loadProjects: () => Promise<void>;
  linkCurrentEnvToFolder: () => Promise<void>;
  unlinkProjectPath: (path: string) => Promise<void>;
  selectEnv: (repoId: string, envId: string | null) => Promise<void>;
  toggleExpandRepo: (repoId: string) => void;

  toggleAddRepo: () => void;
  setNewRepoName: (v: string) => void;
  submitAddRepo: () => Promise<void>;
  startAddEnv: (repoId: string) => void;
  cancelAddEnv: () => void;
  setNewEnvName: (v: string) => void;
  submitAddEnv: (repoId: string) => Promise<void>;

  startRenameRepo: (repoId: string, currentName: string) => void;
  startRenameEnv: (envId: string, currentName: string) => void;
  setRenameDraft: (v: string) => void;
  cancelRename: () => void;
  submitRename: () => Promise<void>;
  requestDeleteRepo: (repoId: string, name: string) => void;
  requestDeleteEnv: (envId: string, repoName: string, envName: string) => void;

  startDuplicateEnv: (envId: string, currentName: string) => void;
  cancelDuplicateEnv: () => void;
  setDuplicateNewName: (v: string) => void;
  toggleDuplicateCopyValues: () => void;
  submitDuplicateEnv: () => Promise<void>;

  requestConfirm: (req: ConfirmRequest) => void;
  setConfirmInput: (v: string) => void;
  cancelConfirm: () => void;
  acceptConfirm: () => Promise<void>;

  setVarSearch: (v: string) => void;
  setNewVarKey: (v: string) => void;
  setNewVarValue: (v: string) => void;
  addVariable: () => Promise<void>;
  commitVariableValue: (varId: string, newValue: string) => Promise<void>;
  commitVariableKey: (varId: string, newKey: string) => Promise<void>;
  deleteVariable: (varId: string, key: string) => Promise<void>;
  toggleVarExpand: (varId: string) => void;
  commitVariableMetadata: (
    varId: string,
    description: string,
    required: boolean,
    rotateAfterDays: number | null,
  ) => Promise<void>;
  toggleReveal: (varId: string) => void;

  toggleVarSelected: (varId: string) => void;
  clearVarSelection: () => void;
  bulkDeleteSelected: () => Promise<void>;
  bulkCopySelectedAsEnvBlock: () => Promise<void>;
  setBulkMoveTarget: (envId: string) => void;
  bulkMoveSelected: () => Promise<void>;
  copyVariable: (value: string, key: string) => Promise<void>;
  copyPlainText: (text: string, label: string) => Promise<void>;

  openImport: () => void;
  closeImport: () => void;
  setImportText: (v: string) => void;
  submitImport: () => Promise<void>;

  exportEnv: () => Promise<void>;

  openHistory: () => Promise<void>;
  closeHistory: () => void;
  toggleSnapshotDiff: (snapshotId: string) => Promise<void>;
  toggleDiffReveal: (key: string) => void;
  requestRestoreSnapshot: (snapshotId: string, timeLabel: string) => Promise<void>;
  restoreVariableFromSnapshot: (snapshotId: string, key: string) => Promise<void>;

  openSettings: () => Promise<void>;
  closeSettings: () => void;
  setPwField: (field: 'pwCurrent' | 'pwNew' | 'pwConfirm', value: string) => void;
  changePassword: (remember: boolean) => Promise<void>;
  generateRecoveryCode: () => Promise<void>;
  dismissRecoveryCode: () => void;
  saveRecoveryCodeToFile: () => Promise<void>;
  exportBackup: () => Promise<void>;
  setAutoLockMinutes: (minutes: number) => Promise<void>;

  setRecoveryMode: (on: boolean) => void;
  setRecoveryCode: (v: string) => void;
  unlockWithRecovery: () => Promise<void>;
  resetPasswordAfterRecovery: (newPassword: string) => Promise<void>;
  restoreBackupFromUnlock: () => Promise<void>;

  openShare: () => Promise<void>;
  closeShare: () => void;
  setInviteEmail: (v: string) => void;
  addMemberAction: () => Promise<void>;
  removeMemberAction: (id: string) => Promise<void>;

  toggleCmdk: () => void;
  closeCmdk: () => void;
  setCmdkQuery: (q: string) => Promise<void>;
  cmdkSelectResult: (r: SearchResult) => Promise<void>;

  openLinkModal: (varId: string, key: string) => Promise<void>;
  closeLinkModal: () => void;
  toggleLinkSelected: (varId: string) => void;
  confirmLink: () => Promise<void>;
  openGroupPopover: (groupId: string) => Promise<void>;
  closeGroupPopover: () => void;
  unlinkFromPopover: (varId: string) => Promise<void>;

  openGenerator: (target: GeneratorTarget) => void;
  closeGenerator: () => void;
  setGeneratorKind: (kind: GeneratorKind) => void;
  setGeneratorLength: (length: number) => void;
  runGenerator: () => Promise<void>;

  openCompare: () => void;
  closeCompare: () => void;
  setCompareEnvB: (envId: string) => Promise<void>;
  refreshCompare: () => Promise<void>;
  toggleCompareReveal: (key: string) => void;
  copyCompareRow: (key: string, direction: 'aToB' | 'bToA') => Promise<void>;
  copyAllMissing: (direction: 'aToB' | 'bToA') => Promise<void>;
  linkCompareMatch: (match: UnlinkedMatch) => Promise<void>;

  openSafety: (tab?: SafetyTab) => Promise<void>;
  closeSafety: () => void;
  setSafetyTab: (tab: SafetyTab) => Promise<void>;
  runLeakScan: () => Promise<void>;
  refreshHealth: () => Promise<void>;
  applyGitignoreFix: (report: LeakReport) => Promise<void>;
  linkDuplicateGroup: (group: DuplicateValueGroup) => Promise<void>;
  jumpToVariable: (envId: string, varId: string) => Promise<void>;

  obNext: () => void;
  obSkip: () => Promise<void>;
  setOnboardingRepoName: (v: string) => void;
  setOnboardingEnvName: (v: string) => void;
  obImportAndNext: () => Promise<void>;
  obFinish: () => Promise<void>;
  replayOnboarding: () => void;

  showToast: (msg: string) => void;
  closeAllOverlays: () => void;
}

export const useVaultStore = create<VaultState>((set, get) => ({
  checkingVault: true,
  vaultExists: false,
  initError: null,
  vaultStatus: null,
  forceExisting: false,
  locked: true,
  authBusy: false,
  authError: null,

  repos: [],
  activeRepoId: null,
  activeEnvId: null,
  expandedRepos: {},
  variables: [],
  varsLoading: false,
  revealed: {},
  varSearch: '',
  linkedGroupCount: 0,
  projects: [],
  expandedVarId: null,
  selectedVarIds: {},
  bulkMoveTargetId: null,

  addingRepo: false,
  newRepoName: '',
  addingEnvFor: null,
  newEnvName: '',
  newVarKey: '',
  newVarValue: '',

  renamingRepoId: null,
  renamingEnvId: null,
  renameDraft: '',

  duplicatingEnvId: null,
  duplicateNewName: '',
  duplicateCopyValues: false,
  duplicateBusy: false,

  confirm: null,
  confirmInput: '',
  confirmBusy: false,

  importOpen: false,
  importText: '',

  shareOpen: false,
  members: [],
  inviteEmail: '',

  historyOpen: false,
  historySnapshots: [],
  expandedSnapshotId: null,
  snapshotDiff: [],
  diffRevealed: {},

  settingsOpen: false,
  needsMigration: false,
  hasRecoveryCode: false,
  backups: [],
  autoLockMinutes: DEFAULT_AUTO_LOCK_MINUTES,
  pwCurrent: '',
  pwNew: '',
  pwConfirm: '',
  pwBusy: false,
  pwError: null,
  recoveryCodeOnce: null,

  recoveryMode: false,
  recoveryCode: '',
  mustResetPassword: false,

  protectedAcknowledged: {},

  cmdkOpen: false,
  cmdkQuery: '',
  cmdkResults: [],

  linkModalOpen: false,
  linkModalVarId: null,
  linkModalKey: '',
  linkCandidates: [],
  linkSelected: {},
  groupPopoverGroupId: null,
  groupPopoverMembers: [],

  generatorOpen: false,
  generatorTarget: null,
  generatorKind: 'hex',
  generatorLength: GENERATOR_DEFAULT_LENGTH.hex,
  generatorBusy: false,

  compareOpen: false,
  compareEnvBId: null,
  compareRows: [],
  compareRevealed: {},
  compareUnlinkedMatches: [],
  compareBusy: false,

  safetyOpen: false,
  safetyTab: 'leaks',
  leakReports: [],
  leakScanning: false,
  leakScanned: false,
  health: null,
  healthLoading: false,

  onboarding: false,
  onboardingStep: 0,
  onboardingRepoName: 'my-project',
  onboardingEnvName: 'local',

  toast: null,

  init: async () => {
    set({ checkingVault: true, initError: null });

    let exists: boolean;
    let status: VaultStatus | null = null;
    try {
      [exists, status] = await Promise.all([api.vaultExists(), api.vaultStatus()]);
    } catch (e) {
      // The lookup failed, so we know nothing about the vault. Offering to
      // create one here is how a recoverable vault looks like a lost one.
      set({ checkingVault: false, initError: String(e) });
      return;
    }
    set({ vaultExists: exists, vaultStatus: status, checkingVault: false });
    if (!exists) return;

    // A keychain that refuses to answer falls through to the password prompt.
    let unlocked = false;
    try {
      unlocked = await api.vaultTryKeychain();
    } catch {
      unlocked = false;
    }
    if (!unlocked) return;
    set({ locked: false });
    await afterUnlock(get);
  },

  setForceExisting: (v) => set({ forceExisting: v, authError: null }),

  createVault: async (password, remember) => {
    set({ authBusy: true, authError: null });
    try {
      await api.vaultCreate(password, remember);
      set({
        locked: false,
        vaultExists: true,
        forceExisting: false,
        onboarding: true,
        onboardingStep: 0,
      });
      await get().refreshRepos();
    } catch (e) {
      set({ authError: String(e) });
    } finally {
      set({ authBusy: false });
    }
  },

  unlockVault: async (password, remember) => {
    set({ authBusy: true, authError: null });
    try {
      await api.vaultUnlock(password, remember);
      set({ locked: false, vaultExists: true, forceExisting: false });
      await afterUnlock(get);
    } catch (e) {
      set({ authError: String(e) });
    } finally {
      set({ authBusy: false });
    }
  },

  lockVault: async () => {
    await api.vaultLock();
    set(lockedState());
  },

  onBackendLock: () => set(lockedState()),

  refreshRepos: async () => {
    const repos = await api.listRepoSummaries();
    set((s) => {
      const expandedRepos = { ...s.expandedRepos };
      repos.forEach((r) => {
        if (!(r.id in expandedRepos)) expandedRepos[r.id] = true;
      });
      let activeRepoId = s.activeRepoId;
      let activeEnvId = s.activeEnvId;
      const currentRepo = repos.find((r) => r.id === activeRepoId);
      if (!currentRepo) {
        activeRepoId = repos[0]?.id ?? null;
        activeEnvId = repos[0]?.envs[0]?.id ?? null;
      } else if (!currentRepo.envs.find((e) => e.id === activeEnvId)) {
        activeEnvId = currentRepo.envs[0]?.id ?? null;
      }
      return { repos, expandedRepos, activeRepoId, activeEnvId };
    });
    await get().refreshVariables();
    const linkedGroupCount = await api.linkedGroupCount();
    set({ linkedGroupCount });
  },

  refreshVariables: async () => {
    const envId = get().activeEnvId;
    if (!envId) {
      set({ variables: [] });
      return;
    }
    const variables = await api.listVariablesWithUsage(envId);
    set({ variables });
  },

  loadProjects: async () => {
    try {
      const projects = await api.listProjects();
      set({ projects });
    } catch (e) {
      get().showToast(String(e));
    }
  },
  linkCurrentEnvToFolder: async () => {
    const envId = get().activeEnvId;
    if (!envId) return;
    try {
      const picked = await open({ directory: true, multiple: false });
      const path = typeof picked === 'string' ? picked : null;
      if (!path) return;
      await api.linkProject(path, envId);
      await get().loadProjects();
      get().showToast('Linked folder');
    } catch (e) {
      get().showToast(String(e));
    }
  },
  unlinkProjectPath: async (path) => {
    try {
      await api.unlinkProject(path);
      await get().loadProjects();
      get().showToast('Unlinked folder');
    } catch (e) {
      get().showToast(String(e));
    }
  },

  /// Clears `variables` before fetching rather than after: leaving them in
  /// place renders the previous environment's secrets under the new
  /// environment's name until the fetch lands.
  selectEnv: async (repoId, envId) => {
    set({
      activeRepoId: repoId,
      activeEnvId: envId,
      variables: [],
      varsLoading: true,
      varSearch: '',
      expandedVarId: null,
      selectedVarIds: {},
      bulkMoveTargetId: null,
    });
    try {
      await get().refreshVariables();
    } finally {
      set({ varsLoading: false });
    }
  },

  toggleExpandRepo: (repoId) =>
    set((s) => ({ expandedRepos: { ...s.expandedRepos, [repoId]: !s.expandedRepos[repoId] } })),

  toggleAddRepo: () => set((s) => ({ addingRepo: !s.addingRepo, newRepoName: '' })),
  setNewRepoName: (v) => set({ newRepoName: v }),
  submitAddRepo: async () => {
    const name = get().newRepoName.trim();
    if (!name) return;
    try {
      const repo = await api.createRepo(name);
      set((s) => ({
        addingRepo: false,
        newRepoName: '',
        activeRepoId: repo.id,
        activeEnvId: null,
        expandedRepos: { ...s.expandedRepos, [repo.id]: true },
      }));
      await get().refreshRepos();
      get().showToast(`Added ${name}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  startAddEnv: (repoId) => set({ addingEnvFor: repoId, newEnvName: '' }),
  cancelAddEnv: () => set({ addingEnvFor: null }),
  setNewEnvName: (v) => set({ newEnvName: v }),
  submitAddEnv: async (repoId) => {
    const name = get().newEnvName.trim();
    if (!name) return;
    try {
      const env = await api.createEnvironment(repoId, name);
      set({ addingEnvFor: null, newEnvName: '', activeRepoId: repoId, activeEnvId: env.id });
      await get().refreshRepos();
      get().showToast(`Added ${name}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  startRenameRepo: (repoId, currentName) =>
    set({ renamingRepoId: repoId, renamingEnvId: null, renameDraft: currentName }),
  startRenameEnv: (envId, currentName) =>
    set({ renamingEnvId: envId, renamingRepoId: null, renameDraft: currentName }),
  setRenameDraft: (v) => set({ renameDraft: v }),
  cancelRename: () => set({ renamingRepoId: null, renamingEnvId: null, renameDraft: '' }),
  submitRename: async () => {
    const { renamingRepoId, renamingEnvId, renameDraft } = get();
    const name = renameDraft.trim();
    if (!name) return;
    try {
      if (renamingRepoId) {
        await api.renameRepo(renamingRepoId, name);
      } else if (renamingEnvId) {
        await api.renameEnvironment(renamingEnvId, name);
      } else {
        return;
      }
      set({ renamingRepoId: null, renamingEnvId: null, renameDraft: '' });
      await get().refreshRepos();
      get().showToast(`Renamed to ${name}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  requestDeleteRepo: (repoId, name) => {
    get().requestConfirm({
      title: `Delete ${name}?`,
      message:
        'This deletes every environment, variable and version-history snapshot under this repository. It cannot be undone.',
      confirmLabel: 'Delete repository',
      danger: true,
      requireTypedName: name,
      onConfirm: async () => {
        await api.deleteRepo(repoId);
        await get().refreshRepos();
        get().showToast(`Deleted ${name}`);
      },
    });
  },

  requestDeleteEnv: (envId, repoName, envName) => {
    get().requestConfirm({
      title: `Delete ${envName}?`,
      message: `This deletes every variable and version-history snapshot in ${repoName}/${envName}. It cannot be undone.`,
      confirmLabel: 'Delete environment',
      danger: true,
      requireTypedName: envName,
      onConfirm: async () => {
        await api.deleteEnvironment(envId);
        await get().refreshRepos();
        get().showToast(`Deleted ${envName}`);
      },
    });
  },

  startDuplicateEnv: (envId, currentName) =>
    set({ duplicatingEnvId: envId, duplicateNewName: `${currentName}-copy`, duplicateCopyValues: false }),
  cancelDuplicateEnv: () => set({ duplicatingEnvId: null, duplicateNewName: '' }),
  setDuplicateNewName: (v) => set({ duplicateNewName: v }),
  toggleDuplicateCopyValues: () => set((s) => ({ duplicateCopyValues: !s.duplicateCopyValues })),
  submitDuplicateEnv: async () => {
    const { duplicatingEnvId, duplicateNewName, duplicateCopyValues } = get();
    const name = duplicateNewName.trim();
    if (!duplicatingEnvId || !name) return;
    set({ duplicateBusy: true });
    try {
      const env = await api.duplicateEnvironment(duplicatingEnvId, name, duplicateCopyValues);
      set({ duplicatingEnvId: null, duplicateNewName: '', activeEnvId: env.id, activeRepoId: env.repoId });
      await get().refreshRepos();
      get().showToast(`Duplicated to ${name}`);
    } catch (e) {
      get().showToast(String(e));
    } finally {
      set({ duplicateBusy: false });
    }
  },

  requestConfirm: (req) => set({ confirm: req, confirmInput: '', confirmBusy: false }),
  setConfirmInput: (v) => set({ confirmInput: v }),
  cancelConfirm: () => set({ confirm: null, confirmInput: '', confirmBusy: false }),
  acceptConfirm: async () => {
    const { confirm, confirmInput, confirmBusy } = get();
    if (!confirm || confirmBusy) return;
    if (confirm.requireTypedName && confirmInput.trim() !== confirm.requireTypedName) return;
    set({ confirmBusy: true });
    try {
      await confirm.onConfirm();
      set({ confirm: null, confirmInput: '' });
    } catch (e) {
      get().showToast(String(e));
    } finally {
      set({ confirmBusy: false });
    }
  },

  setVarSearch: (v) => set({ varSearch: v }),
  setNewVarKey: (v) => set({ newVarKey: v }),
  setNewVarValue: (v) => set({ newVarValue: v }),
  addVariable: async () => {
    const envId = get().activeEnvId;
    const key = get().newVarKey.trim();
    if (!envId || !key) return;
    try {
      await api.addVariable(envId, key, get().newVarValue);
      set({ newVarKey: '', newVarValue: '' });
      await get().refreshRepos();
      get().showToast(`Added ${key}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  commitVariableValue: async (varId, newValue) => {
    try {
      await api.updateVariableValue(varId, newValue);
      await get().refreshRepos();
    } catch (e) {
      get().showToast(String(e));
    }
  },

  commitVariableKey: async (varId, newKey) => {
    const trimmed = newKey.trim();
    const current = get().variables.find((v) => v.id === varId);
    if (!trimmed || !current || trimmed === current.key) {
      await get().refreshVariables();
      return;
    }
    try {
      await api.renameVariableKey(varId, trimmed);
      await get().refreshRepos();
      get().showToast(`Renamed to ${trimmed}`);
    } catch (e) {
      get().showToast(String(e));
      // pull the committed state back so the cell stops showing the rejected key
      await get().refreshVariables();
    }
  },

  deleteVariable: async (varId, key) => {
    const remove = async () => {
      try {
        await api.deleteVariable(varId);
        await get().refreshRepos();
        get().showToast(`Deleted ${key}`);
      } catch (e) {
        get().showToast(String(e));
      }
    };

    const env = activeEnv(get());
    if (env && isProtectedEnv(env.name)) {
      get().requestConfirm({
        title: `Delete ${key} from ${env.name}?`,
        message: `${env.name} holds live credentials. Anything reading this variable in production will stop finding it. You can restore it from history afterwards.`,
        confirmLabel: 'Delete variable',
        danger: true,
        onConfirm: remove,
      });
      return;
    }
    await remove();
  },

  toggleVarExpand: (varId) =>
    set((s) => ({ expandedVarId: s.expandedVarId === varId ? null : varId })),
  commitVariableMetadata: async (varId, description, required, rotateAfterDays) => {
    try {
      await api.setVariableMetadata(
        varId,
        description.trim() ? description : null,
        required,
        rotateAfterDays,
      );
      await get().refreshVariables();
    } catch (e) {
      get().showToast(String(e));
    }
  },

  toggleVarSelected: (varId) =>
    set((s) => ({ selectedVarIds: { ...s.selectedVarIds, [varId]: !s.selectedVarIds[varId] } })),
  clearVarSelection: () => set({ selectedVarIds: {}, bulkMoveTargetId: null }),

  bulkDeleteSelected: async () => {
    const ids = Object.entries(get().selectedVarIds).filter(([, v]) => v).map(([id]) => id);
    if (ids.length === 0) return;
    const count = ids.length;
    const remove = async () => {
      try {
        await api.deleteVariables(ids);
        get().clearVarSelection();
        await get().refreshRepos();
        get().showToast(`Deleted ${count} variable${count === 1 ? '' : 's'}`);
      } catch (e) {
        get().showToast(String(e));
      }
    };

    const env = activeEnv(get());
    if (env && isProtectedEnv(env.name)) {
      get().requestConfirm({
        title: `Delete ${count} variable${count === 1 ? '' : 's'} from ${env.name}?`,
        message: `${env.name} holds live credentials. Anything reading these variables in production will stop finding them. You can restore them from history afterwards.`,
        confirmLabel: 'Delete variables',
        danger: true,
        onConfirm: remove,
      });
      return;
    }
    await remove();
  },

  bulkCopySelectedAsEnvBlock: async () => {
    const ids = Object.entries(get().selectedVarIds).filter(([, v]) => v).map(([id]) => id);
    const vars = get().variables.filter((v) => ids.includes(v.id));
    if (vars.length === 0) return;
    const text = vars.map((v) => `${v.key}=${v.value}`).join('\n');
    const doCopy = async () => {
      try {
        await api.copySecretToClipboard(text);
        get().showToast(`Copied ${vars.length} variable${vars.length === 1 ? '' : 's'} — clipboard clears in 30s`);
      } catch (e) {
        get().showToast(String(e));
      }
    };
    withProtectedEnvAck(get, set, 'copy live secrets to the clipboard', () => void doCopy());
  },

  setBulkMoveTarget: (envId) => set({ bulkMoveTargetId: envId }),
  bulkMoveSelected: async () => {
    const { bulkMoveTargetId } = get();
    const ids = Object.entries(get().selectedVarIds).filter(([, v]) => v).map(([id]) => id);
    if (ids.length === 0 || !bulkMoveTargetId) return;
    const count = ids.length;
    try {
      await api.moveVariables(ids, bulkMoveTargetId);
      get().clearVarSelection();
      await get().refreshRepos();
      get().showToast(`Moved ${count} variable${count === 1 ? '' : 's'}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  toggleReveal: (varId) => {
    const alreadyRevealed = !!get().revealed[varId];
    const doToggle = () =>
      set((s) => ({ revealed: { ...s.revealed, [varId]: !s.revealed[varId] } }));
    // Hiding a value never needs a warning; only exposing one does.
    if (alreadyRevealed) {
      doToggle();
      return;
    }
    withProtectedEnvAck(get, set, 'reveal a live secret', doToggle);
  },

  copyVariable: async (value, key) => {
    const doCopy = async () => {
      try {
        await api.copySecretToClipboard(value);
        get().showToast(`Copied ${key} — clipboard clears in 30s`);
      } catch (e) {
        get().showToast(String(e));
      }
    };
    withProtectedEnvAck(get, set, 'copy a live secret to the clipboard', () => void doCopy());
  },

  /// Copies something that is sensitive but is not a vault variable — today,
  /// the recovery code. Gets the same 30-second clipboard wipe.
  copyPlainText: async (text, label) => {
    try {
      await api.copySecretToClipboard(text);
      get().showToast(`Copied ${label} — clipboard clears in 30s`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  openImport: () => set({ importOpen: true, importText: '' }),
  closeImport: () => set({ importOpen: false }),
  setImportText: (v) => set({ importText: v }),
  submitImport: async () => {
    const envId = get().activeEnvId;
    const text = get().importText;
    if (!envId) {
      get().showToast('Select an environment first');
      return;
    }
    if (!text.trim()) {
      get().showToast('Nothing to import');
      return;
    }

    const runImport = async () => {
      try {
        const count = await api.importEnvText(envId, text);
        set({ importOpen: false, importText: '' });
        await get().refreshRepos();
        get().showToast(`Imported ${count} variable${count === 1 ? '' : 's'}`);
      } catch (e) {
        get().showToast(String(e));
      }
    };

    const env = activeEnv(get());
    if (env && isProtectedEnv(env.name)) {
      // An import silently overwrites matching keys, so say how many before it runs.
      const incoming = parseImportKeys(text);
      const existing = new Set(get().variables.map((v) => v.key));
      const overwrites = incoming.filter((k) => existing.has(k)).length;
      const additions = incoming.length - overwrites;
      get().requestConfirm({
        title: `Import into ${env.name}?`,
        message:
          `${env.name} holds live credentials. This will overwrite ${overwrites} existing value${overwrites === 1 ? '' : 's'}` +
          ` and add ${additions} new variable${additions === 1 ? '' : 's'}.` +
          ' Linked variables propagate the new values everywhere they are used.',
        confirmLabel: 'Import',
        danger: true,
        onConfirm: runImport,
      });
      return;
    }
    await runImport();
  },

  exportEnv: async () => {
    const { activeRepoId, activeEnvId, repos } = get();
    if (!activeRepoId || !activeEnvId) return;
    const repo = repos.find((r) => r.id === activeRepoId);
    const env = repo?.envs.find((e) => e.id === activeEnvId);
    if (!repo || !env) return;

    const runExport = async () => {
      try {
        const path = await save({ defaultPath: `${repo.name}.${env.name}.env` });
        if (!path) return;
        await api.exportEnvToFile(env.id, path);
        get().showToast(`Exported ${repo.name}.${env.name}.env`);
      } catch (e) {
        get().showToast(String(e));
      }
    };

    if (isProtectedEnv(env.name)) {
      get().requestConfirm({
        title: `Export ${env.name} secrets to a file?`,
        message: `This writes ${env.varCount} live credential${env.varCount === 1 ? '' : 's'} to an unencrypted .env file on disk. Delete it when you are done, and keep it out of version control.`,
        confirmLabel: 'Export anyway',
        danger: true,
        onConfirm: runExport,
      });
      return;
    }
    await runExport();
  },

  openHistory: async () => {
    const envId = get().activeEnvId;
    set({
      historyOpen: true,
      historySnapshots: [],
      expandedSnapshotId: null,
      snapshotDiff: [],
      diffRevealed: {},
    });
    if (envId) {
      const historySnapshots = await api.listSnapshotsWithStats(envId);
      set({ historySnapshots });
    }
  },
  closeHistory: () => set({ historyOpen: false, expandedSnapshotId: null, snapshotDiff: [] }),

  toggleSnapshotDiff: async (snapshotId) => {
    if (get().expandedSnapshotId === snapshotId) {
      set({ expandedSnapshotId: null, snapshotDiff: [], diffRevealed: {} });
      return;
    }
    set({ expandedSnapshotId: snapshotId, snapshotDiff: [], diffRevealed: {} });
    try {
      const snapshotDiff = await api.diffSnapshot(snapshotId, 'previous');
      // guard against a slower request landing after the user moved on
      if (get().expandedSnapshotId === snapshotId) set({ snapshotDiff });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  toggleDiffReveal: (key) =>
    set((s) => ({ diffRevealed: { ...s.diffRevealed, [key]: !s.diffRevealed[key] } })),

  /// Shows what restoring would actually do before doing it, so a restore is
  /// never a blind swap of the whole environment.
  requestRestoreSnapshot: async (snapshotId, timeLabel) => {
    let preview: DiffRow[] = [];
    try {
      preview = await api.diffSnapshot(snapshotId, 'current');
    } catch (e) {
      get().showToast(String(e));
      return;
    }

    if (preview.length === 0) {
      get().showToast('That snapshot matches the environment as it is now');
      return;
    }
    const added = preview.filter((d) => d.kind === 'added').length;
    const removed = preview.filter((d) => d.kind === 'removed').length;
    const changed = preview.filter((d) => d.kind === 'changed').length;
    const parts = [
      added ? `restore ${added} variable${added === 1 ? '' : 's'}` : null,
      removed ? `delete ${removed} variable${removed === 1 ? '' : 's'}` : null,
      changed ? `change ${changed} value${changed === 1 ? '' : 's'}` : null,
    ].filter(Boolean);

    get().requestConfirm({
      title: `Restore snapshot from ${timeLabel}?`,
      message: `This will ${parts.join(', ')}. The environment as it stands now is snapshotted first, so you can undo this from history.`,
      confirmLabel: 'Restore',
      danger: false,
      onConfirm: async () => {
        await api.restoreSnapshot(snapshotId);
        set({ historyOpen: false, expandedSnapshotId: null, snapshotDiff: [] });
        await get().refreshRepos();
        get().showToast(`Restored snapshot from ${timeLabel}`);
      },
    });
  },

  restoreVariableFromSnapshot: async (snapshotId, key) => {
    try {
      await api.restoreVariableFromSnapshot(snapshotId, key);
      await get().refreshRepos();
      const historySnapshots = get().activeEnvId
        ? await api.listSnapshotsWithStats(get().activeEnvId!)
        : [];
      set({ historySnapshots, expandedSnapshotId: null, snapshotDiff: [] });
      get().showToast(`Restored ${key}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  openSettings: async () => {
    set({
      settingsOpen: true,
      pwCurrent: '',
      pwNew: '',
      pwConfirm: '',
      pwError: null,
      recoveryCodeOnce: null,
    });
    try {
      const [needsMigration, hasRecoveryCode, backups, stored] = await Promise.all([
        api.vaultNeedsMigration(),
        api.vaultHasRecoveryCode(),
        api.listBackups(),
        api.getMeta(AUTO_LOCK_META_KEY),
      ]);
      set({
        needsMigration,
        hasRecoveryCode,
        backups,
        autoLockMinutes: parseAutoLock(stored),
      });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  closeSettings: () =>
    set({ settingsOpen: false, pwCurrent: '', pwNew: '', pwConfirm: '', pwError: null, recoveryCodeOnce: null }),

  setPwField: (field, value) => set({ [field]: value, pwError: null } as Partial<VaultState>),

  changePassword: async (remember) => {
    const { pwCurrent, pwNew, pwConfirm } = get();
    if (!pwNew) {
      set({ pwError: 'Choose a new master password.' });
      return;
    }
    if (pwNew !== pwConfirm) {
      set({ pwError: 'The new passwords do not match.' });
      return;
    }
    set({ pwBusy: true, pwError: null });
    try {
      await api.vaultChangePassword(pwCurrent, pwNew, remember);
      set({ pwCurrent: '', pwNew: '', pwConfirm: '' });
      const backups = await api.listBackups();
      set({ backups });
      get().showToast('Master password changed');
    } catch (e) {
      set({ pwError: String(e) });
    } finally {
      set({ pwBusy: false });
    }
  },

  generateRecoveryCode: async () => {
    const proceed = async () => {
      try {
        const code = await api.vaultGenerateRecoveryCode();
        const [backups, hasRecoveryCode] = await Promise.all([
          api.listBackups(),
          api.vaultHasRecoveryCode(),
        ]);
        set({ recoveryCodeOnce: code, backups, hasRecoveryCode });
      } catch (e) {
        get().showToast(String(e));
      }
    };

    if (get().hasRecoveryCode) {
      get().requestConfirm({
        title: 'Replace your recovery kit?',
        message:
          'Generating a new recovery code immediately invalidates the existing one. Any printed or saved copy of the old code will stop working.',
        confirmLabel: 'Generate new code',
        danger: true,
        onConfirm: proceed,
      });
      return;
    }
    await proceed();
  },

  dismissRecoveryCode: () => set({ recoveryCodeOnce: null }),

  saveRecoveryCodeToFile: async () => {
    const code = get().recoveryCodeOnce;
    if (!code) return;
    try {
      const path = await save({ defaultPath: 'vault-r-recovery-kit.txt' });
      if (!path) return;
      await api.saveRecoveryKit(path);
      get().showToast('Recovery kit saved');
    } catch (e) {
      get().showToast(String(e));
    }
  },

  exportBackup: async () => {
    try {
      const stamp = new Date().toISOString().slice(0, 10);
      const path = await save({ defaultPath: `vault-r-backup-${stamp}.vrbackup` });
      if (!path) return;
      await api.exportBackup(path);
      get().showToast('Encrypted backup saved');
    } catch (e) {
      get().showToast(String(e));
    }
  },

  setAutoLockMinutes: async (minutes) => {
    set({ autoLockMinutes: minutes });
    try {
      await api.setMeta(AUTO_LOCK_META_KEY, String(minutes));
    } catch (e) {
      get().showToast(String(e));
    }
  },

  setRecoveryMode: (on) => set({ recoveryMode: on, recoveryCode: '', authError: null }),
  setRecoveryCode: (v) => set({ recoveryCode: v }),

  unlockWithRecovery: async () => {
    const code = get().recoveryCode.trim();
    if (!code) return;
    set({ authBusy: true, authError: null });
    try {
      await api.vaultUnlockWithRecovery(code);
      // Getting in with a recovery code means the password is unknown; make
      // setting a new one the only way forward rather than an optional nag.
      set({ locked: false, recoveryMode: false, recoveryCode: '', mustResetPassword: true });
      await afterUnlock(get);
    } catch (e) {
      set({ authError: String(e) });
    } finally {
      set({ authBusy: false });
    }
  },

  resetPasswordAfterRecovery: async (newPassword) => {
    set({ authBusy: true, authError: null });
    try {
      await api.vaultResetPassword(newPassword);
      set({ mustResetPassword: false });
      get().showToast('Master password set');
    } catch (e) {
      set({ authError: String(e) });
    } finally {
      set({ authBusy: false });
    }
  },

  restoreBackupFromUnlock: async () => {
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: 'Vault-R backup', extensions: ['vrbackup'] }],
      });
      const path = typeof picked === 'string' ? picked : null;
      if (!path) return;

      get().requestConfirm({
        title: 'Restore this backup?',
        message:
          'The vault currently on this device will be replaced by the backup. A copy of it is kept first, so this can be undone. Unlock with the master password that protected the backup.',
        confirmLabel: 'Restore backup',
        danger: true,
        onConfirm: async () => {
          await api.restoreBackup(path);
          const status = await api.vaultStatus().catch(() => null);
          set({ authError: null, vaultExists: true, forceExisting: false, vaultStatus: status });
          get().showToast('Backup restored — unlock with its master password');
        },
      });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  openShare: async () => {
    set({ shareOpen: true });
    const members = await api.listMembers();
    set({ members });
  },
  closeShare: () => set({ shareOpen: false }),
  setInviteEmail: (v) => set({ inviteEmail: v }),
  addMemberAction: async () => {
    const email = get().inviteEmail.trim();
    if (!email) return;
    try {
      await api.addMember(email, 'Editor');
      set({ inviteEmail: '' });
      const members = await api.listMembers();
      set({ members });
    } catch (e) {
      get().showToast(String(e));
    }
  },
  removeMemberAction: async (id) => {
    await api.removeMember(id);
    const members = await api.listMembers();
    set({ members });
  },

  toggleCmdk: () => set((s) => ({ cmdkOpen: !s.cmdkOpen, cmdkQuery: '', cmdkResults: [] })),
  closeCmdk: () => set({ cmdkOpen: false }),
  setCmdkQuery: async (q) => {
    set({ cmdkQuery: q });
    if (!q.trim()) {
      set({ cmdkResults: [] });
      return;
    }
    const cmdkResults = await api.search(q);
    set({ cmdkResults });
  },
  cmdkSelectResult: async (r) => {
    set({ cmdkOpen: false, cmdkQuery: '', cmdkResults: [] });
    if (r.envId) {
      await get().selectEnv(r.repoId, r.envId);
    } else {
      const repo = get().repos.find((x) => x.id === r.repoId);
      const firstEnv = repo?.envs[0];
      set({ activeRepoId: r.repoId, activeEnvId: firstEnv?.id ?? null });
      await get().refreshVariables();
    }
  },

  openLinkModal: async (varId, key) => {
    const linkCandidates = await api.linkCandidates(varId);
    set({ linkModalOpen: true, linkModalVarId: varId, linkModalKey: key, linkCandidates, linkSelected: {} });
  },
  closeLinkModal: () =>
    set({ linkModalOpen: false, linkModalVarId: null, linkModalKey: '', linkCandidates: [], linkSelected: {} }),
  toggleLinkSelected: (varId) =>
    set((s) => ({ linkSelected: { ...s.linkSelected, [varId]: !s.linkSelected[varId] } })),
  confirmLink: async () => {
    const { linkModalVarId, linkSelected } = get();
    if (!linkModalVarId) return;
    const selectedIds = Object.entries(linkSelected)
      .filter(([, v]) => v)
      .map(([k]) => k);
    if (selectedIds.length === 0) {
      get().showToast('Select at least one variable to link');
      return;
    }
    try {
      await api.linkVariables([linkModalVarId, ...selectedIds]);
      set({ linkModalOpen: false, linkModalVarId: null, linkModalKey: '', linkCandidates: [], linkSelected: {} });
      await get().refreshRepos();
      get().showToast('Linked variables');
    } catch (e) {
      get().showToast(String(e));
    }
  },
  openGroupPopover: async (groupId) => {
    const groupPopoverMembers = await api.groupMembers(groupId);
    set({ groupPopoverGroupId: groupId, groupPopoverMembers });
  },
  closeGroupPopover: () => set({ groupPopoverGroupId: null, groupPopoverMembers: [] }),
  unlinkFromPopover: async (varId) => {
    try {
      await api.unlinkVariable(varId);
      set({ groupPopoverGroupId: null, groupPopoverMembers: [] });
      await get().refreshRepos();
      get().showToast('Unlinked variable');
    } catch (e) {
      get().showToast(String(e));
    }
  },

  openGenerator: (target) =>
    set({
      generatorOpen: true,
      generatorTarget: target,
      generatorKind: 'hex',
      generatorLength: GENERATOR_DEFAULT_LENGTH.hex,
    }),
  closeGenerator: () => set({ generatorOpen: false, generatorTarget: null }),
  setGeneratorKind: (kind) => set({ generatorKind: kind, generatorLength: GENERATOR_DEFAULT_LENGTH[kind] }),
  setGeneratorLength: (length) => set({ generatorLength: length }),
  runGenerator: async () => {
    const { generatorKind, generatorLength, generatorTarget } = get();
    if (!generatorTarget) return;
    set({ generatorBusy: true });
    try {
      const secret = await api.generateSecret(generatorKind, generatorLength);
      if (generatorTarget.type === 'add') {
        set({ newVarValue: secret });
      } else {
        // An existing row has no separate "save" step, so generating for one
        // commits immediately -- and reveals it, since you cannot see what
        // you just generated otherwise.
        await api.updateVariableValue(generatorTarget.varId, secret);
        set((s) => ({ revealed: { ...s.revealed, [generatorTarget.varId]: true } }));
        await get().refreshRepos();
      }
      set({ generatorOpen: false, generatorTarget: null });
      get().showToast('Generated a new secret');
    } catch (e) {
      get().showToast(String(e));
    } finally {
      set({ generatorBusy: false });
    }
  },

  openCompare: () => set({ compareOpen: true, compareEnvBId: null, compareRows: [], compareRevealed: {}, compareUnlinkedMatches: [] }),
  closeCompare: () => set({ compareOpen: false }),
  setCompareEnvB: async (envId) => {
    set({ compareEnvBId: envId, compareRevealed: {} });
    await get().refreshCompare();
  },
  refreshCompare: async () => {
    const { activeEnvId, compareEnvBId } = get();
    if (!activeEnvId || !compareEnvBId) {
      set({ compareRows: [], compareUnlinkedMatches: [] });
      return;
    }
    set({ compareBusy: true });
    try {
      const [compareRows, compareUnlinkedMatches] = await Promise.all([
        api.diffEnvironments(activeEnvId, compareEnvBId),
        api.unlinkedIdenticalPairs(activeEnvId, compareEnvBId),
      ]);
      set({ compareRows, compareUnlinkedMatches });
    } catch (e) {
      get().showToast(String(e));
    } finally {
      set({ compareBusy: false });
    }
  },
  toggleCompareReveal: (key) => {
    const alreadyRevealed = !!get().compareRevealed[key];
    const doToggle = () =>
      set((s) => ({ compareRevealed: { ...s.compareRevealed, [key]: !s.compareRevealed[key] } }));
    if (alreadyRevealed) {
      doToggle();
      return;
    }
    const { activeEnvId, compareEnvBId } = get();
    withMultiEnvProtectedAck(
      get,
      set,
      [activeEnvId, compareEnvBId],
      'reveal live secrets in the compare view',
      doToggle,
    );
  },
  copyCompareRow: async (key, direction) => {
    const { activeEnvId, compareEnvBId } = get();
    if (!activeEnvId || !compareEnvBId) return;
    const [source, target] = direction === 'aToB' ? [activeEnvId, compareEnvBId] : [compareEnvBId, activeEnvId];
    try {
      await api.copyKeyToEnv(source, target, key);
      await get().refreshCompare();
      await get().refreshRepos();
      get().showToast(`Copied ${key}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },
  copyAllMissing: async (direction) => {
    const { activeEnvId, compareEnvBId } = get();
    if (!activeEnvId || !compareEnvBId) return;
    const [source, target] = direction === 'aToB' ? [activeEnvId, compareEnvBId] : [compareEnvBId, activeEnvId];
    try {
      const count = await api.copyMissingToEnv(source, target);
      await get().refreshCompare();
      await get().refreshRepos();
      get().showToast(`Copied ${count} missing variable${count === 1 ? '' : 's'}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },
  linkCompareMatch: async (match) => {
    try {
      await api.linkVariables([match.varA.id, match.varB.id]);
      await get().refreshCompare();
      await get().refreshRepos();
      get().showToast(`Linked ${match.key}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  /// Opens the panel with its scan already running. A safety panel that waits
  /// for a click before telling you anything gets opened once.
  openSafety: async (tab = 'leaks') => {
    set({ safetyOpen: true, safetyTab: tab });
    await (tab === 'leaks' ? get().runLeakScan() : get().refreshHealth());
  },
  closeSafety: () => set({ safetyOpen: false }),
  setSafetyTab: async (tab) => {
    set({ safetyTab: tab });
    if (tab === 'leaks' && !get().leakScanned) {
      await get().runLeakScan();
    } else if (tab === 'health' && !get().health) {
      await get().refreshHealth();
    }
  },
  runLeakScan: async () => {
    set({ leakScanning: true });
    try {
      set({ leakReports: await api.scanLinkedProjects(), leakScanned: true });
    } catch (e) {
      get().showToast(String(e));
    } finally {
      set({ leakScanning: false });
    }
  },
  refreshHealth: async () => {
    set({ healthLoading: true });
    try {
      set({ health: await api.healthReport() });
    } catch (e) {
      get().showToast(String(e));
    } finally {
      set({ healthLoading: false });
    }
  },
  applyGitignoreFix: async (report) => {
    if (!report.gitRoot) return;
    const patterns = Array.from(
      new Set(report.findings.map((f) => f.fixPattern).filter((p): p is string => !!p)),
    );
    if (patterns.length === 0) return;
    try {
      const added = await api.applyGitignorePatterns(report.gitRoot, patterns);
      await get().runLeakScan();
      get().showToast(
        added === 0
          ? '.gitignore already covered those'
          : `Added ${added} pattern${added === 1 ? '' : 's'} to .gitignore`,
      );
    } catch (e) {
      get().showToast(String(e));
    }
  },
  linkDuplicateGroup: async (group) => {
    try {
      await api.linkVariables(group.locations.map((l) => l.varId));
      await Promise.all([get().refreshHealth(), get().refreshRepos(), get().refreshVariables()]);
      get().showToast(`Linked ${group.locations.length} variables`);
    } catch (e) {
      get().showToast(String(e));
    }
  },
  /// Selects the environment a safety row points at and expands the row, so
  /// "3 problems" turns into "here is the first one" in one click.
  jumpToVariable: async (envId, varId) => {
    const repo = get().repos.find((r) => r.envs.some((e) => e.id === envId));
    if (!repo) return;
    set({ safetyOpen: false });
    await get().selectEnv(repo.id, envId);
    set({ expandedVarId: varId, expandedRepos: { ...get().expandedRepos, [repo.id]: true } });
  },

  obNext: () => set((s) => ({ onboardingStep: s.onboardingStep + 1 })),
  obSkip: async () => {
    await get().obFinish();
  },
  setOnboardingRepoName: (v) => set({ onboardingRepoName: v }),
  setOnboardingEnvName: (v) => set({ onboardingEnvName: v }),
  obImportAndNext: async () => {
    const repoName = get().onboardingRepoName.trim() || 'my-project';
    const envName = get().onboardingEnvName.trim() || 'local';
    const text = get().importText;
    try {
      const repo = await api.createRepo(repoName);
      const env = await api.createEnvironment(repo.id, envName);
      set({ activeRepoId: repo.id, activeEnvId: env.id });
      if (text.trim()) {
        const count = await api.importEnvText(env.id, text);
        get().showToast(`Imported ${count} variable${count === 1 ? '' : 's'}`);
      }
      await get().refreshRepos();
    } catch (e) {
      get().showToast(String(e));
    }
    set({ importText: '', onboardingStep: 2 });
  },
  obFinish: async () => {
    set({ onboarding: false });
    await api.setMeta('onboarding_done', 'true');
  },
  replayOnboarding: () => set({ onboarding: true, onboardingStep: 0 }),

  showToast: (msg) => {
    set({ toast: msg });
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => set({ toast: null }), 2400);
  },

  closeAllOverlays: () =>
    set((s) => ({
      cmdkOpen: false,
      importOpen: false,
      shareOpen: false,
      historyOpen: false,
      linkModalOpen: false,
      groupPopoverGroupId: null,
      generatorOpen: false,
      generatorTarget: null,
      compareOpen: false,
      safetyOpen: false,
      settingsOpen: false,
      // never yank a confirmation out from under an action that is mid-flight
      confirm: s.confirmBusy ? s.confirm : null,
      confirmInput: s.confirmBusy ? s.confirmInput : '',
    })),
}));

/// Also clears the Compare/History/link/import/command-palette caches and
/// closes their overlays — left open, they'd resurface old secrets after
/// the next unlock.
function lockedState(): Partial<VaultState> {
  return {
    locked: true,
    repos: [],
    variables: [],
    projects: [],
    activeRepoId: null,
    activeEnvId: null,
    revealed: {},
    // Every session-scoped concession resets: a fresh unlock warns again.
    protectedAcknowledged: {},
    recoveryCodeOnce: null,
    confirm: null,
    pwCurrent: '',
    pwNew: '',
    pwConfirm: '',
    historySnapshots: [],
    snapshotDiff: [],
    compareRows: [],
    compareUnlinkedMatches: [],
    linkCandidates: [],
    groupPopoverMembers: [],
    newVarValue: '',
    importText: '',
    cmdkResults: [],
    cmdkQuery: '',
    settingsOpen: false,
    historyOpen: false,
    expandedSnapshotId: null,
    diffRevealed: {},
    compareOpen: false,
    compareEnvBId: null,
    compareRevealed: {},
    importOpen: false,
    shareOpen: false,
    linkModalOpen: false,
    linkModalVarId: null,
    linkModalKey: '',
    linkSelected: {},
    groupPopoverGroupId: null,
    generatorOpen: false,
    generatorTarget: null,
    safetyOpen: false,
    cmdkOpen: false,
  };
}

async function afterUnlock(get: () => VaultState) {
  await get().refreshRepos();
  await get().loadProjects();
  const [onboardingDone, autoLock, needsMigration] = await Promise.all([
    api.getMeta('onboarding_done'),
    api.getMeta(AUTO_LOCK_META_KEY),
    api.vaultNeedsMigration(),
  ]);
  useVaultStore.setState({ autoLockMinutes: parseAutoLock(autoLock), needsMigration });
  if (!onboardingDone) {
    useVaultStore.setState({ onboarding: true, onboardingStep: 0 });
  }
}

function parseAutoLock(stored: string | null): number {
  const parsed = Number(stored);
  return stored !== null && Number.isFinite(parsed) && parsed >= 0
    ? parsed
    : DEFAULT_AUTO_LOCK_MINUTES;
}

function activeEnv(state: VaultState) {
  return state.repos
    .find((r) => r.id === state.activeRepoId)
    ?.envs.find((e) => e.id === state.activeEnvId);
}

function findEnvById(state: VaultState, envId: string) {
  for (const repo of state.repos) {
    const env = repo.envs.find((e) => e.id === envId);
    if (env) return env;
  }
  return undefined;
}

/// Runs `action`, but for a production-like environment asks once per session
/// first. One acknowledgement covers every reveal and copy in that environment
/// until the vault is locked — enough to stop an accidental click, not so much
/// that it becomes noise to click through.
function withProtectedEnvAck(
  get: () => VaultState,
  set: (partial: Partial<VaultState>) => void,
  what: string,
  action: () => void,
) {
  const state = get();
  const env = activeEnv(state);
  if (!env || !isProtectedEnv(env.name) || state.protectedAcknowledged[env.id]) {
    action();
    return;
  }
  state.requestConfirm({
    title: `Show ${env.name} secrets?`,
    message: `You are about to ${what} from ${env.name}. These are live credentials — make sure nobody is looking over your shoulder or sharing your screen. Vault-R will not ask again for this environment until you lock it.`,
    confirmLabel: 'Show secrets',
    danger: true,
    onConfirm: () => {
      set({ protectedAcknowledged: { ...get().protectedAcknowledged, [env.id]: true } });
      action();
    },
  });
}

/// As [`withProtectedEnvAck`], but for actions (like the compare view's
/// reveal toggle) that can touch more than one environment's secrets at
/// once -- any of `envIds` that is protected and not yet acknowledged this
/// session triggers one combined confirmation.
function withMultiEnvProtectedAck(
  get: () => VaultState,
  set: (partial: Partial<VaultState>) => void,
  envIds: (string | null | undefined)[],
  what: string,
  action: () => void,
) {
  const state = get();
  const pending = envIds
    .filter((id): id is string => !!id)
    .map((id) => ({ id, env: findEnvById(state, id) }))
    .filter((p): p is { id: string; env: NonNullable<typeof p.env> } => !!p.env)
    .filter((p) => isProtectedEnv(p.env.name) && !state.protectedAcknowledged[p.id]);

  if (pending.length === 0) {
    action();
    return;
  }
  const names = pending.map((p) => p.env.name).join(' and ');
  state.requestConfirm({
    title: `Show ${names} secrets?`,
    message: `You are about to ${what}. These are live credentials — make sure nobody is looking over your shoulder or sharing your screen. Vault-R will not ask again until you lock it.`,
    confirmLabel: 'Show secrets',
    danger: true,
    onConfirm: () => {
      const patch: Record<string, boolean> = {};
      pending.forEach((p) => {
        patch[p.id] = true;
      });
      set({ protectedAcknowledged: { ...get().protectedAcknowledged, ...patch } });
      action();
    },
  });
}

/// The keys an import would touch. Mirrors `parse_env_text` in vault-core
/// closely enough to count overwrites for the confirmation copy; the import
/// itself is still parsed in Rust.
function parseImportKeys(text: string): string[] {
  const keys: string[] = [];
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq <= 0) continue;
    const key = line.slice(0, eq).trim();
    if (key && !keys.includes(key)) keys.push(key);
  }
  return keys;
}
