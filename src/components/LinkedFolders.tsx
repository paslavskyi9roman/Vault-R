import { useVaultStore } from '../store/useVaultStore';
import styles from './LinkedFolders.module.css';

/// Shows which local directories resolve to the current environment via
/// `vault link`, so the mapping that makes `vault run` (no target) work is
/// visible somewhere in the GUI, not just CLI folklore.
export function LinkedFolders({ envId }: { envId: string }) {
  const projects = useVaultStore((s) => s.projects);
  const linkCurrentEnvToFolder = useVaultStore((s) => s.linkCurrentEnvToFolder);
  const unlinkProjectPath = useVaultStore((s) => s.unlinkProjectPath);

  const linked = projects.filter((p) => p.envId === envId);

  return (
    <div className={styles.wrap}>
      <div className={styles.headerRow}>
        <span className={styles.label}>LINKED FOLDERS</span>
        <button className={`v-btn ${styles.addBtn}`} onClick={() => void linkCurrentEnvToFolder()}>
          + Link a folder
        </button>
      </div>
      {linked.length === 0 ? (
        <div className={styles.empty}>
          No folders linked yet. Link one so <code>vault run</code> can omit the target here.
        </div>
      ) : (
        linked.map((p) => (
          <div key={p.id} className={styles.row}>
            <span className={styles.path} title={p.path}>
              {p.path}
            </span>
            <button
              className={`v-btn v-btn--danger ${styles.unlink}`}
              onClick={() => void unlinkProjectPath(p.path)}
            >
              Unlink
            </button>
          </div>
        ))
      )}
    </div>
  );
}
