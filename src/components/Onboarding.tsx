import { type DragEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { dropzoneStyle, dropzoneTextStyle, orDividerStyle, pasteAreaStyle, primaryBtnStyle, secondaryBtnStyle } from './overlayStyles';

export function Onboarding() {
  const onboarding = useVaultStore((s) => s.onboarding);
  const onboardingStep = useVaultStore((s) => s.onboardingStep);
  const onboardingRepoName = useVaultStore((s) => s.onboardingRepoName);
  const onboardingEnvName = useVaultStore((s) => s.onboardingEnvName);
  const setOnboardingRepoName = useVaultStore((s) => s.setOnboardingRepoName);
  const setOnboardingEnvName = useVaultStore((s) => s.setOnboardingEnvName);
  const importText = useVaultStore((s) => s.importText);
  const setImportText = useVaultStore((s) => s.setImportText);
  const obNext = useVaultStore((s) => s.obNext);
  const obSkip = useVaultStore((s) => s.obSkip);
  const obImportAndNext = useVaultStore((s) => s.obImportAndNext);
  const obFinish = useVaultStore((s) => s.obFinish);

  if (!onboarding) return null;

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setImportText(String(reader.result ?? ''));
    reader.readAsText(file);
  }

  return (
    <div style={backdropStyle}>
      <div style={cardStyle}>
        {onboardingStep === 0 && (
          <>
            <div style={glyphStyle}>&#10095;_</div>
            <div style={titleStyle}>Welcome to vault</div>
            <div style={subStyle}>
              One place for every .env, across every repo and every environment &mdash; stop copy-pasting secrets
              between 25 files.
            </div>
            <div style={featureRowStyle}>
              <div style={featureStyle}>
                <div style={featureDotStyle} />
                Linked secrets propagate everywhere they're used
              </div>
              <div style={featureStyle}>
                <div style={featureDotStyle} />
                Full version history with one-click restore
              </div>
              <div style={featureStyle}>
                <div style={featureDotStyle} />
                Ctrl/⌘+K to jump anywhere instantly
              </div>
            </div>
            <button style={primaryBtnStyle} onClick={obNext}>
              Continue
            </button>
          </>
        )}

        {onboardingStep === 1 && (
          <>
            <div style={titleStyle}>Bring in your first .env</div>
            <div style={subStyle}>Name your first repository and environment, then drop a file or paste its contents.</div>
            <div style={nameRowStyle}>
              <input
                style={nameInputStyle}
                placeholder="repository (e.g. api-gateway)"
                value={onboardingRepoName}
                onChange={(e) => setOnboardingRepoName(e.target.value)}
              />
              <input
                style={nameInputStyle}
                placeholder="environment (e.g. local)"
                value={onboardingEnvName}
                onChange={(e) => setOnboardingEnvName(e.target.value)}
              />
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
            <div style={btnRowStyle}>
              <button style={secondaryBtnStyle} onClick={() => void obSkip()}>
                Skip
              </button>
              <button style={{ ...primaryBtnStyle, width: 'auto', flex: 1 }} onClick={() => void obImportAndNext()}>
                Import &amp; continue
              </button>
            </div>
          </>
        )}

        {onboardingStep === 2 && (
          <>
            <div style={glyphStyle}>&#10003;</div>
            <div style={titleStyle}>You're all set</div>
            <div style={subStyle}>
              Your vault is ready. Press Ctrl/⌘+K anytime to jump between repos and environments.
            </div>
            <button style={primaryBtnStyle} onClick={() => void obFinish()}>
              Enter vault
            </button>
          </>
        )}

        <div style={stepDotsStyle}>
          {[0, 1, 2].map((i) => (
            <div key={i} style={{ ...dotStyle, background: i === onboardingStep ? 'var(--accent)' : 'var(--border)' }} />
          ))}
        </div>
      </div>
    </div>
  );
}

const backdropStyle: React.CSSProperties = {
  position: 'fixed',
  inset: 0,
  background: 'rgba(6,7,10,0.75)',
  zIndex: 50,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
};
const cardStyle: React.CSSProperties = {
  width: '440px',
  maxWidth: '90vw',
  background: 'var(--panel)',
  border: '1px solid var(--border)',
  borderRadius: '12px',
  padding: '32px',
  display: 'flex',
  flexDirection: 'column',
  gap: '10px',
  animation: 'vaultFadeIn 0.25s ease',
};
const glyphStyle: React.CSSProperties = { fontFamily: 'var(--font-mono)', color: 'var(--accent)', fontWeight: 700, fontSize: '26px' };
const titleStyle: React.CSSProperties = { fontSize: '19px', fontWeight: 700, color: 'var(--text)' };
const subStyle: React.CSSProperties = { fontSize: '13px', color: 'var(--text-dim)', lineHeight: 1.5, marginBottom: '6px' };
const featureRowStyle: React.CSSProperties = { display: 'flex', flexDirection: 'column', gap: '9px', margin: '4px 0 14px' };
const featureStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '9px', fontSize: '13px', color: 'var(--text-dim)' };
const featureDotStyle: React.CSSProperties = { width: '6px', height: '6px', borderRadius: '50%', background: 'var(--accent)', flexShrink: 0 };
const nameRowStyle: React.CSSProperties = { display: 'flex', gap: '8px', marginBottom: '10px' };
const nameInputStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '8px 10px',
  outline: 'none',
  boxSizing: 'border-box',
};
const btnRowStyle: React.CSSProperties = { display: 'flex', gap: '10px' };
const stepDotsStyle: React.CSSProperties = { display: 'flex', gap: '6px', justifyContent: 'center', marginTop: '10px' };
const dotStyle: React.CSSProperties = { width: '6px', height: '6px', borderRadius: '50%' };
