import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle, closeXStyle } from './overlayStyles';
import { timeAgo } from '../lib/envColor';
import type { DiffRow } from '../lib/api';

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

  if (!historyOpen) return null;

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeHistory} />
      <div style={slideoverStyle}>
        <div style={slideoverHeaderStyle}>
          <span style={slideoverTitleStyle}>Version history</span>
          <button style={closeXStyle} onClick={closeHistory}>
            &times;
          </button>
        </div>
        <div style={slideoverSubStyle}>
          {activeRepo && activeEnv ? `${activeRepo.name} / ${activeEnv.name}` : ''}
        </div>
        {historySnapshots.length === 0 && <div style={emptyStyle}>No snapshots yet.</div>}
        {historySnapshots.map((h) => {
          const label = timeAgo(h.createdAt);
          const expanded = expandedSnapshotId === h.id;
          return (
            <div key={h.id} style={historyItemStyle}>
              <div style={historyRowStyle} onClick={() => void toggleSnapshotDiff(h.id)}>
                <span style={chevronStyle}>{expanded ? '▾' : '▸'}</span>
                <div style={historyTextWrapStyle}>
                  <div style={historyTimeStyle}>{label}</div>
                  <div style={historySummaryStyle}>{h.summary}</div>
                </div>
                <ChangeBadges added={h.added} removed={h.removed} changed={h.changed} />
              </div>

              {expanded && <SnapshotDiff snapshotId={h.id} />}

              <button
                style={historyRestoreBtnStyle}
                onClick={() => void requestRestoreSnapshot(h.id, label)}
              >
                Restore
              </button>
            </div>
          );
        })}
      </div>
    </>
  );
}

function ChangeBadges({ added, removed, changed }: { added: number; removed: number; changed: number }) {
  if (!added && !removed && !changed) return <span style={noChangeStyle}>no change</span>;
  return (
    <span style={badgeWrapStyle}>
      {added > 0 && <span style={{ ...badgeStyle, color: 'var(--accent)' }}>+{added}</span>}
      {removed > 0 && <span style={{ ...badgeStyle, color: 'var(--danger)' }}>&minus;{removed}</span>}
      {changed > 0 && <span style={{ ...badgeStyle, color: 'var(--env-staging)' }}>~{changed}</span>}
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
    return <div style={diffEmptyStyle}>No variables changed in this snapshot.</div>;
  }

  return (
    <div style={diffWrapStyle}>
      {rows.map((row) => {
        const revealed = !!diffRevealed[row.key];
        return (
          <div key={row.key} style={diffRowStyle}>
            <span style={{ ...diffMarkStyle, color: kindColor(row.kind) }}>{kindMark(row.kind)}</span>
            <div style={diffBodyStyle}>
              <div style={diffKeyStyle}>{row.key}</div>
              <div style={diffValueStyle}>
                {revealed ? describeValues(row) : '••••••••'}
              </div>
            </div>
            <button style={diffIconBtnStyle} onClick={() => toggleDiffReveal(row.key)}>
              {revealed ? 'hide' : 'show'}
            </button>
            {row.kind !== 'added' && (
              <button
                style={diffIconBtnStyle}
                title={`Restore ${row.key} to this value`}
                onClick={() => void restoreVariableFromSnapshot(snapshotId, row.key)}
              >
                &#8617;
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

function kindColor(kind: DiffRow['kind']): string {
  if (kind === 'added') return 'var(--accent)';
  if (kind === 'removed') return 'var(--danger)';
  return 'var(--env-staging)';
}

function describeValues(row: DiffRow): string {
  if (row.kind === 'changed') return `${row.oldValue ?? ''} → ${row.newValue ?? ''}`;
  if (row.kind === 'added') return row.newValue ?? '';
  return row.oldValue ?? '';
}

const slideoverStyle: React.CSSProperties = {
  position: 'fixed',
  top: 0,
  right: 0,
  bottom: 0,
  width: '380px',
  background: 'var(--panel)',
  borderLeft: '1px solid var(--border)',
  zIndex: 41,
  padding: '22px 22px',
  overflowY: 'auto',
  animation: 'vaultSlideIn 0.22s ease',
};
const slideoverHeaderStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', marginBottom: '4px' };
const slideoverTitleStyle: React.CSSProperties = { fontSize: '16px', fontWeight: 700, color: 'var(--text)', flex: 1 };
const slideoverSubStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12px',
  color: 'var(--text-faint)',
  marginBottom: '18px',
};
const historyItemStyle: React.CSSProperties = {
  padding: '12px 0',
  borderBottom: '1px solid var(--border-light)',
  display: 'flex',
  flexDirection: 'column',
  gap: '5px',
};
const historyRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'flex-start',
  gap: '8px',
  cursor: 'pointer',
};
const chevronStyle: React.CSSProperties = {
  color: 'var(--text-faint)',
  fontSize: '10px',
  width: '10px',
  paddingTop: '3px',
  flexShrink: 0,
};
const historyTextWrapStyle: React.CSSProperties = { flex: 1, display: 'flex', flexDirection: 'column', gap: '3px' };
const historyTimeStyle: React.CSSProperties = { fontSize: '11px', color: 'var(--text-faint)', fontFamily: 'var(--font-mono)' };
const historySummaryStyle: React.CSSProperties = { fontSize: '13px', color: 'var(--text)' };
const badgeWrapStyle: React.CSSProperties = { display: 'flex', gap: '5px', flexShrink: 0, paddingTop: '2px' };
const badgeStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '10.5px',
  fontWeight: 700,
};
const noChangeStyle: React.CSSProperties = { fontSize: '10.5px', color: 'var(--text-faint)', flexShrink: 0 };
const historyRestoreBtnStyle: React.CSSProperties = {
  alignSelf: 'flex-start',
  fontSize: '11px',
  fontWeight: 700,
  color: 'var(--accent)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 10px',
  cursor: 'pointer',
};
const emptyStyle: React.CSSProperties = { fontSize: '12.5px', color: 'var(--text-faint)' };
const diffWrapStyle: React.CSSProperties = {
  margin: '4px 0 2px 18px',
  borderLeft: '1px solid var(--border)',
  paddingLeft: '10px',
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
};
const diffEmptyStyle: React.CSSProperties = {
  margin: '4px 0 2px 18px',
  fontSize: '11.5px',
  color: 'var(--text-faint)',
};
const diffRowStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '6px' };
const diffMarkStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12px',
  fontWeight: 700,
  width: '10px',
  flexShrink: 0,
};
const diffBodyStyle: React.CSSProperties = { flex: 1, minWidth: 0 };
const diffKeyStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12px',
  color: 'var(--key)',
};
const diffValueStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '11px',
  color: 'var(--text-faint)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};
const diffIconBtnStyle: React.CSSProperties = {
  fontSize: '10px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '4px',
  padding: '3px 6px',
  cursor: 'pointer',
  flexShrink: 0,
};
