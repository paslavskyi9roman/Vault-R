export const overlayBackdropStyle: React.CSSProperties = {
  position: 'fixed',
  inset: 0,
  background: 'rgba(6,7,10,0.6)',
  zIndex: 40,
};

export const modalStyle: React.CSSProperties = {
  position: 'fixed',
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  width: '440px',
  maxHeight: '80vh',
  overflowY: 'auto',
  background: 'var(--panel)',
  border: '1px solid var(--border)',
  borderRadius: '10px',
  zIndex: 41,
  padding: '22px',
  animation: 'vaultFadeIn 0.18s ease',
};

export const modalHeaderStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', marginBottom: '4px' };
export const modalTitleStyle: React.CSSProperties = { fontSize: '16px', fontWeight: 700, color: 'var(--text)', flex: 1 };
export const modalSubStyle: React.CSSProperties = { fontSize: '12.5px', color: 'var(--text-faint)', marginBottom: '18px' };
export const closeXStyle: React.CSSProperties = {
  fontSize: '18px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: 'none',
  cursor: 'pointer',
  lineHeight: 1,
};

export const dropzoneStyle: React.CSSProperties = {
  border: '1px dashed var(--border)',
  borderRadius: '8px',
  padding: '20px',
  textAlign: 'center',
  marginBottom: '10px',
};
export const dropzoneTextStyle: React.CSSProperties = { fontSize: '12.5px', color: 'var(--text-faint)' };
export const orDividerStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--text-faint)',
  textAlign: 'center',
  margin: '4px 0',
};
export const pasteAreaStyle: React.CSSProperties = {
  width: '100%',
  minHeight: '120px',
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '10px 12px',
  outline: 'none',
  boxSizing: 'border-box',
  resize: 'vertical',
  marginBottom: '12px',
};

export const primaryBtnStyle: React.CSSProperties = {
  fontSize: '13px',
  fontWeight: 700,
  color: '#0b1210',
  background: 'var(--accent)',
  border: '1px solid var(--accent)',
  borderRadius: '6px',
  padding: '9px 14px',
  cursor: 'pointer',
  width: '100%',
};

export const secondaryBtnStyle: React.CSSProperties = {
  fontSize: '13px',
  fontWeight: 600,
  color: 'var(--text-dim)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  padding: '9px 14px',
  cursor: 'pointer',
};
