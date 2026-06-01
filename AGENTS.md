# AGENTS.md

## 專案目標

開發一個跨平台桌面應用程式，用來即時顯示 Codex token / usage 剩餘量。

核心需求：

1. 在桌面上常駐顯示 Codex 剩餘用量。
2. 透過本機 Codex CLI 指令查詢目前用量，不直接依賴瀏覽器或網頁爬取。
3. 支援 macOS、Windows、Linux。
4. 使用低資源消耗技術，適合背景常駐。
5. 查詢邏輯必須可配置，因為 Codex CLI 的 usage/status 輸出格式可能會隨版本變動。
6. 應用視窗可以固定在畫面上方，也可以作為一般桌面浮窗。
7. 應用視窗可以任意拖曳，讓使用者放在螢幕任意位置。

---

## 技術選型

使用：

- Desktop shell：Tauri v2
- Backend：Rust
- Frontend：TypeScript + React + Vite
- UI：Tailwind CSS 或 CSS Modules
- Local storage：Tauri Store plugin 或 SQLite
- CLI 執行：Rust `std::process::Command` 或 Tauri shell plugin
- 系統匣：Tauri tray
- 視窗控制：Tauri Window API
- 自動更新：Tauri updater plugin，可後續加入

不使用 Electron，除非有明確需求。此專案應優先保持低記憶體占用、快速啟動與背景常駐穩定性。

---

## 架構原則

採用三層設計：

1. UI Layer  
   負責顯示目前用量、剩餘量、重置時間、錯誤狀態、圖釘狀態、拖曳區域與設定頁。

2. Application Layer  
   負責排程、快取、狀態管理、事件通知、視窗狀態管理與資料正規化。

3. Codex CLI Adapter Layer  
   負責呼叫本機 CLI、解析輸出、處理版本差異與錯誤。

CLI 查詢邏輯不可散落在 UI 元件中，必須集中在 adapter。

視窗控制邏輯不可散落在任意 UI 元件中，必須集中在 window controller 或 dedicated hook。

---

## 主要功能

### 1. 即時用量顯示

主畫面應顯示：

- 目前 Codex usage 狀態
- 剩餘百分比
- 已使用百分比
- 可用額度或剩餘 token，如果 CLI 提供
- reset 時間，如果 CLI 提供
- 最後更新時間
- CLI 查詢狀態：成功、執行中、失敗、未登入、CLI 不存在
- 圖釘狀態：已置頂 / 未置頂

系統匣 tooltip 應顯示簡短資訊，例如：

```txt
Codex: 72% remaining
Updated: 14:32
```

---

### 2. 圖釘置頂模式

應用必須提供一個圖釘 icon 按鈕。

行為規則：

1. 當使用者按下圖釘 icon，使其進入 pinned 狀態時：
   - 視窗必須顯示在所有一般視窗上方。
   - 視窗不可被其他一般軟體覆蓋。
   - app 應使用 Tauri window always-on-top 能力實作。
   - UI 應明確顯示目前處於 pinned 狀態。

2. 當使用者再次按下圖釘 icon，使其離開 pinned 狀態時：
   - 視窗不再強制置頂。
   - 視窗作為一般桌面浮窗存在。
   - 其他軟體可以覆蓋此視窗。
   - UI 應明確顯示目前處於 unpinned 狀態。

3. pinned 狀態必須持久化。
   - app 重啟後應恢復上次的 pinned / unpinned 狀態。
   - 若平台不支援完整 always-on-top 行為，必須顯示降級提示。

建議狀態模型：

```ts
export type WindowPinState = {
  isPinned: boolean;
  updatedAt: string;
};
```

建議 UI 行為：

- pinned：圖釘 icon 顯示 active 狀態
- unpinned：圖釘 icon 顯示 inactive 狀態
- tooltip：
  - pinned：`Always on top enabled`
  - unpinned：`Always on top disabled`

---

### 3. 任意拖曳

應用視窗必須可以任意拖曳。

行為規則：

1. 使用者可以拖曳應用視窗到螢幕任意位置。
2. 拖曳時不應影響用量查詢、polling 或 CLI 執行。
3. 拖曳區域應明確定義，避免按鈕、設定欄位、輸入框與互動元件誤觸拖曳。
4. 若使用無邊框視窗，必須提供自訂 drag region。
5. 視窗位置必須持久化。
6. app 重啟後應恢復上次位置。
7. 若螢幕配置變更導致上次位置超出可視範圍，app 應自動把視窗移回目前可見區域。

