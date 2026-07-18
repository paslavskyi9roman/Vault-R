import { useVaultStore } from '../store/useVaultStore';
import type { GeneratorKind } from '../lib/api';
import { overlayBackdropStyle, modalStyle, modalHeaderStyle, modalTitleStyle, modalSubStyle, closeXStyle, primaryBtnStyle } from './overlayStyles';

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

  if (!generatorOpen) return null;

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeGenerator} />
      <div style={{ ...modalStyle, width: '340px' }}>
        <div style={modalHeaderStyle}>
          <span style={modalTitleStyle}>Generate a secret</span>
          <button style={closeXStyle} onClick={closeGenerator}>
            &times;
          </button>
        </div>
        <div style={modalSubStyle}>Drawn from a cryptographically random source, never derived from anything typed.</div>

        <div style={kindRowStyle}>
          {KIND_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              style={opt.value === generatorKind ? kindBtnActiveStyle : kindBtnStyle}
              onClick={() => setGeneratorKind(opt.value)}
            >
              {opt.label}
            </button>
          ))}
        </div>

        <label style={lengthLabelStyle}>
          {generatorKind === 'words' ? 'Word count' : 'Length'}
          <input
            style={lengthInputStyle}
            type="number"
            min={generatorKind === 'words' ? 3 : 8}
            max={generatorKind === 'words' ? 20 : 128}
            value={generatorLength}
            onChange={(e) => setGeneratorLength(Number(e.target.value))}
          />
        </label>

        <button style={primaryBtnStyle} disabled={generatorBusy} onClick={() => void runGenerator()}>
          {generatorBusy ? 'Generating…' : 'Generate'}
        </button>
      </div>
    </>
  );
}

const kindRowStyle: React.CSSProperties = { display: 'flex', gap: '6px', marginBottom: '14px', flexWrap: 'wrap' };
const kindBtnStyle: React.CSSProperties = {
  fontSize: '12px',
  fontWeight: 600,
  color: 'var(--text-dim)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  padding: '6px 10px',
  cursor: 'pointer',
};
const kindBtnActiveStyle: React.CSSProperties = {
  ...kindBtnStyle,
  color: 'var(--accent)',
  borderColor: 'var(--accent)',
  background: 'var(--accent-dim)',
};
const lengthLabelStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  fontSize: '12.5px',
  color: 'var(--text-dim)',
  marginBottom: '16px',
};
const lengthInputStyle: React.CSSProperties = {
  width: '70px',
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '6px 8px',
  outline: 'none',
  boxSizing: 'border-box',
};
