import { useVaultStore } from '../store/useVaultStore';
import { usePresence, useLastPresent } from '../lib/usePresence';
import styles from './Toast.module.css';

export function Toast() {
  const toastLive = useVaultStore((s) => s.toast);
  const { mounted, state } = usePresence(!!toastLive, 160);
  // The store nulls the message on a timer, so hold it through the fade or
  // the toast would empty itself before it finished leaving.
  const toast = useLastPresent(toastLive);

  if (!mounted || !toast) return null;

  const phase = state === 'entered' ? styles.isEntered : state === 'exiting' ? styles.isExiting : '';

  return (
    <div className={`${styles.toast} ${phase}`} role="status" aria-live="polite">
      {toast}
    </div>
  );
}
