import { RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { DragRegion } from "../components/DragRegion";
import { PinButton } from "../components/PinButton";
import { fetchUsage } from "../features/usage/api";
import { loadUsageConfig, loadCachedSnapshot, saveCachedSnapshot } from "../features/usage/storage";
import type { CodexUsageSnapshot, UsageViewState } from "../features/usage/types";
import {
  formatPercent,
  formatResetTimestamp,
  resolveFiveHourLimit,
  resolveWeeklyLimit,
  snapshotMessage
} from "../features/usage/format";
import {
  canStartManualRefresh,
  mergeUsageRefreshResult,
  nextAutomaticRefreshDelayMs
} from "../features/usage/refresh";
import { getAlwaysOnTop, getWindowPlacement, restoreWindowPlacement, setAlwaysOnTop } from "../features/window/api";
import { useAutoWindowSize } from "../features/window/autoSize";
import { loadPinState, loadWindowPlacement, savePinState, saveWindowPlacement } from "../features/window/storage";
import type { WindowPinState } from "../features/window/types";

function App() {
  const contentRef = useRef<HTMLDivElement>(null);
  const [config] = useState(loadUsageConfig);
  const [usageState, setUsageState] = useState<UsageViewState>({
    kind: "idle",
    snapshot: loadCachedSnapshot()
  });
  const [pinState, setPinState] = useState<WindowPinState>(loadPinState);
  const [pinBusy, setPinBusy] = useState(false);
  const didRefreshOnStartup = useRef(false);
  const isFetchingUsage = useRef(false);
  const consecutiveRefreshFailureCount = useRef(0);
  const lastManualRefreshAt = useRef(0);
  const snapshotRef = useRef(usageState.snapshot);

  const snapshot = usageState.snapshot;
  const fiveHourLimit = resolveFiveHourLimit(snapshot);
  const weeklyLimit = resolveWeeklyLimit(snapshot);
  useAutoWindowSize(contentRef);

  useEffect(() => {
    snapshotRef.current = snapshot;
  }, [snapshot]);

  const applyUsageSnapshot = useCallback((nextSnapshot: CodexUsageSnapshot): CodexUsageSnapshot => {
    const currentSnapshot = snapshotRef.current;
    const displaySnapshot = mergeUsageRefreshResult(currentSnapshot, nextSnapshot);
    saveCachedSnapshot(displaySnapshot);
    snapshotRef.current = displaySnapshot;
    setUsageState({
      kind: displaySnapshot.status === "ok" ? "ready" : "failed",
      snapshot: displaySnapshot
    });
    consecutiveRefreshFailureCount.current =
      displaySnapshot.status === "ok" ? 0 : consecutiveRefreshFailureCount.current + 1;

    return displaySnapshot;
  }, []);

  const refreshUsage = useCallback(async (): Promise<CodexUsageSnapshot | undefined> => {
    if (isFetchingUsage.current) {
      return undefined;
    }

    const currentSnapshot = snapshotRef.current;
    isFetchingUsage.current = true;
    setUsageState({ kind: "loading", snapshot: currentSnapshot });

    try {
      const nextSnapshot = await fetchUsage(config);
      return applyUsageSnapshot(nextSnapshot);
    } catch (error) {
      const fallback: CodexUsageSnapshot = {
        ...currentSnapshot,
        source: "codex-cli",
        status: "command_error",
        errorMessage: error instanceof Error ? error.message : "Unable to fetch Codex usage"
      };
      snapshotRef.current = fallback;
      setUsageState({ kind: "failed", snapshot: fallback });
      consecutiveRefreshFailureCount.current += 1;
      return fallback;
    } finally {
      isFetchingUsage.current = false;
    }
  }, [applyUsageSnapshot, config]);

  const refreshUsageManually = useCallback((): void => {
    const now = Date.now();
    if (!canStartManualRefresh(now, lastManualRefreshAt.current)) {
      return;
    }

    lastManualRefreshAt.current = now;
    void refreshUsage();
  }, [refreshUsage]);

  useEffect(() => {
    void setAlwaysOnTop(pinState.isPinned)
      .then(() => getAlwaysOnTop())
      .then((isPinned) => {
        if (isPinned === pinState.isPinned) {
          return;
        }

        const nextState = { isPinned, updatedAt: new Date().toISOString() };
        savePinState(nextState);
        setPinState(nextState);
      })
      .catch(() => {
        // Window control failure must not block usage querying.
      });
  }, []);

  useEffect(() => {
    const placement = loadWindowPlacement();
    if (!placement) {
      return;
    }

    void restoreWindowPlacement(placement)
      .then(saveWindowPlacement)
      .catch(() => {
        // Window placement failure must not block usage querying.
      });
  }, []);

  useEffect(() => {
    if (didRefreshOnStartup.current) {
      return;
    }

    didRefreshOnStartup.current = true;
    void refreshUsage();
  }, [refreshUsage]);

  useEffect(() => {
    if (usageState.kind === "loading") {
      return;
    }

    const delayMs = nextAutomaticRefreshDelayMs(
      snapshot,
      config.pollIntervalSeconds,
      consecutiveRefreshFailureCount.current
    );
    const timeoutId = window.setTimeout(() => {
      void refreshUsage();
    }, delayMs);

    return () => window.clearTimeout(timeoutId);
  }, [config.pollIntervalSeconds, refreshUsage, snapshot, usageState.kind]);

  const saveCurrentPlacement = useCallback(() => {
    void getWindowPlacement()
      .then(saveWindowPlacement)
      .catch(() => {
        // Drag placement persistence is best effort.
      });
  }, []);

  async function togglePinned(): Promise<void> {
    const nextPinned = !pinState.isPinned;
    setPinBusy(true);

    try {
      await setAlwaysOnTop(nextPinned);
      const nextState = { isPinned: nextPinned, updatedAt: new Date().toISOString() };
      savePinState(nextState);
      setPinState(nextState);
    } finally {
      setPinBusy(false);
    }
  }

  return (
    <DragRegion className="app-shell" element="main" onDragComplete={saveCurrentPlacement}>
      <div className="app-content" ref={contentRef}>
        <header className="window-header">
          <div className="title-stack">
            <span className="app-title">Codex Meter</span>
          </div>
          <div className="window-actions">
            <button
              className="icon-button"
              type="button"
              aria-label="Refresh usage"
              title="Refresh usage"
              disabled={usageState.kind === "loading"}
              onClick={refreshUsageManually}
            >
              <RefreshCw className={usageState.kind === "loading" ? "spin" : ""} size={16} aria-hidden="true" />
            </button>
            <PinButton isPinned={pinState.isPinned} isBusy={pinBusy} onToggle={() => void togglePinned()} />
          </div>
        </header>

        <section className="usage-panel" aria-label="Codex usage">
          <LimitMeter label="5 hours" limit={fiveHourLimit} />
          <LimitMeter label="Weekly" limit={weeklyLimit} />

          {snapshot.status === "ok" ? null : (
            <div className={`status-line status-${snapshot.status}`}>
              <span className="status-dot" />
              <span>{snapshotMessage(snapshot)}</span>
            </div>
          )}
        </section>
      </div>
    </DragRegion>
  );
}

type MetricProps = {
  label: string;
  value: string;
};

function Metric({ label, value }: MetricProps) {
  const [primaryValue, detailValue] = value.split("\n", 2);

  return (
    <div className="metric">
      <span>{label}</span>
      <strong>
        <span className="metric-value">{primaryValue}</span>
        {detailValue ? <span className="metric-detail">{detailValue}</span> : null}
      </strong>
    </div>
  );
}

type LimitMeterProps = {
  label: string;
  limit: {
    usagePercent?: number;
    remainingPercent?: number;
    resetAt?: string;
  };
};

function LimitMeter({ label, limit }: LimitMeterProps) {
  const remainingPercent = limit.remainingPercent ?? 0;
  const progressStyle = useMemo(
    () => ({ width: `${Math.max(0, Math.min(100, remainingPercent))}%` }),
    [remainingPercent]
  );
  const progressTone = getProgressTone(remainingPercent);

  return (
    <div className="limit-meter">
      <div className="limit-heading">
        <span>{label}</span>
        <strong>{formatPercent(limit.remainingPercent)} left</strong>
      </div>
      <div className="progress-track" aria-label={`${label} ${formatPercent(limit.remainingPercent)} remaining`}>
        <div className={`progress-fill progress-fill--${progressTone}`} style={progressStyle} />
      </div>
      <Metric label="Reset" value={formatResetTimestamp(limit.resetAt)} />
    </div>
  );
}

function getProgressTone(remainingPercent: number): "safe" | "warning" | "danger" {
  if (remainingPercent < 20) {
    return "danger";
  }

  if (remainingPercent < 60) {
    return "warning";
  }

  return "safe";
}

export default App;
