export function Skeleton({
  width = '100%',
  height = 12,
  className = '',
}: {
  width?: string | number;
  height?: number;
  className?: string;
}) {
  return <span className={`v-skel ${className}`.trim()} style={{ width, height }} aria-hidden="true" />;
}
