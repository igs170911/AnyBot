# 測試說明（M4：測試基礎）

這篇說明 `tests/harness` 怎麼用、測試怎麼跑。程式碼本身不寫註解，規定都在這裡。

## 跑測試

- Rust 一律用 `cargo nextest run`（不要用 `cargo test`）。make 目標：`make check` 裡有 `test` 關卡；`make check-fast` 跑 `make test-fast`（`--profile quick`：跳過 PostgreSQL 群組的測試，e2e 是分開的目標）。`make test`、`make test-fast`、`make e2e` 都會先拿跨工作區的測試鎖（見「並行工作」節）。
- `make live-model-check`（`tests/live-model`）：跟真的本機模型比對**請求與回應格式**（一般／串流／工具呼叫），設定只讀 `target/live-model/profile.toml`，只信任 `ca_file` 那張憑證（指紋鎖定的驗證器，不准關掉驗證），金鑰用金鑰型別、不印、**絕不放進命令列參數**（`ps` 會看到）；要打真的模型一律走這個程式，真的要手動 `curl` 時把標頭寫進 `target/` 裡權限 600 的檔案、用 `curl -H @檔案`，用完刪掉。假模型用 `vllm` 格式跟真的比，三種請求都要 identical；差異報告寫在 `target/live-model/format-report.txt`。不在 `make check`、`make check-fast`、CI 裡。
- TypeScript 用 `pnpm vitest run`；端對端用 Playwright（見第二節）。
- 逾時設定在 `.config/nextest.toml`：每個測試 30 秒慢速警戒，第 4 次逾時就終止。PostgreSQL 相關的測試另有 90 秒寬限，並在同一群組裡序列化執行（平行度 1）。

## 並行工作（多個 agent 同時做不同里程碑）

- **工作區**：每個 agent 在自己的 git worktree 和分支工作（`AGENTS.md` 第七節）。worktree 由監督者建；臨時自我測試用的 worktree 放在 repo 的 `target/` 底下（已 gitignore），跑完用 `git worktree remove` 收掉。
- **共用編譯輸出**：硬碟容量有限，最多兩條線共用一份編譯輸出。做法：在共用的上一層資料夾放 `.cargo/config.toml`，設 `[build] target-dir = "shared-build"`（位置是 config 檔的相對路徑，放在 worktree 的上一層就會被套用）。編譯輸出共用時出現「Blocking waiting for file lock」是在等別人編譯，屬正常；不准刪共用輸出、不准對它 `cargo clean`。
- **測試暫存照舊在工作區自己的 `target/`**（`target/tests/`、`target/e2e-screens/`、`target/live-model/`、`target/pg-install`…），不放進共用的編譯輸出：兩個工作區的同名測試會互相蓋掉。
- **找編譯輸出的地方一律問 `cargo metadata --format-version 1 --no-deps` 的 `target_directory`**：iron-laws 的「沒被編譯到的檔案」關卡、e2e 啟動 `testenv` 的路徑（`tests/e2e/src/env.ts`）都是這樣；Makefile 裡走 `cargo run`/`cargo build` 的目標由 cargo 自己解析。
- **測試鎖**：`make test`、`make test-fast`、`make e2e` 會先拿一個跨工作區的鎖，鎖檔在 `git rev-parse --git-common-dir` 那個資料夾裡的 `anybot-tests.lock`（所有工作區共用同一個）。macOS 用 `lockf`、Linux 用 `flock`，兩個都沒有（CI 一次只跑一份）就不鎖。拿不到時印一行「waiting for test lock: {鎖檔}」然後等，不放棄。
- **自我測試 A（搬家後的 target-dir）**：在 `target/` 建一個暫時 worktree（`git worktree add --detach target/wt-td main`），把未 commit 的改動複製進去（自我測試要測當前的檔案），在它的上一層 `target/.cargo/config.toml` 寫 `[build] target-dir = "shared-build"`，在 worktree 裡跑 `make check` 要全綠（iron-laws、e2e 都要能找到共用輸出）。跑完 `git worktree remove --force target/wt-td`，刪 `target/.cargo` 與 `target/shared-build`。這不是每次 `make check` 的關卡，改動「找編譯輸出」的程式時手動跑一次。
- **自我測試 B（測試鎖）**：在 `target/` 建兩個暫時 worktree（同上，複製未 commit 的改動），後台先跑 `make test-fast`（A），立刻在第二個跑 `make test-fast`（B）；B 的輸出第一行要出現「waiting for test lock」，而且 B 的完成時間晚於 A，兩個都要全綠。跑完收掉兩個 worktree，鎖檔留在 git common dir（下次照用）。

## 假模型的兩種格式（OpenAI 相容端點）

由建構假模型時選（`model_start_with_format`），或在啟動後用 `format_set` 選；`model_start()` 預設是 `openai`。

| 格式 | 內容 | 什麼時候用 |
|---|---|---|
| `openai`（預設） | 照 OpenAI 官方文件：工具呼叫的 `finish_reason` 是 `tool_calls`；沒有本機模型伺服器特有的欄位（`kv_transfer_params`、`routed_experts`、`weight_versions`、`prompt_logprobs`、`reasoning` 這類都不出現）。 | 產品測試（假模型就是標準 OpenAI 供應商的樣子）。 |
| `vllm` | 照那台本機模型伺服器的樣子：思考內容放在非標準的 `reasoning` 欄位；特有欄位都有（`kv_transfer_params`、`routed_experts`、`weight_versions`、`prompt_logprobs`、`stop_reason`、`token_ids`、`prompt_text`、`prompt_token_ids`、`ec_transfer_params`、`metrics`、`system_fingerprint`）；呼叫工具時 `finish_reason` 是 `stop`。 | 只有 `make live-model-check`：拿它跟真的本機模型比對回應格式，三種請求都要 identical。 |

