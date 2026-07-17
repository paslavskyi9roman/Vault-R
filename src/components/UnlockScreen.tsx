import { useState, type FormEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';

export function UnlockScreen() {
  const vaultExists = useVaultStore((s) => s.vaultExists);
  const authBusy = useVaultStore((s) => s.authBusy);
  const authError = useVaultStore((s) => s.authError);
  const createVault = useVaultStore((s) => s.createVault);
  const unlockVault = useVaultStore((s) => s.unlockVault);

  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [remember, setRemember] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  const isCreate = !vaultExists;

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setLocalError(null);
    if (isCreate) {
      if (password.length < 8) {
        setLocalError('Master password must be at least 8 characters.');
        return;
      }
      if (password !== confirm) {
        setLocalError('Passwords do not match.');
        return;
      }
      void createVault(password, remember);
    } else {
      void unlockVault(password, remember);
    }
  }

  return (
    <div style={rootStyle}>
      <form style={cardStyle} onSubmit={handleSubmit}>
        <div style={glyphStyle}>&#10095;_</div>
        <div style={titleStyle}>{isCreate ? 'Create your vault' : 'Unlock vault'}</div>
        <div style={subStyle}>
          {isCreate
            ? 'Choose a master password. There is no recovery if you forget it — the vault is encrypted with a key derived only from this password.'
            : 'Enter your master password to unlock.'}
        </div>

        <input
          style={inputStyle}
          type="password"
          placeholder="Master password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoFocus
        />
        {isCreate && (
          <input
            style={inputStyle}
            type="password"
            placeholder="Confirm master password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
          />
        )}

        <label style={rememberRowStyle}>
          <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
          <span>Remember on this device</span>
        </label>

        {(localError || authError) && <div style={errorStyle}>{localError ?? authError}</div>}

        <button style={primaryBtnStyle} type="submit" disabled={authBusy}>
          {authBusy ? 'Please wait…' : isCreate ? 'Create vault' : 'Unlock'}
        </button>
      </form>
    </div>
  );
}

const rootStyle: React.CSSProperties = {
  height: '100vh',
  width: '100%',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  background: 'var(--bg)',
};

const cardStyle: React.CSSProperties = {
  width: '380px',
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
  padding: '36px 32px',
  background: 'var(--panel)',
  border: '1px solid var(--border)',
  borderRadius: '12px',
  animation: 'vaultFadeIn 0.25s ease',
};

const glyphStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  color: 'var(--accent)',
  fontWeight: 700,
  fontSize: '22px',
  marginBottom: '4px',
};

const titleStyle: React.CSSProperties = {
  fontSize: '19px',
  fontWeight: 700,
  color: 'var(--text)',
};

const subStyle: React.CSSProperties = {
  fontSize: '12.5px',
  color: 'var(--text-dim)',
  lineHeight: 1.5,
  marginBottom: '8px',
};

const inputStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '9px 12px',
  outline: 'none',
  width: '100%',
  boxSizing: 'border-box',
};

const rememberRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  fontSize: '12.5px',
  color: 'var(--text-dim)',
  cursor: 'pointer',
};

const errorStyle: React.CSSProperties = {
  fontSize: '12px',
  color: 'var(--danger)',
};

const primaryBtnStyle: React.CSSProperties = {
  fontFamily: 'var(--font-sans)',
  fontSize: '13px',
  fontWeight: 700,
  color: '#0b1210',
  background: 'var(--accent)',
  border: '1px solid var(--accent)',
  borderRadius: '6px',
  padding: '10px 12px',
  cursor: 'pointer',
  marginTop: '4px',
};