建議拖曳區域：

- header bar
- usage card 的非互動背景區
- app 外框空白處

不可作為拖曳區域：

- refresh button
- pin button
- settings button
- text input
- select
- checkbox
- slider
- raw output panel
- error details copy button

建議狀態模型：

```ts
export type WindowPlacementState = {
  x: number;
  y: number;
  width: number;
  height: number;
  displayId?: string;
  updatedAt: string;
};
```

---

### 4. CLI 查詢

CLI 查詢必須透過可配置設定執行。

預設設定範例：

```json
{
  "codexCommand": "codex",
  "usageArgs": ["status"],
  "pollIntervalSeconds": 60,
  "timeoutSeconds": 10
}
```

不要假設所有 Codex CLI 版本都有固定格式。實作時應支援多種 parser：

- JSON parser：如果 CLI 支援 JSON 輸出
- text parser：解析一般文字輸出
- custom regex parser：讓使用者自行設定 pattern

建議資料模型：

```ts
export type CodexUsageSnapshot = {
  source: "codex-cli";
  fetchedAt: string;
  rawOutput?: string;

  usagePercent?: number;
  remainingPercent?: number;

  usedTokens?: number;
  remainingTokens?: number;
  totalTokens?: number;

  windowResetAt?: string;
  weeklyResetAt?: string;

  accountPlan?: string;
  model?: string;

  status:
    | "ok"
    | "unknown"
    | "cli_not_found"
    | "not_authenticated"
    | "timeout"
    | "parse_error"
    | "command_error";

  errorMessage?: string;
};
```

---

## Polling 策略

不可用高頻率輪詢。

預設：

- foreground：每 60 秒查詢一次
- background / tray-only：每 120 秒查詢一次
- command error：使用 exponential backoff，最高 10 分鐘
- 使用者手動刷新：立即查詢，但需 debounce，避免連點觸發大量 CLI 呼叫

查詢規則：

1. 同一時間只能有一個 CLI 查詢進行。
2. 若前一次查詢尚未完成，不可啟動新的查詢。
3. 查詢超時必須 kill process。
4. 查詢結果需寫入快取，避免 UI 空白。
5. app 啟動時先顯示上次快取，再背景刷新。
6. 視窗拖曳與 pinned 狀態切換不得觸發額外 CLI 查詢，除非使用者明確按下 refresh。

---

## 安全要求

### CLI 執行安全

禁止把使用者輸入直接拼接成 shell command。

錯誤範例：

```ts
exec(`codex ${userInput}`);
```

正確做法：

```rust
Command::new(command)
  .args(args)
  .output()
```

command 與 args 必須分開處理。

### 環境變數

不可記錄敏感資訊：

- API key
- session token
- cookies
- auth headers
- access token
- refresh token

log 中如出現上述資訊，必須遮蔽。

### Raw output

raw CLI output 只能在 debug mode 或使用者明確開啟時顯示。預設不在 UI 顯示完整 raw output。

---

## 視窗要求

### 視窗模式

app 應支援以下視窗模式：

1. 一般桌面浮窗
   - 預設不置頂。
   - 可被其他應用程式覆蓋。
   - 可拖曳。
   - 可記住位置。

2. 置頂浮窗
   - 使用者按下圖釘 icon 後啟用。
   - 視窗保持在其他一般視窗上方。
   - 可拖曳。
   - 可記住位置。
   - app 重啟後恢復置頂狀態。

### 視窗外觀

建議使用 compact widget 風格：

- 小尺寸
- 低干擾
- 適合放在桌面角落
- 支援透明或半透明背景，但不得犧牲可讀性
- 可選擇無邊框視窗，但必須保留明確拖曳區域

### 視窗控制 API

Tauri backend 或 frontend 應提供以下能力：

```ts
export interface AppWindowController {
  setAlwaysOnTop(enabled: boolean): Promise<void>;
  getAlwaysOnTop(): Promise<boolean>;
  startDragging(): Promise<void>;
  savePlacement(): Promise<void>;
  restorePlacement(): Promise<void>;
  ensureVisibleOnCurrentDisplay(): Promise<void>;
}
```

