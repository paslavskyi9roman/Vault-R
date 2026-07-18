/// Line-art icons drawn in currentColor so they inherit whatever button they
/// sit in. These replace the character entities the UI used to use: some
/// rendered as full-colour emoji on Windows, and the rest carried a different
/// stroke weight and baseline from everything around them.
///
/// All share one 16x16 viewBox and stroke weight so they stay optically
/// consistent when they sit side by side.

interface IconProps {
  size?: number;
  className?: string;
}

function Svg({ size = 14, className, children }: IconProps & { children: React.ReactNode }) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.4}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

/// Body and shackle are balanced so the ink is vertically centred in the
/// viewBox, and sized to carry the same optical weight as the gear it sits
/// next to in the top bar.
export function LockIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <rect x="3" y="6.75" width="10" height="6.75" rx="1.7" />
      <path d="M5.4 6.75V5a2.6 2.6 0 0 1 5.2 0v1.75" />
    </Svg>
  );
}

export function GearIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <circle cx="8" cy="8" r="2.05" />
      <path d="M8 2.3v1.5M8 12.2v1.5M12.03 3.97l-1.06 1.06M5.03 10.97l-1.06 1.06M13.7 8h-1.5M3.8 8H2.3M12.03 12.03l-1.06-1.06M5.03 5.03 3.97 3.97" />
    </Svg>
  );
}

export function PencilIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M11 2.9a1.45 1.45 0 0 1 2.05 2.05L5.3 12.7l-2.7.75.75-2.7z" />
      <path d="M9.95 3.95l2.05 2.05" />
    </Svg>
  );
}

/// Two offset sheets — the conventional duplicate mark, replacing U+29C9.
export function DuplicateIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <rect x="6.1" y="6.1" width="7.6" height="7.6" rx="1.5" />
      <path d="M10 6.1V4a1.5 1.5 0 0 0-1.5-1.5H3.8A1.5 1.5 0 0 0 2.3 4v4.7a1.5 1.5 0 0 0 1.5 1.5h2.3" />
    </Svg>
  );
}

/// Points right at rest. Callers rotate it 90deg to mean expanded, which
/// gives the disclosure a transition instead of swapping two characters.
/// Centred exactly on (8,8) so it pivots cleanly rather than wobbling.
export function ChevronIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M5.7 3.4 10.3 8l-4.6 4.6" />
    </Svg>
  );
}

export function WarningIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M8 2.6 14.4 13.4H1.6z" />
      <path d="M8 6.6v2.7M8 11.5h.01" />
    </Svg>
  );
}

/// An arrow turning back on itself: restore this value.
export function RestoreIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M2.9 4.5h5.4a4.7 4.7 0 1 1-4.7 4.7" />
      <path d="M5.3 2.1 2.9 4.5l2.4 2.4" />
    </Svg>
  );
}

export function PlusIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M8 3.4v9.2M3.4 8h9.2" />
    </Svg>
  );
}

/// Two chain links, replacing U+238D — which is a monostable-pulse symbol and
/// meant nothing here.
export function LinkIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M6.6 9.4a2.6 2.6 0 0 0 3.9.3l2-2a2.6 2.6 0 0 0-3.7-3.7l-1.1 1.1" />
      <path d="M9.4 6.6a2.6 2.6 0 0 0-3.9-.3l-2 2a2.6 2.6 0 0 0 3.7 3.7l1.1-1.1" />
    </Svg>
  );
}

export function CloseIcon({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M3.7 3.7l8.6 8.6M12.3 3.7l-8.6 8.6" />
    </Svg>
  );
}
