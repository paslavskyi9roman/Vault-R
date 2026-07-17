import { useVaultStore } from '../store/useVaultStore';
import { envColor } from '../lib/envColor';
import { VariablesTable } from './VariablesTable';

export function MainPanel() {
  const repos = useVaultStore((s) => s.repos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const variables = useVaultStore((s) => s.variables);
  const openHistory = useVaultStore((s) => s.openHistory);
  const copyVariable = useVaultStore((s) => s.copyVariable);

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  if (!activeRepo) {
    return (
      <div style={mainStyle}>
        <div style={emptyStateStyle}>
          <div style={emptyGlyphStyle}>&#10095;_</div>
          <div style={emptyTitleStyle}>No repositories yet</div>
          <div style={emptySubStyle}>Add a repository from the sidebar to start storing secrets.</div>
        </div>
      </div>
    );
  }

  if (!activeEnv) {
    return (
      <div style={mainStyle}>
        <div style={emptyStateStyle}>
          <div style={emptyTitleStyle}>{activeRepo.name} has no environments yet</div>
          <div style={emptySubStyle}>Use the + next to the repo in the sidebar to add one (e.g. "local").</div>
        </div>
      </div>
    );
  }

  const cliLine1 = `vault export ${activeRepo.name}/${activeEnv.name} > .env`;
  const cliLine2 = `vault run ${activeRepo.name}/${activeEnv.name} -- <command>`;

  return (
    <div style={mainStyle}>
      <div style={breadcrumbRowStyle}>
        <div style={breadcrumbTextStyle}>
          <span style={breadcrumbRepoStyle}>{activeRepo.name}</span>
          <span style={breadcrumbSlashStyle}>/</span>
          <span style={{ ...envBadgeStyle, color: envColor(activeEnv.name) }}>{activeEnv.name}</span>
        </div>
        <div style={varCountTextStyle}>
          {variables.length} variable{variables.length === 1 ? '' : 's'}
        </div>
        <button style={historyBtnStyle} onClick={() => void openHistory()}>
          History
        </button>
      </div>

      <div style={cliBoxStyle}>
        <div style={cliLineRowStyle}>
          <span style={cliPromptStyle}>$</span>
          <span style={cliTextStyle}>{cliLine1}</span>
          <button style={cliCopyBtnStyle} onClick={() => void copyVariable(cliLine1, 'command')}>
            copy
          </button>
        </div>
        <div style={cliLineRowStyle}>
          <span style={cliPromptStyle}>$</span>
          <span style={cliTextStyle}>{cliLine2}</span>
          <button style={cliCopyBtnStyle} onClick={() => void copyVariable(cliLine2, 'command')}>
            copy
          </button>
        </div>
      </div>

      <VariablesTable />
    </div>
  );
}

const mainStyle: React.CSSProperties = { flex: 1, overflowY: 'auto', padding: '26px 34px 60px' };

const emptyStateStyle: React.CSSProperties = {
  height: '100%',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  textAlign: 'center',
  gap: '8px',
  color: 'var(--text-dim)',
};
const emptyGlyphStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  color: 'var(--accent)',
  fontWeight: 700,
  fontSize: '28px',
};
const emptyTitleStyle: React.CSSProperties = { fontSize: '15px', fontWeight: 700, color: 'var(--text)' };
const emptySubStyle: React.CSSProperties = { fontSize: '13px', color: 'var(--text-dim)', maxWidth: '360px' };

const breadcrumbRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '14px',
  marginBottom: '16px',
};
const breadcrumbTextStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '8px' };
const breadcrumbRepoStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '19px',
  fontWeight: 700,
  color: 'var(--text)',
};
const breadcrumbSlashStyle: React.CSSProperties = { color: 'var(--text-faint)', fontSize: '18px' };
const envBadgeStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12px',
  fontWeight: 700,
  background: 'rgba(255,255,255,0.05)',
  borderRadius: '5px',
  padding: '3px 9px',
};
const varCountTextStyle: React.CSSProperties = { fontSize: '12px', color: 'var(--text-faint)', marginLeft: '2px' };
const historyBtnStyle: React.CSSProperties = {
  marginLeft: 'auto',
  fontSize: '12.5px',
  fontWeight: 600,
  color: 'var(--text-dim)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  padding: '7px 12px',
  cursor: 'pointer',
};

const cliBoxStyle: React.CSSProperties = {
  background: 'var(--cli-bg)',
  border: '1px solid var(--border)',
  borderRadius: '8px',
  padding: '12px 14px',
  marginBottom: '20px',
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
};
const cliLineRowStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '10px' };
const cliPromptStyle: React.CSSProperties = { color: 'var(--accent)', fontFamily: 'var(--font-mono)', fontSize: '13px' };
const cliTextStyle: React.CSSProperties = {
  color: 'var(--text-dim)',
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  flex: 1,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};
const cliCopyBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '3px 8px',
  cursor: 'pointer',
};
