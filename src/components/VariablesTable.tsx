import { useEffect, useState, type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import type { VariableWithUsage } from '../lib/api';

export function VariablesTable() {
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const variables = useVaultStore((s) => s.variables);
  const varSearch = useVaultStore((s) => s.varSearch);
  const setVarSearch = useVaultStore((s) => s.setVarSearch);
  const newVarKey = useVaultStore((s) => s.newVarKey);
  const newVarValue = useVaultStore((s) => s.newVarValue);
  const setNewVarKey = useVaultStore((s) => s.setNewVarKey);
  const setNewVarValue = useVaultStore((s) => s.setNewVarValue);
  const addVariable = useVaultStore((s) => s.addVariable);
  const openGenerator = useVaultStore((s) => s.openGenerator);
  const selectedVarIds = useVaultStore((s) => s.selectedVarIds);

  const filtered = variables.filter(
    (v) => !varSearch.trim() || v.key.toLowerCase().includes(varSearch.trim().toLowerCase()),
  );
  const selectedCount = Object.values(selectedVarIds).filter(Boolean).length;

  function onAddKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') void addVariable();
  }

  return (
    <>
      <div style={tableSearchRowStyle}>
        <input
          style={tableSearchInputStyle}
          placeholder="Filter variables in this environment&hellip;"
          value={varSearch}
          onChange={(e) => setVarSearch(e.target.value)}
        />
      </div>

      {selectedCount > 0 && <SelectionActionBar count={selectedCount} />}

      <div style={tableStyle}>
        <div style={tableHeaderRowStyle}>
          <div style={colCheckStyle} />
          <div style={colKeyStyle}>KEY</div>
          <div style={colValueStyle}>VALUE</div>
          <div style={colLinkedStyle}>LINKED</div>
          <div style={colActionsStyle}>&nbsp;</div>
        </div>

        {filtered.map((v) => (
          <VariableRow key={v.id} variable={v} />
        ))}

        {activeEnvId && (
          <div style={addRowStyle}>
            <div style={colCheckStyle} />
            <div style={colKeyStyle}>
              <input
                style={addKeyInputStyle}
                placeholder="NEW_KEY"
                value={newVarKey}
                onChange={(e) => setNewVarKey(e.target.value)}
                onKeyDown={onAddKeyDown}
              />
            </div>
            <div style={colValueStyle}>
              <input
                style={addValueInputStyle}
                placeholder="value"
                value={newVarValue}
                onChange={(e) => setNewVarValue(e.target.value)}
                onKeyDown={onAddKeyDown}
              />
            </div>
            <div style={colLinkedStyle} />
            <div style={colActionsStyle}>
              <button
                style={rowIconBtnStyle}
                title="Generate a value"
                onClick={() => openGenerator({ type: 'add' })}
              >
                &#9881;
              </button>
              <button style={addVarBtnStyle} onClick={() => void addVariable()}>
                add
              </button>
            </div>
          </div>
        )}

        {activeEnvId && filtered.length === 0 && variables.length === 0 && (
          <div style={emptyRowStyle}>No variables yet — add one above or use Import.</div>
        )}
      </div>
    </>
  );
}

function SelectionActionBar({ count }: { count: number }) {
  const repos = useVaultStore((s) => s.repos);
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const bulkMoveTargetId = useVaultStore((s) => s.bulkMoveTargetId);
  const setBulkMoveTarget = useVaultStore((s) => s.setBulkMoveTarget);
  const bulkMoveSelected = useVaultStore((s) => s.bulkMoveSelected);
  const bulkDeleteSelected = useVaultStore((s) => s.bulkDeleteSelected);
  const bulkCopySelectedAsEnvBlock = useVaultStore((s) => s.bulkCopySelectedAsEnvBlock);
  const clearVarSelection = useVaultStore((s) => s.clearVarSelection);

  const envOptions: { id: string; label: string }[] = [];
  repos.forEach((r) =>
    r.envs.forEach((e) => {
      if (e.id !== activeEnvId) envOptions.push({ id: e.id, label: `${r.name}/${e.name}` });
    }),
  );

  return (
    <div style={selectionBarStyle}>
      <span style={selectionCountStyle}>
        {count} selected
      </span>
      <button style={selectionBtnStyle} onClick={() => void bulkCopySelectedAsEnvBlock()}>
        Copy as .env
      </button>
      <select
        style={selectionSelectStyle}
        value={bulkMoveTargetId ?? ''}
        onChange={(e) => setBulkMoveTarget(e.target.value)}
      >
        <option value="" disabled>
          Move to&hellip;
        </option>
        {envOptions.map((o) => (
          <option key={o.id} value={o.id}>
            {o.label}
          </option>
        ))}
      </select>
      <button
        style={selectionBtnStyle}
        disabled={!bulkMoveTargetId}
        onClick={() => void bulkMoveSelected()}
      >
        Move
      </button>
      <button style={selectionBtnDangerStyle} onClick={() => void bulkDeleteSelected()}>
        Delete
      </button>
      <button style={selectionClearBtnStyle} onClick={clearVarSelection}>
        Clear
      </button>
    </div>
  );
}

