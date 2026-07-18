import { useVaultStore } from '../store/useVaultStore';
import { usePresence, useLastPresent } from '../lib/usePresence';
import styles from './GroupPopover.module.css';

export function GroupPopover() {
  const groupPopoverGroupId = useVaultStore((s) => s.groupPopoverGroupId);
  const membersLive = useVaultStore((s) => s.groupPopoverMembers);
  const closeGroupPopover = useVaultStore((s) => s.closeGroupPopover);
  const unlinkFromPopover = useVaultStore((s) => s.unlinkFromPopover);

  const { mounted, state } = usePresence(!!groupPopoverGroupId, 120);
  // closeGroupPopover empties the member list, so the exit frames need the
  // last version that still had rows in it.
  const members = useLastPresent(membersLive.length ? membersLive : null) ?? [];

  if (!mounted) return null;

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeGroupPopover} />
      <div className="v-modal-wrap">
        <div className={`v-modal is-${state}`}>
          <div className="v-modal-header">
            <span className="v-modal-title">Linked secret group</span>
            <button className="v-close-x" onClick={closeGroupPopover} aria-label="Close">
              &times;
            </button>
          </div>
          <div className="v-modal-sub">Editing the value in any one of these updates all of them.</div>
          {members.map((m) => (
            <div key={m.variable.id} className={styles.row}>
              <span className={styles.target}>
                {m.repoName} / {m.envName}
              </span>
              <span className={styles.key}>{m.variable.key}</span>
              <button
                className={`v-btn v-btn--danger ${styles.unlink}`}
                onClick={() => void unlinkFromPopover(m.variable.id)}
              >
                Unlink
              </button>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
