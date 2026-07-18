import { type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle, modalStyle, modalTitleStyle, secondaryBtnStyle } from './overlayStyles';

/// Shared confirmation gate for destructive actions. Driven entirely from the
/// store so any action can raise one via `requestConfirm`.
export function ConfirmDialog() {
  const confirm = useVaultStore((s) => s.confirm);
  const confirmInput = useVaultStore((s) => s.confirmInput);
  const confirmBusy = useVaultStore((s) => s.confirmBusy);
  const setConfirmInput = useVaultStore((s) => s.setConfirmInput);
  const cancelConfirm = useVaultStore((s) => s.cancelConfirm);
  const acceptConfirm = useVaultStore((s) => s.acceptConfirm);

  if (!confirm) return null;

  const needsTyping = !!confirm.requireTypedName;
  const typedOk = !needsTyping || confirmInput.trim() === confirm.requireTypedName;
  const canConfirm = typedOk && !confirmBusy;

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter' && canConfirm) void acceptConfirm();
    if (e.key === 'Escape') cancelConfirm();
  }

  const accent = confirm.danger ? 'var(--danger)' : 'var(--accent)';

  return (
    <>
      <div style={{ ...overlayBackdropStyle, zIndex: 60 }} onClick={confirmBusy ? undefined : cancelConfirm} />
      <div style={{ ...modalStyle, width: '400px', zIndex: 61, borderColor: accent }}>
        <div style={{ ...modalTitleStyle, marginBottom: '8px' }}>{confirm.title}</div>
        <div style={messageStyle}>{confirm.message}</div>

        {needsTyping && (
          <>
            <div style={typeHintStyle}>
              Type <span style={typeTargetStyle}>{confirm.requireTypedName}</span> to confirm.
            </div>
            <input
              style={typeInputStyle}
              value={confirmInput}
              onChange={(e) => setConfirmInput(e.target.value)}
              onKeyDown={onKeyDown}
              placeholder={confirm.requireTypedName}
              autoFocus
              spellCheck={false}
              autoComplete="off"
            />
          </>
        )}

        <div style={actionsStyle}>
          <button style={secondaryBtnStyle} onClick={cancelConfirm} disabled={confirmBusy}>
            Cancel
          </button>
          <button
            style={{
              ...confirmBtnStyle,
              background: accent,
              borderColor: accent,
              opacity: canConfirm ? 1 : 0.4,
              cursor: canConfirm ? 'pointer' : 'not-allowed',
            }}
            onClick={() => void acceptConfirm()}
            disabled={!canConfirm}
          >
            {confirmBusy ? 'Working…' : confirm.confirmLabel}
          </button>
        </div>
      </div>
    </>
  );
}

const messageStyle: React.CSSProperties = {
  fontSize: '12.5px',
  color: 'var(--text-dim)',
  lineHeight: 1.6,
  marginBottom: '16px',
};
const typeHintStyle: React.CSSProperties = {
  fontSize: '11.5px',
  color: 'var(--text-faint)',
  marginBottom: '7px',
};
const typeTargetStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  color: 'var(--text)',
};
const typeInputStyle: React.CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '8px 10px',
  outline: 'none',
  marginBottom: '16px',
};
const actionsStyle: React.CSSProperties = {
  display: 'flex',
  gap: '8px',
  justifyContent: 'flex-end',
};
const confirmBtnStyle: React.CSSProperties = {
  fontSize: '13px',
  fontWeight: 700,
  color: '#0b1210',
  border: '1px solid',
  borderRadius: '6px',
  padding: '9px 14px',
};
