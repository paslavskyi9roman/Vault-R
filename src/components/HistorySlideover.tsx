import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle, closeXStyle } from './overlayStyles';
import { timeAgo } from '../lib/envColor';

export function HistorySlideover() {
  const historyOpen = useVaultStore((s) => s.historyOpen);
  const historySnapshots = useVaultStore((s) => s.historySnapshots);
  const closeHistory = useVaultStore((s) => s.closeHistory);
  const restoreSnapshot = useVaultStore((s) => s.restoreSnapshot);
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
          return (
            <div key={h.id} style={historyItemStyle}>
              <div style={historyTimeStyle}>{label}</div>
              <div style={historySummaryStyle}>{h.summary}</div>
              <button style={historyRestoreBtnStyle} onClick={() => void restoreSnapshot(h.id, label)}>
                Restore
              </button>
            </div>
          );
        })}
      </div>
    </>
  );
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
const historyTimeStyle: React.CSSProperties = { fontSize: '11px', color: 'var(--text-faint)', fontFamily: 'var(--font-mono)' };
const historySummaryStyle: React.CSSProperties = { fontSize: '13px', color: 'var(--text)' };
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
