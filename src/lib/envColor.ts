/// Environments whose secrets are the real ones. Revealing, copying, exporting
/// or overwriting these asks first — the cost of a slip is a live credential,
/// not a local one.
export function isProtectedEnv(name: string): boolean {
  return /^(prod|production|live)$/i.test(name.trim());
}

export function envColor(name: string): string {
  if (name === 'local') return 'var(--env-local)';
  if (name === 'staging') return 'var(--env-staging)';
  if (name === 'production') return 'var(--env-production)';
  return 'var(--env-custom)';
}

export function timeAgo(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime();
  const minutes = Math.round(diffMs / 60000);
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? '' : 's'} ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`;
  const days = Math.round(hours / 24);
  return `${days} day${days === 1 ? '' : 's'} ago`;
}
