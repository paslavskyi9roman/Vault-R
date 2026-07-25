import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import { useFileDrop } from '../lib/useFileDrop';
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
  const showToast = useVaultStore((s) => s.showToast);

  const { mounted, state } = usePresence(onboarding, 140);
  /// Step 1 is the only step with a dropzone to claim drops for.
  const { dragging, dropHandlers } = useFileDrop(
    onboarding && onboardingStep === 1,
    setImportText,
    showToast,
  );

  if (!mounted) return null;

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
