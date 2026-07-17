import { useVaultStore } from '../store/useVaultStore';

export function Toast() {
  const toast = useVaultStore((s) => s.toast);
  if (!toast) return null;
  return <div style={toastStyle}>{toast}</div>;
}

const toastStyle: React.CSSProperties = {
  position: 'fixed',
  bottom: '24px',
  left: '50%',
  transform: 'translateX(-50%)',
  background: 'var(--panel-3)',
  border: '1px solid var(--border)',
  color: 'var(--text)',
  fontSize: '13px',
  padding: '10px 18px',
  borderRadius: '8px',
  zIndex: 60,
  animation: 'vaultFadeIn 0.2s ease',
};