function VariableRow({ variable }: { variable: VariableWithUsage }) {
  const revealed = useVaultStore((s) => !!s.revealed[variable.id]);
  const expanded = useVaultStore((s) => s.expandedVarId === variable.id);
  const toggleReveal = useVaultStore((s) => s.toggleReveal);
  const toggleVarExpand = useVaultStore((s) => s.toggleVarExpand);
  const commitVariableValue = useVaultStore((s) => s.commitVariableValue);
  const commitVariableKey = useVaultStore((s) => s.commitVariableKey);
  const deleteVariable = useVaultStore((s) => s.deleteVariable);
  const copyVariable = useVaultStore((s) => s.copyVariable);
  const openLinkModal = useVaultStore((s) => s.openLinkModal);
  const openGroupPopover = useVaultStore((s) => s.openGroupPopover);
  const openGenerator = useVaultStore((s) => s.openGenerator);
  const selected = useVaultStore((s) => !!s.selectedVarIds[variable.id]);
  const toggleVarSelected = useVaultStore((s) => s.toggleVarSelected);

  const [draft, setDraft] = useState(variable.value);
  useEffect(() => setDraft(variable.value), [variable.value, variable.id]);

  const [editingKey, setEditingKey] = useState(false);
  const [keyDraft, setKeyDraft] = useState(variable.key);
  useEffect(() => setKeyDraft(variable.key), [variable.key, variable.id]);

  const missingRequired = variable.required && !variable.value.trim();

  function commit() {
    if (draft !== variable.value) void commitVariableValue(variable.id, draft);
  }

  function commitKey() {
    setEditingKey(false);
    if (keyDraft.trim() !== variable.key) void commitVariableKey(variable.id, keyDraft);
  }

  return (
    <>
      <div style={tableRowStyle}>
        <div style={colCheckStyle}>
          <input
            type="checkbox"
            checked={selected}
            onChange={() => toggleVarSelected(variable.id)}
          />
        </div>
        <div style={colKeyStyle}>
          {editingKey ? (
            <input
              style={keyInputStyle}
              value={keyDraft}
              onChange={(e) => setKeyDraft(e.target.value)}
              onBlur={commitKey}
              onKeyDown={(e) => {
                if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
                if (e.key === 'Escape') {
                  setKeyDraft(variable.key);
                  setEditingKey(false);
                }
              }}
              autoFocus
              spellCheck={false}
              autoComplete="off"
            />
          ) : (
            <span style={keyRowStyle}>
              <span style={keyTextStyle} title="Click to rename" onClick={() => setEditingKey(true)}>
                {variable.key}
              </span>
              {missingRequired && (
                <span style={requiredBadgeStyle} title="Required but empty">
                  &#9888;
                </span>
              )}
            </span>
          )}
        </div>
        <div style={colValueStyle}>
          <input
            style={valueInputStyle}
            type={revealed ? 'text' : 'password'}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
            }}
            spellCheck={false}
          />
        </div>
        <div style={colLinkedStyle}>
          {variable.groupId && (
            <span style={linkedPillStyle} onClick={() => openGroupPopover(variable.groupId!)}>
              &#9101; linked &times;{variable.groupUsage}
            </span>
          )}
        </div>
        <div style={colActionsStyle}>
          <button style={rowIconBtnStyle} onClick={() => toggleReveal(variable.id)}>
            {revealed ? 'hide' : 'show'}
          </button>
          <button style={rowIconBtnStyle} onClick={() => void copyVariable(variable.value, variable.key)}>
            copy
          </button>
          <button
            style={rowIconBtnStyle}
            title="Generate a new value"
            onClick={() => openGenerator({ type: 'row', varId: variable.id })}
          >
            &#9881;
          </button>
          {!variable.groupId && (
            <button style={rowIconBtnStyle} onClick={() => void openLinkModal(variable.id, variable.key)}>
              link
            </button>
          )}
          <button
            style={rowIconBtnStyle}
            title="Description and required flag"
            onClick={() => toggleVarExpand(variable.id)}
          >
            {expanded ? '▲' : '…'}
          </button>
          <button style={rowIconBtnDangerStyle} onClick={() => void deleteVariable(variable.id, variable.key)}>
            del
          </button>
        </div>
      </div>
      {expanded && <VariableDetail variable={variable} />}
    </>
  );
}

