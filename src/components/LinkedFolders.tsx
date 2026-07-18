import { useVaultStore } from '../store/useVaultStore';

/// Shows which local directories resolve to the current environment via
/// `vault link`, so the mapping that makes `vault run` (no target) work is
/// visible somewhere in the GUI, not just CLI folklore.
export function LinkedFolders({ envId }: { envId: string }) {
  const projects = useVaultStore((s) => s.projects);
  const linkCurrentEnvToFolder = useVaultStore((s) => s.linkCurrentEnvToFolder);
  const unlinkProjectPath = useVaultStore((s) => s.unlinkProjectPath);

  const linked = projects.filter((p) => p.envId === envId);

  return (
    <div style={wrapStyle}>
      <div style={headerRowStyle}>
        <span style={labelStyle}>LINKED FOLDERS</span>
        <button style={addBtnStyle} onClick={() => void linkCurrentEnvToFolder()}>
          + Link a folder
        </button>
      </div>
      {linked.length === 0 ? (
        <div style={emptyStyle}>
          No folders linked yet. Link one so <code>vault run</code> can omit the target here.
        </div>
      ) : (
        linked.map((p) => (
          <div key={p.id} style={rowStyle}>
            <span style={pathStyle} title={p.path}>
              {p.path}
            </span>
            <button style={unlinkBtnStyle} onClick={() => void unlinkProjectPath(p.path)}>
              Unlink
            </button>
          </div>
        ))
      )}
    </div>
  );
}

const wrapStyle: React.CSSProperties = {
  marginBottom: '20px',
  border: '1px solid var(--border)',
  borderRadius: '8px',
  padding: '10px 14px',
};
const headerRowStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', marginBottom: '6px' };
const labelStyle: React.CSSProperties = {
  fontSize: '10.5px',
  fontWeight: 700,
  color: 'var(--text-faint)',
  letterSpacing: '0.5px',
  flex: 1,
};
const addBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 600,
  color: 'var(--accent)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 8px',
  cursor: 'pointer',
};
const emptyStyle: React.CSSProperties = { fontSize: '11.5px', color: 'var(--text-faint)', lineHeight: 1.5 };
const rowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '5px 0',
};
const pathStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  fontFamily: 'var(--font-mono)',
  fontSize: '11.5px',
  color: 'var(--text-dim)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};
const unlinkBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--danger)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '3px 8px',
  cursor: 'pointer',
  flexShrink: 0,
};
