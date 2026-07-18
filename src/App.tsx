import { useEffect, useRef } from 'react';
import { useVaultStore } from './store/useVaultStore';
import { UnlockScreen, ResetPasswordScreen } from './components/UnlockScreen';
import { SettingsModal } from './components/SettingsModal';
import { TopBar } from './components/TopBar';
import { Sidebar } from './components/Sidebar';
import { MainPanel } from './components/MainPanel';
import { HistorySlideover } from './components/HistorySlideover';
import { CompareView } from './components/CompareView';
import { ShareModal } from './components/ShareModal';
import { ImportModal } from './components/ImportModal';
import { CommandPalette } from './components/CommandPalette';
import { LinkModal } from './components/LinkModal';
import { GroupPopover } from './components/GroupPopover';
import { GeneratorPopover } from './components/GeneratorPopover';
import { Onboarding } from './components/Onboarding';
import { ConfirmDialog } from './components/ConfirmDialog';
import { Toast } from './components/Toast';

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

  if (checkingVault) {
    return <div style={loadingStyle} />;
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
    <div style={rootStyle}>
      <TopBar />
      <div style={bodyStyle}>
        <Sidebar />
        <MainPanel />
      </div>

      <HistorySlideover />
      <CompareView />
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

const rootStyle: React.CSSProperties = {
  height: '100vh',
  width: '100%',
  display: 'flex',
  flexDirection: 'column',
  background: 'var(--bg)',
  color: 'var(--text)',
  overflow: 'hidden',
  position: 'relative',
};
const bodyStyle: React.CSSProperties = { flex: 1, display: 'flex', overflow: 'hidden' };
const loadingStyle: React.CSSProperties = { height: '100vh', width: '100%', background: 'var(--bg)' };

export default App;