function VariableDetail({ variable }: { variable: VariableWithUsage }) {
  const commitVariableMetadata = useVaultStore((s) => s.commitVariableMetadata);
  const [description, setDescription] = useState(variable.description ?? '');
  const [required, setRequired] = useState(variable.required);
  useEffect(() => {
    setDescription(variable.description ?? '');
    setRequired(variable.required);
  }, [variable.id, variable.description, variable.required]);

  function commit(nextDescription: string, nextRequired: boolean) {
    void commitVariableMetadata(variable.id, nextDescription, nextRequired);
  }

  return (
    <div style={detailRowStyle}>
      <textarea
        style={detailTextareaStyle}
        placeholder="What is this for? (e.g. get this from the Stripe dashboard)"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        onBlur={() => commit(description, required)}
        rows={2}
      />
      <label style={detailRequiredLabelStyle}>
        <input
          type="checkbox"
          checked={required}
          onChange={(e) => {
            setRequired(e.target.checked);
            commit(description, e.target.checked);
          }}
        />
        Required (gates <code>vault check</code>)
      </label>
    </div>
  );
}

const tableSearchRowStyle: React.CSSProperties = { marginBottom: '10px' };
const tableSearchInputStyle: React.CSSProperties = {
  width: '100%',
  maxWidth: '360px',
  fontSize: '13px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '8px 12px',
  outline: 'none',
  boxSizing: 'border-box',
};

