export type LatencySeverity = "good" | "warn" | "slow" | "critical" | "unknown";

function classifyLatency(value: number | null, thresholds: readonly number[]): LatencySeverity {
  if (value == null || !Number.isFinite(value) || value < 0) return "unknown";
  if (value >= thresholds[2]) return "critical";
  if (value >= thresholds[1]) return "slow";
  if (value >= thresholds[0]) return "warn";
  return "good";
}

export function firstResponseSeverity(value: number | null): LatencySeverity {
  return classifyLatency(value, [10_000, 30_000, 60_000]);
}

export function durationSeverity(value: number | null): LatencySeverity {
  return classifyLatency(value, [60_000, 180_000, 300_000]);
}

export function formatRequestLatency(value: number | null): string {
  if (value == null || !Number.isFinite(value) || value < 0) return "—";
  if (value < 1000) return `${Math.round(value)}ms`;
  if (value < 60_000) return `${(value / 1000).toFixed(2)}s`;
  const seconds = Math.round(value / 1000);
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

/** Match sub2api: full output tokens (including reasoning) / full request duration. */
export function formatOutputTokensPerSecond(outputTokens: number | null, durationMs: number | null): string {
  if (outputTokens == null || !Number.isFinite(outputTokens) || outputTokens <= 0
    || durationMs == null || !Number.isFinite(durationMs) || durationMs <= 0) return "—";
  const rate = outputTokens / (durationMs / 1000);
  return Number.isFinite(rate) ? `${rate.toFixed(1)} tok/s` : "—";
}
