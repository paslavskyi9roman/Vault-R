import { useVaultStore, type SafetyTab } from '../store/useVaultStore';
import { overlayBackdropStyle, closeXStyle } from './overlayStyles';
import type { DuplicateValueGroup, LeakReport, SecretHealthRow } from '../lib/api';

/// The safety panel: what git can see (leak guard) and what is wrong inside
/// the vault (secret health).
///
/// Nothing in either tab reveals a secret value — findings carry keys, paths
/// and ages only — so unlike the compare and history views this needs no
/// production acknowledgement and is safe to open while sharing a screen.
export function SafetyPanel() {
  const safetyOpen = useVaultStore((s) => s.safetyOpen);
  const safetyTab = useVaultStore((s) => s.safetyTab);
  const closeSafety = useVaultStore((s) => s.closeSafety);
  const setSafetyTab = useVaultStore((s) => s.setSafetyTab);

  if (!safetyOpen) return null;

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeSafety} />
      <div style={slideoverStyle}>
        <div style={headerStyle}>
          <span style={titleStyle}>Safety</span>
          <button style={closeXStyle} onClick={closeSafety}>
            &times;
          </button>
        </div>
        <div style={tabRowStyle}>
          <TabButton tab="leaks" label="Git leak guard" active={safetyTab === 'leaks'} onSelect={setSafetyTab} />
          <TabButton tab="health" label="Secret health" active={safetyTab === 'health'} onSelect={setSafetyTab} />
        </div>
        {safetyTab === 'leaks' ? <LeakTab /> : <HealthTab />}
      </div>
    </>
  );
}

function TabButton({
  tab,
  label,
  active,
  onSelect,
}: {
  tab: SafetyTab;
  label: string;
  active: boolean;
  onSelect: (tab: SafetyTab) => Promise<void>;
}) {
  return (
    <button
      style={active ? { ...tabBtnStyle, ...tabBtnActiveStyle } : tabBtnStyle}
      onClick={() => void onSelect(tab)}
    >
      {label}
    </button>
  );
}

// ---------------------------------------------------------------------
// Git leak guard
// ---------------------------------------------------------------------

function LeakTab() {
  const leakReports = useVaultStore((s) => s.leakReports);
  const leakScanning = useVaultStore((s) => s.leakScanning);
  const leakScanned = useVaultStore((s) => s.leakScanned);
  const runLeakScan = useVaultStore((s) => s.runLeakScan);
  const projects = useVaultStore((s) => s.projects);

  if (leakScanning) return <div style={emptyStyle}>Scanning your linked folders…</div>;

  if (projects.length === 0) {
    return (
      <div style={emptyStyle}>
        No folders are linked yet. Run <code>vault link &lt;repo&gt;/&lt;env&gt;</code> in a project
        directory (or use "Link a folder" on an environment) and Vault-R can check whether any of
        these secrets are already committed.
      </div>
    );
  }

  const total = leakReports.reduce((n, r) => n + r.findings.length, 0);

  return (
    <>
      <div style={summaryRowStyle}>
        <span style={total > 0 ? summaryBadStyle : summaryGoodStyle}>
          {total > 0
            ? `${total} finding${total === 1 ? '' : 's'} across ${leakReports.length} folder${leakReports.length === 1 ? '' : 's'}`
            : leakScanned
              ? 'No secrets are visible to git.'
              : ''}
        </span>
        <button style={smallBtnStyle} onClick={() => void runLeakScan()}>
          Rescan
        </button>
      </div>
      {leakReports.map((report) => (
        <LeakReportBlock key={report.path} report={report} />
      ))}
    </>
  );
}

