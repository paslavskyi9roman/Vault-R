import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useVaultStore } from './store/useVaultStore';
import { api } from './lib/api';
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

/// Throttled to 5s so mouse movement isn't a stream of IPC calls; the backend
/// owns the actual timeout.
function useActivityReporting(enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;

    let last = 0;
    const bump = () => {
      const now = Date.now();
      if (now - last < 5_000) return;
      last = now;
      void api.notifyActivity();
    };
    const events: (keyof WindowEventMap)[] = ['mousemove', 'keydown', 'pointerdown', 'wheel'];
    events.forEach((name) => window.addEventListener(name, bump, { passive: true }));

    return () => events.forEach((name) => window.removeEventListener(name, bump));
  }, [enabled]);
}

function App() {
  const checkingVault = useVaultStore((s) => s.checkingVault);
  const initError = useVaultStore((s) => s.initError);
  const locked = useVaultStore((s) => s.locked);
  const mustResetPassword = useVaultStore((s) => s.mustResetPassword);
  const init = useVaultStore((s) => s.init);
  const lockVault = useVaultStore((s) => s.lockVault);
  const onBackendLock = useVaultStore((s) => s.onBackendLock);
  const toggleCmdk = useVaultStore((s) => s.toggleCmdk);
  const closeAllOverlays = useVaultStore((s) => s.closeAllOverlays);

  useEffect(() => {
    void init();
  }, [init]);

  useActivityReporting(!locked);

  // Reflect a backend-initiated auto-lock (idle enforcer) in the UI.
  useEffect(() => {
    const unlisten = listen('vault-locked', () => onBackendLock());
    return () => void unlisten.then((off) => off());
  }, [onBackendLock]);

  // Keyed on visibility, not focus: a native file dialog steals focus
  // without hiding the window, and locking then would abort the dialog.
  useEffect(() => {
    if (locked) return;
    const onVisibility = () => {
      if (document.visibilityState === 'hidden') void lockVault();
    };
    const onPageHide = () => void lockVault();
    document.addEventListener('visibilitychange', onVisibility);
    window.addEventListener('pagehide', onPageHide);
    return () => {
      document.removeEventListener('visibilitychange', onVisibility);
      window.removeEventListener('pagehide', onPageHide);
    };
  }, [locked, lockVault]);

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
