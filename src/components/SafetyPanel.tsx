import { useVaultStore, type SafetyTab } from '../store/useVaultStore';
import { usePresence } from '../lib/usePresence';
import { Spinner } from './Spinner';
import { Skeleton } from './Skeleton';
import { CloseIcon } from './icons';
import type { DuplicateValueGroup, LeakReport, SecretHealthRow } from '../lib/api';
import styles from './SafetyPanel.module.css';

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

  const { mounted, state } = usePresence(safetyOpen, 160);
  if (!mounted) return null;

  return (
    <>
      <div className={`v-backdrop is-${state}`} onClick={closeSafety} />
      <aside className={`v-slideover ${styles.panel} is-${state}`}>
        <div className={styles.header}>
          <span className={styles.title}>Safety</span>
          <button className="v-close-x" onClick={closeSafety} aria-label="Close">
            <CloseIcon size={13} />
          </button>
        </div>
        <div className={styles.tabRow} role="tablist">
          <TabButton tab="leaks" label="Git leak guard" active={safetyTab === 'leaks'} onSelect={setSafetyTab} />
          <TabButton tab="health" label="Secret health" active={safetyTab === 'health'} onSelect={setSafetyTab} />
        </div>
        {safetyTab === 'leaks' ? <LeakTab /> : <HealthTab />}
      </aside>
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
      className={`v-btn ${styles.tabBtn}`}
      role="tab"
      aria-selected={active}
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

  if (leakScanning) {
    return (
      <div className={`${styles.empty} ${styles.scanning}`}>
        <Spinner size={12} />
        Scanning your linked folders…
      </div>
    );
  }

  if (projects.length === 0) {
    return (
      <div className={styles.empty}>
        No folders are linked yet. Run <code>vault link &lt;repo&gt;/&lt;env&gt;</code> in a project
        directory (or use "Link a folder" on an environment) and Vault-R can check whether any of
        these secrets are already committed.
      </div>
    );
  }

  const total = leakReports.reduce((n, r) => n + r.findings.length, 0);

  return (
    <>
      <div className={styles.summaryRow}>
        <span className={styles.summary} data-tone={total > 0 ? 'bad' : 'good'}>
          {total > 0
            ? `${total} finding${total === 1 ? '' : 's'} across ${leakReports.length} folder${leakReports.length === 1 ? '' : 's'}`
            : leakScanned
              ? 'No secrets are visible to git.'
              : ''}
        </span>
        <button className={`v-btn ${styles.smallBtn}`} onClick={() => void runLeakScan()}>
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
    <div className={styles.block}>
      <div className={styles.blockPath} title={report.path}>
        {report.path}
      </div>
      {report.note && <div className={styles.note}>{report.note}</div>}
      {report.findings.length === 0 && !report.note && (
        <div className={styles.clean}>Clean — {report.filesScanned} tracked file(s) searched.</div>
      )}
      {report.findings.map((f, i) => (
        <div key={`${f.path}-${f.line ?? 0}-${i}`} className={styles.finding}>
          <div className={styles.findingHead}>
            <span className={styles.sev} data-severity={f.severity === 'critical' ? 'critical' : 'warn'}>
              {f.severity === 'critical' ? 'EXPOSED' : 'AT RISK'}
            </span>
            <span className={styles.findingPath}>
              {f.path}
              {f.line !== null && `:${f.line}`}
            </span>
          </div>
          {f.key && (
            <div className={styles.findingKey}>
              {f.key}
              {f.repoName && f.envName && (
                <span className={styles.findingOrigin}>
                  {' '}
                  from {f.repoName}/{f.envName}
                </span>
              )}
            </div>
          )}
          <div className={styles.findingDetail}>{f.detail}</div>
        </div>
      ))}
      {fixable && report.gitRoot && (
        <button className={`v-btn ${styles.fixBtn}`} onClick={() => void applyGitignoreFix(report)}>
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

  if (healthLoading || !health) {
    return (
      <>
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className={styles.skelBlock}>
            <Skeleton height={12} width={`${58 - i * 7}%`} />
            <Skeleton height={10} width={`${40 + i * 5}%`} />
          </div>
        ))}
      </>
    );
  }

  const clean = health.rows.length === 0 && health.duplicates.length === 0;

  return (
    <>
      <div className={styles.summaryRow}>
        <span className={styles.summary} data-tone={clean ? 'good' : 'bad'}>
          {clean
            ? `All ${health.totalSecrets} secrets look healthy.`
            : `${health.rows.length} of ${health.totalSecrets} secrets need attention`}
        </span>
        <button className={`v-btn ${styles.smallBtn}`} onClick={() => void refreshHealth()}>
          Recheck
        </button>
      </div>

      {!clean && (
        <div className={styles.countRow}>
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
          <div className={styles.sectionHead}>Identical values that are not linked</div>
          <div className={styles.sectionSub}>
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
    <span className={styles.countPill}>
      <b className={styles.countValue} data-tone={bad ? 'bad' : 'warn'}>
        {value}
      </b>{' '}
      {label}
    </span>
  );
}

function HealthRow({ row }: { row: SecretHealthRow }) {
  const jumpToVariable = useVaultStore((s) => s.jumpToVariable);
  return (
    <div className={styles.block}>
      <button className={styles.healthRowHead} onClick={() => void jumpToVariable(row.envId, row.varId)}>
        <span className={styles.healthKey}>{row.key}</span>
        <span className={styles.healthWhere}>
          {row.repoName}/{row.envName}
        </span>
      </button>
      {row.issues.map((issue, i) => (
        <div key={i} className={styles.findingDetail}>
          <span className={styles.sev} data-severity={issue.severity === 'critical' ? 'critical' : 'warn'}>
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
    <div className={styles.block}>
      <div className={styles.healthKey}>{group.key || 'Same value, different keys'}</div>
      {group.locations.map((l) => (
        <div key={l.varId} className={styles.healthWhere}>
          {l.repoName}/{l.envName} · {l.key}
        </div>
      ))}
      <button className={`v-btn ${styles.fixBtn}`} onClick={() => void linkDuplicateGroup(group)}>
        Link these {group.locations.length}
      </button>
    </div>
  );
}
