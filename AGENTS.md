# AnyBotNext · 給寫程式的 agent 的規則（每次開始前先讀完）

這是 AnyBot 新版，用 clean room 的方式重寫。`spec/` 是唯一的需求來源。
開始任何工作前先讀：`spec/README.md`、`spec/04-product-core.md`、`spec/05-engineering-principles.md`、`spec/07-repo-layout.md`、`spec/10-behavior/README.md`，再讀這次任務指定的檔案。

## 一、絕對規則（違反就停下來回報，不要自己繞過）

1. **只在這個資料夾裡工作。** 不要讀、不要搜尋、不要列出這個資料夾以外的任何檔案。特別是 `~/Documents/AnyBotRebuild` 和 `~/Documents/AnyBot`：那是舊版，是污染源，一個字都不要看。也不要上網找舊版 AnyBot 的原始碼。
2. **規格沒寫的行為，不要猜。** 不要憑印象補「舊版大概怎麼做」。寫進回報的「要問的問題」。
3. **`spec/09-decisions.md` 是最終決定。** 跟其他文件不一致時，以它為準。
4. **不 push、不建遠端 repo。** 只在本地 commit；每個里程碑做完、`make check` 全綠後 commit 一次，訊息用繁體中文寫做了什麼。
5. **不動這個資料夾以外的東西。** 可以安裝的開發工具只有：`rustup component add rustfmt clippy`；`cargo install --locked` 這幾個：`cargo-deny`、`cargo-machete`、`cargo-nextest`、`sqlx-cli`、`tauri-cli`；`npm install -g pnpm`。要裝別的先停下來回報。
6. **不放任何金鑰、帳號、個人資料進 repo。** 新版也不讀環境變數做設定（`spec/02` A10）。
7. **暫存檔、紀錄檔一律放在 repo 的 `target/` 裡**（已經在 `.gitignore`），不要寫到 `/tmp` 或其他地方。

## 二、開發原則（`spec/05` 摘要）

- **能用工具擋的就用工具擋，不靠感覺。**
- **「完成」＝`make check` 全綠，而且這次相關的規格測試都通過。** 沒到這個狀態，不要回報「完成」。
- **邊寫邊跑 `make check-fast`**（有了之後）：每改完一批就跑，錯了當場修，不要累積到最後。
- **依賴方向**照 `spec/07` §3，由 `tools/` 裡的檢查工具擋。
- **禁用的寫法**照 `spec/07` §3，由 clippy 的禁用清單擋：讀環境變數、執行外部程式、發 HTTP、讀寫任意檔案，只准在指定的套件。
- 正式程式碼裡不准 `unwrap`、`expect`、`panic!`；測試裡可以。
- **測試要標出它驗的是哪個規格測試**，例如在測試上一行寫 `// spec: T-chat-steer-1`。追蹤表檢查會比對 `spec/10-behavior/` 裡的編號。
- 金鑰一律用專用型別：不能印、不能直接序列化。

## 三、技術選型（`spec/06`）

Rust stable、edition 2024；tokio、axum、sqlx、utoipa、reqwest（rustls）、thiserror／anyhow、tracing。TypeScript strict、React＋Vite、Biome、Vitest、Playwright；pnpm workspace；桌面 Tauri 2。伺服器資料庫 PostgreSQL，客戶端 SQLite。

## 四、回報格式（每個里程碑結束都要寫）

用繁體中文：
1. 做了什麼（照任務的項目逐條）。
2. `make check` 最後的輸出（貼最後 30 行）。
3. 偏離規格的地方，和原因。
4. 要問的問題。
