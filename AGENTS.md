# AGENTS.md

## Default Working Rules

- Do not run `npm run build` or `pnpm build` automatically.
- Do not run build commands at the end of a task unless the user explicitly asks for them.
- Only run the smallest relevant verification step needed.
- Prefer focused tests, lint, or typecheck over a full build when validating changes.
- Before running any expensive command, explain why it is needed.
- This project uses `pnpm`; do not introduce npm or yarn lockfiles.

---

## Project Goal

Codex Meter is a lightweight cross-platform desktop widget for showing local Codex usage / token availability.

Core goals:

1. Keep a compact desktop window visible with current Codex usage.
2. Query local Codex state through the Codex CLI / Codex local auth context, not through browser automation or web scraping.
3. Support macOS, Windows, and Linux through Tauri v2.
4. Keep resource use low enough for a background desktop widget.
5. Keep usage parsing and fetching logic centralized and configurable because Codex CLI output can change.
6. Let the user toggle always-on-top with a pin button.
7. Let the user drag the frameless widget and persist its placement.

---

## Current Stack

- Desktop shell: Tauri v2
- Backend: Rust
- Frontend: React 19 + TypeScript + Vite
- UI styling: plain CSS in `src/styles.css`
- Icons: `lucide-react`
- Frontend persistence: `localStorage`
- CLI execution: Rust `std::process::Command`
- HTTP client: Rust `reqwest` for current Codex OAuth usage probing
- Tests: Vitest for frontend units, Rust unit tests in `src-tauri`

Do not add Electron.

---

## Current Implementation Snapshot

Implemented:

- Compact frameless widget window configured in `src-tauri/tauri.conf.json`.
- Usage display for the weekly remaining limit.
- Manual refresh button with debounce.
- Automatic polling with minimum 60 second delay and retry backoff.
- Cached last usage snapshot to avoid blank UI on startup.
- Pin button using Tauri always-on-top commands.
- Persisted pinned state.
- Drag region component with placement save on drag completion.
- Window placement restore with visible-bounds correction.
- Content-aware window sizing, constrained to a compact 280–420 × 160–360 logical-pixel range.
- The Tauri window remains hidden until its first content-driven resize completes, avoiding a visible resize flash.
- Rust Codex adapter with timeout handling and command output capture.
- Text parser and JSON parser.
- Codex `app-server` RPC probe for `account/rateLimits/read`.
- OAuth usage probe using the local Codex auth file, preferred for the default app-server configuration; app-server RPC is the fallback.
- Mock CLI in `dev/mock-codex-cli`.
- Unit tests for usage refresh/storage/format, pin state, auto-sizing, parser behavior, adapter helpers, and placement correction.

Not yet implemented or incomplete:

- Settings page UI.
- Regex parser mode.
- Tray integration and tray tooltip.
- Tauri Store plugin or SQLite persistence.
- Startup-on-login setting.
- Tray-only/background mode.
- Raw output debug panel.
- Integration tests with separate mock binaries and slow/non-zero command scenarios.
- Packaging, signing, updater, and release workflow.

---

## Architecture

Keep the three-layer separation:

1. UI Layer
   - React components in `src/app` and `src/components`.
   - Displays usage, status, refresh, pin, and drag affordances.
   - Must not contain CLI execution or parser logic.

2. Application Layer
   - Frontend state, storage, polling, formatting, and window APIs in `src/features`.
   - Handles cache merge behavior, refresh debounce, backoff, and persisted UI state.
   - Window state and usage state must remain separate.

3. Adapter Layer
   - Rust modules under `src-tauri/src/codex`.
   - Owns CLI execution, app-server RPC, OAuth usage probing, parsing, timeout handling, and sanitization.
   - Parser implementations live in `src-tauri/src/codex/parser.rs`.

Window control belongs in `src-tauri/src/commands/window.rs`, `src-tauri/src/window`, and frontend `src/features/window`. The adaptive sizing hook measures cloned rendered content; keep sizing calculations and Tauri resize calls out of React view components.

Usage fetching belongs in `src-tauri/src/commands/usage.rs`, `src-tauri/src/codex`, and frontend `src/features/usage`.

