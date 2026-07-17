import { useVaultStore } from '../store/useVaultStore';

const isMac = navigator.platform.toLowerCase().includes('mac');

export function TopBar() {
  const toggleCmdk = useVaultStore((s) => s.toggleCmdk);
  const openImport = useVaultStore((s) => s.openImport);
  const openShare = useVaultStore((s) => s.openShare);
  const exportEnv = useVaultStore((s) => s.exportEnv);
  const replayOnboarding = useVaultStore((s) => s.replayOnboarding);
  const lockVault = useVaultStore((s) => s.lockVault);

  return (
    <div style={topBarStyle}>
      <div style={logoWrapStyle}>
        <span style={logoGlyphStyle}>&#10095;_</span>
        <span style={logoTextStyle}>vault</span>
      </div>
      <div style={cmdkTriggerStyle} onClick={toggleCmdk}>
        <span style={cmdkIconStyle}>&#8981;</span>
        <span style={cmdkTriggerTextStyle}>Search repos, environments, secrets&hellip;</span>
        <span style={cmdkShortcutStyle}>{isMac ? '⌘K' : 'Ctrl+K'}</span>
      </div>
      <div style={topBarActionsStyle}>
        <button style={topBarBtnStyle} onClick={openImport}>
          Import
        </button>
        <button style={topBarBtnStyle} onClick={openShare}>
          Share
        </button>
        <button style={topBarBtnPrimaryStyle} onClick={() => void exportEnv()}>
          Export .env
        </button>
        <button style={replayBtnStyle} onClick={replayOnboarding} title="Replay onboarding">
          ?
        </button>
        <button style={lockBtnStyle} onClick={() => void lockVault()} title="Lock vault">
          &#128274;
        </button>
      </div>
    </div>
  );
}

const topBarStyle: React.CSSProperties = {
  height: '54px',
  flexShrink: 0,
  display: 'flex',
  alignItems: 'center',
  gap: '18px',
  padding: '0 18px',
  borderBottom: '1px solid var(--border)',
  background: 'var(--panel)',
};

const logoWrapStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '7px', flexShrink: 0 };
const logoGlyphStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  color: 'var(--accent)',
  fontWeight: 700,
  fontSize: '15px',
};
const logoTextStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontWeight: 700,
  fontSize: '15px',
  color: 'var(--text)',
  letterSpacing: '0.3px',
};

const cmdkTriggerStyle: React.CSSProperties = {
  flex: 1,
  maxWidth: '520px',
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '7px',
  padding: '7px 12px',
  cursor: 'pointer',
};
const cmdkIconStyle: React.CSSProperties = { color: 'var(--text-faint)', fontSize: '13px' };
const cmdkTriggerTextStyle: React.CSSProperties = { color: 'var(--text-faint)', fontSize: '13px', flex: 1 };
const cmdkShortcutStyle: React.CSSProperties = {
  color: 'var(--text-faint)',
  fontSize: '11px',
  fontFamily: 'var(--font-mono)',
  background: 'var(--panel-3)',
  borderRadius: '4px',
  padding: '1px 6px',
};

const topBarActionsStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  marginLeft: 'auto',
};
const topBarBtnStyle: React.CSSProperties = {
  fontSize: '12.5px',
  fontWeight: 600,
  color: 'var(--text-dim)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  padding: '7px 12px',
  cursor: 'pointer',
};
const topBarBtnPrimaryStyle: React.CSSProperties = {
  fontSize: '12.5px',
  fontWeight: 700,
  color: '#0b1210',
  background: 'var(--accent)',
  border: '1px solid var(--accent)',
  borderRadius: '6px',
  padding: '7px 12px',
  cursor: 'pointer',
};
const replayBtnStyle: React.CSSProperties = {
  width: '28px',
  height: '28px',
  borderRadius: '50%',
  border: '1px solid var(--border)',
  background: 'transparent',
  color: 'var(--text-faint)',
  cursor: 'pointer',
  fontSize: '12px',
};
const lockBtnStyle: React.CSSProperties = {
  width: '28px',
  height: '28px',
  borderRadius: '50%',
  border: '1px solid var(--border)',
  background: 'transparent',
  color: 'var(--text-faint)',
  cursor: 'pointer',
  fontSize: '12px',
};
