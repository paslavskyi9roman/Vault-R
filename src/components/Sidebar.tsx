import { type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { envColor } from '../lib/envColor';
import { Spinner } from './Spinner';
import styles from './Sidebar.module.css';

export function Sidebar() {
  const repos = useVaultStore((s) => s.repos);
  const expandedRepos = useVaultStore((s) => s.expandedRepos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const addingRepo = useVaultStore((s) => s.addingRepo);
  const newRepoName = useVaultStore((s) => s.newRepoName);
  const addingEnvFor = useVaultStore((s) => s.addingEnvFor);
  const newEnvName = useVaultStore((s) => s.newEnvName);
  const linkedGroupCount = useVaultStore((s) => s.linkedGroupCount);
  const renamingRepoId = useVaultStore((s) => s.renamingRepoId);
  const renamingEnvId = useVaultStore((s) => s.renamingEnvId);
  const renameDraft = useVaultStore((s) => s.renameDraft);
  const duplicatingEnvId = useVaultStore((s) => s.duplicatingEnvId);
  const duplicateNewName = useVaultStore((s) => s.duplicateNewName);
  const duplicateCopyValues = useVaultStore((s) => s.duplicateCopyValues);
  const duplicateBusy = useVaultStore((s) => s.duplicateBusy);

  const toggleExpandRepo = useVaultStore((s) => s.toggleExpandRepo);
  const toggleAddRepo = useVaultStore((s) => s.toggleAddRepo);
  const setNewRepoName = useVaultStore((s) => s.setNewRepoName);
  const submitAddRepo = useVaultStore((s) => s.submitAddRepo);
  const startAddEnv = useVaultStore((s) => s.startAddEnv);
  const cancelAddEnv = useVaultStore((s) => s.cancelAddEnv);
  const setNewEnvName = useVaultStore((s) => s.setNewEnvName);
  const submitAddEnv = useVaultStore((s) => s.submitAddEnv);
  const selectEnv = useVaultStore((s) => s.selectEnv);
  const startRenameRepo = useVaultStore((s) => s.startRenameRepo);
  const startRenameEnv = useVaultStore((s) => s.startRenameEnv);
  const setRenameDraft = useVaultStore((s) => s.setRenameDraft);
  const cancelRename = useVaultStore((s) => s.cancelRename);
  const submitRename = useVaultStore((s) => s.submitRename);
  const requestDeleteRepo = useVaultStore((s) => s.requestDeleteRepo);
  const requestDeleteEnv = useVaultStore((s) => s.requestDeleteEnv);
  const startDuplicateEnv = useVaultStore((s) => s.startDuplicateEnv);
  const cancelDuplicateEnv = useVaultStore((s) => s.cancelDuplicateEnv);
  const setDuplicateNewName = useVaultStore((s) => s.setDuplicateNewName);
  const toggleDuplicateCopyValues = useVaultStore((s) => s.toggleDuplicateCopyValues);
  const submitDuplicateEnv = useVaultStore((s) => s.submitDuplicateEnv);

  return (
    <nav className={styles.sidebar}>
      <div className={styles.header}>
        <span className={styles.headerText}>REPOSITORIES</span>
        <button className={`v-btn ${styles.addBtn}`} onClick={toggleAddRepo} title="Add a repository">
          +
        </button>
      </div>

      {addingRepo && (
        <InlineForm
          placeholder="repo-name"
          value={newRepoName}
          onChange={setNewRepoName}
          onSubmit={() => void submitAddRepo()}
          onCancel={toggleAddRepo}
        />
      )}

      <div className={styles.repoList}>
        {repos.map((repo) => {
          const expanded = !!expandedRepos[repo.id];
          return (
            <div key={repo.id}>
              {renamingRepoId === repo.id ? (
                <InlineForm
                  placeholder="repo-name"
                  value={renameDraft}
                  onChange={setRenameDraft}
                  onSubmit={() => void submitRename()}
                  onCancel={cancelRename}
                  submitLabel="Save"
                />
              ) : (
                <div className={styles.row}>
                  <button
                    className={styles.repoMain}
                    onClick={() => toggleExpandRepo(repo.id)}
                    aria-expanded={expanded}
                  >
                    <span className={styles.chevron}>{expanded ? '▾' : '▸'}</span>
                    <span className={styles.repoName}>{repo.name}</span>
                    <span className={styles.countPill}>{repo.envs.length}</span>
                  </button>
                  <div className={styles.actions}>
                    <button
                      className={styles.action}
                      title={`Add environment to ${repo.name}`}
                      onClick={() => startAddEnv(repo.id)}
                    >
                      +
                    </button>
                    <button
                      className={styles.action}
                      title={`Rename ${repo.name}`}
                      onClick={() => startRenameRepo(repo.id, repo.name)}
                    >
                      &#9998;
                    </button>
                    <button
                      className={`${styles.action} ${styles.actionDanger}`}
                      title={`Delete ${repo.name}`}
                      onClick={() => requestDeleteRepo(repo.id, repo.name)}
                    >
                      &times;
                    </button>
                  </div>
                </div>
              )}

              {expanded && (
                <div>
                  {repo.envs.map((env) => {
                    const isActive = repo.id === activeRepoId && env.id === activeEnvId;
                    if (renamingEnvId === env.id) {
                      return (
                        <InlineForm
                          key={env.id}
                          placeholder="env-name"
                          value={renameDraft}
                          onChange={setRenameDraft}
                          onSubmit={() => void submitRename()}
                          onCancel={cancelRename}
                          submitLabel="Save"
                          nested
                        />
                      );
                    }
                    if (duplicatingEnvId === env.id) {
                      return (
                        <div key={env.id} className={styles.duplicateForm}>
                          <input
                            className={`v-input ${styles.inlineInput}`}
                            placeholder="new-env-name"
                            value={duplicateNewName}
                            onChange={(e) => setDuplicateNewName(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') void submitDuplicateEnv();
                              if (e.key === 'Escape') cancelDuplicateEnv();
                            }}
                            autoFocus
                            spellCheck={false}
                            autoComplete="off"
                          />
                          <label className={styles.duplicateCheckbox}>
                            <input
                              className="v-check"
                              type="checkbox"
                              checked={duplicateCopyValues}
                              onChange={toggleDuplicateCopyValues}
                            />
                            copy values
                          </label>
                          <button
                            className={`v-btn ${styles.inlineConfirm}`}
                            onClick={() => void submitDuplicateEnv()}
                            disabled={duplicateBusy}
                          >
                            Duplicate
                            {duplicateBusy && <Spinner size={10} />}
                          </button>
                          <button className={styles.inlineCancel} onClick={cancelDuplicateEnv} aria-label="Cancel">
                            &times;
                          </button>
                        </div>
                      );
                    }
                    return (
                      <div key={env.id} className={styles.row} data-active={isActive}>
                        <button className={styles.envMain} onClick={() => void selectEnv(repo.id, env.id)}>
                          <span
                            className={styles.dot}
                            style={{ ['--dot' as string]: envColor(env.name) }}
                          />
                          <span className={styles.envName}>{env.name}</span>
                          <span className={styles.envCount}>{env.varCount}</span>
                        </button>
                        <div className={styles.actions}>
                          <button
                            className={styles.action}
                            title={`Duplicate ${env.name}`}
                            onClick={() => startDuplicateEnv(env.id, env.name)}
                          >
                            &#10697;
                          </button>
                          <button
                            className={styles.action}
                            title={`Rename ${env.name}`}
                            onClick={() => startRenameEnv(env.id, env.name)}
                          >
                            &#9998;
                          </button>
                          <button
                            className={`${styles.action} ${styles.actionDanger}`}
                            title={`Delete ${env.name}`}
                            onClick={() => requestDeleteEnv(env.id, repo.name, env.name)}
                          >
                            &times;
                          </button>
                        </div>
                      </div>
                    );
                  })}

                  {addingEnvFor === repo.id && (
                    <InlineForm
                      placeholder="env-name"
                      value={newEnvName}
                      onChange={setNewEnvName}
                      onSubmit={() => void submitAddEnv(repo.id)}
                      onCancel={cancelAddEnv}
                      nested
                    />
                  )}
                </div>
              )}
            </div>
          );
        })}

        {repos.length === 0 && !addingRepo && (
          <div className={styles.emptyHint}>No repositories yet. Click + to add one.</div>
        )}
      </div>

      <div className={styles.footer}>
        <div className={styles.footerLine}>
          {linkedGroupCount} linked secret group{linkedGroupCount === 1 ? '' : 's'} across repos
        </div>
      </div>
    </nav>
  );
}

function InlineForm(props: {
  placeholder: string;
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  submitLabel?: string;
  nested?: boolean;
}) {
  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') props.onSubmit();
    if (e.key === 'Escape') props.onCancel();
  }
  return (
    <div className={`${styles.inlineForm} ${props.nested ? styles.inlineFormEnv : ''}`}>
      <input
        className={`v-input ${styles.inlineInput}`}
        placeholder={props.placeholder}
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
        onKeyDown={onKeyDown}
        autoFocus
        spellCheck={false}
        autoComplete="off"
      />
      <button className={`v-btn ${styles.inlineConfirm}`} onClick={props.onSubmit}>
        {props.submitLabel ?? 'Add'}
      </button>
      <button className={styles.inlineCancel} onClick={props.onCancel} aria-label="Cancel">
        &times;
      </button>
    </div>
  );
}
