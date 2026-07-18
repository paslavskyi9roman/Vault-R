import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import styles from './CommandPalette.module.css';

interface QuickAction {
  id: string;
  label: string;
  run: () => void;
}

export function CommandPalette() {
  const cmdkOpen = useVaultStore((s) => s.cmdkOpen);
  const cmdkQuery = useVaultStore((s) => s.cmdkQuery);
  const cmdkResults = useVaultStore((s) => s.cmdkResults);
  const setCmdkQuery = useVaultStore((s) => s.setCmdkQuery);
  const cmdkSelectResult = useVaultStore((s) => s.cmdkSelectResult);
  const closeCmdk = useVaultStore((s) => s.closeCmdk);
  const openImport = useVaultStore((s) => s.openImport);
  const exportEnv = useVaultStore((s) => s.exportEnv);
  const openHistory = useVaultStore((s) => s.openHistory);
  const openShare = useVaultStore((s) => s.openShare);

  // Shorter than the other overlays: this one gets dismissed constantly and
  // any lag on the way out reads as sluggish.
  const { mounted, state } = usePresence(cmdkOpen, 100);
  if (!mounted) return null;

  const quickActions: QuickAction[] = [
    { id: 'a1', label: 'Import .env file', run: () => { closeCmdk(); openImport(); } },
    { id: 'a2', label: 'Export current .env', run: () => { closeCmdk(); void exportEnv(); } },
    { id: 'a3', label: 'View version history', run: () => { closeCmdk(); void openHistory(); } },
    { id: 'a4', label: 'Manage team access', run: () => { closeCmdk(); void openShare(); } },
  ];

  const showingQuickActions = !cmdkQuery.trim();
  const phase = state === 'entered' ? styles.isEntered : state === 'exiting' ? styles.isExiting : '';

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeCmdk} />
      <div className={`${styles.palette} ${phase}`}>
        <input
          className={`v-input ${styles.input}`}
          placeholder="Jump to a repo, environment, or secret&hellip;"
          value={cmdkQuery}
          onChange={(e) => void setCmdkQuery(e.target.value)}
          autoFocus
        />
        <div className={styles.results}>
          {showingQuickActions
            ? quickActions.map((a) => (
                <button key={a.id} className={styles.resultRow} onClick={a.run}>
                  <span className={styles.resultLabel}>{a.label}</span>
                  <span className={styles.resultSub}>Action</span>
                </button>
              ))
            : cmdkResults.map((r, i) => (
                <button key={i} className={styles.resultRow} onClick={() => void cmdkSelectResult(r)}>
                  <span className={styles.resultLabel}>{r.label}</span>
                  <span className={styles.resultSub}>{r.sublabel}</span>
                </button>
              ))}
          {!showingQuickActions && cmdkResults.length === 0 && (
            <div className={styles.empty}>No matches.</div>
          )}
        </div>
      </div>
    </>
  );
}
