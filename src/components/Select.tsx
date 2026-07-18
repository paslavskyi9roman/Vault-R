import { useEffect, useId, useRef, useState, type KeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { ChevronIcon } from './icons';
import styles from './Select.module.css';

export interface SelectOption {
  value: string;
  label: string;
}

/// A listbox that can actually be themed. A native <select> renders its popup
/// through the OS, so it stays light-on-white inside a dark app no matter what
/// CSS it is given.
///
/// The popup is portalled to the body and positioned from the trigger's box,
/// because both call sites live inside scroll containers that would otherwise
/// clip it.
export function Select({
  value,
  options,
  placeholder = 'Choose…',
  onChange,
  className = '',
  ariaLabel,
  disabled,
}: {
  value: string | null;
  options: SelectOption[];
  placeholder?: string;
  onChange: (value: string) => void;
  className?: string;
  ariaLabel?: string;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [box, setBox] = useState<{ top: number; left: number; width: number } | null>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const id = useId();

  const selectedIndex = options.findIndex((o) => o.value === value);
  const selected = selectedIndex >= 0 ? options[selectedIndex] : null;

  function openList() {
    const r = triggerRef.current?.getBoundingClientRect();
    if (r) {
      // Estimated popup height; flip above the trigger when there is no room.
      const estimated = Math.min(options.length * 34 + 8, 240);
      const below = window.innerHeight - r.bottom - 8;
      const top = below < estimated && r.top > below ? r.top - estimated - 4 : r.bottom + 4;
      setBox({ top, left: r.left, width: r.width });
    }
    setActive(selectedIndex >= 0 ? selectedIndex : 0);
    setOpen(true);
  }

  function close(refocus = true) {
    setOpen(false);
    if (refocus) triggerRef.current?.focus();
  }

  function choose(index: number) {
    const option = options[index];
    if (!option) return;
    onChange(option.value);
    close();
  }

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node;
      if (triggerRef.current?.contains(target) || listRef.current?.contains(target)) return;
      setOpen(false);
    };
    // The popup is placed from a snapshot of the trigger's box, so anything
    // that moves the trigger invalidates it.
    const dismiss = () => setOpen(false);
    document.addEventListener('pointerdown', onPointerDown, true);
    window.addEventListener('scroll', dismiss, true);
    window.addEventListener('resize', dismiss);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true);
      window.removeEventListener('scroll', dismiss, true);
      window.removeEventListener('resize', dismiss);
    };
  }, [open]);

  // Keep the highlighted row in view when arrowing past the visible window.
  useEffect(() => {
    if (!open) return;
    listRef.current?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [open, active]);

  function onKeyDown(e: KeyboardEvent<HTMLButtonElement>) {
    if (!open) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        openList();
      }
      return;
    }
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setActive((i) => Math.min(i + 1, options.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setActive((i) => Math.max(i - 1, 0));
        break;
      case 'Home':
        e.preventDefault();
        setActive(0);
        break;
      case 'End':
        e.preventDefault();
        setActive(options.length - 1);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        choose(active);
        break;
      case 'Escape':
        e.preventDefault();
        // Without this the app-level handler would close the whole slideover
        // rather than just this popup.
        e.stopPropagation();
        close();
        break;
      case 'Tab':
        setOpen(false);
        break;
    }
  }

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={`${styles.trigger} ${className}`}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={`${id}-list`}
        /* Belongs on the focused element, not the listbox — focus stays here
           while the popup is open, so this is what announces the highlight. */
        aria-activedescendant={open ? `${id}-${active}` : undefined}
        aria-label={ariaLabel}
        disabled={disabled}
        onClick={() => (open ? close(false) : openList())}
        onKeyDown={onKeyDown}
      >
        <span className={styles.label} data-placeholder={!selected}>
          {selected ? selected.label : placeholder}
        </span>
        <ChevronIcon size={11} className={styles.chevron} />
      </button>

      {open &&
        box &&
        createPortal(
          <div
            ref={listRef}
            id={`${id}-list`}
            className={`${styles.list} v-enter`}
            role="listbox"
            aria-label={ariaLabel}
            style={{ position: 'fixed', top: box.top, left: box.left, minWidth: box.width }}
          >
            {options.map((option, i) => (
              <div
                key={option.value}
                id={`${id}-${i}`}
                data-index={i}
                role="option"
                aria-selected={option.value === value}
                data-active={i === active}
                className={styles.option}
                onPointerEnter={() => setActive(i)}
                onClick={() => choose(i)}
              >
                {option.label}
              </div>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
}
