import { useVaultStore } from '../store/useVaultStore';
import { envColor } from '../lib/envColor';
import { VariablesTable } from './VariablesTable';
import { LinkedFolders } from './LinkedFolders';
import styles from './MainPanel.module.css';

export function MainPanel() {
  const repos = useVaultStore((s) => s.repos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const variables = useVaultStore((s) => s.variables);
  const openHistory = useVaultStore((s) => s.openHistory);
  const openCompare = useVaultStore((s) => s.openCompare);
  const copyVariable = useVaultStore((s) => s.copyVariable);

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  if (!activeRepo) {
    return (
      <div className={styles.main}>
        <div className={styles.emptyState}>
          <div className={styles.emptyGlyph}>&#10095;_</div>
          <div className={styles.emptyTitle}>No repositories yet</div>
          <div className={styles.emptySub}>Add a repository from the sidebar to start storing secrets.</div>
        </div>
      </div>
    );
  }

  if (!activeEnv) {
    return (
      <div className={styles.main}>
        <div className={styles.emptyState}>
          <div className={styles.emptyTitle}>{activeRepo.name} has no environments yet</div>
          <div className={styles.emptySub}>
            Use the + next to the repo in the sidebar to add one (e.g. "local").
          </div>
        </div>
      </div>
    );
  }

  const cliLine1 = `vault export ${activeRepo.name}/${activeEnv.name} > .env`;
  const cliLine2 = `vault run ${activeRepo.name}/${activeEnv.name} -- <command>`;
  const requiredCount = variables.filter((v) => v.required).length;
  const missingCount = variables.filter((v) => v.required && !v.value.trim()).length;

  return (
    <div className={styles.main}>
      <div className={styles.breadcrumbRow}>
        <div className={styles.breadcrumbText}>
          <span className={styles.breadcrumbRepo}>{activeRepo.name}</span>
          <span className={styles.breadcrumbSlash}>/</span>
          <span
            className={styles.envBadge}
            style={{ ['--env-tone' as string]: envColor(activeEnv.name) }}
          >
            {activeEnv.name}
          </span>
        </div>
        <div className={styles.varCount}>
          {variables.length} variable{variables.length === 1 ? '' : 's'}
          {requiredCount > 0 && (
            <span className={styles.required} data-missing={missingCount > 0}>
              {' · '}
              {missingCount > 0 ? `${missingCount}/${requiredCount} required missing` : `${requiredCount} required`}
            </span>
          )}
        </div>
        <div className={styles.panelActions}>
          <button className="v-btn" onClick={openCompare}>
            Compare
          </button>
          <button className="v-btn" onClick={() => void openHistory()}>
            History
          </button>
        </div>
      </div>

      <div className={styles.cliBox}>
        <div className={styles.cliLineRow}>
          <span className={styles.cliPrompt}>$</span>
          <span className={styles.cliText}>{cliLine1}</span>
          <button className={`v-btn ${styles.cliCopy}`} onClick={() => void copyVariable(cliLine1, 'command')}>
            copy
          </button>
        </div>
        <div className={styles.cliLineRow}>
          <span className={styles.cliPrompt}>$</span>
          <span className={styles.cliText}>{cliLine2}</span>
          <button className={`v-btn ${styles.cliCopy}`} onClick={() => void copyVariable(cliLine2, 'command')}>
            copy
          </button>
        </div>
      </div>

      <LinkedFolders envId={activeEnv.id} />

      <VariablesTable />
    </div>
  );
}
