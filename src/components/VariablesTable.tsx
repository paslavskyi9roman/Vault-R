import { useEffect, useState, type KeyboardEvent } from 'react';
import { useVaultStore } from '../store/useVaultStore';
import { Skeleton } from './Skeleton';
import { GearIcon, WarningIcon, ChevronIcon, CloseIcon, LinkIcon } from './icons';
import { Select } from './Select';
import type { VariableWithUsage } from '../lib/api';
import styles from './VariablesTable.module.css';

export function VariablesTable() {
  const activeEnvId = useVaultStore((s) => s.activeEnvId);
  const variables = useVaultStore((s) => s.variables);
  const varsLoading = useVaultStore((s) => s.varsLoading);
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
      <div className={styles.searchRow}>
        <input
          className={`v-input ${styles.searchInput}`}
          placeholder="Filter variables in this environment&hellip;"
          value={varSearch}
          onChange={(e) => setVarSearch(e.target.value)}
        />
      </div>

      {selectedCount > 0 && <SelectionActionBar count={selectedCount} />}

      <div className={styles.table}>
        <div className={styles.headerRow}>
          <div className={styles.colCheck} />
          <div className={styles.colKey}>KEY</div>
          <div className={styles.colValue}>VALUE</div>
          <div className={styles.colLinked}>LINKED</div>
          <div className={styles.colActions}>&nbsp;</div>
        </div>

        {varsLoading ? (
          <LoadingRows />
        ) : (
          filtered.map((v) => <VariableRow key={v.id} variable={v} />)
        )}

        {activeEnvId && !varsLoading && (
          <div className={styles.addRow}>
            <div className={styles.colCheck} />
            <div className={styles.colKey}>
              <input
                className={styles.addKeyInput}
                placeholder="NEW_KEY"
                value={newVarKey}
                onChange={(e) => setNewVarKey(e.target.value)}
                onKeyDown={onAddKeyDown}
              />
            </div>
            <div className={styles.colValue}>
              <input
                className={styles.addValueInput}
                placeholder="value"
                value={newVarValue}
                onChange={(e) => setNewVarValue(e.target.value)}
                onKeyDown={onAddKeyDown}
              />
            </div>
            <div className={styles.colLinked} />
            <div className={styles.colActions}>
              <button
                className={`v-btn ${styles.rowBtn} ${styles.rowIconBtn}`}
                title="Generate a value"
                aria-label="Generate a value"
                onClick={() => openGenerator({ type: 'add' })}
              >
                <GearIcon size={12} />
              </button>
              <button className={`v-btn ${styles.addBtn}`} onClick={() => void addVariable()}>
                add
              </button>
            </div>
          </div>
        )}

        {activeEnvId && !varsLoading && filtered.length === 0 && variables.length === 0 && (
          <div className={styles.emptyRow}>No variables yet — add one above or use Import.</div>
        )}

        {activeEnvId && !varsLoading && filtered.length === 0 && variables.length > 0 && (
          <div className={styles.emptyRow}>No variables match "{varSearch.trim()}".</div>
        )}
      </div>
    </>
  );
}

