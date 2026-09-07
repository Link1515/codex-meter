import { RefreshCw } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { DragRegion } from "../components/DragRegion";
import { PinButton } from "../components/PinButton";
import { fetchUsage } from "../features/usage/api";
import { defaultUsageConfig } from "../features/usage/defaults";
import { loadCachedSnapshot, saveCachedSnapshot } from "../features/usage/storage";
import type { CodexUsageSnapshot, UsageViewState } from "../features/usage/types";
import {
  formatDatedResetTimestamp,
  formatPercent,
  formatCompactResetTimestamp,
  snapshotMessage
} from "../features/usage/format";
import {
  canStartManualRefresh,
  mergeUsageRefreshResult,
  nextUsagePollingCheckDelayMs
} from "../features/usage/refresh";
import {
  getAlwaysOnTop,
  getWindowPlacement,
  getWindowPollingAllowed,
  restoreWindowPlacement,
  setAlwaysOnTop
} from "../features/window/api";
import { useAutoWindowSize } from "../features/window/autoSize";
import { loadPinState, loadWindowPlacement, savePinState, saveWindowPlacement } from "../features/window/storage";
import type { WindowPinState } from "../features/window/types";
import { usageRefreshRequestedEvent, useWindowPollingEligibility } from "../features/window/visibility";

function App() {
  const contentRef = useRef<HTMLDivElement>(null);
  const config = defaultUsageConfig;
  const [usageState, setUsageState] = useState<UsageViewState>({
    kind: "idle",
    snapshot: loadCachedSnapshot()
  });
  const [pinState, setPinState] = useState<WindowPinState>(loadPinState);
  const [pinBusy, setPinBusy] = useState(false);
  const isFetchingUsage = useRef(false);
  const consecutiveRefreshFailureCount = useRef(0);
  const lastManualRefreshAt = useRef(0);
  const snapshotRef = useRef(usageState.snapshot);

  const snapshot = usageState.snapshot;
  const fiveHourLimit = snapshot.fiveHourUsageLimit ?? {};
  const weeklyLimit = snapshot.weeklyUsageLimit ?? {};
  const { isWindowPollingAllowed, setWindowPollingAllowed } = useWindowPollingEligibility();
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
    if (!isWindowPollingAllowed) {
      return;
    }

    void refreshUsage();
  }, [isWindowPollingAllowed, refreshUsage]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let isActive = true;
    let unlisten: UnlistenFn | undefined;

    void listen<void>(usageRefreshRequestedEvent, () => {
      void refreshUsage();
    }).then((removeListener) => {
      if (isActive) {
        unlisten = removeListener;
      } else {
        removeListener();
      }
    });

    return () => {
      isActive = false;
      unlisten?.();
    };
  }, [refreshUsage]);

  useEffect(() => {
    if (usageState.kind === "loading") {
      return;
    }

    let isActive = true;
    let timeoutId: number | undefined;
    const schedulePollingCheck = (isAllowed: boolean) => {
      const delayMs = nextUsagePollingCheckDelayMs(
        snapshot,
        config.pollIntervalSeconds,
        consecutiveRefreshFailureCount.current,
        isAllowed
      );
      timeoutId = window.setTimeout(runPollingCheck, delayMs);
    };
    const runPollingCheck = () => {
      if (!isTauri()) {
        void refreshUsage();
        return;
      }

      void getWindowPollingAllowed()
        .then((allowed) => {
          if (!isActive) {
            return;
          }

          setWindowPollingAllowed(allowed);
          if (allowed) {
            void refreshUsage();
            return;
          }

          schedulePollingCheck(false);
        })
        .catch(() => {
          if (!isActive) {
            return;
          }

          setWindowPollingAllowed(false);
          schedulePollingCheck(false);
        });
    };

    if (isWindowPollingAllowed) {
      schedulePollingCheck(true);
    } else {
      runPollingCheck();
    }

    return () => {
      isActive = false;
      if (timeoutId !== undefined) {
        window.clearTimeout(timeoutId);
      }
    };
  }, [config.pollIntervalSeconds, isWindowPollingAllowed, refreshUsage, setWindowPollingAllowed, snapshot, usageState.kind]);

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
    } catch {
      // Keep the persisted state unchanged when window control fails.
    } finally {
      setPinBusy(false);
    }
  }

  return (
    <DragRegion className="app-shell" element="main" onDragComplete={saveCurrentPlacement}>
      <div className="app-content" ref={contentRef}>
        <header className="window-header">
          <span className="app-title">Codex Meter</span>
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
          <LimitMeter label="5h" limit={fiveHourLimit} />
          <LimitMeter label="Weekly" limit={weeklyLimit} showResetDate />

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

type LimitMeterProps = {
  label: string;
  showResetDate?: boolean;
  limit: {
    remainingPercent?: number;
    resetAt?: string;
  };
};

function LimitMeter({ label, limit, showResetDate = false }: LimitMeterProps) {
  const remainingPercent =
    typeof limit.remainingPercent === "number" && Number.isFinite(limit.remainingPercent)
      ? limit.remainingPercent
      : 0;
  const normalizedRemainingPercent = Math.max(0, Math.min(100, remainingPercent));
  const progressTone = getProgressTone(normalizedRemainingPercent);
  const remainingLabel = formatPercent(limit.remainingPercent);
  const resetLabel = showResetDate
    ? formatDatedResetTimestamp(limit.resetAt)
    : formatCompactResetTimestamp(limit.resetAt);
  const progressStroke = `${normalizedRemainingPercent} 100`;

  return (
    <div className="limit-meter">
      <div className="limit-dial" role="img" aria-label={`${label}: ${remainingLabel} remaining, resets ${resetLabel}`}>
        <svg viewBox="0 0 76 76" aria-hidden="true">
          <path className="dial-track" pathLength="100" d="M 18 57 A 28 28 0 1 1 58 57" />
          {normalizedRemainingPercent > 0 ? (
            <path
              className={`dial-fill dial-fill--${progressTone}`}
              pathLength="100"
              d="M 18 57 A 28 28 0 1 1 58 57"
              style={{ strokeDasharray: progressStroke }}
            />
          ) : null}
        </svg>
        <div className="dial-content">
          <span className="dial-label">{label}</span>
          <strong className="limit-remaining">{remainingLabel}</strong>
        </div>
      </div>
      <span className="limit-reset">{resetLabel}</span>
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