實作時不得把視窗控制與 Codex CLI 查詢耦合在一起。

---

## UI 要求

主畫面包含：

1. Usage card
2. Progress bar
3. Last updated timestamp
4. Refresh button
5. Status indicator
6. Pin button
7. Settings shortcut
8. Drag region

狀態顯示規則：

- `ok`：顯示正常用量資訊
- `unknown`：顯示「尚無可用資料」
- `cli_not_found`：提示使用者設定 Codex CLI 路徑
- `not_authenticated`：提示使用者先在本機登入 Codex CLI
- `timeout`：提示 CLI 查詢逾時
- `parse_error`：提示目前 CLI 輸出格式無法解析
- `command_error`：顯示簡短錯誤，不顯示敏感 raw output

Pin button 狀態顯示規則：

- pinned：
  - icon 顯示 active
  - tooltip 顯示 `Always on top enabled`
  - 可選擇顯示小型 `Pinned` badge

- unpinned：
  - icon 顯示 inactive
  - tooltip 顯示 `Always on top disabled`
  - 不顯示或弱化 badge

拖曳 UX 規則：

- header bar 可拖曳
- 使用者游標移到可拖曳區域時可顯示 move cursor
- 互動元件不得被 drag region 覆蓋
- 拖曳時不應觸發 click action
- 拖曳後應 debounce 儲存視窗位置

---

## Settings 頁面

Settings 必須提供：

- Codex CLI command path
- CLI arguments
- Poll interval
- Timeout seconds
- Parser mode：JSON / Text / Regex
- Regex pattern
- 是否開機自動啟動
- 是否只顯示於系統匣
- 是否預設開啟 pinned / always-on-top
- 是否記住視窗位置
- 是否重啟後恢復上次視窗位置
- 是否顯示 raw CLI output
- 手動測試 CLI command

測試 CLI command 時需顯示：

- exit code
- stdout 摘要
- stderr 摘要
- parser 結果
- 錯誤原因

---

## Parser 設計

parser 必須是可替換模組。

建議介面：

```ts
export interface UsageParser {
  parse(input: string): CodexUsageSnapshot;
}
```

若 backend parser 以 Rust 實作，則應提供等價 trait：

```rust
pub trait UsageParser {
    fn parse(&self, input: &str) -> CodexUsageSnapshot;
}
```

parser 不應 panic。任何解析失敗都要回傳 `parse_error`。

---

## 錯誤處理

必須明確區分以下錯誤：

1. CLI binary 不存在
2. CLI exit code 非 0
3. CLI 執行逾時
4. CLI 尚未登入
5. CLI 輸出為空
6. CLI 輸出格式無法解析
7. 權限不足
8. 設定檔損壞
9. 視窗置頂 API 失敗
10. 視窗位置恢復失敗
11. 視窗位置超出目前螢幕可視範圍

所有錯誤都必須轉換成穩定的 app-level error type。

視窗控制錯誤不得影響 Codex usage 查詢；Codex usage 查詢錯誤也不得影響視窗拖曳或 pinned 狀態切換。

---

## 測試要求

至少包含以下測試：

### Unit tests

- JSON parser 測試
- text parser 測試
- regex parser 測試
- malformed output 測試
- empty output 測試
- timeout handling 測試
- command not found 測試
- pinned state reducer 測試
- window placement validation 測試
- out-of-bounds window placement correction 測試

### Integration tests

- mock Codex CLI binary
- 模擬 stdout
- 模擬 stderr
- 模擬 non-zero exit code
- 模擬慢速 command
- 模擬 pinned / unpinned 狀態切換
- 模擬視窗位置儲存與恢復

不要在測試中依賴真實 Codex CLI 或真實帳號。

---

## Mock CLI

專案應提供 mock CLI，方便本機開發。

範例輸出：

```txt
Codex usage
Model: gpt-5-codex
Usage: 28%
Remaining: 72%
Window resets at: 2026-01-01T18:00:00+08:00
Weekly resets at: 2026-01-04T00:00:00+08:00
```

mock CLI 可放在：

```txt
dev/mock-codex-cli/
```

---

## 建議目錄結構

