# 測試說明（M4：測試基礎）

這篇說明 `tests/harness` 怎麼用、測試怎麼跑。程式碼本身不寫註解，規定都在這裡。

## 跑測試

- Rust 一律用 `cargo nextest run`（不要用 `cargo test`）。.make 目標：`make check` 裡有 `test` 關卡。
- TypeScript 用 `pnpm vitest run`；端對端用 Playwright（見第二節）。
- 逾時設定在 `.config/nextest.toml`：每個測試 30 秒慢速警戒，第 4 次逾時就終止。PostgreSQL 相關的測試另有 90 秒寬限，並在同一群組裡序列化執行（平行度 1）。

## harness 提供什麼（`tests/harness/src/`）

| 模組 | 用途 | 注意事項 |
|---|---|---|
| `scratch.rs` | 所有測試檔案都放在 `target/`（repo 內、已 gitignore） | 不准寫到 `target/` 以外 |
| `process.rs` / `launch.rs` / `running.rs` | 啟動程式、等輸出、用 PID 收掉自己啟動的程式 | 每個等待都有期限；逾時就收 PID 並回錯誤 |
| `ports.rs` | 保留空閒 port，占用時排除 | 不准在正式程式加 `SO_REUSEPORT` 之类 |
| `netctl.rs` / `netproxy.rs` | 網路控制：allow / cut / 代理模式；所有測試流量走代理 | 測試要用代理出口，不准直接連外 |
| `fake_clock.rs` | 假時鐘：實現在 `anybot_foundation::clock::Clock`；`advance()` 依到期順序喚醒 | 正式程式只准透過 `Clock` 介面等時間；`tokio::time::sleep/timeout` 在產品程式被 clippy 禁用，foundation 與 harness 豁免 |
| `model_script.rs` / `model_openai.rs` / `model_anthropic.rs` / `model_plan.rs` / `fake_model.rs` | 假模型伺服器：照腳本回 OpenAI / Anthropic responses（串流、JSON、tool、reasoning、中斷、卡死、無回應、錯誤狀態），並記錄收到的請求 | 腳本依序回應；空腳本回錯誤 |
| `http.rs` / `realtime.rs` | 測試用的 HTTP/SSE/WebSocket helper；`timeout_real` 给非同步等待包上真時間期限 | 等待外部的東西一律要有期限 |
| `postgres.rs` | 測試用 PostgreSQL（K90：不用 Docker）：`pg_start` 把程式下載到 `target/pg-install`（共用快取），資料夾在 `target/`，TCP only，帳密為套件內建的 superuser；`pg_create_database` / `pg_drop_database` 走 admin pool；`pg_migrations_dir()` 指向 `tests/harness/fixtures/migrations/server` | 每個測試用獨立 base dir 與獨立 database；stop 會釋放 port 並清掉資料夾 |
| `fake_push.rs` / `push_crypto.rs` | 假推播（RFC 8030 的推播服務）：訂閱註冊、PUT `/push/{id}`，用測試自己產生的金鑰（RFC 8291 aes128gcm，金鑰在 body 的前 86 字节裡）解開內容，記錄 `subscription_id`、明文、`ttl`、`topic`、`urgency` | 加解密用 `sad-web-push`（純 Rust，不用 OpenSSL），不手寫；自我測試見 `tests/harness/tests/fake_push.rs` |

## 測試命名與分級（`spec/09` K86、`spec/05` 鐵律 4）

- Rust 測試函式 `t_chat_steer_1` 對應 `spec/10-behavior/` 的 `T-chat-steer-1`；TS 測試標題開頭寫 `T-chat-steer-1`。
- 帶編號的名字＝驗完那一列的全部預期；只驗一部分用描述性名字，編號留在基準檔。
- 「整合」「端對端」測試要從外面打真的程式（執行檔、HTTP），不能只呼叫內部函式。
- 測試裡不准有中文；需要中日韓測試資料時用 Unicode 跳脫。

## 時鐘在伺服器測試裡怎么用

- `crates/server` 的整合測試把 `Arc<FakeClock>` 放進 `AppState::with_clock`，從外面打 HTTP（見 `crates/server/tests/health_clock.rs`：假時鐘前進一小時，`/v1/health` 的 `now` 必須剛好位移 3,600,000 毫秒）。
- 伺服器整合測試裡要等真實時間的地方用 `test_harness::sleep_real`，不准直接用 `tokio::time`。

## 端對端（M4 B 節）

- 測試環境程式 `tests/testenv`（執行檔 `testenv`）：啟動假網路代理、假模型、假推播、假時鐘、真正的伺服器（`AppState::with_clock`），再啟動控制 API；啟動完成後在 stdout 印一行 `testenv ready {port 報告}`，測試由 `globalSetup` 啟動它、解析這行、把 port 寫進 `target/scratch/testenv-ports.json`，收工時用 PID 收掉。
- 控制 API（只給測試環境用，不在產品裡）：`/control/status`、`/control/model/script`（POST 寫腳本）、`/control/model/requests`（GET 讀記錄）、`/control/clock/advance`（POST 快轉假時鐘）、`/control/net/cut`、`/control/net/restore`、`/control/push/records`（GET 讀推播記錄）。
- Playwright 設定在 `tests/e2e/playwright.config.ts`，情境專案：
  - `phone-max-font`：WebKit + iPhone 15 Pro 視口（393×852，deviceScaleFactor 3）。最大字級模擬方式：Playwright 沒有字級縮放選項，測試在頁面注入 `html { font-size: 170%; }`，等於 iOS Safari 最大 Dynamic Type 的放大效果，專案用自訂屬性 `maxTextZoom: "1.7"` 標明倍率。
  - `desktop-large`：Chromium 1600×1000。
  - `rtl-arabic`：locale `ar`（RTL 版面規則要等 M8 的 UI，現在只做煙霧測試）。
  - `dark-mode`：colorScheme dark。
  - `lang-en`、`lang-zh-TW`：語言情境。
- 畫面文字一律用訊息代碼查 `locales/*.ftl`（`src/ftl.ts`），動作詞彙對應的輔助函式在 `src/act.ts`（按〔按鈕上的字〕、填欄位、出現在螢幕／不出現、版面量測、console 檢查）。
- 時間快轉連瀏覽器一起轉：控制 API `/control/status` 回 `now_ms`，測試用 `src/act.ts` 的 `installFakeClock`／`syncBrowserClock`（Playwright 的 `page.clock`）把瀏覽器時鐘設成假時鐘的現在時間；自我測試快轉一小時後，瀏覽器 `Date.now()` 必須剛好等於 `/v1/health` 的新 `now`。
- 端對端測試從外面打真程式：煙霧測試打 `vite preview` 的靜態站；自我測試（`selftest.spec.ts`）用控制 API 寫模型腳本、直接打假模型、讀請求記錄、快轉時鐘後驗證 `/v1/health` 的 `now` 位移 3,600,000 毫秒。
- 執行：`make e2e`（先 `cargo build -p testenv`，再跑 Playwright）；`make check` 含 e2e 閘。CI（`.gitea/workflows/ci.yml`）加裝 `pnpm --filter=@anybot/e2e exec playwright install --with-deps chromium webkit`。
- 輸出（截圖、trace、html 報告）一律在 `target/playwright`、`target/playwright-html`。
