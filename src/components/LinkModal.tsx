import { useVaultStore } from '../store/useVaultStore';
import { usePresence, useLastPresent } from '../lib/usePresence';
import styles from './LinkModal.module.css';

export function LinkModal() {
  const linkModalOpen = useVaultStore((s) => s.linkModalOpen);
  const linkModalKeyLive = useVaultStore((s) => s.linkModalKey);
  const candidatesLive = useVaultStore((s) => s.linkCandidates);
  const linkSelected = useVaultStore((s) => s.linkSelected);
  const toggleLinkSelected = useVaultStore((s) => s.toggleLinkSelected);
  const confirmLink = useVaultStore((s) => s.confirmLink);
  const closeLinkModal = useVaultStore((s) => s.closeLinkModal);

  const { mounted, state } = usePresence(linkModalOpen, 120);
  // closeLinkModal clears the key and candidate list, so hold the last set
  // for the exit frames.
  const linkModalKey = useLastPresent(linkModalKeyLive || null) ?? '';
  const linkCandidates = useLastPresent(candidatesLive.length ? candidatesLive : null) ?? [];

  if (!mounted) return null;

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeLinkModal} />
      <div className="v-modal-wrap">
        <div className={`v-modal is-${state}`}>
          <div className="v-modal-header">
            <span className="v-modal-title">Link "{linkModalKey}"</span>
            <button className="v-close-x" onClick={closeLinkModal} aria-label="Close">
              &times;
            </button>
          </div>
          <div className="v-modal-sub">
            Editing any variable in the group updates them all everywhere they're used. The value from this
            variable is applied to everything you select.
          </div>

          {linkCandidates.length === 0 && (
            <div className={styles.empty}>
              No other "{linkModalKey}" variables found in other repos or environments.
            </div>
          )}

          {linkCandidates.map((c) => (
            <label key={c.variable.id} className={styles.row}>
              <input
                type="checkbox"
                checked={!!linkSelected[c.variable.id]}
                onChange={() => toggleLinkSelected(c.variable.id)}
              />
              <span className={styles.target}>
                {c.repoName} / {c.envName}
              </span>
              {c.variable.groupId && <span className={styles.alreadyLinked}>already linked</span>}
            </label>
          ))}

          {linkCandidates.length > 0 && (
            <button
              className={`v-btn v-btn--primary v-btn--block ${styles.submit}`}
              onClick={() => void confirmLink()}
            >
              Link selected
            </button>
          )}
        </div>
      </div>
    </>
  );
}
