import { RefreshCw, Settings } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { DragRegion } from "../components/DragRegion";
import { PinButton } from "../components/PinButton";
import { fetchUsage } from "../features/usage/api";
import { loadUsageConfig, loadCachedSnapshot, saveCachedSnapshot } from "../features/usage/storage";
import type { CodexUsageSnapshot, UsageViewState } from "../features/usage/types";
import { commandPreview, formatPercent, formatTimestamp, snapshotMessage, statusLabel } from "../features/usage/format";
import { setAlwaysOnTop } from "../features/window/api";
import { loadPinState, savePinState } from "../features/window/storage";
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

  const snapshot = usageState.snapshot;
  const remainingPercent = snapshot.remainingPercent ?? 0;
  const usedPercent = snapshot.usagePercent ?? (snapshot.remainingPercent ? 100 - snapshot.remainingPercent : 0);
  const progressStyle = useMemo(
    () => ({ width: `${Math.max(0, Math.min(100, remainingPercent))}%` }),
    [remainingPercent]
  );

  useEffect(() => {
    void setAlwaysOnTop(pinState.isPinned).catch(() => {
      // Window control failure must not block usage querying.
    });
  }, []);

  useEffect(() => {
    if (didRefreshOnStartup.current) {
      return;
    }

    didRefreshOnStartup.current = true;
    void refreshUsage();
  }, []);

  async function refreshUsage(): Promise<void> {
    if (usageState.kind === "loading") {
      return;
    }

    setUsageState({ kind: "loading", snapshot });

    try {
      const nextSnapshot = await fetchUsage(config);
      saveCachedSnapshot(nextSnapshot);
      setUsageState({
        kind: nextSnapshot.status === "ok" ? "ready" : "failed",
        snapshot: nextSnapshot
      });
    } catch (error) {
      const fallback: CodexUsageSnapshot = {
        ...snapshot,
        source: "codex-cli",
        status: "command_error",
        errorMessage: error instanceof Error ? error.message : "Unable to fetch Codex usage"
      };
      setUsageState({ kind: "failed", snapshot: fallback });
    }
  }

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
        <DragRegion>
          <div className="title-stack">
            <span className="app-title">Codex Meter</span>
            <span className="app-subtitle">{statusLabel(snapshot.status)}</span>
          </div>
        </DragRegion>
        <div className="window-actions">
          <button className="icon-button" type="button" aria-label="Settings" title="Settings">
            <Settings size={16} aria-hidden="true" />
          </button>
          <PinButton isPinned={pinState.isPinned} isBusy={pinBusy} onToggle={() => void togglePinned()} />
        </div>
      </header>

      <section className="usage-panel" aria-label="Codex usage">
        <div className="usage-topline">
          <div>
            <div className="metric-label">Remaining</div>
            <div className="metric-value">{formatPercent(snapshot.remainingPercent)}</div>
          </div>
          {pinState.isPinned ? <span className="pin-badge">Pinned</span> : null}
        </div>

        <div className="progress-track" aria-label={`${formatPercent(snapshot.remainingPercent)} remaining`}>
          <div className="progress-fill" style={progressStyle} />
        </div>

        <div className="usage-grid">
          <Metric label="Used" value={formatPercent(usedPercent)} />
          <Metric label="Updated" value={formatTimestamp(snapshot.fetchedAt)} />
        </div>

        <div className={`status-line status-${snapshot.status}`}>
          <span className="status-dot" />
          <span>{snapshotMessage(snapshot)}</span>
        </div>
      </section>

      <footer className="footer-row">
        <span className="command-preview">{commandPreview(config)}</span>
        <button
          className="refresh-button"
          type="button"
          disabled={usageState.kind === "loading"}
          onClick={() => void refreshUsage()}
        >
          <RefreshCw className={usageState.kind === "loading" ? "spin" : ""} size={15} aria-hidden="true" />
          Refresh
        </button>
      </footer>
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

export default App;
