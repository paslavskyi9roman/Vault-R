import { useVaultStore } from '../store/useVaultStore';
import { LockIcon, GearIcon } from './icons';
import styles from './TopBar.module.css';

const isMac = navigator.platform.toLowerCase().includes('mac');

export function TopBar() {
  const toggleCmdk = useVaultStore((s) => s.toggleCmdk);
  const openImport = useVaultStore((s) => s.openImport);
  const exportEnv = useVaultStore((s) => s.exportEnv);
  const replayOnboarding = useVaultStore((s) => s.replayOnboarding);
  const lockVault = useVaultStore((s) => s.lockVault);
  const openSettings = useVaultStore((s) => s.openSettings);
  const openSafety = useVaultStore((s) => s.openSafety);
  const needsMigration = useVaultStore((s) => s.needsMigration);

  return (
    <header className={styles.bar}>
      {needsMigration && (
        <button
          className={styles.migrateHint}
          onClick={() => void openSettings()}
          title="This vault uses the original storage format. Lock it and unlock with your master password to upgrade."
        >
          Upgrade available
        </button>
      )}
      <div className={styles.logoWrap}>
        <span className={styles.logoGlyph}>&#10095;_</span>
        <span className={styles.logoText}>vault</span>
      </div>
      <button className={styles.cmdkTrigger} onClick={toggleCmdk}>
        <span className={styles.cmdkIcon}>&#8981;</span>
        <span className={styles.cmdkText}>Search repos, environments, secrets&hellip;</span>
        <span className={styles.cmdkShortcut}>{isMac ? '⌘K' : 'Ctrl+K'}</span>
      </button>
      <div className={styles.actions}>
        <button
          className="v-btn"
          onClick={() => void openSafety()}
          title="Check whether any of these secrets are committed, and what needs rotating"
        >
          Safety
        </button>
        <button className="v-btn" onClick={openImport}>
          Import
        </button>
        <button className="v-btn v-btn--primary" onClick={() => void exportEnv()}>
          Export .env
        </button>
        <button className="v-btn v-btn--icon" onClick={replayOnboarding} title="Replay onboarding">
          ?
        </button>
        <button
          className="v-btn v-btn--icon"
          onClick={() => void openSettings()}
          title="Vault settings, backups and recovery"
          aria-label="Vault settings"
        >
          <GearIcon />
        </button>
        <button
          className="v-btn v-btn--icon"
          onClick={() => void lockVault()}
          title="Lock vault"
          aria-label="Lock vault"
        >
          <LockIcon />
        </button>
      </div>
    </header>
  );
}
