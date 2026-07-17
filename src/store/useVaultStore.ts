import { create } from 'zustand';
import { save } from '@tauri-apps/plugin-dialog';
import {
  api,
  type GroupMember,
  type Member,
  type RepoSummary,
  type SearchResult,
  type Snapshot,
  type VariableWithUsage,
} from '../lib/api';

let toastTimer: ReturnType<typeof setTimeout> | undefined;

interface VaultState {
  // ---- lifecycle ----
  checkingVault: boolean;
  vaultExists: boolean;
  locked: boolean;
  authBusy: boolean;
  authError: string | null;

  // ---- core data ----
  repos: RepoSummary[];
  activeRepoId: string | null;
  activeEnvId: string | null;
  expandedRepos: Record<string, boolean>;
  variables: VariableWithUsage[];
  revealed: Record<string, boolean>;
  varSearch: string;
  linkedGroupCount: number;

  // ---- inline add forms ----
  addingRepo: boolean;
  newRepoName: string;
  addingEnvFor: string | null;
  newEnvName: string;
  newVarKey: string;
  newVarValue: string;

  // ---- import modal ----
  importOpen: boolean;
  importText: string;

  // ---- share modal ----
  shareOpen: boolean;
  members: Member[];
  inviteEmail: string;

  // ---- history slideover ----
  historyOpen: boolean;
  historySnapshots: Snapshot[];

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

  // ---- onboarding ----
  onboarding: boolean;
  onboardingStep: number;
  onboardingRepoName: string;
  onboardingEnvName: string;

  // ---- toast ----
  toast: string | null;

  // ---- actions ----
  init: () => Promise<void>;
  createVault: (password: string, remember: boolean) => Promise<void>;
  unlockVault: (password: string, remember: boolean) => Promise<void>;
  lockVault: () => Promise<void>;

  refreshRepos: () => Promise<void>;
  refreshVariables: () => Promise<void>;
  selectEnv: (repoId: string, envId: string | null) => Promise<void>;
  toggleExpandRepo: (repoId: string) => void;

  toggleAddRepo: () => void;
  setNewRepoName: (v: string) => void;
  submitAddRepo: () => Promise<void>;
  startAddEnv: (repoId: string) => void;
  cancelAddEnv: () => void;
  setNewEnvName: (v: string) => void;
  submitAddEnv: (repoId: string) => Promise<void>;

  setVarSearch: (v: string) => void;
  setNewVarKey: (v: string) => void;
  setNewVarValue: (v: string) => void;
  addVariable: () => Promise<void>;
  commitVariableValue: (varId: string, newValue: string) => Promise<void>;
  deleteVariable: (varId: string, key: string) => Promise<void>;
  toggleReveal: (varId: string) => void;
  copyVariable: (value: string, key: string) => Promise<void>;

  openImport: () => void;
  closeImport: () => void;
  setImportText: (v: string) => void;
  submitImport: () => Promise<void>;

  exportEnv: () => Promise<void>;

  openHistory: () => Promise<void>;
  closeHistory: () => void;
  restoreSnapshot: (snapshotId: string, timeLabel: string) => Promise<void>;

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
  locked: true,
  authBusy: false,
  authError: null,

  repos: [],
  activeRepoId: null,
  activeEnvId: null,
  expandedRepos: {},
  variables: [],
  revealed: {},
  varSearch: '',
  linkedGroupCount: 0,

  addingRepo: false,
  newRepoName: '',
  addingEnvFor: null,
  newEnvName: '',
  newVarKey: '',
  newVarValue: '',

  importOpen: false,
  importText: '',

  shareOpen: false,
  members: [],
  inviteEmail: '',

  historyOpen: false,
  historySnapshots: [],

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

  onboarding: false,
  onboardingStep: 0,
  onboardingRepoName: 'my-project',
  onboardingEnvName: 'local',

  toast: null,

  init: async () => {
    const exists = await api.vaultExists();
    set({ vaultExists: exists, checkingVault: false });
    if (exists) {
      const unlocked = await api.vaultTryKeychain();
      if (unlocked) {
        set({ locked: false });
        await afterUnlock(get);
      }
    }
  },

  createVault: async (password, remember) => {
    set({ authBusy: true, authError: null });
    try {
      await api.vaultCreate(password, remember);
      set({ locked: false, vaultExists: true, onboarding: true, onboardingStep: 0 });
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
      set({ locked: false });
      await afterUnlock(get);
    } catch (e) {
      set({ authError: String(e) });
    } finally {
      set({ authBusy: false });
    }
  },

  lockVault: async () => {
    await api.vaultLock();
    set({
      locked: true,
      repos: [],
      variables: [],
      activeRepoId: null,
      activeEnvId: null,
      revealed: {},
    });
  },

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

  selectEnv: async (repoId, envId) => {
    set({ activeRepoId: repoId, activeEnvId: envId, varSearch: '' });
    await get().refreshVariables();
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

  deleteVariable: async (varId, key) => {
    try {
      await api.deleteVariable(varId);
      await get().refreshRepos();
      get().showToast(`Deleted ${key}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  toggleReveal: (varId) => set((s) => ({ revealed: { ...s.revealed, [varId]: !s.revealed[varId] } })),

  copyVariable: async (value, key) => {
    try {
      await api.copySecretToClipboard(value);
      get().showToast(`Copied ${key}`);
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
    try {
      const count = await api.importEnvText(envId, text);
      set({ importOpen: false, importText: '' });
      await get().refreshRepos();
      get().showToast(`Imported ${count} variable${count === 1 ? '' : 's'}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  exportEnv: async () => {
    const { activeRepoId, activeEnvId, repos } = get();
    if (!activeRepoId || !activeEnvId) return;
    const repo = repos.find((r) => r.id === activeRepoId);
    const env = repo?.envs.find((e) => e.id === activeEnvId);
    if (!repo || !env) return;
    try {
      const path = await save({ defaultPath: `${repo.name}.${env.name}.env` });
      if (!path) return;
      await api.exportEnvToFile(env.id, path);
      get().showToast(`Exported ${repo.name}.${env.name}.env`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  openHistory: async () => {
    const envId = get().activeEnvId;
    set({ historyOpen: true, historySnapshots: [] });
    if (envId) {
      const historySnapshots = await api.listSnapshots(envId);
      set({ historySnapshots });
    }
  },
  closeHistory: () => set({ historyOpen: false }),
  restoreSnapshot: async (snapshotId, timeLabel) => {
    try {
      await api.restoreSnapshot(snapshotId);
      set({ historyOpen: false });
      await get().refreshRepos();
      get().showToast(`Restored snapshot from ${timeLabel}`);
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
    set({
      cmdkOpen: false,
      importOpen: false,
      shareOpen: false,
      historyOpen: false,
      linkModalOpen: false,
      groupPopoverGroupId: null,
    }),
}));

async function afterUnlock(get: () => VaultState) {
  await get().refreshRepos();
  const onboardingDone = await api.getMeta('onboarding_done');
  if (!onboardingDone) {
    useVaultStore.setState({ onboarding: true, onboardingStep: 0 });
  }
}
