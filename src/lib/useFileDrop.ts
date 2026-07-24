import { useCallback, useEffect, useRef, useState, type DragEvent } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { api } from './api';

/// Tauri reports drop coordinates in physical device pixels relative to the
/// webview; `getBoundingClientRect` is in CSS pixels. Without the divide, the
/// hit test misses the dropzone entirely on any HiDPI display.
function isOverElement(el: HTMLElement | null, position: { x: number; y: number }): boolean {
  if (!el) return false;
  const rect = el.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) return false;
  const scale = window.devicePixelRatio || 1;
  const x = position.x / scale;
  const y = position.y / scale;
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
}

/// Wires a dropzone to file drops.
///
/// The webview never sees an HTML5 `drop`: Tauri claims OS drag-and-drop at the
/// window level (`dragDropEnabled` defaults on) and re-emits it as its own
/// event carrying file *paths*, so the DOM handlers alone are dead code inside
/// the app. This listens to the Tauri event, hit-tests the drop against the
/// zone's rect, and reads the file through the backend. The returned DOM
/// handlers are the fallback for running the frontend in a plain browser.
///
/// `enabled` gates the subscription so a closed modal isn't hit-testing drops
/// meant for whatever is on screen instead.
export function useFileDrop(
  enabled: boolean,
  onText: (text: string) => void,
  onError?: (message: string) => void,
) {
  const zoneRef = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState(false);

  /// Kept in refs so a new callback identity each render doesn't tear down and
  /// re-establish the webview listener mid-drag.
  const onTextRef = useRef(onText);
  const onErrorRef = useRef(onError);
  onTextRef.current = onText;
  onErrorRef.current = onError;

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
        if (payload.type === 'leave') {
          setDragging(false);
          return;
        }
        const over = isOverElement(zoneRef.current, payload.position);
        if (payload.type !== 'drop') {
          setDragging(over);
          return;
        }
        setDragging(false);
        if (!over) return;
        const path = payload.paths[0];
        if (!path) return;
        api
          .readDroppedFile(path)
          .then((text) => onTextRef.current(text))
          .catch((e) => onErrorRef.current?.(String(e)));
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
  }, [enabled]);

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

  return { zoneRef, dragging, dropHandlers: { onDragOver, onDragLeave, onDrop } };
}