---

## Important Files

```txt
src/app/App.tsx                         Main widget UI and polling orchestration
src/styles.css                          Widget styles
src/components/DragRegion.tsx           Tauri drag region wrapper
src/components/PinButton.tsx            Always-on-top pin button
src/features/usage/defaults.ts          Default CLI/RPC config
src/features/usage/refresh.ts           Refresh debounce, cache merge, backoff
src/features/usage/storage.ts           Usage config and snapshot localStorage
src/features/usage/format.ts            Display formatting
src/features/window/api.ts              Frontend Tauri window command wrappers
src/features/window/autoSize.ts         Content measurement and bounded adaptive window sizing
src/features/window/storage.ts          Pin and placement localStorage
src-tauri/src/codex/adapter.rs          CLI/RPC/OAuth usage adapter
src-tauri/src/codex/errors.rs           Tauri command error model
src-tauri/src/codex/parser.rs           Text and JSON parsers
src-tauri/src/codex/types.rs            Backend usage data model
src-tauri/src/commands/usage.rs         Tauri usage commands
src-tauri/src/commands/window.rs        Tauri window commands
src-tauri/src/window/placement.rs       Placement bounds correction
dev/mock-codex-cli/                     Local mock CLI
tests/unit/                             Vitest unit tests
```

---

## Default Usage Config

The current default frontend config is:

```ts
{
  codexCommand: "codex",
  usageArgs: ["-s", "read-only", "-a", "never", "app-server"],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Json"
}
```

The backend treats configs containing `app-server` as Codex usage probes. Except for the development mock, the default configuration first tries the local OAuth usage endpoint and falls back to the Codex app-server RPC probe. For normal stdout parsing, use `parserMode: "Text"` or `parserMode: "Json"` with non-`app-server` args.

Development mock alias:

```ts
codexCommand: "__codex_meter_mock__"
```

That alias resolves to `dev/mock-codex-cli/index.js` in the Rust adapter.

---

## Security Rules

- Never concatenate user input into a shell command string.
- Always keep command and args separate in Rust:

```rust
Command::new(command).args(args)
```

- Keep sensitive values out of logs and UI:
  - API keys
  - auth headers
  - cookies
  - access tokens
  - refresh tokens
  - sessions
- Use the existing sanitization path before showing command errors.
- Raw CLI output should remain hidden unless a future explicit debug setting enables it.
- Do not read browser cookies or automate browser pages.
- Do not modify Codex CLI internals.
- Do not attempt to bypass Codex usage limits.

Current implementation note: `src-tauri/src/codex/adapter.rs` can read the local Codex auth file to probe the Codex OAuth usage endpoint. Treat this path as sensitive: never log tokens, never persist tokens, and keep failures sanitized.

---

## Polling Rules

- Do not use high-frequency polling.
- Minimum automatic refresh delay is 60 seconds.
- Manual refresh is debounced.
- Only one usage fetch should run at a time.
- Timeout must kill the child process.
- On refresh failure, keep displayable cached usage while updating status/error metadata.
- Window dragging and pin toggling must not trigger usage refresh.

Current constants live in `src/features/usage/refresh.ts`.

---

## Window Rules

- The main window is frameless and user-non-resizable. It adapts to rendered content within the configured minimum and maximum dimensions.
- The initial window must remain hidden until the first sizing attempt completes; later sizing failures must fall back to showing it.
- Tauri briefly enables resizing only while applying a programmatic size, and must restore the non-resizable state afterwards.
- Pin state defaults to unpinned and is stored in `localStorage`.
- Pin toggle uses Tauri `set_always_on_top`.
- Dragging uses `start_dragging`; avoid making buttons or interactive controls drag regions.
- Placement is saved after drag completion.
- Restore must correct out-of-bounds placement against available monitor bounds.
- Window control failures must not block usage querying.
- Usage query failures must not block window control.

---

## UI Rules

The first screen is the actual compact meter, not a marketing page.

Current main UI contains:

- App title
- Refresh icon button
- Pin icon button
- Weekly limit meter
- Error/status line for non-ok snapshots

