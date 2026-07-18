import { useState, type FormEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { Spinner } from './Spinner';
import styles from './UnlockScreen.module.css';

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
      <div className={styles.root}>
        <form className={`${styles.card} v-enter`} onSubmit={handleSubmit}>
          <div className={styles.glyph}>&#10095;_</div>
          <div className={styles.title}>Use your recovery code</div>
          <div className={styles.sub}>
            Enter the recovery code from your recovery kit. You will be asked to set a new master
            password straight afterwards.
          </div>
          <input
            className={`v-input ${styles.input}`}
            placeholder="XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"
            value={recoveryCode}
            onChange={(e) => setRecoveryCode(e.target.value)}
            autoFocus
            spellCheck={false}
            autoComplete="off"
          />
          {authError && <div className={styles.error}>{authError}</div>}
          <button
            className={`v-btn v-btn--primary v-btn--block ${styles.submit}`}
            type="submit"
            disabled={authBusy}
            data-pending={authBusy}
          >
            Unlock with recovery code
            {authBusy && <Spinner size={14} />}
          </button>
          <button className={styles.linkBtn} type="button" onClick={() => setRecoveryMode(false)}>
            Back to master password
          </button>
        </form>
      </div>
    );
  }

  return (
    <div className={styles.root}>
      <form className={`${styles.card} v-enter`} onSubmit={handleSubmit}>
        <div className={styles.glyph}>&#10095;_</div>
        <div className={styles.title}>{isCreate ? 'Create your vault' : 'Unlock vault'}</div>
        <div className={styles.sub}>
          {isCreate
            ? 'Choose a master password. It is the only thing that decrypts this vault — create a recovery kit from settings afterwards so a forgotten password is not the end of the story.'
            : 'Enter your master password to unlock.'}
        </div>

        <input
          className={`v-input ${styles.input}`}
          type="password"
          placeholder="Master password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoFocus
        />
        {isCreate && (
          <input
            className={`v-input ${styles.input}`}
            type="password"
            placeholder="Confirm master password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
          />
        )}

        <label className={styles.rememberRow}>
          <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
          <span>Remember on this device</span>
        </label>

        {(localError || authError) && <div className={styles.error}>{localError ?? authError}</div>}

        <button
          className={`v-btn v-btn--primary v-btn--block ${styles.submit}`}
          type="submit"
          disabled={authBusy}
          data-pending={authBusy}
        >
          {isCreate ? 'Create vault' : 'Unlock'}
          {authBusy && <Spinner size={14} />}
        </button>

        {!isCreate && (
          <div className={styles.footerLinks}>
            <button className={styles.linkBtn} type="button" onClick={() => setRecoveryMode(true)}>
              Forgot password?
            </button>
            <button
              className={styles.linkBtn}
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
    <div className={styles.root}>
      <form className={`${styles.card} v-enter`} onSubmit={handleSubmit}>
        <div className={styles.glyph}>&#10095;_</div>
        <div className={styles.title}>Set a new master password</div>
        <div className={styles.sub}>
          Your vault is unlocked. Choose a new master password to lock it with — your recovery code
          keeps working.
        </div>
        <input
          className={`v-input ${styles.input}`}
          type="password"
          placeholder="New master password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoFocus
        />
        <input
          className={`v-input ${styles.input}`}
          type="password"
          placeholder="Confirm new master password"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
        />
        {(localError || authError) && <div className={styles.error}>{localError ?? authError}</div>}
        <button
          className={`v-btn v-btn--primary v-btn--block ${styles.submit}`}
          type="submit"
          disabled={authBusy}
          data-pending={authBusy}
        >
          Set master password
          {authBusy && <Spinner size={14} />}
        </button>
      </form>
    </div>
  );
}
