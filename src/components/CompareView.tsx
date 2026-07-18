import { useVaultStore } from '../store/useVaultStore';
import { overlayBackdropStyle, closeXStyle } from './overlayStyles';
import { envColor } from '../lib/envColor';
import type { DiffRow } from '../lib/api';

export function CompareView() {
  const compareOpen = useVaultStore((s) => s.compareOpen);
  const closeCompare = useVaultStore((s) => s.closeCompare);
  const repos = useVaultStore((s) => s.repos);
  const activeRepoId = useVaultStore((s) => s.activeRepoId);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const compareEnvBId = useVaultStore((s) => s.compareEnvBId);
  const setCompareEnvB = useVaultStore((s) => s.setCompareEnvB);
  const compareRows = useVaultStore((s) => s.compareRows);
  const compareUnlinkedMatches = useVaultStore((s) => s.compareUnlinkedMatches);
  const compareBusy = useVaultStore((s) => s.compareBusy);
  const copyAllMissing = useVaultStore((s) => s.copyAllMissing);
  const linkCompareMatch = useVaultStore((s) => s.linkCompareMatch);

  if (!compareOpen) return null;

  const activeRepo = repos.find((r) => r.id === activeRepoId);
  const activeEnv = activeRepo?.envs.find((e) => e.id === activeEnvId);

  const envOptions: { id: string; label: string }[] = [];
  repos.forEach((r) => r.envs.forEach((e) => {
    if (e.id !== activeEnvId) envOptions.push({ id: e.id, label: `${r.name}/${e.name}` });
  }));

  let envBLabel = '';
  repos.forEach((r) => r.envs.forEach((e) => {
    if (e.id === compareEnvBId) envBLabel = `${r.name}/${e.name}`;
  }));

  const onlyInA = compareRows.filter((r) => r.kind === 'removed');
  const onlyInB = compareRows.filter((r) => r.kind === 'added');
  const differing = compareRows.filter((r) => r.kind === 'changed');

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeCompare} />
      <div style={slideoverStyle}>
        <div style={slideoverHeaderStyle}>
          <span style={slideoverTitleStyle}>Compare environments</span>
          <button style={closeXStyle} onClick={closeCompare}>
            &times;
          </button>
        </div>

        <div style={pickerRowStyle}>
          <span style={{ ...envPillStyle, color: activeEnv ? envColor(activeEnv.name) : undefined }}>
            {activeRepo && activeEnv ? `${activeRepo.name}/${activeEnv.name}` : '—'}
          </span>
          <span style={vsStyle}>vs</span>
          <select
            style={envSelectStyle}
            value={compareEnvBId ?? ''}
            onChange={(e) => void setCompareEnvB(e.target.value)}
          >
            <option value="" disabled>
              Choose an environment…
            </option>
            {envOptions.map((o) => (
              <option key={o.id} value={o.id}>
                {o.label}
              </option>
            ))}
          </select>
        </div>

        {!compareEnvBId && <div style={emptyStyle}>Pick an environment to compare against.</div>}

        {compareEnvBId && !compareBusy && compareRows.length === 0 && compareUnlinkedMatches.length === 0 && (
          <div style={emptyStyle}>These environments match exactly.</div>
        )}

        {compareUnlinkedMatches.length > 0 && (
          <div style={sectionStyle}>
            <div style={sectionTitleStyle}>
              {compareUnlinkedMatches.length} secret{compareUnlinkedMatches.length === 1 ? '' : 's'} match but
              {compareUnlinkedMatches.length === 1 ? " isn't" : " aren't"} linked
            </div>
            {compareUnlinkedMatches.map((m) => (
              <div key={m.key} style={matchRowStyle}>
                <span style={diffKeyStyle}>{m.key}</span>
                <button style={smallBtnStyle} onClick={() => void linkCompareMatch(m)}>
                  Link these
                </button>
              </div>
            ))}
          </div>
        )}

        {onlyInA.length > 0 && (
          <DiffSection
            title={`Only in ${activeEnv?.name ?? 'A'}`}
            rows={onlyInA}
            onCopyAll={() => void copyAllMissing('aToB')}
            copyAllLabel={`Copy all → ${envBLabel}`}
          />
        )}
        {onlyInB.length > 0 && (
          <DiffSection
            title={`Only in ${envBLabel || 'B'}`}
            rows={onlyInB}
            onCopyAll={() => void copyAllMissing('bToA')}
            copyAllLabel={`Copy all → ${activeEnv?.name ?? 'A'}`}
          />
        )}
        {differing.length > 0 && <DiffSection title="Different values" rows={differing} />}
      </div>
    </>
  );
}

