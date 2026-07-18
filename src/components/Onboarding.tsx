import { useState, type DragEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import styles from './Onboarding.module.css';

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

  const [dragging, setDragging] = useState(false);
  const { mounted, state } = usePresence(onboarding, 140);

  if (!mounted) return null;

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setImportText(String(reader.result ?? ''));
    reader.readAsText(file);
  }

  const phase = state === 'entered' ? styles.isEntered : state === 'exiting' ? styles.isExiting : '';

  return (
    <div className={`${styles.backdrop} ${phase}`}>
      <div className={styles.card}>
        {onboardingStep === 0 && (
          <>
            <div className={styles.glyph}>&#10095;_</div>
            <div className={styles.title}>Welcome to vault</div>
            <div className={styles.sub}>
              One place for every .env, across every repo and every environment &mdash; stop copy-pasting secrets
              between 25 files.
            </div>
            <div className={styles.featureRow}>
              <div className={styles.feature}>
                <div className={styles.featureDot} />
                Linked secrets propagate everywhere they're used
              </div>
              <div className={styles.feature}>
                <div className={styles.featureDot} />
                Full version history with one-click restore
              </div>
              <div className={styles.feature}>
                <div className={styles.featureDot} />
                Ctrl/⌘+K to jump anywhere instantly
              </div>
            </div>
            <button className="v-btn v-btn--primary v-btn--block" onClick={obNext}>
              Continue
            </button>
          </>
        )}

        {onboardingStep === 1 && (
          <>
            <div className={styles.title}>Bring in your first .env</div>
            <div className={styles.sub}>
              Name your first repository and environment, then drop a file or paste its contents.
            </div>
            <div className={styles.nameRow}>
              <input
                className={`v-input ${styles.nameInput}`}
                placeholder="repository (e.g. api-gateway)"
                value={onboardingRepoName}
                onChange={(e) => setOnboardingRepoName(e.target.value)}
              />
              <input
                className={`v-input ${styles.nameInput}`}
                placeholder="environment (e.g. local)"
                value={onboardingEnvName}
                onChange={(e) => setOnboardingEnvName(e.target.value)}
              />
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
            <div className={styles.btnRow}>
              <button className="v-btn" onClick={() => void obSkip()}>
                Skip
              </button>
              <button
                className={`v-btn v-btn--primary ${styles.btnRowPrimary}`}
                onClick={() => void obImportAndNext()}
              >
                Import &amp; continue
              </button>
            </div>
          </>
        )}

        {onboardingStep === 2 && (
          <>
            <div className={styles.glyph}>&#10003;</div>
            <div className={styles.title}>You're all set</div>
            <div className={styles.sub}>
              Your vault is ready. Press Ctrl/⌘+K anytime to jump between repos and environments.
            </div>
            <button className="v-btn v-btn--primary v-btn--block" onClick={() => void obFinish()}>
              Enter vault
            </button>
          </>
        )}

        <div className={styles.stepDots}>
          {[0, 1, 2].map((i) => (
            <div key={i} className={styles.dot} data-active={i === onboardingStep} />
          ))}
        </div>
      </div>
    </div>
  );
}
