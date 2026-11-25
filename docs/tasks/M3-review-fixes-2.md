# M3 第二輪審查要修的地方

審查的人重跑了 `make check`（全綠，127 個測試），並在複本裡塞了這些違規：

| 塞了什麼 | 結果 |
|---|---|
| 缺例子、4xx、5xx、`Idempotency-Key` 的 POST | 擋下 |
| `home_dir`、`temp_dir`、`read_dir` | 擋下 |
| 伺服器的 tokio 開 `process`、`fs` | 擋下 |
| 正式依賴用 harness | 擋下 |
| 縮排的 `t_agent_ext_protocol_5` | 擋下 |
| `.redocly.lint-ignore.yaml` | **沒擋** |
| redocly 設定裡有 `warn` | **沒擋** |

另外實際跑了執行檔，找到下面這些。先讀 `spec/09` 新加的 K88、K89。

## 必修：程式錯誤

1. **拿掉 `SO_REUSEPORT`。**
   - 審查的人同時開了兩個 `anybot-server`，**兩個都綁在 8787、都在跑**。第二個沒有照 `install-n22` S3 說「無法使用連接埠 8787」、結束碼 1。
   - 這也讓同一台電腦上的別的程式能搶走這個埠的連線。
   - 你開它是為了讓測試不要時好時壞。**不准為了讓測試過而改正式程式的行為**（`AGENTS.md` §三新加的）。測試不穩就修測試：
     - harness 收程式時等它真的結束。
     - 會用到 8787 的測試排成一個一個跑。
     - 綁埠失敗時重試幾次。
   - 加一個黑箱測試：先開伺服器 A，再開伺服器 B。B 要印「無法使用連接埠 8787」、結束碼 1；A 照常回 `/v1/health`。
2. **連線位址沒寫埠時，啟動失敗。**
   - `postgresql://u:密碼@localhost/資料庫` 會被判成「啟動檔的埠格式錯」。
   - `url` 套件的 `port_or_known_default()` 不認得 PostgreSQL，所以拿不到預設值。
   - 沒寫埠要用 5432。加測試。
3. **協定版本的寬限期算錯。**
   - `cutoff_date` 是手寫的月份加法：6 月 + 6 個月 → `total_month` = 12 → 年份加 1、月份 0 → 變成 12 月。**6 月發佈的版本，寬限期會算到隔年 12 月**，多了一年。
   - 日期計算用函式庫，不要手寫曆法（`AGENTS.md` §三）。例如 `jiff` 的月份加法，本身就會把不存在的日期夾到月底。
   - 版本表的日期用日期型別，不要用三個 `u32`。
   - 測試：發佈月份 1 到 12 月每一個都要測，包含 6 月和 8/31。
   - 不准用 `unwrap_or(0)`、`unwrap_or(UNIX_EPOCH)`、`unwrap_or_default()` 把算不出來的情況悄悄變成某個值。算不出來就回錯誤，或者判成拒絕，並且要有測試。整個 repo 都照這條檢查一遍。

## 必修：回報跟實際檔案不一致

4. **API lint 的自我測試沒有任何關卡在跑。**
   - 回報寫「`tests/fixtures/api-lint/` 自測（base 通過、5 個變體各觸發一條規則）」，但 Makefile、Rust、TypeScript 都沒有地方用到這些檔案。
   - 加 `api-lint-selftest` 接進 `make check`：
     - `base.yaml` 要通過。
     - 每個 `no-*.yaml` 要失敗，而且輸出裡要有**它該觸發的那條規則名稱**。因為別的原因失敗的，不算擋下。
5. **iron-laws 還沒擋 redocly 的放行方式**（上一份任務第 11 項）：
   - 任何地方出現 `.redocly.lint-ignore.yaml`。
   - redocly 設定檔裡有 `off` 或 `warn`。
   - 兩種都要有「塞進去會被擋」的測試。
6. **`redocly-asyncapi.yaml` 一個字都沒改**（上一份任務第 11 項）。開 redocly 對 AsyncAPI 支援的全部規則，設成 error。哪些規則它不支援，寫在回報裡。
7. **回報第 9 項寫「T-sync-id-1 改名為帶編號的測試」**，但程式其實改成了描述性的名字（這是對的）。回報要照實際寫。

## 必修：開源（`spec/09` K88）

8. **授權全文和欄位。**
   - repo 根目錄放 `LICENSE-MIT`、`LICENSE-APACHE`，用標準全文。
   - MIT 的版權行寫「Copyright (c) 2026 The AnyBot Authors」。
   - 每個 `package.json`（含 private 的）都標 `"license": "MIT OR Apache-2.0"`；`tauri.conf.json` 的 bundle 也標。
   - 加一道檢查：所有 `Cargo.toml` 和 `package.json` 的授權欄位都必須是這個值。
9. **擋 TypeScript 正式依賴的授權。**
   - 用 `pnpm licenses list --prod --json` 這類方式，檢查會跟著產品發出去的套件。
   - 准許的授權清單跟 `deny.toml` 的一樣。不在清單上的、認不出來的都擋。
   - 要加新的授權，先停下來回報。
   - 接進 `make check`，附「塞一個不准的授權會被擋」的測試。

## 建議一起做

10. harness 的 HTTP 是直接開 TCP、自己解析回應，遇到 chunked 這類回應會解析錯。harness 可以用 HTTP 客戶端套件（`spec/09` K87），改用現成的。
11. `api/openapi.yaml` 的 contact 網址指到 `/v1/health`，沒有意義。現在還沒有公開網址，只留名稱就好。

## 你的問題

1. **對**。送事件的 HTTP API 和存進資料庫在 M6；其他套件做到它們的里程碑（M5–M7）時再接新的型別。
2. 照 K84，判斷只用到「取代它的那個新版」的發佈日期，所以 v1 自己的日期在 v2 出現之前不影響任何判斷。表格先放 2026-01-01；第一次正式發佈時改成真的日期，這件事會寫進發佈的檢查清單。
3. 規格前後矛盾，已經修正（`spec/09` K89、`spec/07` §3）：
   - 只有 `migrate-old` 可以在遷移時讀舊版用過的環境變數，其他照樣全部禁用。
   - `T-install-n23-3` 在遷移的里程碑做。
   - `T-install-n23-2` 是「實機」，在發佈的里程碑用真的機器跑。
   - 兩個都留在基準檔。

修完 `make check` 全綠再 commit 一次（不准改寫歷史），照 `AGENTS.md` 第六節回報。**回報前逐項對照實際檔案**：這次有三項回報寫了、檔案裡卻沒有。
