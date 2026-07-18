import { type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { envColor } from '../lib/envColor';

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

  return (
    <div style={sidebarStyle}>
      <div style={sidebarHeaderStyle}>
        <span style={sidebarHeaderTextStyle}>REPOSITORIES</span>
        <button style={addBtnStyle} onClick={toggleAddRepo}>
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
          style={inlineAddFormStyle}
        />
      )}

      <div style={repoListStyle}>
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
                  style={inlineAddFormStyle}
                />
              ) : (
                <div style={repoRowStyle} onClick={() => toggleExpandRepo(repo.id)}>
                  <span style={chevronStyle}>{expanded ? '▾' : '▸'}</span>
                  <span style={repoNameStyle}>{repo.name}</span>
                  <span style={countPillStyle}>{repo.envs.length}</span>
                  <span
                    style={rowActionStyle}
                    title={`Add environment to ${repo.name}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      startAddEnv(repo.id);
                    }}
                  >
                    +
                  </span>
                  <span
                    style={rowActionStyle}
                    title={`Rename ${repo.name}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      startRenameRepo(repo.id, repo.name);
                    }}
                  >
                    &#9998;
                  </span>
                  <span
                    style={rowActionDangerStyle}
                    title={`Delete ${repo.name}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      requestDeleteRepo(repo.id, repo.name);
                    }}
                  >
                    &times;
                  </span>
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
                          style={inlineAddEnvFormStyle}
                        />
                      );
                    }
                    return (
                      <div
                        key={env.id}
                        style={{
                          ...envRowStyle,
                          background: isActive ? 'var(--accent-dim)' : 'transparent',
                          borderLeft: isActive ? '2px solid var(--accent)' : '2px solid transparent',
                        }}
                        onClick={() => void selectEnv(repo.id, env.id)}
                      >
                        <span style={{ ...dotStyle, background: envColor(env.name) }} />
                        <span style={{ ...envNameStyle, color: isActive ? 'var(--text)' : 'var(--text-dim)' }}>
                          {env.name}
                        </span>
                        <span style={envCountStyle}>{env.varCount}</span>
                        <span
                          style={rowActionStyle}
                          title={`Rename ${env.name}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            startRenameEnv(env.id, env.name);
                          }}
                        >
                          &#9998;
                        </span>
                        <span
                          style={rowActionDangerStyle}
                          title={`Delete ${env.name}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            requestDeleteEnv(env.id, repo.name, env.name);
                          }}
                        >
                          &times;
                        </span>
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
                      style={inlineAddEnvFormStyle}
                    />
                  )}
                </div>
              )}
            </div>
          );
        })}

        {repos.length === 0 && !addingRepo && (
          <div style={emptyHintStyle}>No repositories yet. Click + to add one.</div>
        )}
      </div>

      <div style={sidebarFooterStyle}>
        <div style={sidebarFooterLineStyle}>
          {linkedGroupCount} linked secret group{linkedGroupCount === 1 ? '' : 's'} across repos
        </div>
      </div>
    </div>
  );
}

function InlineForm(props: {
  placeholder: string;
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  submitLabel?: string;
  style: React.CSSProperties;
}) {
  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') props.onSubmit();
    if (e.key === 'Escape') props.onCancel();
  }
  return (
    <div style={props.style}>
      <input
        style={inlineAddInputStyle}
        placeholder={props.placeholder}
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
        onKeyDown={onKeyDown}
        autoFocus
        spellCheck={false}
        autoComplete="off"
      />
      <button style={inlineAddConfirmStyle} onClick={props.onSubmit}>
        {props.submitLabel ?? 'Add'}
      </button>
      <button style={inlineAddCancelStyle} onClick={props.onCancel}>
        &times;
      </button>
    </div>
  );
}

const sidebarStyle: React.CSSProperties = {
  width: '260px',
  flexShrink: 0,
  background: 'var(--panel)',
  borderRight: '1px solid var(--border)',
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
};
const sidebarHeaderStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', padding: '14px 14px 8px' };
const sidebarHeaderTextStyle: React.CSSProperties = {
  fontSize: '10.5px',
  fontWeight: 700,
  letterSpacing: '0.8px',
  color: 'var(--text-faint)',
  flex: 1,
};
const addBtnStyle: React.CSSProperties = {
  width: '20px',
  height: '20px',
  borderRadius: '5px',
  border: '1px solid var(--border)',
  background: 'transparent',
  color: 'var(--text-dim)',
  cursor: 'pointer',
  fontSize: '13px',
  lineHeight: '1',
};
const rowActionStyle: React.CSSProperties = {
  width: '16px',
  height: '16px',
  borderRadius: '4px',
  color: 'var(--text-faint)',
  cursor: 'pointer',
  fontSize: '12px',
  textAlign: 'center',
  lineHeight: '16px',
  flexShrink: 0,
};
const rowActionDangerStyle: React.CSSProperties = { ...rowActionStyle, fontSize: '14px' };
const repoListStyle: React.CSSProperties = { flex: 1, overflowY: 'auto', paddingBottom: '10px' };
const inlineAddFormStyle: React.CSSProperties = { display: 'flex', gap: '5px', padding: '4px 14px 10px' };
const inlineAddEnvFormStyle: React.CSSProperties = { display: 'flex', gap: '5px', padding: '4px 14px 8px 30px' };
const inlineAddInputStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  fontFamily: 'var(--font-mono)',
  fontSize: '12px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  color: 'var(--text)',
  padding: '5px 8px',
  outline: 'none',
};
const inlineAddConfirmStyle: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 700,
  color: 'var(--accent)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '0 8px',
  cursor: 'pointer',
};
const inlineAddCancelStyle: React.CSSProperties = {
  fontSize: '13px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: 'none',
  cursor: 'pointer',
};
const repoRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '9px 14px',
  cursor: 'pointer',
  userSelect: 'none',
};
const chevronStyle: React.CSSProperties = { color: 'var(--text-faint)', fontSize: '10px', width: '10px' };
const repoNameStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  color: 'var(--text)',
  fontWeight: 600,
  flex: 1,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};
const countPillStyle: React.CSSProperties = {
  fontSize: '10px',
  color: 'var(--text-faint)',
  background: 'var(--panel-2)',
  borderRadius: '8px',
  padding: '1px 6px',
};
const envRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '9px',
  padding: '7px 14px 7px 30px',
  cursor: 'pointer',
};
const dotStyle: React.CSSProperties = { width: '6px', height: '6px', borderRadius: '50%', flexShrink: 0 };
const envNameStyle: React.CSSProperties = { fontFamily: 'var(--font-mono)', fontSize: '12.5px', flex: 1 };
const envCountStyle: React.CSSProperties = { fontSize: '10.5px', color: 'var(--text-faint)' };
const sidebarFooterStyle: React.CSSProperties = { padding: '10px 14px', borderTop: '1px solid var(--border)' };
const sidebarFooterLineStyle: React.CSSProperties = { fontSize: '10.5px', color: 'var(--text-faint)', lineHeight: 1.5 };
const emptyHintStyle: React.CSSProperties = {
  padding: '10px 14px',
  fontSize: '11.5px',
  color: 'var(--text-faint)',
  lineHeight: 1.5,
};
