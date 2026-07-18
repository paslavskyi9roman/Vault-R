import { type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import styles from './ShareModal.module.css';

export function ShareModal() {
  const shareOpen = useVaultStore((s) => s.shareOpen);
  const members = useVaultStore((s) => s.members);
  const inviteEmail = useVaultStore((s) => s.inviteEmail);
  const setInviteEmail = useVaultStore((s) => s.setInviteEmail);
  const addMemberAction = useVaultStore((s) => s.addMemberAction);
  const removeMemberAction = useVaultStore((s) => s.removeMemberAction);
  const closeShare = useVaultStore((s) => s.closeShare);

  const { mounted, state } = usePresence(shareOpen, 120);
  if (!mounted) return null;

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') void addMemberAction();
  }

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeShare} />
      <div className="v-modal-wrap">
        <div className={`v-modal is-${state}`}>
          <div className="v-modal-header">
            <span className="v-modal-title">Manage access</span>
            <button className="v-close-x" onClick={closeShare} aria-label="Close">
              &times;
            </button>
          </div>
          <div className="v-modal-sub">
            Anyone added can view and edit secrets in this vault. This list is stored locally — invited teammates
            don't get real network access yet.
          </div>
          <div className={styles.inviteRow}>
            <input
              className={`v-input ${styles.inviteInput}`}
              placeholder="teammate@company.com"
              value={inviteEmail}
              onChange={(e) => setInviteEmail(e.target.value)}
              onKeyDown={onKeyDown}
            />
            <button
              className={`v-btn v-btn--primary ${styles.inviteBtn}`}
              onClick={() => void addMemberAction()}
            >
              Invite
            </button>
          </div>
          {members.map((m) => (
            <div key={m.id} className={styles.memberRow}>
              <div className={styles.avatar}>{m.email.slice(0, 2).toUpperCase()}</div>
              <div className={styles.email}>{m.email}</div>
              <div className={styles.role}>{m.role}</div>
              <button
                className={`v-btn v-btn--danger ${styles.remove}`}
                onClick={() => void removeMemberAction(m.id)}
              >
                remove
              </button>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
