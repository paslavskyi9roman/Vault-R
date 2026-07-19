import { useVaultStore } from '../store/useVaultStore';
import type { GeneratorKind } from '../lib/api';
import { usePresence } from '../lib/usePresence';
import { Spinner } from './Spinner';
import { CloseIcon } from './icons';
import styles from './GeneratorPopover.module.css';

const KIND_OPTIONS: { value: GeneratorKind; label: string }[] = [
  { value: 'hex', label: 'Hex' },
  { value: 'base64', label: 'Base64' },
  { value: 'alnum', label: 'Alphanumeric' },
  { value: 'words', label: 'Passphrase' },
];

export function GeneratorPopover() {
  const generatorOpen = useVaultStore((s) => s.generatorOpen);
  const generatorKind = useVaultStore((s) => s.generatorKind);
  const generatorLength = useVaultStore((s) => s.generatorLength);
  const generatorBusy = useVaultStore((s) => s.generatorBusy);
  const closeGenerator = useVaultStore((s) => s.closeGenerator);
  const setGeneratorKind = useVaultStore((s) => s.setGeneratorKind);
  const setGeneratorLength = useVaultStore((s) => s.setGeneratorLength);
  const runGenerator = useVaultStore((s) => s.runGenerator);

  const { mounted, state } = usePresence(generatorOpen, 120);
  if (!mounted) return null;

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeGenerator} />
      <div className="v-modal-wrap">
        <div className={`v-modal ${styles.dialog} is-${state}`}>
          <div className="v-modal-header">
            <span className="v-modal-title">Generate a secret</span>
            <button className="v-close-x" onClick={closeGenerator} aria-label="Close">
              <CloseIcon size={13} />
            </button>
          </div>
          <div className="v-modal-sub">
            Drawn from a cryptographically random source, never derived from anything typed.
          </div>

          <div className={styles.kindRow}>
            {KIND_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                className={`v-btn ${styles.kindBtn}`}
                aria-pressed={opt.value === generatorKind}
                onClick={() => setGeneratorKind(opt.value)}
              >
                {opt.label}
              </button>
            ))}
          </div>

          <label className={styles.lengthLabel}>
            {generatorKind === 'words' ? 'Word count' : 'Length'}
            <input
              className={`v-input ${styles.lengthInput}`}
              type="number"
              min={generatorKind === 'words' ? 3 : 8}
              max={generatorKind === 'words' ? 20 : 128}
              value={generatorLength}
              onChange={(e) => setGeneratorLength(Number(e.target.value))}
            />
          </label>

          <button
            className="v-btn v-btn--primary v-btn--block"
            disabled={generatorBusy}
            data-pending={generatorBusy}
            onClick={() => void runGenerator()}
          >
            Generate
            {generatorBusy && <Spinner size={13} />}
          </button>
        </div>
      </div>
    </>
  );
}
