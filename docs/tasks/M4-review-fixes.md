# M4 審核後要修的

M4 的 `make check` 全綠：252 個 Rust 測試，瀏覽器測試也過了。

審核通過的部分：
- 沒被編譯到的檔案擋得住（我塞了一個孤兒檔、一個孤兒測試檔，都被擋）。
- 直接讀系統時間、`tokio::time::sleep` 擋得住。
- 拿掉 `terminate-after` 擋得住。
- 假時鐘、假模型（含思考內容、金鑰標頭只記有沒有帶）、測試用 PostgreSQL（下載有驗雜湊）、假推播都做好了。
- 測試環境程式只聽本機。
- 兩輪的指令都沒碰舊版，也沒撞到沙盒。

下面是沒做完或跟任務不符的，全部修完、`make check` 全綠再 commit 一次。

## 一、沒做的任務項目

1. **「A 段附加：跟真的本機模型比對格式」整段沒做，回報也沒提。**
   - 照 `docs/tasks/M4-test-foundation.md` 那一節做：`make live-model-check`，設定只從 `target/live-model/profile.toml` 讀，只信任 `ca_file`，金鑰用金鑰型別裝、不印。
   - 比對三種請求的**格式**：一般回覆、串流回覆、呼叫工具。
   - `target/live-model/` 裡已經有 `profile.toml`、`ca.pem`、`key`，可以真的跑。
   - 跑完把差異寫進 `target/live-model/` 的報告。如果假模型跟真的不一樣，修假模型。回報附上差異和你怎麼修的。
   - 這個目標不進 `make check`、`make check-fast`、CI。
2. **網路控制的自我測試只測了「模式有沒有切過去」，沒有讓真的連線穿過代理。**
   - 現在的 `netctl_modes_round_trip`、`netctl_global_cut_overrides_port` 只檢查 `net_mode` 的回傳值。
   - 任務要求每一種行為都有自我測試，要用真的客戶端、真的連線去驗：
     - 斷網：已經連上的連線被切斷，新的連線被拒絕；恢復後可以連。
     - 變慢：量得到延遲，頻寬限制也量得到。
     - 伺服器停機：對外連不上；恢復後可以連。
     - 中間的代理會緩衝串流：客戶端要等連線結束才一次收到內容。
     - 閒置 N 秒就切：閒置超過 N 秒後連線被關掉；在 N 秒內有資料就不會。
     - 帶來源位址 X：後面的伺服器看得到 X。
   - 時間用短的（毫秒到一兩秒），讓測試快。
3. **手機情境不對。**
   - 「小 iPhone」要用最小的機型 iPhone SE（第 3 代），寬 375、高 667，用 WebKit。現在是 iPhone 15 Pro（393 寬），不是小尺寸的機型。
   - 「最大字級」要用 iOS 最大的輔助使用字級：內文 53pt，預設 17pt，大約 310%。現在是 170%，那不是最大。
   - 回報寫清楚你怎麼模擬、為什麼是這個比例。
4. **每種情境都要留一張截圖**，不是只有失敗時才留。
   - 放在 `target/e2e-screens/<情境名稱>.png`，回報列出路徑。我要用截圖看畫面。
5. **`make check-fast` 完全不跑測試。**
   - 任務的意思是：`check-fast` 跑 Rust 測試，但跳過要 PostgreSQL 的測試和端對端測試。`make check` 和 CI 全部都跑。
   - 用 nextest 的過濾或分組來分，不要靠測試自己判斷要不要跳過。

## 二、設定錯誤

6. **`.config/nextest.toml` 有 nextest 不認得的設定。**
   - `[[profile.default.serial]]` 不是 nextest 的設定，nextest 每次都印 `ignoring unknown configuration keys: profile.default.serial`，那一段根本沒有作用。
   - 拿掉它。要讓用到同一個埠的測試不同時跑，用 nextest 的 `test-groups` 加 `overrides`。
   - 關卡：nextest 印出「ignoring unknown configuration keys」就要擋（例如在 `make` 裡跑 `cargo nextest show-config test-groups`，看到這句就失敗）。附「塞一個不認得的設定會被擋」的測試。

## 三、規則

7. **你第三次寫 `/tmp`**（這次是 `/tmp/x`），違反 `AGENTS.md` 第一節第 7 條。
   - 從這一輪起，沙盒會直接擋住寫 `/tmp`，寫了會得到 `Operation not permitted`。暫存檔放 `target/`。
   - pnpm 自己的鎖檔不受影響。
8. **測試用的 PostgreSQL 也在寫 `/tmp`。**
   - 它把連線用的 socket 檔和鎖檔放在 `/tmp/.s.PGSQL.<埠>`。我在擋住 `/tmp` 的沙盒裡跑 `make check`，PostgreSQL 就起不來。
   - 測試一樣要遵守「暫存檔只放 `target/`」：把 socket 目錄（`unix_socket_directories`）設到 `target/` 裡那個資料庫自己的資料夾。注意 macOS 的 socket 路徑上限大約 104 個字元。
   - 附自我測試：啟動後 socket 檔在 `target/` 裡，`/tmp` 沒有這次的 `.s.PGSQL.<埠>`。
   - 這一輪沙盒還暫時放行 `/tmp/.s.PGSQL.*`，你修好後我會收緊。

## 你上次問的問題

1. 控制 API 的路徑用 `/control/*` 可以，寫進 `docs/testing.md`。
2. 推播訂閱金鑰的產生，等 M8 接 PWA 訂閱、真的需要時再抽成 harness 的公開函式，這次不動。

## 回報

照 `AGENTS.md` 第六節，逐項對照實際檔案。另外附上：
- 假模型照的是 OpenAI、Anthropic 哪一版（或哪一天）的官方文件。上一輪只寫「照公開文件」。
- `make live-model-check` 的實際輸出和差異報告的路徑。
- 每種情境的截圖路徑。
