import { useState, type FormEvent, type ReactNode } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import type { VaultStatus } from '../lib/api';
import { Spinner } from './Spinner';
import {
  LockIcon,
  WarningIcon,
  KeyIcon,
  ArchiveIcon,
  PasswordIcon,
  RestoreIcon,
  EyeIcon,
  EyeOffIcon,
} from './icons';
import styles from './UnlockScreen.module.css';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatWhen(ms: number | null): string {
  if (!ms) return 'unknown';
  return new Date(ms).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

function ManifestRow({ term, children }: { term: string; children: ReactNode }) {
  return (
    <div className={`${styles.row} v-enter`}>
      <dt>{term}</dt>
      <dd>{children}</dd>
    </div>
  );
}

/// Stays on screen when no vault was found, so "create a vault" is never the
/// only thing the app has to say about secrets the user knows they saved.
function Manifest({ status }: { status: VaultStatus | null }) {
  if (!status) return null;
  const found = status.exists;

  return (
    <div className={styles.manifest}>
      <div className={`${styles.manifestHead} v-enter`}>
        <span className={styles.manifestLabel}>Manifest</span>
        <span className={styles.state} data-found={found}>
          {found ? <LockIcon size={11} /> : <WarningIcon size={11} />}
          {found ? 'Sealed' : 'No vault file'}
        </span>
      </div>

      <dl className={styles.rows}>
        <ManifestRow term="location">{status.dir}</ManifestRow>

        <ManifestRow term="file">
          {found ? (
            <>
              <strong>{status.fileName}</strong> · {formatBytes(status.bytes)}
            </>
          ) : (
            <span className={styles.warn}>{status.fileName} is not in this folder</span>
          )}
        </ManifestRow>

        {found && (
          <ManifestRow term="format">
            {status.format === 2 ? (
              <>
                <strong>v2</strong> · current
              </>
            ) : (
              <>
                <strong>v1</strong> · <span className={styles.warn}>legacy</span>, upgrades on your
                next password unlock
              </>
            )}
          </ManifestRow>
        )}

        {found && <ManifestRow term="updated">{formatWhen(status.modifiedMs)}</ManifestRow>}

        <ManifestRow term="backups">
          {status.backupCount > 0 ? (
            <>
              <strong>{status.backupCount}</strong> on this device
            </>
          ) : (
            <span className={styles.warn}>
              none yet{status.format === 1 ? ' — v1 vaults are not backed up' : ''}
            </span>
          )}
        </ManifestRow>
      </dl>
    </div>
  );
}

function AuthLayout({ tagline, aside, children }: { tagline: string; aside?: ReactNode; children: ReactNode }) {
  return (
    <div className={styles.root}>
      <div className={styles.shell}>
        <div className={styles.brand}>
          <span className={styles.eyebrow}>Local secrets vault</span>
          <span className={styles.wordmark}>
            VAULT<em>·R</em>
          </span>
          <p className={styles.tagline}>{tagline}</p>
          {aside}
        </div>
        {children}
      </div>
    </div>
  );
}

function PasswordField({
  value,
  onChange,
  placeholder,
  autoFocus,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  autoFocus?: boolean;
}) {
  const [shown, setShown] = useState(false);
  return (
    <div className={styles.field}>
      <input
        className={`${styles.input} ${styles.inputToggle}`}
        type={shown ? 'text' : 'password'}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        autoFocus={autoFocus}
      />
      <button
        className={styles.reveal}
        type="button"
        onClick={() => setShown((s) => !s)}
        aria-label={shown ? 'Hide password' : 'Show password'}
        title={shown ? 'Hide password' : 'Show password'}
      >
        {shown ? <EyeOffIcon size={15} /> : <EyeIcon size={15} />}
      </button>
    </div>
  );
}

function ErrorNote({ children }: { children: ReactNode }) {
  return (
    <div className={styles.error} role="alert">
      <WarningIcon size={14} className={styles.errorIcon} />
      <span>{children}</span>
    </div>
  );
}

function Route({ icon, label, onClick }: { icon: ReactNode; label: string; onClick: () => void }) {
  return (
    <button className={styles.route} type="button" onClick={onClick}>
      <span>{label}</span>
      {icon}
    </button>
  );
}

/// Never offers to create a vault: we do not know whether one exists, and
/// creating over a vault we failed to read is the unrecoverable mistake here.
export function StartupErrorScreen() {
  const initError = useVaultStore((s) => s.initError);
  const vaultStatus = useVaultStore((s) => s.vaultStatus);
  const checkingVault = useVaultStore((s) => s.checkingVault);
  const init = useVaultStore((s) => s.init);

  return (
    <AuthLayout
      tagline="Your secrets stay encrypted on this device until you unlock them."
      aside={<Manifest status={vaultStatus} />}
    >
      <div className={`${styles.card} v-enter`}>
        <div className={styles.title}>Can&rsquo;t read the vault folder</div>
        <p className={styles.sub}>
          The app could not check whether a vault is on this device, so it will not offer to make a
          new one — an existing vault stays where it is.
        </p>
        <ErrorNote>{initError}</ErrorNote>
        <button
          className={`v-btn v-btn--primary v-btn--block ${styles.submit}`}
          type="button"
          onClick={() => void init()}
          disabled={checkingVault}
          data-pending={checkingVault}
        >
          Check again
          {checkingVault && <Spinner size={14} />}
        </button>
      </div>
    </AuthLayout>
  );
}

export function UnlockScreen() {
  const vaultExists = useVaultStore((s) => s.vaultExists);
  const vaultStatus = useVaultStore((s) => s.vaultStatus);
  const forceExisting = useVaultStore((s) => s.forceExisting);
  const setForceExisting = useVaultStore((s) => s.setForceExisting);
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

  const isCreate = !vaultExists && !forceExisting;
  const error = localError ?? authError;

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
        setLocalError('Those two passwords do not match.');
        return;
      }
      void createVault(password, remember);
    } else {
      void unlockVault(password, remember);
    }
  }

  if (recoveryMode) {
    return (
      <AuthLayout
        tagline="Your secrets stay encrypted on this device until you unlock them."
        aside={<Manifest status={vaultStatus} />}
      >
        <form className={`${styles.card} v-enter`} onSubmit={handleSubmit}>
          <div className={styles.title}>Use your recovery code</div>
          <p className={styles.sub}>
            Enter the code from your recovery kit. You&rsquo;ll set a new master password straight
            afterwards.
          </p>
          <div className={styles.field}>
            <input
              className={styles.input}
              placeholder="XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"
              value={recoveryCode}
              onChange={(e) => setRecoveryCode(e.target.value)}
              autoFocus
              spellCheck={false}
              autoComplete="off"
            />
          </div>
          {authError && <ErrorNote>{authError}</ErrorNote>}
          <button
            className={`v-btn v-btn--primary v-btn--block ${styles.submit}`}
            type="submit"
            disabled={authBusy}
            data-pending={authBusy}
          >
            Unlock with recovery code
            {authBusy && <Spinner size={14} />}
          </button>
          <div className={styles.routes}>
            <Route
              icon={<PasswordIcon size={13} className={styles.routeIcon} />}
              label="Back to master password"
              onClick={() => setRecoveryMode(false)}
            />
          </div>
        </form>
      </AuthLayout>
    );
  }

  return (
    <AuthLayout
      tagline="Your secrets stay encrypted on this device until you unlock them."
      aside={<Manifest status={vaultStatus} />}
    >
      <form className={`${styles.card} v-enter`} onSubmit={handleSubmit}>
        <div className={styles.title}>{isCreate ? 'Create your vault' : 'Unlock vault'}</div>
        <p className={styles.sub}>
          {isCreate
            ? 'Choose a master password. It is the only thing that decrypts this vault — make a recovery kit from settings straight afterwards.'
            : 'Enter your master password to unlock this vault.'}
        </p>

        <PasswordField
          value={password}
          onChange={setPassword}
          placeholder="Master password"
          autoFocus
        />
        {isCreate && (
          <PasswordField
            value={confirm}
            onChange={setConfirm}
            placeholder="Confirm master password"
          />
        )}

        <label className={styles.remember}>
          <input
            className="v-check"
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
          />
          <span>Remember on this device</span>
        </label>

        {error && <ErrorNote>{error}</ErrorNote>}

        <button
          className={`v-btn v-btn--primary v-btn--block ${styles.submit}`}
          type="submit"
          disabled={authBusy}
          data-pending={authBusy}
        >
          {isCreate ? 'Create vault' : 'Unlock'}
          {authBusy && <Spinner size={14} />}
        </button>

        <div className={styles.routes}>
          <span className={styles.routesLabel}>
            {isCreate ? 'Already have a vault?' : "Can't get in?"}
          </span>

          {isCreate && (
            <Route
              icon={<PasswordIcon size={13} className={styles.routeIcon} />}
              label="Unlock an existing vault instead"
              onClick={() => setForceExisting(true)}
            />
          )}
          <Route
            icon={<KeyIcon size={13} className={styles.routeIcon} />}
            label="Use a recovery code"
            onClick={() => setRecoveryMode(true)}
          />
          <Route
            icon={<ArchiveIcon size={13} className={styles.routeIcon} />}
            label="Restore from a backup file"
            onClick={() => void restoreBackupFromUnlock()}
          />
          {!isCreate && !vaultExists && (
            <Route
              icon={<RestoreIcon size={13} className={styles.routeIcon} />}
              label="Back to setting up a new vault"
              onClick={() => setForceExisting(false)}
            />
          )}
        </div>
      </form>
    </AuthLayout>
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
      setLocalError('Those two passwords do not match.');
      return;
    }
    void resetPasswordAfterRecovery(password);
  }

  return (
    <AuthLayout tagline="Your vault is open. Choose the password it locks with from now on.">
      <form className={`${styles.card} v-enter`} onSubmit={handleSubmit}>
        <div className={styles.title}>Set a new master password</div>
        <p className={styles.sub}>
          Your recovery code keeps working, so store it somewhere safe alongside this password.
        </p>
        <PasswordField
          value={password}
          onChange={setPassword}
          placeholder="New master password"
          autoFocus
        />
        <PasswordField
          value={confirm}
          onChange={setConfirm}
          placeholder="Confirm new master password"
        />
        {(localError || authError) && <ErrorNote>{localError ?? authError}</ErrorNote>}
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
    </AuthLayout>
  );
}
