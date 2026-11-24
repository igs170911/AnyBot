# 里程碑 M2：TypeScript 畫面骨架、桌面殼，和三道新關卡

延續 M1 的「先打底」：**這個里程碑也不寫任何功能邏輯**，只把骨架和關卡補齊。先讀 `spec/06` §2.4（畫面、Tauri 的評估）、`spec/07` §2、§3（§3 剛補了 `apps/desktop` 的依賴規則）。

## 要做的

1. **pnpm 工作區**
   - 沒有 pnpm 就裝：`npm install -g pnpm`。
   - 根目錄：`package.json`、`pnpm-workspace.yaml`、`biome.json`、`tsconfig.base.json`（strict 全開）。
   - 鎖檔 `pnpm-lock.yaml` 要 commit。

2. **四個 TypeScript 套件**（都只是骨架）
   - `web/ui`：共用的畫面元件庫（React）。只放一個空的外殼元件，不做任何頁面或功能。
   - `web/api-client`：之後由 `api/openapi.yaml` 產生的客戶端。現在只放套件骨架，README 寫明產生的來源和指令；`api/openapi.yaml` 會在之後的里程碑由 protocol 產生。
   - `apps/web`：手機和瀏覽器用的 PWA（Vite＋React），畫面只顯示 `web/ui` 的外殼。
   - `apps/desktop`：Tauri 2。`src-tauri/` 是 Rust 套件，加進 Cargo workspace；前端載入 `web/ui` 的外殼。依賴規則照 `spec/07` §3：**桌面殼不准依賴 server、store-server、runner、models、tools、computers、mcp、external-agents**（視窗只是畫面）。可以用 `cargo install --locked tauri-cli`。

3. **把 TypeScript 接進 `make check`**，依序加在 M1 的關卡後面：
   1. `pnpm install --frozen-lockfile`
   2. `pnpm biome ci`（格式＋lint）
   3. 各套件的型別檢查（`tsc --noEmit`）
   4. 各套件的單元測試（`vitest run`）
   5. 各套件的建置（`vite build`）

   Tauri 的 Rust 部分走 M1 原本的 clippy、測試、deny、machete、depcheck。

4. **spec-trace 也掃 TypeScript**：`.ts`、`.tsx` 裡註解的 `spec: T-…` 也算；範圍一樣只限產品程式碼。

5. **基準檔只准變短**（回答你上次的問題 1）
   - spec-trace 加一個選項 `--against <git ref>`：把現在的基準檔跟那個 ref 的基準檔比，**多出來的編號只准是規格在這段變更裡新增的**；其他多出來的一律擋。
   - `make check` 在本機跟 `HEAD` 比；CI 跟主分支（main）的分岔點比。
   - 要有單元測試，包含「刪掉一個測試、把編號塞回基準檔」會被擋的情況。

6. **`make check-fast`**：只跑 cargo fmt 檢查、cargo clippy、depcheck、biome。要快，給寫程式時邊寫邊跑。

7. **CI**：`.gitea/workflows/ci.yml` 跟著更新。Linux 上的 Tauri 需要 WebKitGTK 等系統套件，在 CI 裡裝。

8. **README** 補上 TypeScript 和桌面的部分。

全部做完、`make check` 全綠之後，commit 一次。

## 不要做

- 任何畫面上的功能、頁面、路由。
- 任何 API、資料庫、功能邏輯。
- 資源上限的關卡（桌面閒置 CPU、記憶體）：等桌面真的有東西再做。

## 回報

照 `AGENTS.md` 第四節的格式。
