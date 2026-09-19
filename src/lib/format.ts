/** Formatting helpers shared across the UI. */

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** i;
  return `${value >= 10 || i === 0 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

export function formatTokens(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(n < 10_000 ? 1 : 0)}k`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}

const timeFmt = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' });
const dateFmt = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' });
const fullFmt = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
  hour: 'numeric',
  minute: '2-digit',
});

export const formatTime = (ms: number) => timeFmt.format(ms);
export const formatFull = (ms: number) => fullFmt.format(ms);

/** Short relative label for the sidebar: "2m", "3h", "Sep 14". */
export function formatRelative(ms: number): string {
  const diff = Date.now() - ms;
  if (diff < 60_000) return 'now';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h`;
  if (diff < 7 * 86_400_000) return `${Math.floor(diff / 86_400_000)}d`;
  return dateFmt.format(ms);
}

/**
 * Bucket a timestamp into the sidebar's date groups.
 * Uses local midnight boundaries, not fixed 24h windows, so "Yesterday"
 * means yesterday rather than "25 hours ago".
 */
export function dateGroup(ms: number): string {
  const d = new Date(ms);
  const today = new Date();
  const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
  const startOfDay = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((startOfToday - startOfDay) / 86_400_000);

  if (days <= 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return 'Previous 7 days';
  if (days < 30) return 'Previous 30 days';
  if (d.getFullYear() === today.getFullYear()) {
    return d.toLocaleDateString(undefined, { month: 'long' });
  }
  return String(d.getFullYear());
}

/** Estimated cost in the pricing table's currency, or null if unpriced. */
export function estimateCost(
  model: string,
  inputTokens: number,
  outputTokens: number,
  pricing: { models: Record<string, { input: number; output: number }> } | undefined,
): number | null {
  if (!pricing?.models) return null;
  // Exact id first, then the longest prefix match so dated ids such as
  // `claude-sonnet-4-5-20260101` resolve to their family entry.
  const exact = pricing.models[model];
  const entry =
    exact ??
    Object.entries(pricing.models)
      .filter(([id]) => model.startsWith(id))
      .sort((a, b) => b[0].length - a[0].length)[0]?.[1];
  if (!entry) return null;
  return (inputTokens / 1e6) * entry.input + (outputTokens / 1e6) * entry.output;
}

export function formatCost(value: number, currency = 'USD'): string {
  const fmt = new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency,
    minimumFractionDigits: value < 1 ? 3 : 2,
    maximumFractionDigits: value < 1 ? 4 : 2,
  });
  return fmt.format(value);
}
