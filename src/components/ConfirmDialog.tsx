import { type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { usePresence, useLastPresent } from '../lib/usePresence';
import { Spinner } from './Spinner';
import styles from './ConfirmDialog.module.css';

/// Shared confirmation gate for destructive actions. Driven entirely from the
/// store so any action can raise one via `requestConfirm`.
export function ConfirmDialog() {
  const confirmLive = useVaultStore((s) => s.confirm);
  const confirmInput = useVaultStore((s) => s.confirmInput);
  const confirmBusy = useVaultStore((s) => s.confirmBusy);
  const setConfirmInput = useVaultStore((s) => s.setConfirmInput);
  const cancelConfirm = useVaultStore((s) => s.cancelConfirm);
  const acceptConfirm = useVaultStore((s) => s.acceptConfirm);

  const { mounted, state } = usePresence(!!confirmLive, 120);
  // cancelConfirm nulls `confirm`, so keep the last one to draw the exit with.
  const confirm = useLastPresent(confirmLive);

  if (!mounted || !confirm) return null;

  const needsTyping = !!confirm.requireTypedName;
  const typedOk = !needsTyping || confirmInput.trim() === confirm.requireTypedName;
  const canConfirm = typedOk && !confirmBusy;

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter' && canConfirm) void acceptConfirm();
    if (e.key === 'Escape') cancelConfirm();
  }

  const tone = { '--accent-tone': confirm.danger ? 'var(--danger)' : 'var(--accent)' } as React.CSSProperties;

  return (
    <>
      <div
        className={`v-backdrop v-backdrop--top is-${state}`}
        onClick={confirmBusy ? undefined : cancelConfirm}
      />
      <div className="v-modal-wrap v-modal-wrap--top" style={tone}>
        <div className={`v-modal ${styles.dialog} is-${state}`}>
          <div className={`v-modal-title ${styles.title}`}>{confirm.title}</div>
          <div className={styles.message}>{confirm.message}</div>

          {needsTyping && (
            <>
              <div className={styles.typeHint}>
                Type <span className={styles.typeTarget}>{confirm.requireTypedName}</span> to confirm.
              </div>
              <input
                className={`v-input ${styles.typeInput}`}
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

          <div className={styles.actions}>
            <button className="v-btn" onClick={cancelConfirm} disabled={confirmBusy}>
              Cancel
            </button>
            <button
              className={`v-btn ${styles.confirmBtn}`}
              onClick={() => void acceptConfirm()}
              disabled={!canConfirm}
              data-pending={confirmBusy}
            >
              {confirm.confirmLabel}
              {confirmBusy && <Spinner size={13} />}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