function DiffSection({
  title,
  rows,
  onCopyAll,
  copyAllLabel,
}: {
  title: string;
  rows: DiffRow[];
  onCopyAll?: () => void;
  copyAllLabel?: string;
}) {
  return (
    <div style={sectionStyle}>
      <div style={sectionHeaderRowStyle}>
        <span style={sectionTitleStyle}>{title}</span>
        {onCopyAll && (
          <button style={smallBtnStyle} onClick={onCopyAll}>
            {copyAllLabel}
          </button>
        )}
      </div>
      {rows.map((row) => (
        <CompareRow key={row.key} row={row} />
      ))}
    </div>
  );
}

function CompareRow({ row }: { row: DiffRow }) {
  const revealed = useVaultStore((s) => !!s.compareRevealed[row.key]);
  const toggleCompareReveal = useVaultStore((s) => s.toggleCompareReveal);
  const copyCompareRow = useVaultStore((s) => s.copyCompareRow);

  return (
    <div style={diffRowStyle}>
      <div style={diffBodyStyle}>
        <div style={diffKeyStyle}>{row.key}</div>
        <div style={diffValueStyle}>{revealed ? describeValues(row) : '••••••••'}</div>
      </div>
      <button style={diffIconBtnStyle} onClick={() => toggleCompareReveal(row.key)}>
        {revealed ? 'hide' : 'show'}
      </button>
      {row.kind !== 'added' && (
        <button
          style={diffIconBtnStyle}
          title="Copy A's value into B"
          onClick={() => void copyCompareRow(row.key, 'aToB')}
        >
          copy &rarr;
        </button>
      )}
      {row.kind !== 'removed' && (
        <button
          style={diffIconBtnStyle}
          title="Copy B's value into A"
          onClick={() => void copyCompareRow(row.key, 'bToA')}
        >
          &larr; copy
        </button>
      )}
    </div>
  );
}

function describeValues(row: DiffRow): string {
  if (row.kind === 'changed') return `${row.oldValue ?? ''} → ${row.newValue ?? ''}`;
  if (row.kind === 'added') return row.newValue ?? '';
  return row.oldValue ?? '';
}

const slideoverStyle: React.CSSProperties = {
  position: 'fixed',
  top: 0,
  right: 0,
  bottom: 0,
  width: '480px',
  background: 'var(--panel)',
  borderLeft: '1px solid var(--border)',
  zIndex: 41,
  padding: '22px 22px',
  overflowY: 'auto',
  animation: 'vaultSlideIn 0.22s ease',
};
const slideoverHeaderStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', marginBottom: '14px' };
const slideoverTitleStyle: React.CSSProperties = { fontSize: '16px', fontWeight: 700, color: 'var(--text)', flex: 1 };
const pickerRowStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '18px' };
const envPillStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  fontWeight: 700,
  background: 'rgba(255,255,255,0.05)',
  borderRadius: '5px',
  padding: '5px 9px',
};
const vsStyle: React.CSSProperties = { fontSize: '11.5px', color: 'var(--text-faint)' };
const envSelectStyle: React.CSSProperties = {
  flex: 1,
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '6px 8px',
  outline: 'none',
};
const emptyStyle: React.CSSProperties = { fontSize: '12.5px', color: 'var(--text-faint)' };
const sectionStyle: React.CSSProperties = { marginBottom: '18px' };
const sectionHeaderRowStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '8px' };
const sectionTitleStyle: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 700,
  color: 'var(--text-faint)',
  letterSpacing: '0.4px',
  flex: 1,
};
const matchRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '7px 0',
  borderBottom: '1px solid var(--border-light)',
};
const smallBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 600,
  color: 'var(--accent)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 8px',
  cursor: 'pointer',
  whiteSpace: 'nowrap',
};
const diffRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  padding: '7px 0',
  borderBottom: '1px solid var(--border-light)',
};
const diffBodyStyle: React.CSSProperties = { flex: 1, minWidth: 0 };
const diffKeyStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  color: 'var(--key)',
};
const diffValueStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '11px',
  color: 'var(--text-faint)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};
const diffIconBtnStyle: React.CSSProperties = {
  fontSize: '10px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '4px',
  padding: '3px 6px',
  cursor: 'pointer',
  flexShrink: 0,
  whiteSpace: 'nowrap',
};
