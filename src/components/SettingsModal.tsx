import { useState } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import {
  overlayBackdropStyle,
  modalStyle,
  modalHeaderStyle,
  modalTitleStyle,
  modalSubStyle,
  closeXStyle,
  primaryBtnStyle,
  secondaryBtnStyle,
} from './overlayStyles';
import { timeAgo } from '../lib/envColor';

const AUTO_LOCK_CHOICES = [
  { minutes: 0, label: 'Off' },
  { minutes: 5, label: '5 min' },
  { minutes: 15, label: '15 min' },
  { minutes: 30, label: '30 min' },
  { minutes: 60, label: '1 hour' },
];

export function SettingsModal() {
  const settingsOpen = useVaultStore((s) => s.settingsOpen);
  const closeSettings = useVaultStore((s) => s.closeSettings);
  const needsMigration = useVaultStore((s) => s.needsMigration);
  const recoveryCodeOnce = useVaultStore((s) => s.recoveryCodeOnce);

  if (!settingsOpen) return null;

  return (
    <>
      <div style={overlayBackdropStyle} onClick={closeSettings} />
      <div style={{ ...modalStyle, width: '480px' }}>
        <div style={modalHeaderStyle}>
          <span style={modalTitleStyle}>Vault settings</span>
          <button style={closeXStyle} onClick={closeSettings}>
            &times;
          </button>
        </div>
        <div style={modalSubStyle}>Security, backups and recovery for this device.</div>

        {needsMigration && <MigrationNotice />}
        {recoveryCodeOnce ? <RecoveryCodeReveal code={recoveryCodeOnce} /> : <RecoverySection />}
        <AutoLockSection />
        <BackupSection />
        <PasswordSection />
      </div>
    </>
  );
}

function MigrationNotice() {
  return (
    <div style={noticeStyle}>
      This vault still uses the original storage format. Lock it and unlock once with your master
      password to upgrade it — backups, password changes and recovery kits become available after
      that.
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={sectionStyle}>
      <div style={sectionTitleStyle}>{title}</div>
      {children}
    </div>
  );
}

function AutoLockSection() {
  const autoLockMinutes = useVaultStore((s) => s.autoLockMinutes);
  const setAutoLockMinutes = useVaultStore((s) => s.setAutoLockMinutes);

  return (
    <Section title="Auto-lock">
      <div style={sectionBodyStyle}>
        Lock the vault automatically after a period with no keyboard or mouse activity.
      </div>
      <div style={choiceRowStyle}>
        {AUTO_LOCK_CHOICES.map((choice) => {
          const active = autoLockMinutes === choice.minutes;
          return (
            <button
              key={choice.minutes}
              style={{
                ...choiceBtnStyle,
                color: active ? 'var(--accent)' : 'var(--text-dim)',
                borderColor: active ? 'var(--accent)' : 'var(--border)',
                background: active ? 'var(--accent-dim)' : 'transparent',
              }}
              onClick={() => void setAutoLockMinutes(choice.minutes)}
            >
              {choice.label}
            </button>
          );
        })}
      </div>
    </Section>
  );
}

function BackupSection() {
  const backups = useVaultStore((s) => s.backups);
  const exportBackup = useVaultStore((s) => s.exportBackup);
  const needsMigration = useVaultStore((s) => s.needsMigration);

  return (
    <Section title="Backups">
      <div style={sectionBodyStyle}>
        A backup is a copy of your vault file and stays encrypted — it can only be opened with the
        master password it had when the copy was taken. Vault-R also keeps the last 10 copies
        automatically, on every unlock and before anything destructive.
      </div>
      <div style={sectionBodyStyle}>
        {backups.length > 0 ? (
          <span style={mutedMonoStyle}>
            Last automatic backup {timeAgo(backups[0].createdAt)} · {backups.length} kept
          </span>
        ) : (
          <span style={mutedMonoStyle}>No automatic backups yet.</span>
        )}
      </div>
      <button style={secondaryBtnStyle} onClick={() => void exportBackup()} disabled={needsMigration}>
        Export encrypted backup…
      </button>
    </Section>
  );
}

function RecoverySection() {
  const hasRecoveryCode = useVaultStore((s) => s.hasRecoveryCode);
  const generateRecoveryCode = useVaultStore((s) => s.generateRecoveryCode);
  const needsMigration = useVaultStore((s) => s.needsMigration);

  return (
    <Section title="Recovery kit">
      <div style={sectionBodyStyle}>
        {hasRecoveryCode
          ? 'This vault has a recovery kit. The code was shown once when you created it — if you no longer have it, generate a new one.'
          : 'Without a recovery kit, forgetting your master password means losing every secret in this vault. A recovery code unlocks it if that happens.'}
      </div>
      <button
        style={secondaryBtnStyle}
        onClick={() => void generateRecoveryCode()}
        disabled={needsMigration}
      >
        {hasRecoveryCode ? 'Generate a new recovery kit…' : 'Create recovery kit'}
      </button>
    </Section>
  );
}

