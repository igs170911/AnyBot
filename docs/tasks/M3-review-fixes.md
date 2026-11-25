# M3 審查後要修的地方

審查的人重跑了 `make check`（全綠，91 個測試）。另外在一個複本裡做了這些事：

- 手改 `api/openapi.yaml` → 被 `api-docs` 擋下。
- 把 `/v1/health` 改名 → 被 `api-compat` 擋下。
- **在 `api/openapi.yaml` 加一個 POST：沒有例子、沒有 4xx、沒有 5xx、沒有 `Idempotency-Key` → redocly 說「valid」，沒擋下。**

另外讀程式碼找到幾個真的錯誤。先讀 `spec/09` 新加的 K84–K87，再照下面修。

## 必修：程式錯誤

1. **伺服器只在第一個位址有服務。**
   - `apps/server/src/main.rs` 用 `for` 迴圈一個接一個 `axum::serve(...).await`。
   - 第一個會一直跑下去，所以 `[::1]` 綁好了，卻永遠不會接連線。
   - 但啟動那一行寫「聽 127.0.0.1:N、[::1]:N」。
   - 改成同時服務全部位址，任何一個停掉就結束碼 1，並印出原因。
   - 測試：在兩個隨機埠（`127.0.0.1:0`、`[::1]:0`）啟動，兩個都要回 `/v1/health`。
2. **`--help`、`--version` 印到 stderr。** 要印到 stdout，結束碼 0。
3. **測試會改 repo 裡被追蹤的檔案。**
   - 現在的測試會 `chmod` `tests/fixtures/bootstrap/` 裡的檔案。
   - 改成每個測試在 `target/` 裡建自己的資料夾，在那裡寫啟動檔、設權限。
   - 刪掉那兩個 fixture。
4. **啟動檔的連線位址是手寫解析的。**
   - 密碼裡有 `@`、`/`、`%` 就會錯；`?sslmode=require` 會被當成資料庫名稱的一部分；IPv6 主機 `[::1]` 也不支援。
   - 改用標準的 URL 解析套件（例如 `url`）。
   - 要支援：`postgres://` 和 `postgresql://`；帳號和密碼的百分比編碼；IPv6 主機；查詢參數保留下來，不算進資料庫名稱。
   - 每種情況都要有測試，也要測錯誤訊息裡不會出現密碼。
   - 檔案只開一次，權限從開著的檔案讀（不要先查權限、再另外打開來讀）。
5. **`/v1/health` 的時間格式化失敗會靜靜變成空字串**（`unwrap_or_default`）。改成回 500 problem+json。

## 必修：跟規格不一致

6. **任務狀態要照 `spec/09` K32、K57。**
   - 5 種主狀態：進行中、等你、卡住、完成、失敗。
   - 「完成」底下有細項：已交付、你已確認、已驗收、未驗證、已停止、已取消。
   - 現在的 `Queued`、`Paused`、`Cancelled` 是規格沒有的，拿掉。
   - 覺得需要別的狀態，寫進問題問，不要自己加。
7. **事件的外框要照 `spec/10-behavior/6-…` 的 `agent-ext-protocol` S2。**
   - 每個事件都帶：協定版本、這一輪的 ID、產生它的那一端給的全域唯一 ID。這一輪的 ID 放在外框，不是只放在某些種類裡。
   - 遞增編號是**伺服器收到時**給的，所以要分成兩個型別：
     - 「送上來的事件」：沒有編號。
     - 「伺服器記下的紀錄」：有編號。
   - 規格沒說每個事件都要帶 `task_id`，從外框拿掉。「任務狀態變更」這個種類自己帶它是哪個任務。
   - S6 後半：收到不認得的事件種類，解析不能整批失敗，要能把原始內容留下來（記下來、不顯示）。寫單元測試。
8. **協定版本的判斷要照 `spec/09` K84。**
   - 用一張表列出每個版本和它的發佈日期。
   - 規則：
     - 最新版一律接受。
     - 被取代的舊版：從新版發佈那天（UTC）起，加 6 個日曆月之前都接受，之後拒絕。日期不存在時取那個月的最後一天，例如 8/31 加 6 個月是 2/28 或 2/29。
   - 用 `Clock` 判斷。
   - 單元測試要包含邊界：差一天之前接受、差一天之後拒絕、剛好那天。
   - 現在的 `!=` 判斷會讓舊裝置在新版發佈那一刻全部斷線。
9. **測試名稱帶規格編號的條件**（`spec/09` K86，`AGENTS.md` §二第 4 條已更新）。
   - `t_agent_ext_protocol_5` 只測了一半：「Bot 設定出現訊息」要等 Bot 設定做出來。
   - `t_sync_id_1` 是縮小版。
   - 這兩個都改成描述性的名字。審查的人已經把 `T-agent-ext-protocol-5` 放回基準檔了。
   - spec-trace 加一道檢查：基準檔裡的編號如果已經有同名測試，就擋。要附單元測試。
10. **規格測試要從外面打真的程式**（`spec/09` K86、K87；`spec/07` §3 新加的兩列）。
    - 新增根目錄 `tests/harness`（測試輔助套件的第一版，M4 會繼續擴充）。它要能：
      - 跑一個執行檔，帶參數。
      - 收 stdout、stderr、結束碼。
      - 用 PID 收掉自己啟動的程式，**不准用名字殺程式**。
      - 在 `target/` 裡給每個測試一個自己的資料夾。
    - `T-install-n22-1..4`、`T-server-n14-2` 改成跑真的 `anybot-server` 執行檔。建議放在 `apps/server/tests/`，用 `env!("CARGO_BIN_EXE_…")` 找執行檔，跑法交給 harness。
    - 原本呼叫內部函式的版本保留成單元測試，改用描述性的名字。
    - 依賴檢查要擋兩件事：
      - 產品套件在正式依賴（不是開發用依賴）裡用到 harness。
      - harness 依賴產品套件。
    - harness 的豁免寫在它自己的 `clippy.toml`，iron-laws 的表格也要更新。
    - 測試不能因為 8787 被別的程式占用就失敗或時好時壞。要用到 8787 的測試，用 nextest 的 test-group 排成一個一個跑。

