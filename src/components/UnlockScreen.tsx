import { useState, type FormEvent, type ReactNode } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import type { VaultStatus } from '../lib/api';

const iconProps = {
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.75,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
  'aria-hidden': true,
};

const LockIcon = () => (
  <svg {...iconProps}>
    <rect x="4" y="10.5" width="16" height="10" rx="2" />
    <path d="M8 10.5V7a4 4 0 0 1 8 0v3.5" />
  </svg>
);

const AlertIcon = () => (
  <svg {...iconProps}>
    <path d="M12 4.5 2.8 20h18.4L12 4.5Z" />
    <path d="M12 10v4" />
    <path d="M12 17.2h.01" />
  </svg>
);

const KeyIcon = () => (
  <svg {...iconProps}>
    <circle cx="8" cy="15" r="4" />
    <path d="M11 12.5 20 3.5" />
    <path d="M17.5 6l2.5 2.5" />
  </svg>
);

const ArchiveIcon = () => (
  <svg {...iconProps}>
    <rect x="3" y="4.5" width="18" height="4.5" rx="1.5" />
    <path d="M5 9v9a1.5 1.5 0 0 0 1.5 1.5h11A1.5 1.5 0 0 0 19 18V9" />
    <path d="M10 13h4" />
  </svg>
);

const PasswordIcon = () => (
  <svg {...iconProps}>
    <rect x="3" y="7" width="18" height="10" rx="2.5" />
    <path d="M7.5 12h.01M12 12h.01M16.5 12h.01" />
  </svg>
);

const RetryIcon = () => (
  <svg {...iconProps}>
    <path d="M20 12a8 8 0 1 1-2.6-5.9" />
    <path d="M20 4v4.5h-4.5" />
  </svg>
);

const EyeIcon = () => (
  <svg {...iconProps}>
    <path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z" />
    <circle cx="12" cy="12" r="2.75" />
  </svg>
);

const EyeOffIcon = () => (
  <svg {...iconProps}>
    <path d="M10.7 6.2A8.9 8.9 0 0 1 12 6c6 0 9.5 6 9.5 6a16 16 0 0 1-3.2 3.8" />
    <path d="M6.3 8.1A16 16 0 0 0 2.5 12S6 18 12 18a8.8 8.8 0 0 0 3.5-.7" />
    <path d="M4 4l16 16" />
  </svg>
);

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
    <div className="auth-row">
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
    <div className="auth-manifest">
      <div className="auth-manifest-head">
        <span className="auth-manifest-label">Manifest</span>
        <span className={`auth-state ${found ? 'is-sealed' : 'is-missing'}`}>
          {found ? <LockIcon /> : <AlertIcon />}
          {found ? 'Sealed' : 'No vault file'}
        </span>
      </div>

      <dl className="auth-rows">
        <ManifestRow term="location">{status.dir}</ManifestRow>

        <ManifestRow term="file">
          {found ? (
            <>
              <strong>{status.fileName}</strong> · {formatBytes(status.bytes)}
            </>
          ) : (
            <span className="auth-warn">{status.fileName} is not in this folder</span>
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
                <strong>v1</strong> · <span className="auth-warn">legacy</span>, upgrades on your
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
            <span className="auth-warn">
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
    <div className="auth-root">
      <div className="auth-shell">
        <div className="auth-brand">
          <span className="auth-eyebrow">Local secrets vault</span>
          <span className="auth-wordmark">
            VAULT<em>·R</em>
          </span>
          <p className="auth-tagline">{tagline}</p>
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
    <div className="auth-field">
      <input
        className="auth-input has-toggle"
        type={shown ? 'text' : 'password'}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        autoFocus={autoFocus}
      />
      <button
        className="auth-reveal"
        type="button"
        onClick={() => setShown((s) => !s)}
        aria-label={shown ? 'Hide password' : 'Show password'}
        title={shown ? 'Hide password' : 'Show password'}
      >
        {shown ? <EyeOffIcon /> : <EyeIcon />}
      </button>
    </div>
  );
}

