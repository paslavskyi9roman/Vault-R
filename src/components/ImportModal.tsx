import { useState, type DragEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
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

  const [dragging, setDragging] = useState(false);
  const { mounted, state } = usePresence(importOpen, 120);

  if (!mounted) return null;

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setImportText(String(reader.result ?? ''));
    reader.readAsText(file);
  }

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
          <div
            className="v-dropzone"
            data-dragging={dragging}
            onDragOver={(e) => {
              e.preventDefault();
              setDragging(true);
            }}
            onDragLeave={() => setDragging(false)}
            onDrop={onDrop}
          >
            <div className="v-dropzone-text">Drag &amp; drop a .env file here</div>
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