```txt
.
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/
│   │   │   ├── usage.rs
│   │   │   └── window.rs
│   │   ├── codex/
│   │   │   ├── adapter.rs
│   │   │   ├── parser.rs
│   │   │   ├── types.rs
│   │   │   └── errors.rs
│   │   ├── window/
│   │   │   ├── controller.rs
│   │   │   ├── placement.rs
│   │   │   └── types.rs
│   │   └── settings/
│   │       └── store.rs
│   └── tauri.conf.json
├── src/
│   ├── app/
│   ├── components/
│   │   ├── PinButton.tsx
│   │   └── DragRegion.tsx
│   ├── features/
│   │   ├── usage/
│   │   └── window/
│   ├── lib/
│   ├── types/
│   └── main.tsx
├── dev/
│   └── mock-codex-cli/
├── tests/
├── package.json
├── pnpm-lock.yaml
└── AGENTS.md
```

---

## 實作順序

### Phase 1：最小可用版本

1. 建立 Tauri + React + TypeScript 專案
2. 實作 CLI command 設定
3. 實作手動刷新
4. 實作 text parser
5. 顯示 usage / remaining percent
6. 實作錯誤狀態
7. 加入 mock CLI

### Phase 2：桌面浮窗與置頂

1. 實作 compact widget 視窗
2. 實作可拖曳區域
3. 實作圖釘 icon button
4. 實作 always-on-top toggle
5. 儲存 pinned 狀態
6. 儲存視窗位置
7. app 重啟後恢復 pinned 狀態與視窗位置
8. 螢幕配置變更時修正超出可視範圍的位置

### Phase 3：背景更新與系統匣

1. 加入 polling
2. 加入快取
3. 加入系統匣
4. 加入 foreground / background 不同 polling interval
5. 加入 manual refresh debounce

### Phase 4：設定與 parser 強化

1. Settings 頁面
2. Regex parser
3. JSON parser
4. CLI 測試工具
5. raw output debug panel
6. 視窗設定選項

### Phase 5：打包與發布

1. macOS build
2. Windows build
3. Linux build
4. code signing
5. auto update
6. release workflow

---

## 非目標

本專案不做以下事項：

- 不爬取 ChatGPT / Codex 網頁
- 不儲存 OpenAI 帳號密碼
- 不攔截網路流量
- 不讀取瀏覽器 cookie
- 不嘗試繞過 Codex usage limit
- 不修改 Codex CLI 內部檔案
- 不依賴未授權 API
- 不以高頻率輪詢方式逼近即時狀態
- 不在未經使用者同意時強制置頂
- 不在未經使用者同意時重設視窗位置

---

## 程式碼風格

TypeScript：

- 使用 strict mode
- 避免 `any`
- 使用 discriminated union 表示狀態
- UI state 與 domain model 分離
- window state 與 usage state 分離

Rust：

- 不使用 `unwrap()` 處理可預期錯誤
- 使用明確 error enum
- CLI process 必須設定 timeout
- parser 不可 panic
- logging 不可包含敏感資訊
- window command 必須回傳明確錯誤

---

## Commit 規範

使用 Conventional Commits：

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
docs: document mock cli usage
```

---

## 驗收標準

完成後應符合：

1. app 可在 macOS、Windows、Linux 執行。
2. 使用者可設定 Codex CLI 路徑與查詢參數。
3. app 可定期呼叫本機 CLI 查詢 usage。
4. UI 可顯示剩餘量與最後更新時間。
5. CLI 不存在、未登入、逾時、解析失敗時都有明確提示。
6. app 不會高頻呼叫 CLI。
7. 測試不依賴真實 Codex 帳號。
8. 不儲存或輸出敏感憑證。
9. 系統匣可顯示簡短剩餘量。
10. parser 可因應 Codex CLI 輸出格式變更而替換或設定。
11. 使用者可透過圖釘 icon 切換 always-on-top。
12. pinned 狀態下，視窗會保持在其他一般視窗上方。
13. unpinned 狀態下，視窗可被其他軟體覆蓋，作為一般桌面浮窗。
14. 使用者可以任意拖曳視窗位置。
15. app 可記住並恢復上次視窗位置。
16. app 重啟後可恢復上次 pinned / unpinned 狀態。
17. 視窗位置若超出目前螢幕可視範圍，app 會自動修正到可見區域。
18. 視窗控制錯誤不會中斷 Codex usage 查詢。
```