import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import { timeAgo } from '../lib/envColor';
import { ChevronIcon, RestoreIcon, CloseIcon } from './icons';
import type { DiffRow } from '../lib/api';
import styles from './HistorySlideover.module.css';

export function HistorySlideover() {
  const historyOpen = useVaultStore((s) => s.historyOpen);
  const historySnapshots = useVaultStore((s) => s.historySnapshots);
  const expandedSnapshotId = useVaultStore((s) => s.expandedSnapshotId);
  const closeHistory = useVaultStore((s) => s.closeHistory);
  const toggleSnapshotDiff = useVaultStore((s) => s.toggleSnapshotDiff);
  const requestRestoreSnapshot = useVaultStore((s) => s.requestRestoreSnapshot);
  const repos = useVaultStore((s) => s.repos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);

  const { mounted, state } = usePresence(historyOpen, 160);
  if (!mounted) return null;

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeHistory} />
      <aside className={`v-slideover is-${state}`}>
        <div className={styles.header}>
          <span className={styles.title}>Version history</span>
          <button className="v-close-x" onClick={closeHistory} aria-label="Close">
            <CloseIcon size={13} />
          </button>
        </div>
        <div className={styles.sub}>
          {activeRepo && activeEnv ? `${activeRepo.name} / ${activeEnv.name}` : ''}
        </div>
        {historySnapshots.length === 0 && <div className={styles.empty}>No snapshots yet.</div>}
        {historySnapshots.map((h) => {
          const label = timeAgo(h.createdAt);
          const expanded = expandedSnapshotId === h.id;
          return (
            <div key={h.id} className={styles.item}>
              <button
                className={styles.row}
                onClick={() => void toggleSnapshotDiff(h.id)}
                aria-expanded={expanded}
              >
                <ChevronIcon size={11} className={styles.chevron} />
                <div className={styles.textWrap}>
                  <div className={styles.time}>{label}</div>
                  <div className={styles.summary}>{h.summary}</div>
                </div>
                <ChangeBadges added={h.added} removed={h.removed} changed={h.changed} />
              </button>

              {expanded && <SnapshotDiff snapshotId={h.id} />}

              <button
                className={`v-btn ${styles.restore}`}
                onClick={() => void requestRestoreSnapshot(h.id, label)}
              >
                Restore
              </button>
            </div>
          );
        })}
      </aside>
    </>
  );
}

function ChangeBadges({ added, removed, changed }: { added: number; removed: number; changed: number }) {
  if (!added && !removed && !changed) return <span className={styles.noChange}>no change</span>;
  return (
    <span className={styles.badgeWrap}>
      {added > 0 && (
        <span className={styles.badge} data-kind="added">
          +{added}
        </span>
      )}
      {removed > 0 && (
        <span className={styles.badge} data-kind="removed">
          &minus;{removed}
        </span>
      )}
      {changed > 0 && (
        <span className={styles.badge} data-kind="changed">
          ~{changed}
        </span>
      )}
    </span>
  );
}

/// The keys this snapshot touched. Values stay masked until asked for — the
/// point of the view is "what changed", not "show me every old secret".
function SnapshotDiff({ snapshotId }: { snapshotId: string }) {
  const rows = useVaultStore((s) => s.snapshotDiff);
  const diffRevealed = useVaultStore((s) => s.diffRevealed);
  const toggleDiffReveal = useVaultStore((s) => s.toggleDiffReveal);
  const restoreVariableFromSnapshot = useVaultStore((s) => s.restoreVariableFromSnapshot);

  if (rows.length === 0) {
    return <div className={styles.diffEmpty}>No variables changed in this snapshot.</div>;
  }

  return (
    <div className={styles.diffWrap}>
      {rows.map((row) => {
        const revealed = !!diffRevealed[row.key];
        return (
          <div key={row.key} className={styles.diffRow}>
            <span className={styles.diffMark} data-kind={row.kind}>
              {kindMark(row.kind)}
            </span>
            <div className={styles.diffBody}>
              <div className={styles.diffKey}>{row.key}</div>
              <div className={styles.diffValue}>{revealed ? describeValues(row) : '••••••••'}</div>
            </div>
            <button className={`v-btn ${styles.diffBtn}`} onClick={() => toggleDiffReveal(row.key)}>
              {revealed ? 'hide' : 'show'}
            </button>
            {row.kind !== 'added' && (
              <button
                className={`v-btn ${styles.diffBtn} ${styles.diffIconBtn}`}
                title={`Restore ${row.key} to this value`}
                aria-label={`Restore ${row.key} to this value`}
                onClick={() => void restoreVariableFromSnapshot(snapshotId, row.key)}
              >
                <RestoreIcon size={11} />
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}

function kindMark(kind: DiffRow['kind']): string {
  if (kind === 'added') return '+';
  if (kind === 'removed') return '−';
  return '~';
}

function describeValues(row: DiffRow): string {
  if (row.kind === 'changed') return `${row.oldValue ?? ''} → ${row.newValue ?? ''}`;
  if (row.kind === 'added') return row.newValue ?? '';
  return row.oldValue ?? '';
}