/// Mirrors the real row geometry so nothing shifts when the values land.
function LoadingRows() {
  return (
    <>
      {[0, 1, 2, 3].map((i) => (
        <div key={i} className={styles.skelRow}>
          <div className={styles.colCheck} />
          <div className={styles.colKey}>
            <Skeleton height={12} width={`${72 - i * 9}%`} />
          </div>
          <div className={styles.colValue}>
            <Skeleton height={12} width={`${58 + i * 7}%`} />
          </div>
          <div className={styles.colLinked} />
          <div className={styles.colActions} />
        </div>
      ))}
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
    <div className={`${styles.selectionBar} v-enter`}>
      <span className={styles.selectionCount}>{count} selected</span>
      <button
        className={`v-btn ${styles.selectionBtn}`}
        onClick={() => void bulkCopySelectedAsEnvBlock()}
      >
        Copy as .env
      </button>
      <Select
        className={styles.selectionSelect}
        value={bulkMoveTargetId}
        options={envOptions.map((o) => ({ value: o.id, label: o.label }))}
        placeholder="Move to…"
        ariaLabel="Move selected variables to"
        onChange={setBulkMoveTarget}
      />
      <button
        className={`v-btn ${styles.selectionBtn}`}
        disabled={!bulkMoveTargetId}
        onClick={() => void bulkMoveSelected()}
      >
        Move
      </button>
      <button
        className={`v-btn v-btn--danger ${styles.selectionBtn} ${styles.selectionBtnDanger}`}
        onClick={() => void bulkDeleteSelected()}
      >
        Delete
      </button>
      <button className={styles.selectionClear} onClick={clearVarSelection} aria-label="Clear selection">
        <CloseIcon size={13} />
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
      <div className={styles.row} data-selected={selected}>
        <div className={styles.colCheck}>
          <input
            className="v-check"
            type="checkbox"
            checked={selected}
            onChange={() => toggleVarSelected(variable.id)}
            aria-label={`Select ${variable.key}`}
          />
        </div>
        <div className={styles.colKey}>
          {editingKey ? (
            <input
              className={styles.keyInput}
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
            <span className={styles.keyRow}>
              <button className={styles.keyText} title="Click to rename" onClick={() => setEditingKey(true)}>
                {variable.key}
              </button>
              {missingRequired && (
                <span className={styles.requiredBadge} title="Required but empty" role="img" aria-label="Required but empty">
                  <WarningIcon size={12} />
                </span>
              )}
            </span>
          )}
        </div>
        <div className={styles.colValue}>
          <input
            className={styles.valueInput}
            data-revealed={revealed}
            type={revealed ? 'text' : 'password'}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
            }}
            spellCheck={false}
            aria-label={`Value of ${variable.key}`}
          />
        </div>
        <div className={styles.colLinked}>
          {variable.groupId && (
            <button className={styles.linkedPill} onClick={() => openGroupPopover(variable.groupId!)}>
              <LinkIcon size={11} />
              linked &times;{variable.groupUsage}
            </button>
          )}
        </div>
        <div className={`${styles.colActions} ${styles.rowActions}`}>
          <button className={`v-btn ${styles.rowBtn}`} onClick={() => toggleReveal(variable.id)}>
            {revealed ? 'hide' : 'show'}
          </button>
          <button
            className={`v-btn ${styles.rowBtn}`}
            onClick={() => void copyVariable(variable.value, variable.key)}
          >
            copy
          </button>
          <button
            className={`v-btn ${styles.rowBtn} ${styles.rowIconBtn}`}
            title="Generate a new value"
            aria-label={`Generate a new value for ${variable.key}`}
            onClick={() => openGenerator({ type: 'row', varId: variable.id })}
          >
            <GearIcon size={12} />
          </button>
          {!variable.groupId && (
            <button
              className={`v-btn ${styles.rowBtn}`}
              onClick={() => void openLinkModal(variable.id, variable.key)}
            >
              link
            </button>
          )}
          <button
            className={`v-btn ${styles.rowBtn} ${styles.rowIconBtn} ${styles.expandBtn}`}
            title="Description and required flag"
            aria-label={`Details for ${variable.key}`}
            aria-expanded={expanded}
            onClick={() => toggleVarExpand(variable.id)}
          >
            <ChevronIcon size={12} />
          </button>
          <button
            className={`v-btn v-btn--danger ${styles.rowBtn}`}
            onClick={() => void deleteVariable(variable.id, variable.key)}
          >
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
  const [rotateAfter, setRotateAfter] = useState(
    variable.rotateAfterDays === null ? '' : String(variable.rotateAfterDays),
  );
  useEffect(() => {
    setDescription(variable.description ?? '');
    setRequired(variable.required);
    setRotateAfter(variable.rotateAfterDays === null ? '' : String(variable.rotateAfterDays));
  }, [variable.id, variable.description, variable.required, variable.rotateAfterDays]);

  /// An unparseable or non-positive interval means "no policy" rather than an
  /// error toast on every keystroke — the field is cleared to say the same
  /// thing, so treating them alike is what the user expects.
  function parseRotation(raw: string): number | null {
    const parsed = Number(raw.trim());
    return raw.trim() && Number.isFinite(parsed) && parsed >= 1 ? Math.floor(parsed) : null;
  }

  function commit(nextDescription: string, nextRequired: boolean, nextRotation: string) {
    void commitVariableMetadata(
      variable.id,
      nextDescription,
      nextRequired,
      parseRotation(nextRotation),
    );
  }

  return (
    <div className={`${styles.detailRow} v-enter`}>
      <textarea
        className={`v-input ${styles.detailTextarea}`}
        placeholder="What is this for? (e.g. get this from the Stripe dashboard)"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        onBlur={() => commit(description, required, rotateAfter)}
        rows={2}
      />
      <label className={styles.detailLabel}>
        <input
          className="v-check"
          type="checkbox"
          checked={required}
          onChange={(e) => {
            setRequired(e.target.checked);
            commit(description, e.target.checked, rotateAfter);
          }}
        />
        Required (gates <code>vault check</code>)
      </label>
      <label className={styles.detailLabel}>
        Rotate every
        <input
          className={`v-input ${styles.detailRotationInput}`}
          type="number"
          min={1}
          placeholder="—"
          value={rotateAfter}
          onChange={(e) => setRotateAfter(e.target.value)}
          onBlur={() => commit(description, required, rotateAfter)}
        />
        days (blank = no reminder)
      </label>
    </div>
  );
}
