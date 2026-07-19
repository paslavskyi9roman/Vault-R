export function Spinner({ size = 14, className = '' }: { size?: number; className?: string }) {
  return (
    <span
      className={`v-spinner ${className}`.trim()}
      style={{ ['--spinner-size' as string]: `${size}px` }}
      role="status"
      aria-label="Loading"
    />
  );
}