const tableStyle: React.CSSProperties = { border: '1px solid var(--border)', borderRadius: '8px', overflow: 'hidden' };
const tableHeaderRowStyle: React.CSSProperties = {
  display: 'flex',
  background: 'var(--panel-2)',
  padding: '9px 14px',
  borderBottom: '1px solid var(--border)',
};
const tableRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  padding: '10px 14px',
  borderBottom: '1px solid var(--border-light)',
};
const addRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  padding: '10px 14px',
  borderTop: '1px dashed var(--border)',
  background: 'rgba(255,255,255,0.015)',
};
const emptyRowStyle: React.CSSProperties = {
  padding: '18px 14px',
  fontSize: '12.5px',
  color: 'var(--text-faint)',
  textAlign: 'center',
};
const colCheckStyle: React.CSSProperties = {
  width: '24px',
  flexShrink: 0,
  display: 'flex',
  alignItems: 'center',
  boxSizing: 'border-box',
};
const colKeyStyle: React.CSSProperties = {
  width: '24%',
  fontSize: '10.5px',
  fontWeight: 700,
  color: 'var(--text-faint)',
  letterSpacing: '0.5px',
  paddingRight: '10px',
  boxSizing: 'border-box',
};
const colValueStyle: React.CSSProperties = {
  width: '42%',
  fontSize: '10.5px',
  fontWeight: 700,
  color: 'var(--text-faint)',
  letterSpacing: '0.5px',
  paddingRight: '10px',
  boxSizing: 'border-box',
};
const colLinkedStyle: React.CSSProperties = {
  width: '16%',
  fontSize: '10.5px',
  fontWeight: 700,
  color: 'var(--text-faint)',
  letterSpacing: '0.5px',
  boxSizing: 'border-box',
};
const colActionsStyle: React.CSSProperties = {
  width: '16%',
  display: 'flex',
  gap: '6px',
  justifyContent: 'flex-end',
  boxSizing: 'border-box',
};
const keyRowStyle: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: '6px' };
const keyTextStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  color: 'var(--key)',
  fontWeight: 600,
  cursor: 'text',
};
const requiredBadgeStyle: React.CSSProperties = {
  color: 'var(--danger)',
  fontSize: '12px',
  cursor: 'default',
};
const detailRowStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  padding: '10px 14px 14px',
  borderBottom: '1px solid var(--border-light)',
  background: 'rgba(255,255,255,0.015)',
};
const detailTextareaStyle: React.CSSProperties = {
  width: '100%',
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '8px 10px',
  outline: 'none',
  boxSizing: 'border-box',
  resize: 'vertical',
};
const detailRequiredLabelStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  fontSize: '12px',
  color: 'var(--text-dim)',
};
const keyInputStyle: React.CSSProperties = {
  width: '100%',
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  color: 'var(--key)',
  fontWeight: 600,
  background: 'var(--panel-2)',
  border: '1px solid var(--border)',
  borderRadius: '4px',
  outline: 'none',
  padding: '2px 5px',
  boxSizing: 'border-box',
};
const valueInputStyle: React.CSSProperties = {
  width: '100%',
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  color: 'var(--text)',
  background: 'transparent',
  border: 'none',
  outline: 'none',
  padding: '2px 0',
  boxSizing: 'border-box',
};
const linkedPillStyle: React.CSSProperties = {
  fontSize: '10.5px',
  color: 'var(--accent)',
  background: 'var(--accent-dim)',
  borderRadius: '5px',
  padding: '2px 7px',
  whiteSpace: 'nowrap',
  cursor: 'pointer',
};
const rowIconBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 8px',
  cursor: 'pointer',
};
const rowIconBtnDangerStyle: React.CSSProperties = {
  fontSize: '11px',
  color: 'var(--danger)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 8px',
  cursor: 'pointer',
};
const addKeyInputStyle: React.CSSProperties = {
  width: '100%',
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  color: 'var(--key)',
  background: 'transparent',
  border: 'none',
  outline: 'none',
  boxSizing: 'border-box',
};
const addValueInputStyle: React.CSSProperties = {
  width: '100%',
  fontFamily: 'var(--font-mono)',
  fontSize: '12.5px',
  color: 'var(--text)',
  background: 'transparent',
  border: 'none',
  outline: 'none',
  boxSizing: 'border-box',
};
const addVarBtnStyle: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 700,
  color: 'var(--accent)',
  background: 'transparent',
  border: '1px solid var(--border)',
  borderRadius: '5px',
  padding: '4px 10px',
  cursor: 'pointer',
};
const selectionBarStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '8px 12px',
  marginBottom: '8px',
  background: 'var(--accent-dim)',
  border: '1px solid var(--accent)',
  borderRadius: '8px',
};
const selectionCountStyle: React.CSSProperties = {
  fontSize: '12.5px',
  fontWeight: 700,
  color: 'var(--text)',
  marginRight: '4px',
};
const selectionBtnStyle: React.CSSProperties = {
  fontSize: '11.5px',
  fontWeight: 600,
  color: 'var(--text-dim)',
  background: 'var(--panel)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  padding: '5px 10px',
  cursor: 'pointer',
};
const selectionBtnDangerStyle: React.CSSProperties = {
  ...selectionBtnStyle,
  color: 'var(--danger)',
  marginLeft: 'auto',
};
const selectionClearBtnStyle: React.CSSProperties = {
  fontSize: '13px',
  color: 'var(--text-faint)',
  background: 'transparent',
  border: 'none',
  cursor: 'pointer',
  padding: '5px',
};
const selectionSelectStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '11.5px',
  background: 'var(--panel)',
  border: '1px solid var(--border)',
  borderRadius: '6px',
  color: 'var(--text)',
  padding: '5px 8px',
  outline: 'none',
};
