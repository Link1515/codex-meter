export const manualRefreshDebounceMs = 1000;

export function canStartManualRefresh(
  nowMs: number,
  lastRefreshMs: number,
  debounceMs = manualRefreshDebounceMs
): boolean {
  return nowMs - lastRefreshMs >= debounceMs;
}