The widget is initially hidden by Tauri configuration and is revealed by `useAutoWindowSize` after the initial sizing attempt.

When adding UI:

- Use `lucide-react` icons when an icon exists.
- Keep controls compact and suitable for a desktop widget.
- Keep text inside fixed-size controls from overflowing.
- Do not put CLI logic in React components.
- Do not add visible instructions about how to use obvious controls.
- Avoid nested cards and decorative gradients/orbs.

---

## Parser Rules

Backend parser trait:

```rust
pub trait UsageParser {
    fn parse(&self, input: &str) -> CodexUsageSnapshot;
}
```

Current parser modes:

- `Text`
- `Json`

Rules:

- Parsers must not panic.
- Empty output returns `parse_error`.
- Malformed output returns `parse_error`.
- Authentication-looking output returns `not_authenticated`.
- Percent values must be clamped or validated to the 0-100 range.
- Add new parser modes by extending both Rust and TypeScript types.

Regex parser is planned but not currently implemented.

---

## Error Handling

Keep stable app-level usage statuses:

```ts
"ok" | "unknown" | "cli_not_found" | "not_authenticated" | "timeout" | "parse_error" | "command_error"
```

Backend window and usage commands should return explicit `AppError` values, not panic.

Expected error classes to preserve:

- CLI binary missing
- CLI non-zero exit
- CLI timeout
- Codex not authenticated
- empty output
- parser failure
- invalid config
- window always-on-top failure
- window placement read/restore failure
- out-of-bounds placement correction

---

## Verification

Use the smallest relevant command.

Frontend typecheck:

```txt
pnpm typecheck
```

Frontend unit tests:

```txt
pnpm exec vitest run tests/unit/<file>.test.ts
```

All frontend unit tests:

```txt
pnpm test
```

Rust unit tests:

```txt
cargo test --manifest-path src-tauri/Cargo.toml
```

Do not run build commands unless explicitly requested.

---

## Mock CLI

Mock CLI location:

```txt
dev/mock-codex-cli/
```

Expected sample output:

```txt
Codex usage
Model: gpt-5-codex
Weekly usage limit: 45%
Weekly resets at: 2026-01-04T00:00:00+08:00
```

Do not make tests depend on a real Codex CLI install or real OpenAI account.

---

## Roadmap

Near-term:

1. Add Settings UI for CLI command, args, poll interval, timeout, parser mode, and the existing backend manual command test.
2. Add regex parser mode.
3. Add raw output debug mode gated behind an explicit setting.
4. Add tray integration and tooltip.
5. Add integration tests for mock CLI stdout/stderr/non-zero/slow command scenarios.

Later:

1. Tauri Store plugin or SQLite persistence.
2. Startup-on-login option.
3. Tray-only/background mode.
4. Auto updater.
5. Cross-platform packaging, signing, and release workflow.

---

## Non-Goals

- No Electron.
- No ChatGPT or Codex web page scraping.
- No browser cookie reads.
- No network interception.
- No OpenAI password storage.
- No forced always-on-top without user action or restored user preference.
- No forced window position reset without user action, except visible-bounds correction during restore.
- No high-frequency polling.
- No Codex usage limit bypass behavior.

---

## Code Style

TypeScript:

- Use strict types.
- Avoid `any`.
- Prefer discriminated unions for UI state.
- Keep domain models separate from view state.
- Keep usage state separate from window state.

Rust:

- Do not use `unwrap()` for expected failures.
- Use explicit error enums / app errors.
- Set command timeouts.
- Kill timed-out child processes.
- Keep parser code non-panicking.
- Sanitize logs and UI-facing command errors.
- Return explicit window command errors.

---

## Commit Style

Use Conventional Commits:

```txt
feat: add codex cli usage adapter
feat: add always-on-top pin toggle
feat: persist draggable window placement
fix: handle cli timeout correctly
fix: restore window inside visible display bounds
refactor: isolate usage parser
refactor: isolate window controller
test: add malformed output parser tests
test: add pinned window state tests
docs: update project agent instructions
```
