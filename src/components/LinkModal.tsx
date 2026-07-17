import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle, modalStyle, modalHeaderStyle, modalTitleStyle, modalSubStyle, closeXStyle, primaryBtnStyle } from './overlayStyles';

export function LinkModal() {
  const linkModalOpen = useVaultStore((s) => s.linkModalOpen);
  const linkModalKey = useVaultStore((s) => s.linkModalKey);
  const linkCandidates = useVaultStore((s) => s.linkCandidates);
  const linkSelected = useVaultStore((s) => s.linkSelected);
  const toggleLinkSelected = useVaultStore((s) => s.toggleLinkSelected);
  const confirmLink = useVaultStore((s) => s.confirmLink);
  const closeLinkModal = useVaultStore((s) => s.closeLinkModal);

  if (!linkModalOpen) return null;

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeLinkModal} />
      <div style={modalStyle}>
        <div style={modalHeaderStyle}>
          <span style={modalTitleStyle}>Link "{linkModalKey}"</span>
          <button style={closeXStyle} onClick={closeLinkModal}>
            &times;
          </button>
        </div>
        <div style={modalSubStyle}>
          Editing any variable in the group updates them all everywhere they're used. The value from this
          variable is applied to everything you select.
        </div>

        {linkCandidates.length === 0 && (
          <div style={emptyStyle}>No other "{linkModalKey}" variables found in other repos or environments.</div>
        )}

        {linkCandidates.map((c) => (
          <label key={c.variable.id} style={rowStyle}>
            <input
              type="checkbox"
              checked={!!linkSelected[c.variable.id]}
              onChange={() => toggleLinkSelected(c.variable.id)}
            />
            <span style={targetStyle}>
              {c.repoName} / {c.envName}
            </span>
            {c.variable.groupId && <span style={alreadyLinkedStyle}>already linked</span>}
          </label>
        ))}

        {linkCandidates.length > 0 && (
          <button style={{ ...primaryBtnStyle, marginTop: '14px' }} onClick={() => void confirmLink()}>
            Link selected
          </button>
        )}
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
  cursor: 'pointer',
};
const targetStyle: React.CSSProperties = { fontFamily: 'var(--font-mono)', fontSize: '13px', color: 'var(--text)', flex: 1 };
const alreadyLinkedStyle: React.CSSProperties = { fontSize: '10.5px', color: 'var(--accent)' };
const emptyStyle: React.CSSProperties = { fontSize: '12.5px', color: 'var(--text-faint)', padding: '8px 0' };