function LeakReportBlock({ report }: { report: LeakReport }) {
  const applyGitignoreFix = useVaultStore((s) => s.applyGitignoreFix);
  const fixable = report.findings.some((f) => f.fixPattern);

  return (
    <div style={blockStyle}>
      <div style={blockPathStyle} title={report.path}>
        {report.path}
      </div>
      {report.note && <div style={noteStyle}>{report.note}</div>}
      {report.findings.length === 0 && !report.note && (
        <div style={cleanStyle}>Clean — {report.filesScanned} tracked file(s) searched.</div>
      )}
      {report.findings.map((f, i) => (
        <div key={`${f.path}-${f.line ?? 0}-${i}`} style={findingStyle}>
          <div style={findingHeadStyle}>
            <span style={f.severity === 'critical' ? sevCriticalStyle : sevWarnStyle}>
              {f.severity === 'critical' ? 'EXPOSED' : 'AT RISK'}
            </span>
            <span style={findingPathStyle}>
              {f.path}
              {f.line !== null && `:${f.line}`}
            </span>
          </div>
          {f.key && (
            <div style={findingKeyStyle}>
              {f.key}
              {f.repoName && f.envName && (
                <span style={findingOriginStyle}>
                  {' '}
                  from {f.repoName}/{f.envName}
                </span>
              )}
            </div>
          )}
          <div style={findingDetailStyle}>{f.detail}</div>
        </div>
      ))}
      {fixable && report.gitRoot && (
        <button style={fixBtnStyle} onClick={() => void applyGitignoreFix(report)}>
          Add to .gitignore
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------
// Secret health
// ---------------------------------------------------------------------

function HealthTab() {
  const health = useVaultStore((s) => s.health);
  const healthLoading = useVaultStore((s) => s.healthLoading);
  const refreshHealth = useVaultStore((s) => s.refreshHealth);

  if (healthLoading || !health) return <div style={emptyStyle}>Checking your secrets…</div>;

  const clean = health.rows.length === 0 && health.duplicates.length === 0;

  return (
    <>
      <div style={summaryRowStyle}>
        <span style={clean ? summaryGoodStyle : summaryBadStyle}>
          {clean
            ? `All ${health.totalSecrets} secrets look healthy.`
            : `${health.rows.length} of ${health.totalSecrets} secrets need attention`}
        </span>
        <button style={smallBtnStyle} onClick={() => void refreshHealth()}>
          Recheck
        </button>
      </div>

      {!clean && (
        <div style={countRowStyle}>
          <Count label="empty" value={health.emptyCount} bad />
          <Count label="placeholder" value={health.placeholderCount} bad />
          <Count label="stale" value={health.staleCount} />
          <Count label="due to rotate" value={health.rotationDueCount} bad />
        </div>
      )}

      {health.rows.map((row) => (
        <HealthRow key={row.varId} row={row} />
      ))}

      {health.duplicates.length > 0 && (
        <>
          <div style={sectionHeadStyle}>Identical values that are not linked</div>
          <div style={sectionSubStyle}>
            Link them and one edit updates every copy — that is what link groups are for.
          </div>
          {health.duplicates.map((group, i) => (
            <DuplicateBlock key={`${group.key}-${i}`} group={group} />
          ))}
        </>
      )}
    </>
  );
}

function Count({ label, value, bad }: { label: string; value: number; bad?: boolean }) {
  if (value === 0) return null;
  return (
    <span style={countPillStyle}>
      <b style={bad ? countBadStyle : countWarnStyle}>{value}</b> {label}
    </span>
  );
}

function HealthRow({ row }: { row: SecretHealthRow }) {
  const jumpToVariable = useVaultStore((s) => s.jumpToVariable);
  return (
    <div style={blockStyle}>
      <div style={healthRowHeadStyle} onClick={() => void jumpToVariable(row.envId, row.varId)}>
        <span style={healthKeyStyle}>{row.key}</span>
        <span style={healthWhereStyle}>
          {row.repoName}/{row.envName}
        </span>
      </div>
      {row.issues.map((issue, i) => (
        <div key={i} style={findingDetailStyle}>
          <span style={issue.severity === 'critical' ? sevCriticalStyle : sevWarnStyle}>
            {issue.kind === 'rotationDue' ? 'ROTATE' : issue.kind.toUpperCase()}
          </span>{' '}
          {issue.detail}
        </div>
      ))}
    </div>
  );
}

function DuplicateBlock({ group }: { group: DuplicateValueGroup }) {
  const linkDuplicateGroup = useVaultStore((s) => s.linkDuplicateGroup);
  return (
    <div style={blockStyle}>
      <div style={healthKeyStyle}>{group.key || 'Same value, different keys'}</div>
      {group.locations.map((l) => (
        <div key={l.varId} style={healthWhereStyle}>
          {l.repoName}/{l.envName} · {l.key}
        </div>
      ))}
      <button style={fixBtnStyle} onClick={() => void linkDuplicateGroup(group)}>
        Link these {group.locations.length}
      </button>
    </div>
  );
}

const slideoverStyle: React.CSSProperties = {
  position: 'fixed',
  top: 0,
  right: 0,
  bottom: 0,
  width: '420px',
  background: 'var(--panel)',
  borderLeft: '1px solid var(--border)',
  zIndex: 41,
  padding: '22px',
  overflowY: 'auto',
  animation: 'vaultSlideIn 0.22s ease',
};
const headerStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', marginBottom: '12px' };
const titleStyle: React.CSSProperties = { fontSize: '16px', fontWeight: 700, color: 'var(--text)', flex: 1 };

const tabRowStyle: React.CSSProperties = { display: 'flex', gap: '6px', marginBottom: '14px' };
const tabBtnStyle: React.CSSProperties = {
  flex: 1,
  fontSize: '12px',
  fontWeight: 600,
  color: 'var(--text-dim)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  padding: '7px 10px',
  cursor: 'pointer',
};
const tabBtnActiveStyle: React.CSSProperties = {
  color: 'var(--accent)',
  background: 'var(--accent-dim)',
  borderColor: 'var(--accent)',
};

const summaryRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '10px',
  marginBottom: '12px',
};
const summaryGoodStyle: React.CSSProperties = { flex: 1, fontSize: '12.5px', color: 'var(--accent)' };
const summaryBadStyle: React.CSSProperties = { flex: 1, fontSize: '12.5px', color: 'var(--danger)', fontWeight: 600 };
const smallBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--text-dim)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 9px',
  cursor: 'pointer',
  flexShrink: 0,
};

const countRowStyle: React.CSSProperties = {
  display: 'flex',
  flexWrap: 'wrap',
  gap: '10px',
  marginBottom: '14px',
};
const countPillStyle: React.CSSProperties = { fontSize: '11.5px', color: 'var(--text-faint)' };
const countBadStyle: React.CSSProperties = { color: 'var(--danger)' };
const countWarnStyle: React.CSSProperties = { color: 'var(--env-staging)' };

const blockStyle: React.CSSProperties = {
  border: '1px solid var(--border)',
  borderRadius: '8px',
  padding: '10px 12px',
  marginBottom: '10px',
};
const blockPathStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '11.5px',
  color: 'var(--text-dim)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
  marginBottom: '6px',
};
const noteStyle: React.CSSProperties = { fontSize: '11.5px', color: 'var(--text-faint)', lineHeight: 1.5 };
const cleanStyle: React.CSSProperties = { fontSize: '11.5px', color: 'var(--text-faint)' };
const emptyStyle: React.CSSProperties = {
  fontSize: '12.5px',
  color: 'var(--text-faint)',
  lineHeight: 1.6,
  padding: '8px 0',
};