function ErrorNote({ children }: { children: ReactNode }) {
  return (
    <div className="auth-error" role="alert">
      <AlertIcon />
      <span>{children}</span>
    </div>
  );
}

function Route({ icon, label, onClick }: { icon: ReactNode; label: string; onClick: () => void }) {
  return (
    <button className="auth-route" type="button" onClick={onClick}>
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
      <div className="auth-card">
        <div className="auth-title">Can&rsquo;t read the vault folder</div>
        <p className="auth-sub">
          The app could not check whether a vault is on this device, so it will not offer to make a
          new one — an existing vault stays where it is.
        </p>
        <ErrorNote>{initError}</ErrorNote>
        <button className="auth-submit" type="button" onClick={() => void init()} disabled={checkingVault}>
          {checkingVault ? 'Checking…' : 'Check again'}
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
        <form className="auth-card" onSubmit={handleSubmit}>
          <div className="auth-title">Use your recovery code</div>
          <p className="auth-sub">
            Enter the code from your recovery kit. You&rsquo;ll set a new master password straight
            afterwards.
          </p>
          <div className="auth-field">
            <input
              className="auth-input"
              placeholder="XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"
              value={recoveryCode}
              onChange={(e) => setRecoveryCode(e.target.value)}
              autoFocus
              spellCheck={false}
              autoComplete="off"
            />
          </div>
          {authError && <ErrorNote>{authError}</ErrorNote>}
          <button className="auth-submit" type="submit" disabled={authBusy}>
            {authBusy ? 'Unlocking…' : 'Unlock with recovery code'}
          </button>
          <div className="auth-routes">
            <Route
              icon={<PasswordIcon />}
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
      <form className="auth-card" onSubmit={handleSubmit}>
        <div className="auth-title">{isCreate ? 'Create your vault' : 'Unlock vault'}</div>
        <p className="auth-sub">
          {isCreate
            ? 'Choose a master password. It is the only thing that decrypts this vault — make a recovery kit from settings straight afterwards.'
            : 'Enter your master password to unlock this vault.'}
        </p>

        <PasswordField
          value={password}
          onChange={setPassword}
          placeholder={isCreate ? 'Master password' : 'Master password'}
          autoFocus
        />
        {isCreate && (
          <PasswordField
            value={confirm}
            onChange={setConfirm}
            placeholder="Confirm master password"
          />
        )}

        <label className="auth-remember">
          <input
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
          />
          <span>Remember on this device</span>
        </label>

        {error && <ErrorNote>{error}</ErrorNote>}

        <button className="auth-submit" type="submit" disabled={authBusy}>
          {authBusy ? 'Working…' : isCreate ? 'Create vault' : 'Unlock'}
        </button>

        <div className="auth-routes">
          <span className="auth-routes-label">
            {isCreate ? 'Already have a vault?' : 'Can’t get in?'}
          </span>

          {isCreate && (
            <Route
              icon={<PasswordIcon />}
              label="Unlock an existing vault instead"
              onClick={() => setForceExisting(true)}
            />
          )}
          <Route
            icon={<KeyIcon />}
            label="Use a recovery code"
            onClick={() => setRecoveryMode(true)}
          />
          <Route
            icon={<ArchiveIcon />}
            label="Restore from a backup file"
            onClick={() => void restoreBackupFromUnlock()}
          />
          {!isCreate && !vaultExists && (
            <Route
              icon={<RetryIcon />}
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

  const error = localError ?? authError;

  return (
    <AuthLayout tagline="Your vault is open. Give it a new master password to lock it with — your recovery code keeps working.">
      <form className="auth-card" onSubmit={handleSubmit}>
        <div className="auth-title">Set a new master password</div>
        <p className="auth-sub">
          This replaces the password you could not recall. Nothing else about the vault changes.
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
        {error && <ErrorNote>{error}</ErrorNote>}
        <button className="auth-submit" type="submit" disabled={authBusy}>
          {authBusy ? 'Saving…' : 'Set master password'}
        </button>
      </form>
    </AuthLayout>
  );
}
