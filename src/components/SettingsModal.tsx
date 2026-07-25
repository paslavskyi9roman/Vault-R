import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import { Spinner } from './Spinner';
import { CloseIcon } from './icons';
import { timeAgo } from '../lib/envColor';
import styles from './SettingsModal.module.css';

const AUTO_LOCK_CHOICES = [
  { minutes: 0, label: 'Off' },
  { minutes: 5, label: '5 min' },
  { minutes: 15, label: '15 min' },
  { minutes: 30, label: '30 min' },
  { minutes: 60, label: '1 hour' },
];

export function SettingsModal() {
  const settingsOpen = useVaultStore((s) => s.settingsOpen);
  const closeSettings = useVaultStore((s) => s.closeSettings);
  const needsMigration = useVaultStore((s) => s.needsMigration);
  const recoveryCodeOnce = useVaultStore((s) => s.recoveryCodeOnce);

  const { mounted, state } = usePresence(settingsOpen, 120);
  if (!mounted) return null;

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeSettings} />
      <div className="v-modal-wrap">
        <div className={`v-modal ${styles.dialog} is-${state}`}>
          <div className="v-modal-header">
            <span className="v-modal-title">Vault settings</span>
            <button className="v-close-x" onClick={closeSettings} aria-label="Close">
              <CloseIcon size={13} />
            </button>
          </div>
          <div className="v-modal-sub">Security, backups and recovery for this device.</div>

          {needsMigration && <MigrationNotice />}
          {recoveryCodeOnce ? <RecoveryCodeReveal code={recoveryCodeOnce} /> : <RecoverySection />}
          <AutoLockSection />
          <BackupSection />
          <PasswordSection />
          <AboutFooter />
        </div>
      </div>
    </>
  );
}

function MigrationNotice() {
  return (
    <div className={styles.notice}>
      This vault still uses the original storage format. Lock it and unlock once with your master
      password to upgrade it — backups, password changes and recovery kits become available after
      that.
    </div>
  );
}

// Bug reports ask for a version, so there has to be somewhere to read one
// without dropping to the CLI for `vault --version`.
function AboutFooter() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    void getVersion().then(setVersion, () => setVersion(null));
  }, []);

  return <div className={styles.about}>Vault-R{version ? ` v${version}` : ''}</div>;
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className={styles.section}>
      <div className={styles.sectionTitle}>{title}</div>
      {children}
    </div>
  );
}

function AutoLockSection() {
  const autoLockMinutes = useVaultStore((s) => s.autoLockMinutes);
  const setAutoLockMinutes = useVaultStore((s) => s.setAutoLockMinutes);

  return (
    <Section title="Auto-lock">
      <div className={styles.sectionBody}>
        Lock the vault automatically after a period with no keyboard or mouse activity.
      </div>
      <div className={styles.choiceRow}>
        {AUTO_LOCK_CHOICES.map((choice) => (
          <button
            key={choice.minutes}
            className={`v-btn ${styles.choiceBtn}`}
            aria-pressed={autoLockMinutes === choice.minutes}
            onClick={() => void setAutoLockMinutes(choice.minutes)}
          >
            {choice.label}
          </button>
        ))}
      </div>
    </Section>
  );
}

function BackupSection() {
  const backups = useVaultStore((s) => s.backups);
  const exportBackup = useVaultStore((s) => s.exportBackup);
  const needsMigration = useVaultStore((s) => s.needsMigration);

  return (
    <Section title="Backups">
      <div className={styles.sectionBody}>
        A backup is a copy of your vault file and stays encrypted — it can only be opened with the
        master password it had when the copy was taken. Vault-R also keeps the last 10 copies
        automatically, on every unlock and before anything destructive.
      </div>
      <div className={styles.sectionBody}>
        {backups.length > 0 ? (
          <span className={styles.mutedMono}>
            Last automatic backup {timeAgo(backups[0].createdAt)} · {backups.length} kept
          </span>
        ) : (
          <span className={styles.mutedMono}>No automatic backups yet.</span>
        )}
      </div>
      <button
        className={`v-btn ${styles.selfStart}`}
        onClick={() => void exportBackup()}
        disabled={needsMigration}
      >
        Export encrypted backup…
      </button>
    </Section>
  );
}

