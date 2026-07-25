import { useCallback, useEffect, useRef, useState, type DragEvent } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { api } from './api';

function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/// Wires a dropzone to file drops.
///
/// The webview never sees an HTML5 `drop`: Tauri claims OS drag-and-drop at
/// the window level (`dragDropEnabled` defaults on) and re-emits it as its own
/// event carrying file *paths*, so the DOM handlers alone are dead code inside
/// the app. This listens to the Tauri event and reads the file through the
/// backend. The returned DOM handlers are the fallback for running the
/// frontend in a plain browser.
///
/// Deliberately ignores the position the event reports, because that number
/// cannot be trusted to hit-test a rect:
///   - macOS gets it wrong by a constant roughly the height of the titlebar.
///     wry subtracts the *webview's* height from a `draggingLocation()` given
///     in *window* space, so the two coordinate systems differ by whatever
///     chrome sits above the content view (tauri-apps/tauri#10744, open).
///   - Every platform reports garbage while devtools are open, which is the
///     state the app is in for most of its development.
///   - Windows reports raw client-area pixels with no DPI scaling, so the
///     conversion to CSS pixels drifts under per-monitor or fractional scaling.
///
/// Nothing is lost by ignoring it. Both dropzones live inside a modal that
/// covers the whole window, so while one is open there is no other target a
/// drop could have been meant for — anywhere in the window is unambiguous.
/// `enabled` is what scopes the listener, and it is the caller's job to keep
/// that true only while its zone is the active one.
export function useFileDrop(
  enabled: boolean,
  onText: (text: string) => void,
  onError?: (message: string) => void,
) {
  const [dragging, setDragging] = useState(false);

  /// Kept in refs so a new callback identity each render doesn't tear down and
  /// re-establish the webview listener mid-drag.
  const onTextRef = useRef(onText);
  const onErrorRef = useRef(onError);
  onTextRef.current = onText;
  onErrorRef.current = onError;

  const readPaths = useCallback((paths: string[]) => {
    const path = paths[0];
    if (!path) return;
    api
      .readDroppedFile(path)
      .then((text) => {
        onTextRef.current(text);
        if (paths.length > 1) {
          onErrorRef.current?.(`Dropped ${paths.length} files — read ${baseName(path)}.`);
        }
      })
      .catch((e) => onErrorRef.current?.(String(e)));
  }, []);

  useEffect(() => {
    if (!enabled) {
      setDragging(false);
      return;
    }
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;
    let disposed = false;

    void getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === 'drop') {
          setDragging(false);
          readPaths(payload.paths);
          return;
        }
        setDragging(payload.type !== 'leave');
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /// No listener means the dropzone stays inert; pasting still works.
      });

    return () => {
      disposed = true;
      unlisten?.();
      setDragging(false);
    };
  }, [enabled, readPaths]);

  const onDragOver = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setDragging(true);
  }, []);

  /// `dragleave` also fires when the cursor crosses onto a child node, which
  /// would flicker the highlight off mid-hover. Only a departure from the zone
  /// itself counts.
  const onDragLeave = useCallback((e: DragEvent<HTMLDivElement>) => {
    if (e.currentTarget.contains(e.relatedTarget as Node | null)) return;
    setDragging(false);
  }, []);

  const onDrop = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => onTextRef.current(String(reader.result ?? ''));
    reader.onerror = () => onErrorRef.current?.("Couldn't read that file.");
    reader.readAsText(file);
  }, []);

  return { dragging, dropHandlers: { onDragOver, onDragLeave, onDrop } };
}
