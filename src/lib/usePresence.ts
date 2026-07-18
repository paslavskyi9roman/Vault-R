import { useEffect, useRef, useState } from 'react';

export type PresenceState = 'entering' | 'entered' | 'exiting' | 'exited';

function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/// Keeps a component mounted for `exitMs` after `open` goes false so a CSS exit
/// transition has something to run against. Returns whether to render at all,
/// plus the state that drives the `is-*` class.
///
/// The CSS side of reduced motion can't shorten a timeout, so this checks the
/// same media query itself — otherwise an overlay would sit invisible but
/// mounted for the full duration, swallowing clicks.
export function usePresence(open: boolean, exitMs = 120) {
  const [mounted, setMounted] = useState(open);
  const [state, setState] = useState<PresenceState>(open ? 'entered' : 'exited');

  useEffect(() => {
    if (open) {
      setMounted(true);
      setState('entering');

      /// Two frames, not one: the first lets React commit the entering (start)
      /// styles, the second lets the engine paint them, so the flip to
      /// `entered` actually transitions. A single frame snaps intermittently
      /// in WKWebView.
      let inner = 0;
      const outer = requestAnimationFrame(() => {
        inner = requestAnimationFrame(() => setState('entered'));
      });
      return () => {
        cancelAnimationFrame(outer);
        cancelAnimationFrame(inner);
      };
    }

    setState((prev) => (prev === 'exited' ? prev : 'exiting'));
    const timer = window.setTimeout(
      () => {
        setMounted(false);
        setState('exited');
      },
      prefersReducedMotion() ? 0 : exitMs,
    );
    return () => window.clearTimeout(timer);
  }, [open, exitMs]);

  return { mounted, state };
}

/// Holds the last non-null value so an overlay whose payload the store clears
/// on close still has something to draw during its exit frames.
export function useLastPresent<T>(value: T | null | undefined): T | null {
  const ref = useRef<T | null>(value ?? null);
  if (value != null) ref.current = value;
  return ref.current;
}
