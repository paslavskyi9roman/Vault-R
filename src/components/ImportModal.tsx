import { type DragEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import {
  overlayBackdropStyle,
  modalStyle,
  modalHeaderStyle,
  modalTitleStyle,
  modalSubStyle,
  closeXStyle,
  dropzoneStyle,
  dropzoneTextStyle,
  orDividerStyle,
  pasteAreaStyle,
  primaryBtnStyle,
} from './overlayStyles';

export function ImportModal() {
  const importOpen = useVaultStore((s) => s.importOpen);
  const importText = useVaultStore((s) => s.importText);
  const setImportText = useVaultStore((s) => s.setImportText);
  const closeImport = useVaultStore((s) => s.closeImport);
  const submitImport = useVaultStore((s) => s.submitImport);
  const repos = useVaultStore((s) => s.repos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);

  if (!importOpen) return null;

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setImportText(String(reader.result ?? ''));
    reader.readAsText(file);
  }

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeImport} />
      <div style={modalStyle}>
        <div style={modalHeaderStyle}>
          <span style={modalTitleStyle}>Import .env</span>
          <button style={closeXStyle} onClick={closeImport}>
            &times;
          </button>
        </div>
        <div style={modalSubStyle}>
          Into {activeRepo && activeEnv ? `${activeRepo.name} / ${activeEnv.name}` : 'the active environment'}
        </div>
        <div style={dropzoneStyle} onDragOver={(e) => e.preventDefault()} onDrop={onDrop}>
          <div style={dropzoneTextStyle}>Drag &amp; drop a .env file here</div>
        </div>
        <div style={orDividerStyle}>or paste below</div>
        <textarea
          style={pasteAreaStyle}
          placeholder={'KEY=value\nANOTHER_KEY=value'}
          value={importText}
          onChange={(e) => setImportText(e.target.value)}
        />
        <button style={primaryBtnStyle} onClick={() => void submitImport()}>
          Import variables
        </button>
      </div>
    </>
  );
}