## harness 提供什麼（`tests/harness/src/`）

| 模組 | 用途 | 注意事項 |
|---|---|---|
| `scratch.rs` | 所有測試檔案都放在 `target/`（repo 內、已 gitignore） | 不准寫到 `target/` 以外 |
| `process.rs` / `launch.rs` / `running.rs` | 啟動程式、等輸出、用 PID 收掉自己啟動的程式 | 每個等待都有期限；逾時就收 PID 並回錯誤 |
| `ports.rs` | 保留空閒 port，占用時排除 | 不准在正式程式加 `SO_REUSEPORT` 之类 |
| `netctl.rs` / `netproxy.rs` / `origin.rs` | 網路控制：allow / cut / slow / down / buffer / idle-cut / source-addr；所有測試流量走代理 | 自我測試在 `tests/netctl_real.rs`：真的連線穿過代理驗每種行為（延遲、頻寬可量測），不准直接連外 |
| `fake_clock.rs` | 假時鐘：實現在 `anybot_foundation::clock::Clock`；`advance()` 依到期順序喚醒 | 正式程式只准透過 `Clock` 介面等時間；`tokio::time::sleep/timeout` 在產品程式被 clippy 禁用，foundation 與 harness 豁免 |
| `model_script.rs` / `model_openai.rs` / `model_vllm.rs` / `model_anthropic.rs` / `model_plan.rs` / `fake_model.rs` | 假模型伺服器：照腳本回 responses（串流、JSON、tool、中斷、卡死、無回應、錯誤狀態），並記錄收到的請求 | 腳本依序回應；空腳本回錯誤；OpenAI 相容端點有兩種格式（見下面「假模型的兩種格式」） |
| `http.rs` / `realtime.rs` | 測試用的 HTTP/SSE/WebSocket helper；`timeout_real` 给非同步等待包上真時間期限 | 等待外部的東西一律要有期限 |
| `postgres.rs` | 測試用 PostgreSQL（K90：不用 Docker）：`pg_start` 把程式下載到 `target/pg-install`（共用快取），資料夾在 `target/`，`unix_socket_directories`（`socket_dir`）設在 `target/` 裡那個資料庫自己的資料夾（macOS socket 路徑上限約 104 字元，base dir 要短），帳密為套件內建的 superuser；`pg_create_database` / `pg_drop_database` 走 admin pool；`pg_migrations_dir()` 指向 `tests/harness/fixtures/migrations/server` | 每個測試用獨立 base dir 與獨立 database；stop 會釋放 port 並清掉資料夾；不准讓 socket 檔寫到 `/tmp` |
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
  - `phone-max-font`：WebKit + `iPhone SE (3rd gen)` 裝置描述（375×667、deviceScaleFactor 2，是最小的現役 iPhone 尺寸）。最大字級模擬方式：iOS 最大輔助字級是內文 53pt、預設 17pt，53/17 ≈ 3.1；Playwright 沒有字級縮放選項，測試在頁面注入 `html { font-size: 310%; }`（專案自訂屬性 `maxTextZoom: "3.1"`），比例與 iOS Dynamic Type 最大值一致。
  - 每個情境每個測試都留截圖（`screenshot: "on"`），路徑在 `target/e2e-screens/<專案>--<測試標題>.png`（`src/act.ts` 的 `registerScenarioScreenshots` 用 `test.afterEach` 存檔，不只失敗時）。
  - `desktop-large`：Chromium 1600×1000。
  - `rtl-arabic`：locale `ar`（RTL 版面規則要等 M8 的 UI，現在只做煙霧測試）。
  - `dark-mode`：colorScheme dark。
  - `lang-en`、`lang-zh-TW`：語言情境。
- 畫面文字一律用訊息代碼查 `locales/*.ftl`（`src/ftl.ts`），動作詞彙對應的輔助函式在 `src/act.ts`（按〔按鈕上的字〕、填欄位、出現在螢幕／不出現、版面量測、console 檢查）。
- 時間快轉連瀏覽器一起轉：控制 API `/control/status` 回 `now_ms`，測試用 `src/act.ts` 的 `installFakeClock`／`syncBrowserClock`（Playwright 的 `page.clock`）把瀏覽器時鐘設成假時鐘的現在時間；自我測試快轉一小時後，瀏覽器 `Date.now()` 必須剛好等於 `/v1/health` 的新 `now`。
- 端對端測試從外面打真程式：煙霧測試打 `vite preview` 的靜態站；自我測試（`selftest.spec.ts`）用控制 API 寫模型腳本、直接打假模型、讀請求記錄、快轉時鐘後驗證 `/v1/health` 的 `now` 位移 3,600,000 毫秒。
- 執行：`make e2e`（先 `cargo build -p testenv`，再跑 Playwright；e2e 前有跨工作區測試鎖，見「並行工作」）；`make check` 含 e2e 閘。CI（`.gitea/workflows/ci.yml`）加裝 `pnpm --filter=@anybot/e2e exec playwright install --with-deps chromium webkit`。
- 輸出（截圖、trace、html 報告）一律在 `target/playwright`、`target/playwright-html`。
