import { useEffect, useState, type FormEvent } from "react";
import { isWslSupported } from "../features/usage/api";
import type { CliUsageConfig } from "../features/usage/types";

type Props = {
  config: CliUsageConfig;
  busy: boolean;
  onSave: (config: CliUsageConfig) => void;
  onCancel: () => void;
};

export function UsageSettings({ config, busy, onSave, onCancel }: Props) {
  const [executionMode, setExecutionMode] = useState(config.executionMode);
  const [error, setError] = useState("");
  const [wslSupported, setWslSupported] = useState<boolean | undefined>(undefined);

  useEffect(() => {
    let active = true;
    void isWslSupported().catch(() => false).then((supported) => {
      if (!active) return;
      setWslSupported(supported);
      if (!supported) setExecutionMode("native");
    });
    return () => { active = false; };
  }, []);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      onSave({ ...config, executionMode });
    } catch (error) {
      setError(error instanceof Error ? error.message : "Unable to save settings");
    }
  }

  return (
    <form className="settings-panel" onSubmit={submit} aria-label="Usage settings">
      <label data-no-drag>
        Run Codex in
        <select
          disabled={wslSupported === undefined}
          value={wslSupported ? executionMode : "native"}
          onChange={(event) => setExecutionMode(event.target.value as CliUsageConfig["executionMode"])}
        >
          <option value="native">Local system</option>
          {wslSupported && <option value="wsl">WSL (Windows)</option>}
        </select>
      </label>
      {error && <p className="settings-error" role="alert">{error}</p>}
      <div className="settings-actions">
        <button type="button" onClick={onCancel}>Cancel</button>
        <button type="submit" disabled={busy || wslSupported === undefined}>Save & refresh</button>
      </div>
    </form>
  );
}