const findingStyle: React.CSSProperties = { padding: '8px 0', borderTop: '1px solid var(--border-light)' };
const findingHeadStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '8px' };
const sevCriticalStyle: React.CSSProperties = {
  fontSize: '9.5px',
  fontWeight: 700,
  letterSpacing: '0.5px',
  color: 'var(--danger)',
};
const sevWarnStyle: React.CSSProperties = {
  fontSize: '9.5px',
  fontWeight: 700,
  letterSpacing: '0.5px',
  color: 'var(--env-staging)',
};
const findingPathStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '11.5px',
  color: 'var(--text)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};
const findingKeyStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12px',
  color: 'var(--key)',
  marginTop: '3px',
};
const findingOriginStyle: React.CSSProperties = { color: 'var(--text-faint)', fontSize: '11px' };
const findingDetailStyle: React.CSSProperties = {
  fontSize: '11.5px',
  color: 'var(--text-faint)',
  lineHeight: 1.5,
  marginTop: '3px',
};
const fixBtnStyle: React.CSSProperties = {
  marginTop: '8px',
  fontSize: '11.5px',
  fontWeight: 600,
  color: 'var(--accent)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '5px 10px',
  cursor: 'pointer',
};

const healthRowHeadStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'baseline',
  gap: '8px',
  cursor: 'pointer',
};
const healthKeyStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  color: 'var(--key)',
};
const healthWhereStyle: React.CSSProperties = { fontSize: '11px', color: 'var(--text-faint)' };
const sectionHeadStyle: React.CSSProperties = {
  fontSize: '10.5px',
  fontWeight: 700,
  color: 'var(--text-faint)',
  letterSpacing: '0.5px',
  marginTop: '18px',
};
const sectionSubStyle: React.CSSProperties = {
  fontSize: '11.5px',
  color: 'var(--text-faint)',
  lineHeight: 1.5,
  margin: '4px 0 10px',
};
