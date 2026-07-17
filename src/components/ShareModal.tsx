import { type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle, modalStyle, modalHeaderStyle, modalTitleStyle, modalSubStyle, closeXStyle } from './overlayStyles';

export function ShareModal() {
  const shareOpen = useVaultStore((s) => s.shareOpen);
  const members = useVaultStore((s) => s.members);
  const inviteEmail = useVaultStore((s) => s.inviteEmail);
  const setInviteEmail = useVaultStore((s) => s.setInviteEmail);
  const addMemberAction = useVaultStore((s) => s.addMemberAction);
  const removeMemberAction = useVaultStore((s) => s.removeMemberAction);
  const closeShare = useVaultStore((s) => s.closeShare);

  if (!shareOpen) return null;

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') void addMemberAction();
  }

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeShare} />
      <div style={modalStyle}>
        <div style={modalHeaderStyle}>
          <span style={modalTitleStyle}>Manage access</span>
          <button style={closeXStyle} onClick={closeShare}>
            &times;
          </button>
        </div>
        <div style={modalSubStyle}>
          Anyone added can view and edit secrets in this vault. This list is stored locally — invited teammates
          don't get real network access yet.
        </div>
        <div style={inviteRowStyle}>
          <input
            style={inviteInputStyle}
            placeholder="teammate@company.com"
            value={inviteEmail}
            onChange={(e) => setInviteEmail(e.target.value)}
            onKeyDown={onKeyDown}
          />
          <button style={inviteBtnStyle} onClick={() => void addMemberAction()}>
            Invite
          </button>
        </div>
        {members.map((m) => (
          <div key={m.id} style={memberRowStyle}>
            <div style={memberAvatarStyle}>{m.email.slice(0, 2).toUpperCase()}</div>
            <div style={memberEmailStyle}>{m.email}</div>
            <div style={memberRoleStyle}>{m.role}</div>
            <button style={memberRemoveStyle} onClick={() => void removeMemberAction(m.id)}>
              remove
            </button>
          </div>
        ))}
      </div>
    </>
  );
}

const inviteRowStyle: React.CSSProperties = { display: 'flex', gap: '8px', marginBottom: '14px' };
const inviteInputStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  fontSize: '13px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '8px 12px',
  outline: 'none',
};
const inviteBtnStyle: React.CSSProperties = {
  fontSize: '12.5px',
  fontWeight: 700,
  color: '#0b1210',
  background: 'var(--accent)',
  border: '1px solid var(--accent)',
  borderRadius: '6px',
  padding: '8px 14px',
  cursor: 'pointer',
};
const memberRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '10px',
  padding: '9px 0',
  borderTop: '1px solid var(--border-light)',
};
const memberAvatarStyle: React.CSSProperties = {
  width: '26px',
  height: '26px',
  borderRadius: '50%',
  background: 'var(--panel-3)',
  color: 'var(--text-dim)',
  fontSize: '9px',
  fontWeight: 700,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  fontFamily: 'var(--font-mono)',
  flexShrink: 0,
};
const memberEmailStyle: React.CSSProperties = { fontSize: '13px', color: 'var(--text)', flex: 1 };
const memberRoleStyle: React.CSSProperties = { fontSize: '11.5px', color: 'var(--text-faint)' };
const memberRemoveStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--danger)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 8px',
  cursor: 'pointer',
};