function RecoverySection() {
  const hasRecoveryCode = useVaultStore((s) => s.hasRecoveryCode);
  const generateRecoveryCode = useVaultStore((s) => s.generateRecoveryCode);
  const needsMigration = useVaultStore((s) => s.needsMigration);

  return (
    <Section title="Recovery kit">
      <div className={styles.sectionBody}>
        {hasRecoveryCode
          ? 'This vault has a recovery kit. The code was shown once when you created it — if you no longer have it, generate a new one.'
          : 'Without a recovery kit, forgetting your master password means losing every secret in this vault. A recovery code unlocks it if that happens.'}
      </div>
      <button
        className={`v-btn ${styles.selfStart}`}
        onClick={() => void generateRecoveryCode()}
        disabled={needsMigration}
      >
        {hasRecoveryCode ? 'Generate a new recovery kit…' : 'Create recovery kit'}
      </button>
    </Section>
  );
}

/// The code exists in recoverable form for exactly as long as this panel is on
/// screen, so the copy has to be blunt about saving it now.
function RecoveryCodeReveal({ code }: { code: string }) {
  const dismissRecoveryCode = useVaultStore((s) => s.dismissRecoveryCode);
  const saveRecoveryCodeToFile = useVaultStore((s) => s.saveRecoveryCodeToFile);
  const copyPlainText = useVaultStore((s) => s.copyPlainText);

  return (
    <Section title="Your recovery code">
      <div className={styles.sectionBody}>
        Write this down or save it somewhere safe and offline.{' '}
        <strong className={styles.strong}>It will not be shown again.</strong> Anyone holding it can
        read every secret in this vault.
      </div>
      <div className={styles.codeBox}>{code}</div>
      <div className={styles.buttonRow}>
        <button className="v-btn" onClick={() => void copyPlainText(code, 'Recovery code')}>
          Copy
        </button>
        <button className="v-btn" onClick={() => void saveRecoveryCodeToFile()}>
          Save as file…
        </button>
        <button className="v-btn v-btn--primary" onClick={dismissRecoveryCode}>
          I&rsquo;ve saved it
        </button>
      </div>
    </Section>
  );
}

function PasswordSection() {
  const pwCurrent = useVaultStore((s) => s.pwCurrent);
  const pwNew = useVaultStore((s) => s.pwNew);
  const pwConfirm = useVaultStore((s) => s.pwConfirm);
  const pwBusy = useVaultStore((s) => s.pwBusy);
  const pwError = useVaultStore((s) => s.pwError);
  const setPwField = useVaultStore((s) => s.setPwField);
  const changePassword = useVaultStore((s) => s.changePassword);
  const needsMigration = useVaultStore((s) => s.needsMigration);
  const [remember, setRemember] = useState(true);

  return (
    <Section title="Master password">
      <div className={styles.sectionBody}>
        Changing this re-locks the vault file with a new password. Your secrets are not re-encrypted
        and any recovery kit stays valid.
      </div>
      <input
        className={`v-input ${styles.pwInput}`}
        type="password"
        placeholder="Current password"
        value={pwCurrent}
        onChange={(e) => setPwField('pwCurrent', e.target.value)}
        disabled={needsMigration}
        autoComplete="current-password"
        spellCheck={false}
      />
      <input
        className={`v-input ${styles.pwInput}`}
        type="password"
        placeholder="New password"
        value={pwNew}
        onChange={(e) => setPwField('pwNew', e.target.value)}
        disabled={needsMigration}
        autoComplete="new-password"
        spellCheck={false}
      />
      <input
        className={`v-input ${styles.pwInput}`}
        type="password"
        placeholder="Confirm new password"
        value={pwConfirm}
        onChange={(e) => setPwField('pwConfirm', e.target.value)}
        disabled={needsMigration}
        autoComplete="new-password"
        spellCheck={false}
      />
      <label className={styles.checkboxRow}>
        <input
          className="v-check"
          type="checkbox"
          checked={remember}
          onChange={(e) => setRemember(e.target.checked)}
        />
        <span>Remember on this device</span>
      </label>
      {pwError && <div className={styles.error}>{pwError}</div>}
      <button
        className={`v-btn v-btn--primary ${styles.selfStart}`}
        onClick={() => void changePassword(remember)}
        disabled={pwBusy || needsMigration}
        data-pending={pwBusy}
      >
        Change master password
        {pwBusy && <Spinner size={13} />}
      </button>
    </Section>
  );
}
