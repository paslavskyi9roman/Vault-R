import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import { useFileDrop } from '../lib/useFileDrop';
import { CloseIcon } from './icons';

export function ImportModal() {
  const importOpen = useVaultStore((s) => s.importOpen);
  const importText = useVaultStore((s) => s.importText);
  const setImportText = useVaultStore((s) => s.setImportText);
  const closeImport = useVaultStore((s) => s.closeImport);
  const submitImport = useVaultStore((s) => s.submitImport);
  const repos = useVaultStore((s) => s.repos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const showToast = useVaultStore((s) => s.showToast);
  const onboarding = useVaultStore((s) => s.onboarding);

  const { mounted, state } = usePresence(importOpen, 120);
  /// Onboarding renders over this one and has its own dropzone; only one of
  /// them may claim window-wide drops at a time.
  const { dragging, dropHandlers } = useFileDrop(
    importOpen && !onboarding,
    setImportText,
    showToast,
  );

  if (!mounted) return null;

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeImport} />
      <div className="v-modal-wrap">
        <div className={`v-modal is-${state}`}>
          <div className="v-modal-header">
            <span className="v-modal-title">Import .env</span>
            <button className="v-close-x" onClick={closeImport} aria-label="Close">
              <CloseIcon size={13} />
            </button>
          </div>
          <div className="v-modal-sub">
            Into {activeRepo && activeEnv ? `${activeRepo.name} / ${activeEnv.name}` : 'the active environment'}
          </div>
          <div className="v-dropzone" data-dragging={dragging} {...dropHandlers}>
            <div className="v-dropzone-text">
              {dragging ? 'Release to read the file' : 'Drag & drop a .env file here'}
            </div>
          </div>
          <div className="v-or">or paste below</div>
          <textarea
            className="v-paste"
            placeholder={'KEY=value\nANOTHER_KEY=value'}
            value={importText}
            onChange={(e) => setImportText(e.target.value)}
          />
          <button className="v-btn v-btn--primary v-btn--block" onClick={() => void submitImport()}>
            Import variables
          </button>
        </div>
      </div>
    </>
  );
}
