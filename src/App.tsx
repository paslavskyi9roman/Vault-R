import { useEffect } from 'react';
import { useVaultStore } from './store/useVaultStore';
import { UnlockScreen } from './components/UnlockScreen';
import { TopBar } from './components/TopBar';
import { Sidebar } from './components/Sidebar';
import { MainPanel } from './components/MainPanel';
import { HistorySlideover } from './components/HistorySlideover';
import { ShareModal } from './components/ShareModal';
import { ImportModal } from './components/ImportModal';
import { CommandPalette } from './components/CommandPalette';
import { LinkModal } from './components/LinkModal';
import { GroupPopover } from './components/GroupPopover';
import { Onboarding } from './components/Onboarding';
import { Toast } from './components/Toast';

function App() {
  const checkingVault = useVaultStore((s) => s.checkingVault);
  const locked = useVaultStore((s) => s.locked);
  const init = useVaultStore((s) => s.init);
  const toggleCmdk = useVaultStore((s) => s.toggleCmdk);
  const closeAllOverlays = useVaultStore((s) => s.closeAllOverlays);

  useEffect(() => {
    void init();
  }, [init]);

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
    return <UnlockScreen />;
  }

  return (
    <div style={rootStyle}>
      <TopBar />
      <div style={bodyStyle}>
        <Sidebar />
        <MainPanel />
      </div>

      <HistorySlideover />
      <ShareModal />
      <ImportModal />
      <CommandPalette />
      <LinkModal />
      <GroupPopover />
      <Onboarding />
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
