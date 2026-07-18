import { useState, type FormEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';

export function UnlockScreen() {
  const vaultExists = useVaultStore((s) => s.vaultExists);
  const authBusy = useVaultStore((s) => s.authBusy);
  const authError = useVaultStore((s) => s.authError);
  const createVault = useVaultStore((s) => s.createVault);
  const unlockVault = useVaultStore((s) => s.unlockVault);
  const recoveryMode = useVaultStore((s) => s.recoveryMode);
  const setRecoveryMode = useVaultStore((s) => s.setRecoveryMode);
  const recoveryCode = useVaultStore((s) => s.recoveryCode);
  const setRecoveryCode = useVaultStore((s) => s.setRecoveryCode);
  const unlockWithRecovery = useVaultStore((s) => s.unlockWithRecovery);
  const restoreBackupFromUnlock = useVaultStore((s) => s.restoreBackupFromUnlock);

  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [remember, setRemember] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  const isCreate = !vaultExists;

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setLocalError(null);
    if (recoveryMode) {
      void unlockWithRecovery();
      return;
    }
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

  if (recoveryMode) {
    return (
      <div style={rootStyle}>
        <form style={cardStyle} onSubmit={handleSubmit}>
          <div style={glyphStyle}>&#10095;_</div>
          <div style={titleStyle}>Use your recovery code</div>
          <div style={subStyle}>
            Enter the recovery code from your recovery kit. You will be asked to set a new master
            password straight afterwards.
          </div>
          <input
            style={inputStyle}
            placeholder="XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"
            value={recoveryCode}
            onChange={(e) => setRecoveryCode(e.target.value)}
            autoFocus
            spellCheck={false}
            autoComplete="off"
          />
          {authError && <div style={errorStyle}>{authError}</div>}
          <button style={primaryBtnStyle} type="submit" disabled={authBusy}>
            {authBusy ? 'Please wait…' : 'Unlock with recovery code'}
          </button>
          <button style={linkBtnStyle} type="button" onClick={() => setRecoveryMode(false)}>
            Back to master password
          </button>
        </form>
      </div>
    );
  }

  return (
    <div style={rootStyle}>
      <form style={cardStyle} onSubmit={handleSubmit}>
        <div style={glyphStyle}>&#10095;_</div>
        <div style={titleStyle}>{isCreate ? 'Create your vault' : 'Unlock vault'}</div>
        <div style={subStyle}>
          {isCreate
            ? 'Choose a master password. It is the only thing that decrypts this vault — create a recovery kit from settings afterwards so a forgotten password is not the end of the story.'
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

        {!isCreate && (
          <div style={footerLinksStyle}>
            <button style={linkBtnStyle} type="button" onClick={() => setRecoveryMode(true)}>
              Forgot password?
            </button>
            <button
              style={linkBtnStyle}
              type="button"
              onClick={() => void restoreBackupFromUnlock()}
            >
              Restore from backup…
            </button>
          </div>
        )}
      </form>
    </div>
  );
}

/// Shown after a recovery unlock: the vault is open but its master password is
/// unknown, so setting one is the only way forward.
export function ResetPasswordScreen() {
  const authBusy = useVaultStore((s) => s.authBusy);
  const authError = useVaultStore((s) => s.authError);
  const resetPasswordAfterRecovery = useVaultStore((s) => s.resetPasswordAfterRecovery);

  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setLocalError(null);
    if (password.length < 8) {
      setLocalError('Master password must be at least 8 characters.');
      return;
    }
    if (password !== confirm) {
      setLocalError('Passwords do not match.');
      return;
    }
    void resetPasswordAfterRecovery(password);
  }

  return (
    <div style={rootStyle}>
      <form style={cardStyle} onSubmit={handleSubmit}>
        <div style={glyphStyle}>&#10095;_</div>
        <div style={titleStyle}>Set a new master password</div>
        <div style={subStyle}>
          Your vault is unlocked. Choose a new master password to lock it with — your recovery code
          keeps working.
        </div>
        <input
          style={inputStyle}
          type="password"
          placeholder="New master password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoFocus
        />
        <input
          style={inputStyle}
          type="password"
          placeholder="Confirm new master password"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
        />
        {(localError || authError) && <div style={errorStyle}>{localError ?? authError}</div>}
        <button style={primaryBtnStyle} type="submit" disabled={authBusy}>
          {authBusy ? 'Please wait…' : 'Set master password'}
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

const footerLinksStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  gap: '8px',
  marginTop: '2px',
};

const linkBtnStyle: React.CSSProperties = {
  fontSize: '11.5px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: 'none',
  cursor: 'pointer',
  padding: 0,
  textDecoration: 'underline',
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
