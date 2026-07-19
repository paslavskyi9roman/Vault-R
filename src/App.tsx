import { useEffect, useRef } from 'react';
import { useVaultStore } from './store/useVaultStore';
import { UnlockScreen, ResetPasswordScreen, StartupErrorScreen } from './components/UnlockScreen';
import { SettingsModal } from './components/SettingsModal';
import { TopBar } from './components/TopBar';
import { Sidebar } from './components/Sidebar';
import { MainPanel } from './components/MainPanel';
import { HistorySlideover } from './components/HistorySlideover';
import { CompareView } from './components/CompareView';
import { SafetyPanel } from './components/SafetyPanel';
import { ShareModal } from './components/ShareModal';
import { ImportModal } from './components/ImportModal';
import { CommandPalette } from './components/CommandPalette';
import { LinkModal } from './components/LinkModal';
import { GroupPopover } from './components/GroupPopover';
import { GeneratorPopover } from './components/GeneratorPopover';
import { Onboarding } from './components/Onboarding';
import { ConfirmDialog } from './components/ConfirmDialog';
import { Toast } from './components/Toast';
import { Spinner } from './components/Spinner';
import styles from './App.module.css';

/// Locks the vault after `minutes` with no keyboard or pointer activity.
/// 0 disables it. Activity is recorded into a ref rather than state so that
/// moving the mouse does not re-render the whole app.
function useIdleAutoLock(enabled: boolean, minutes: number) {
  const lockVault = useVaultStore((s) => s.lockVault);
  const lastActivity = useRef(Date.now());

  useEffect(() => {
    if (!enabled || minutes <= 0) return;

    const bump = () => {
      lastActivity.current = Date.now();
    };
    const events: (keyof WindowEventMap)[] = ['mousemove', 'keydown', 'pointerdown', 'wheel'];
    events.forEach((name) => window.addEventListener(name, bump, { passive: true }));

    const timeoutMs = minutes * 60_000;
    const interval = window.setInterval(() => {
      if (Date.now() - lastActivity.current >= timeoutMs) {
        void lockVault();
      }
    }, 15_000);

    return () => {
      events.forEach((name) => window.removeEventListener(name, bump));
      window.clearInterval(interval);
    };
  }, [enabled, minutes, lockVault]);
}

function App() {
  const checkingVault = useVaultStore((s) => s.checkingVault);
  const initError = useVaultStore((s) => s.initError);
  const locked = useVaultStore((s) => s.locked);
  const mustResetPassword = useVaultStore((s) => s.mustResetPassword);
  const autoLockMinutes = useVaultStore((s) => s.autoLockMinutes);
  const init = useVaultStore((s) => s.init);
  const toggleCmdk = useVaultStore((s) => s.toggleCmdk);
  const closeAllOverlays = useVaultStore((s) => s.closeAllOverlays);

  useEffect(() => {
    void init();
  }, [init]);

  useIdleAutoLock(!locked, autoLockMinutes);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        toggleCmdk();
      } else if (e.key === 'Escape') {
        closeAllOverlays();
      }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [toggleCmdk, closeAllOverlays]);

  if (checkingVault && !initError) {
    return (
      <div className={styles.splash}>
        <div className={`${styles.splashInner} v-enter`}>
          <span className={styles.splashGlyph}>&#10095;_</span>
          <span className={styles.splashText}>vault</span>
        </div>
        <span className={`${styles.splashSpinnerWrap} v-enter`}>
          <Spinner size={13} />
        </span>
      </div>
    );
  }

  // Falling through to the lock screen here would offer to create a vault
  // over one we merely failed to read.
  if (initError) {
    return (
      <>
        <StartupErrorScreen />
        <Toast />
      </>
    );
  }

  if (locked) {
    return (
      <>
        <UnlockScreen />
        <ConfirmDialog />
        <Toast />
      </>
    );
  }

  // Unlocked via a recovery code: no usable master password until one is set.
  if (mustResetPassword) {
    return (
      <>
        <ResetPasswordScreen />
        <Toast />
      </>
    );
  }

  return (
    <div className={styles.root}>
      <TopBar />
      <div className={styles.body}>
        <Sidebar />
        <MainPanel />
      </div>

      <HistorySlideover />
      <CompareView />
      <SafetyPanel />
      <ShareModal />
      <ImportModal />
      <CommandPalette />
      <LinkModal />
      <GroupPopover />
      <GeneratorPopover />
      <Onboarding />
      <SettingsModal />
      <ConfirmDialog />
      <Toast />
    </div>
  );
}

export default App;
