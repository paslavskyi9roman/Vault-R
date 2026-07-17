import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle, modalStyle, modalHeaderStyle, modalTitleStyle, modalSubStyle, closeXStyle } from './overlayStyles';

export function GroupPopover() {
  const groupPopoverGroupId = useVaultStore((s) => s.groupPopoverGroupId);
  const groupPopoverMembers = useVaultStore((s) => s.groupPopoverMembers);
  const closeGroupPopover = useVaultStore((s) => s.closeGroupPopover);
  const unlinkFromPopover = useVaultStore((s) => s.unlinkFromPopover);

  if (!groupPopoverGroupId) return null;

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeGroupPopover} />
      <div style={modalStyle}>
        <div style={modalHeaderStyle}>
          <span style={modalTitleStyle}>Linked secret group</span>
          <button style={closeXStyle} onClick={closeGroupPopover}>
            &times;
          </button>
        </div>
        <div style={modalSubStyle}>Editing the value in any one of these updates all of them.</div>
        {groupPopoverMembers.map((m) => (
          <div key={m.variable.id} style={rowStyle}>
            <span style={targetStyle}>
              {m.repoName} / {m.envName}
            </span>
            <span style={keyStyle}>{m.variable.key}</span>
            <button style={unlinkBtnStyle} onClick={() => void unlinkFromPopover(m.variable.id)}>
              Unlink
            </button>
          </div>
        ))}
      </div>
    </>
  );
}

const rowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '10px',
  padding: '9px 0',
  borderTop: '1px solid var(--border-light)',
};
const targetStyle: React.CSSProperties = { fontFamily: 'var(--font-mono)', fontSize: '13px', color: 'var(--text)', flex: 1 };
const keyStyle: React.CSSProperties = { fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--key)' };
const unlinkBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--danger)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 8px',
  cursor: 'pointer',
};
