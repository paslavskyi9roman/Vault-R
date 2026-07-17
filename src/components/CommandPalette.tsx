import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle } from './overlayStyles';

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

  if (!cmdkOpen) return null;

  const quickActions: QuickAction[] = [
    { id: 'a1', label: 'Import .env file', run: () => { closeCmdk(); openImport(); } },
    { id: 'a2', label: 'Export current .env', run: () => { closeCmdk(); void exportEnv(); } },
    { id: 'a3', label: 'View version history', run: () => { closeCmdk(); void openHistory(); } },
    { id: 'a4', label: 'Manage team access', run: () => { closeCmdk(); void openShare(); } },
  ];

  const showingQuickActions = !cmdkQuery.trim();

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeCmdk} />
      <div style={paletteStyle}>
        <input
          style={inputStyle}
          placeholder="Jump to a repo, environment, or secret&hellip;"
          value={cmdkQuery}
          onChange={(e) => void setCmdkQuery(e.target.value)}
          autoFocus
        />
        <div style={resultsStyle}>
          {showingQuickActions
            ? quickActions.map((a) => (
                <div key={a.id} style={resultRowStyle} onClick={a.run}>
                  <span style={resultLabelStyle}>{a.label}</span>
                  <span style={resultSubStyle}>Action</span>
                </div>
              ))
            : cmdkResults.map((r, i) => (
                <div key={i} style={resultRowStyle} onClick={() => void cmdkSelectResult(r)}>
                  <span style={resultLabelStyle}>{r.label}</span>
                  <span style={resultSubStyle}>{r.sublabel}</span>
                </div>
              ))}
          {!showingQuickActions && cmdkResults.length === 0 && (
            <div style={emptyStyle}>No matches.</div>
          )}
        </div>
      </div>
    </>
  );
}

const paletteStyle: React.CSSProperties = {
  position: 'fixed',
  top: '18%',
  left: '50%',
  transform: 'translateX(-50%)',
  width: '520px',
  maxWidth: '90vw',
  background: 'var(--panel)',
  border: '1px solid var(--border)',
  borderRadius: '10px',
  zIndex: 41,
  padding: '10px',
  animation: 'vaultFadeIn 0.15s ease',
};
const inputStyle: React.CSSProperties = {
  width: '100%',
  fontSize: '14px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '7px',
  color: 'var(--text)',
  padding: '10px 12px',
  outline: 'none',
  boxSizing: 'border-box',
  marginBottom: '6px',
};
const resultsStyle: React.CSSProperties = { display: 'flex', flexDirection: 'column' };
const resultRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '10px 10px',
  borderRadius: '6px',
  cursor: 'pointer',
};
const resultLabelStyle: React.CSSProperties = { fontFamily: 'var(--font-mono)', fontSize: '13px', color: 'var(--text)' };
const resultSubStyle: React.CSSProperties = { fontSize: '11.5px', color: 'var(--text-faint)' };
const emptyStyle: React.CSSProperties = { padding: '10px', fontSize: '12.5px', color: 'var(--text-faint)' };