## 必修：關卡的漏洞

11. **API 文件的 lint 要真的擋**（第 3 項的要求，加上 `spec/05` §2「每個寫入 API 都帶 Idempotency-Key」）。
    - `redocly.yaml` 先 `extends: [recommended-strict]`，再加以下規則，全部設成 error：
      - 每個 API 都有標籤（非空）。
      - 至少一個 4xx、至少一個 5xx。
      - 錯誤回應用 problem+json。
      - 每個請求內容和回應內容都有例子。
      - POST／PUT／PATCH／DELETE 都要有 `Idempotency-Key` 標頭參數。
    - AsyncAPI 那份也開 redocly 支援的全部規則，設成 error。
    - **自我測試**：
      - 在 `tests/fixtures/api-lint/` 放幾份壞的 yaml，每份只缺一樣。
      - 加一個關卡，確認每份都被 lint 擋下（沒擋下就失敗），並接進 `make check`。
    - iron-laws 要擋兩件事：
      - 出現 `.redocly.lint-ignore.yaml`（redocly 的放行檔，等於關掉檢查）。
      - redocly 設定裡有 `off` 或 `warn`。
12. **`api-compat` 要跟 `$(SPEC_TRACE_REF)` 比。**
    - 任務是這樣寫的，但 Makefile 寫死成 `HEAD`，回報也沒寫到這個偏離。
    - CI 要裝 oasdiff。現在 CI 沒裝，在 CI 上一定會失敗。
13. **`make check-fast` 要包含 `api-client` 和 `api-html`。** 它們也是「不一致就擋」的關卡。
14. **把 noyalib 換成 `yaml_serde`。**
    - noyalib 是 0.0.x 版，半年發了 64 版，只有一個作者；這個作者上一個 YAML 套件（serde_yml）已經被 RustSec 標成不安全、沒人維護。
    - `yaml_serde` 是 YAML 官方組織維護的 serde_yaml 後繼版本，utoipa 6 的 `yaml` 功能自己也用它。能直接用 utoipa 的 `yaml` 功能就用。
    - `deny.toml` 的 `[bans]` 加上禁用 `noyalib`、`serde_yml`、`serde_yaml`，各寫原因。
15. **禁用清單有漏洞**（`clippy.toml`，每一條都要附「塞進去會被擋」的測試）。要補的分四類：
    - **讀環境變數**：`std::env::home_dir`（只有 foundation 豁免，`spec/07` §3）、`std::env::temp_dir`。
    - **執行外部程式**：
      - `tokio::process::Command` 的 `new`、`spawn`、`output`、`status`。
      - `std::os::unix::process::CommandExt::exec`。
    - **發 HTTP**：`reqwest::get`、`reqwest::blocking` 的 `Client::new` 和 `get`。
    - **讀寫檔案**：
      - `std::fs` 的 `copy`、`create_dir`、`remove_dir`、`remove_dir_all`、`read_dir`、`set_permissions`、`hard_link`，以及 `DirBuilder`。
      - `tokio::fs` 的全部函式。

    另外依賴檢查要用套件設定擋：
    - tokio 的 `process` 功能只准 `computers` 和 `tests/harness`。
    - tokio 的 `fs` 功能只准 `computers`、`blob`、`store-client`、`foundation`、`tests/harness`。
    - HTTP 客戶端套件（`reqwest`、`ureq`、`isahc`、`attohttpc`、`curl`）只准 `spec/07` §3 列的那幾個套件，加上 `tests/harness`。

## 必修：其他

16. **本機資料夾的預設位置照 `spec/09` K85。**
    - 位置是作業系統的 App 資料夾，加上桌面 App 的識別碼。現在的「AnyBotNext」是 repo 的名字，不是規格定的。
    - 識別碼在 Rust 裡只定義一次。
    - 加一個檢查，確認它跟 `apps/desktop/src-tauri/tauri.conf.json` 的 `identifier` 一致。
17. **整合測試的位置（你的第 5 題）。**
    - 單元測試放在程式旁邊的 `*_tests.rs`。
    - 會啟動伺服器、打 HTTP、跑執行檔的，放在套件的 `tests/` 資料夾，或根目錄 `tests/` 底下。
    - `crates/server/src/health_tests.rs` 裡打 HTTP 的那個搬到 `crates/server/tests/`。
18. **小地方**：「伺服器活着」改成「伺服器活著」（繁體）。

## 你的問題

1. 子指令還沒實作時用「結束碼 2＋這個子指令還沒有實作：…」，**可以**，做到那個功能時換掉。
2. **對**，不實作 `--port`。規格就是要 `--port 1` 被當成不認得的旗標。
3. **對**。M3 先固定 8787；M6 起，要聽的位址從資料庫的伺服器設定讀（`server-n14` S3）。
4. **對**，`T-conn-*` 在 M4。
5. 見上面第 17 項。

## 不要動

- `Cargo.toml` 的 `license` 欄位：在等使用者決定，這次不要改。
- `tools/spec-trace/baseline.txt`：除了「有測試的編號要拿掉」，不要動。`T-agent-ext-protocol-5` 是審查的人放回去的。

修完 `make check` 全綠再 commit 一次（不准改寫歷史），照 `AGENTS.md` 第六節回報。回報的每一項都要跟實際檔案一致，偏離任務的地方（例如這次的 `HEAD`）都要寫出來。
