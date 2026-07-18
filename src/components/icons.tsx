/// Line-art icons drawn in currentColor so they inherit whatever button they
/// sit in. These replace the character entities that rendered as full-colour
/// emoji on Windows, which fought with the monochrome chrome around them.

interface IconProps {
  size?: number;
}

function Svg({ size = 14, children }: IconProps & { children: React.ReactNode }) {
  return (
    <svg
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
export function LockIcon({ size }: IconProps) {
  return (
    <Svg size={size}>
      <rect x="3" y="6.75" width="10" height="6.75" rx="1.7" />
      <path d="M5.4 6.75V5a2.6 2.6 0 0 1 5.2 0v1.75" />
    </Svg>
  );
}

export function GearIcon({ size }: IconProps) {
  return (
    <Svg size={size}>
      <circle cx="8" cy="8" r="2.05" />
      <path d="M8 2.3v1.5M8 12.2v1.5M12.03 3.97l-1.06 1.06M5.03 10.97l-1.06 1.06M13.7 8h-1.5M3.8 8H2.3M12.03 12.03l-1.06-1.06M5.03 5.03 3.97 3.97" />
    </Svg>
  );
}