/// The code exists in recoverable form for exactly as long as this panel is on
/// screen, so the copy has to be blunt about saving it now.
function RecoveryCodeReveal({ code }: { code: string }) {
  const dismissRecoveryCode = useVaultStore((s) => s.dismissRecoveryCode);
  const saveRecoveryCodeToFile = useVaultStore((s) => s.saveRecoveryCodeToFile);
  const copyPlainText = useVaultStore((s) => s.copyPlainText);

  return (
    <Section title="Your recovery code">
      <div style={sectionBodyStyle}>
        Write this down or save it somewhere safe and offline.{' '}
        <strong style={{ color: 'var(--text)' }}>It will not be shown again.</strong> Anyone holding
        it can read every secret in this vault.
      </div>
      <div style={codeBoxStyle}>{code}</div>
      <div style={buttonRowStyle}>
        <button style={secondaryBtnStyle} onClick={() => void copyPlainText(code, 'Recovery code')}>
          Copy
        </button>
        <button style={secondaryBtnStyle} onClick={() => void saveRecoveryCodeToFile()}>
          Save as file…
        </button>
        <button style={{ ...primaryBtnStyle, width: 'auto' }} onClick={dismissRecoveryCode}>
          I&rsquo;ve saved it
        </button>
      </div>
    </Section>
  );
}

function PasswordSection() {
  const pwCurrent = useVaultStore((s) => s.pwCurrent);
  const pwNew = useVaultStore((s) => s.pwNew);
  const pwConfirm = useVaultStore((s) => s.pwConfirm);
  const pwBusy = useVaultStore((s) => s.pwBusy);
  const pwError = useVaultStore((s) => s.pwError);
  const setPwField = useVaultStore((s) => s.setPwField);
  const changePassword = useVaultStore((s) => s.changePassword);
  const needsMigration = useVaultStore((s) => s.needsMigration);
  const [remember, setRemember] = useState(true);

  return (
    <Section title="Master password">
      <div style={sectionBodyStyle}>
        Changing this re-locks the vault file with a new password. Your secrets are not re-encrypted
        and any recovery kit stays valid.
      </div>
      <input
        style={inputStyle}
        type="password"
        placeholder="Current password"
        value={pwCurrent}
        onChange={(e) => setPwField('pwCurrent', e.target.value)}
        disabled={needsMigration}
      />
      <input
        style={inputStyle}
        type="password"
        placeholder="New password"
        value={pwNew}
        onChange={(e) => setPwField('pwNew', e.target.value)}
        disabled={needsMigration}
      />
      <input
        style={inputStyle}
        type="password"
        placeholder="Confirm new password"
        value={pwConfirm}
        onChange={(e) => setPwField('pwConfirm', e.target.value)}
        disabled={needsMigration}
      />
      <label style={checkboxRowStyle}>
        <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
        <span>Remember on this device</span>
      </label>
      {pwError && <div style={errorStyle}>{pwError}</div>}
      <button
        style={{ ...primaryBtnStyle, opacity: pwBusy || needsMigration ? 0.5 : 1 }}
        onClick={() => void changePassword(remember)}
        disabled={pwBusy || needsMigration}
      >
        {pwBusy ? 'Changing…' : 'Change master password'}
      </button>
    </Section>
  );
}

const sectionStyle: React.CSSProperties = {
  borderTop: '1px solid var(--border-light)',
  paddingTop: '14px',
  marginTop: '14px',
  display: 'flex',
  flexDirection: 'column',
  gap: '9px',
};
const sectionTitleStyle: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 700,
  letterSpacing: '0.7px',
  color: 'var(--text-faint)',
  textTransform: 'uppercase',
};
const sectionBodyStyle: React.CSSProperties = {
  fontSize: '12.5px',
  color: 'var(--text-dim)',
  lineHeight: 1.6,
};
const mutedMonoStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '11.5px',
  color: 'var(--text-faint)',
};
const noticeStyle: React.CSSProperties = {
  fontSize: '12.5px',
  color: 'var(--text-dim)',
  lineHeight: 1.6,
  background: 'var(--panel-2)',
  border: '1px solid var(--env-staging)',
  borderRadius: '6px',
  padding: '10px 12px',
};
const choiceRowStyle: React.CSSProperties = { display: 'flex', gap: '6px', flexWrap: 'wrap' };
const choiceBtnStyle: React.CSSProperties = {
  fontSize: '12px',
  fontWeight: 600,
  border: '1px solid',
  borderRadius: '6px',
  padding: '6px 12px',
  cursor: 'pointer',
};
const inputStyle: React.CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  fontSize: '13px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '9px 11px',
  outline: 'none',
};
const checkboxRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  fontSize: '12.5px',
  color: 'var(--text-dim)',
  cursor: 'pointer',
};
const errorStyle: React.CSSProperties = { fontSize: '12px', color: 'var(--danger)', lineHeight: 1.5 };
const codeBoxStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '15px',
  letterSpacing: '1px',
  color: 'var(--accent)',
  background: 'var(--cli-bg)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  padding: '14px 12px',
  textAlign: 'center',
  wordBreak: 'break-all',
};
const buttonRowStyle: React.CSSProperties = { display: 'flex', gap: '8px', flexWrap: 'wrap' };
