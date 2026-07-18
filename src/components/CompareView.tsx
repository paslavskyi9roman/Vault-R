import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import { envColor } from '../lib/envColor';
import { Skeleton } from './Skeleton';
import type { DiffRow } from '../lib/api';
import styles from './CompareView.module.css';

export function CompareView() {
  const compareOpen = useVaultStore((s) => s.compareOpen);
  const closeCompare = useVaultStore((s) => s.closeCompare);
  const repos = useVaultStore((s) => s.repos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const compareEnvBId = useVaultStore((s) => s.compareEnvBId);
  const setCompareEnvB = useVaultStore((s) => s.setCompareEnvB);
  const compareRows = useVaultStore((s) => s.compareRows);
  const compareUnlinkedMatches = useVaultStore((s) => s.compareUnlinkedMatches);
  const compareBusy = useVaultStore((s) => s.compareBusy);
  const copyAllMissing = useVaultStore((s) => s.copyAllMissing);
  const linkCompareMatch = useVaultStore((s) => s.linkCompareMatch);

  const { mounted, state } = usePresence(compareOpen, 160);
  if (!mounted) return null;

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  const envOptions: { id: string; label: string }[] = [];
  repos.forEach((r) => r.envs.forEach((e) => {
    if (e.id !== activeEnvId) envOptions.push({ id: e.id, label: `${r.name}/${e.name}` });
  }));

  let envBLabel = '';
  repos.forEach((r) => r.envs.forEach((e) => {
    if (e.id === compareEnvBId) envBLabel = `${r.name}/${e.name}`;
  }));

  const onlyInA = compareRows.filter((r) => r.kind === 'removed');
  const onlyInB = compareRows.filter((r) => r.kind === 'added');
  const differing = compareRows.filter((r) => r.kind === 'changed');

  const envTone = { '--env-tone': activeEnv ? envColor(activeEnv.name) : 'var(--text)' } as React.CSSProperties;

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeCompare} />
      <aside className={`v-slideover v-slideover--wide is-${state}`}>
        <div className={styles.header}>
          <span className={styles.title}>Compare environments</span>
          <button className="v-close-x" onClick={closeCompare} aria-label="Close">
            &times;
          </button>
        </div>

        <div className={styles.pickerRow}>
          <span className={styles.envPill} style={envTone}>
            {activeRepo && activeEnv ? `${activeRepo.name}/${activeEnv.name}` : '—'}
          </span>
          <span className={styles.vs}>vs</span>
          <select
            className={`v-input ${styles.envSelect}`}
            value={compareEnvBId ?? ''}
            onChange={(e) => void setCompareEnvB(e.target.value)}
          >
            <option value="" disabled>
              Choose an environment…
            </option>
            {envOptions.map((o) => (
              <option key={o.id} value={o.id}>
                {o.label}
              </option>
            ))}
          </select>
        </div>

        {!compareEnvBId && <div className={styles.empty}>Pick an environment to compare against.</div>}

        {compareBusy && (
          <div className={styles.section}>
            {[0, 1, 2].map((i) => (
              <div key={i} className={styles.skelRow}>
                <div className={styles.skelBody}>
                  <Skeleton height={11} width={`${52 - i * 8}%`} />
                  <Skeleton height={9} width={`${34 + i * 6}%`} />
                </div>
              </div>
            ))}
          </div>
        )}

        {compareEnvBId && !compareBusy && compareRows.length === 0 && compareUnlinkedMatches.length === 0 && (
          <div className={styles.empty}>These environments match exactly.</div>
        )}

        {!compareBusy && compareUnlinkedMatches.length > 0 && (
          <div className={styles.section}>
            <div className={styles.sectionTitle}>
              {compareUnlinkedMatches.length} secret{compareUnlinkedMatches.length === 1 ? '' : 's'} match but
              {compareUnlinkedMatches.length === 1 ? " isn't" : " aren't"} linked
            </div>
            {compareUnlinkedMatches.map((m) => (
              <div key={m.key} className={styles.matchRow}>
                <span className={styles.diffKey}>{m.key}</span>
                <button className={`v-btn ${styles.smallBtn}`} onClick={() => void linkCompareMatch(m)}>
                  Link these
                </button>
              </div>
            ))}
          </div>
        )}

        {!compareBusy && onlyInA.length > 0 && (
          <DiffSection
            title={`Only in ${activeEnv?.name ?? 'A'}`}
            rows={onlyInA}
            onCopyAll={() => void copyAllMissing('aToB')}
            copyAllLabel={`Copy all → ${envBLabel}`}
          />
        )}
        {!compareBusy && onlyInB.length > 0 && (
          <DiffSection
            title={`Only in ${envBLabel || 'B'}`}
            rows={onlyInB}
            onCopyAll={() => void copyAllMissing('bToA')}
            copyAllLabel={`Copy all → ${activeEnv?.name ?? 'A'}`}
          />
        )}
        {!compareBusy && differing.length > 0 && <DiffSection title="Different values" rows={differing} />}
      </aside>
    </>
  );
}

function DiffSection({
  title,
  rows,
  onCopyAll,
  copyAllLabel,
}: {
  title: string;
  rows: DiffRow[];
  onCopyAll?: () => void;
  copyAllLabel?: string;
}) {
  return (
    <div className={styles.section}>
      <div className={styles.sectionHeaderRow}>
        <span className={styles.sectionTitle}>{title}</span>
        {onCopyAll && (
          <button className={`v-btn ${styles.smallBtn}`} onClick={onCopyAll}>
            {copyAllLabel}
          </button>
        )}
      </div>
      {rows.map((row) => (
        <CompareRow key={row.key} row={row} />
      ))}
    </div>
  );
}

function CompareRow({ row }: { row: DiffRow }) {
  const revealed = useVaultStore((s) => !!s.compareRevealed[row.key]);
  const toggleCompareReveal = useVaultStore((s) => s.toggleCompareReveal);
  const copyCompareRow = useVaultStore((s) => s.copyCompareRow);

  return (
    <div className={styles.diffRow}>
      <div className={styles.diffBody}>
        <div className={styles.diffKey}>{row.key}</div>
        <div className={styles.diffValue}>{revealed ? describeValues(row) : '••••••••'}</div>
      </div>
      <button className={`v-btn ${styles.diffBtn}`} onClick={() => toggleCompareReveal(row.key)}>
        {revealed ? 'hide' : 'show'}
      </button>
      {row.kind !== 'added' && (
        <button
          className={`v-btn ${styles.diffBtn}`}
          title="Copy A's value into B"
          onClick={() => void copyCompareRow(row.key, 'aToB')}
        >
          copy &rarr;
        </button>
      )}
      {row.kind !== 'removed' && (
        <button
          className={`v-btn ${styles.diffBtn}`}
          title="Copy B's value into A"
          onClick={() => void copyCompareRow(row.key, 'bToA')}
        >
          &larr; copy
        </button>
      )}
    </div>
  );
}

function describeValues(row: DiffRow): string {
  if (row.kind === 'changed') return `${row.oldValue ?? ''} → ${row.newValue ?? ''}`;
  if (row.kind === 'added') return row.newValue ?? '';
  return row.oldValue ?? '';
}
