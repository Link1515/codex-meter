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
import { getAlwaysOnTop, getWindowPlacement, restoreWindowPlacement, setAlwaysOnTop } from "../features/window/api";
import { loadPinState, loadWindowPlacement, savePinState, saveWindowPlacement } from "../features/window/storage";
import type { WindowPinState } from "../features/window/types";

function App() {
  const [config] = useState(loadUsageConfig);
  const [usageState, setUsageState] = useState<UsageViewState>({
    kind: "idle",
    snapshot: loadCachedSnapshot()
  });
  const [pinState, setPinState] = useState<WindowPinState>(loadPinState);
  const [pinBusy, setPinBusy] = useState(false);
  const didRefreshOnStartup = useRef(false);
  const isFetchingUsage = useRef(false);
  const snapshotRef = useRef(usageState.snapshot);

  const snapshot = usageState.snapshot;
  const fiveHourLimit = resolveFiveHourLimit(snapshot);
  const weeklyLimit = resolveWeeklyLimit(snapshot);

  useEffect(() => {
    snapshotRef.current = snapshot;
  }, [snapshot]);

  const refreshUsage = useCallback(async (): Promise<void> => {
    if (isFetchingUsage.current) {
      return;
    }

    const currentSnapshot = snapshotRef.current;
    isFetchingUsage.current = true;
    setUsageState({ kind: "loading", snapshot: currentSnapshot });

    try {
      const nextSnapshot = await fetchUsage(config);
      saveCachedSnapshot(nextSnapshot);
      setUsageState({
        kind: nextSnapshot.status === "ok" ? "ready" : "failed",
        snapshot: nextSnapshot
      });
    } catch (error) {
      const fallback: CodexUsageSnapshot = {
        ...currentSnapshot,
        source: "codex-cli",
        status: "command_error",
        errorMessage: error instanceof Error ? error.message : "Unable to fetch Codex usage"
      };
      setUsageState({ kind: "failed", snapshot: fallback });
    } finally {
      isFetchingUsage.current = false;
    }
  }, [config]);

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
    const intervalSeconds = Math.max(30, config.pollIntervalSeconds);
    const intervalId = window.setInterval(() => {
      void refreshUsage();
    }, intervalSeconds * 1000);

    return () => window.clearInterval(intervalId);
  }, [config.pollIntervalSeconds, refreshUsage]);

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
    <main className="app-shell">
      <header className="window-header">
        <DragRegion onDragComplete={saveCurrentPlacement}>
          <div className="title-stack">
            <span className="app-title">Codex Meter</span>
          </div>
        </DragRegion>
        <div className="window-actions">
          <button
            className="icon-button"
            type="button"
            aria-label="Refresh usage"
            title="Refresh usage"
            disabled={usageState.kind === "loading"}
            onClick={() => void refreshUsage()}
          >
            <RefreshCw className={usageState.kind === "loading" ? "spin" : ""} size={16} aria-hidden="true" />
          </button>
          <PinButton isPinned={pinState.isPinned} isBusy={pinBusy} onToggle={() => void togglePinned()} />
        </div>
      </header>

      <section className="usage-panel" aria-label="Codex usage">
        <LimitMeter label="5 hour" limit={fiveHourLimit} />
        <LimitMeter label="Weekly" limit={weeklyLimit} />

        {snapshot.status === "ok" ? null : (
          <div className={`status-line status-${snapshot.status}`}>
            <span className="status-dot" />
            <span>{snapshotMessage(snapshot)}</span>
          </div>
        )}
      </section>
    </main>
  );
}

type MetricProps = {
  label: string;
  value: string;
};

function Metric({ label, value }: MetricProps) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
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

  return (
    <div className="limit-meter">
      <div className="limit-heading">
        <span>{label}</span>
        <strong>{formatPercent(limit.remainingPercent)} left</strong>
      </div>
      <div className="progress-track" aria-label={`${label} ${formatPercent(limit.remainingPercent)} remaining`}>
        <div className="progress-fill" style={progressStyle} />
      </div>
      <Metric label="Reset" value={formatResetTimestamp(limit.resetAt)} />
    </div>
  );
}

export default App;
