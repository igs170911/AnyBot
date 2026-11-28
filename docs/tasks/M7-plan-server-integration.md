# M7 剩下的工作：伺服器整合（M7j，規劃）

這份是規劃，不是程式。範圍：別的四份規劃（`M7-plan-turn-task.md`、`M7-plan-trust-cards.md`、`M7-plan-tools-delivery.md`、`M7-plan-brain-context.md`）和手機畫面那份（`M7-plan-phone-ui.md`）標成「M7j」或交給伺服器做的列，加上它們要伺服器先準備好的入口。

- **不在這份的**：turn 區的 T12–T20（伺服器收一句話、執行端領信、信箱路由、任務通知、卡住偵測）。這份只用它們做好的介面（見 E 段）。
- **列的來源**：`tools/spec-trace/baseline.txt` 裡還沒驗的列（已經有 `fn t_<id>` 的不算）。
- **已經做完、不再列的**：`T-computers-n2-5`、`T-computers-online-1`、`2`、`4`、`5`、`7`、`T-safety-n10-1`、`2`、`3`、`5`、`6`、`T-safety-n11-1`、`T-server-n1-1`、`T-server-n2-1`、`2`、`5`、`T-sync-append-4`。
- **使用者已經定的**（`docs/tasks/M7-handover.md` 第二節）：
  - 各規劃 D 段的問題都照「建議」走。
  - 逐字顯示：執行端打 `POST /v1/streams/typing`，伺服器在現有 SSE 加 `typing` 提示，不存紀錄，M7j 做。
  - 端對端列裡「另一台裝置的動作」可以用照公開 API 收送的替身（預期沒寫那一台自己的畫面時），回報寫明。
  - 形式驗證：在 `tests/` 用 Rust 手寫窮舉，不引進 TLA+ 等工具。
  - `/goal`、`/plan` 放 M9。
- **驗收的層次**：
  - 整合列照 K118 ⑱：每個預期都是資料時才准用替身裝置、替身執行端。
  - 整合列預期裡的「畫面…」照 brain 區 Q-28（已照建議定）：看前端畫那個畫面時用的 API 和 SSE 回的資料。真的要看像素的寫成端對端，歸手機畫面區（M7k）或桌面（M8）。
  - 名稱帶編號＝整列預期都驗到（K86）；只驗一半的用描述性名稱。
- **現在 `dev` 已經有、這份直接用的**（查過程式）：
  - 電腦在線狀態：`GET /v1/computers` 回 `ComputerView { online, offline_at_ms, offline_reason, … }`；上線、離線都寫一筆 `computers_changed` 到帳號那條流（`computers_log.rs`、`presence_sweep.rs`），所以 SSE 已經會送「有新的」。
  - 卡片答案只收一次：`api_referee_card.rs` 的 `ensure_card_open`（`card-not-waiting`、`card-already-answered`）。
  - 永久刪除：`/v1/streams/actions` 的 `purge_permanent`；`/v1/streams/record` 對已永久刪除的回 410 `record-source-purged`。
  - 任務事件 `KnownTaskEventKind::DeviceReport { kind, file }`，`DeviceReportKind { Shown, ThumbsUp, ThumbsDown, Opened }`；`domain::task_replay::replay` 算任務狀態。
  - 裝置不准寫的紀錄種類：`domain::record_rules::DEVICE_FORBIDDEN_KINDS`。
  - 假推播：`tests/harness/src/fake_push.rs`（RFC 8291 解密、記 `ttl`、`topic`、`urgency`），testenv 的 `GET /control/push/records`。
  - `sad-web-push`（`default-features = false`）已經在 workspace 的 `Cargo.toml`，現在只有 harness 用。
  - 來源位址：`req_source.rs`、`domain::source_addr`（`t_safety_n11_1`、`t_server_n1_1` 已驗）。

---

## A) 範圍分類

### A.1 總數

| 分類 | 列數 | 說明 |
|---|---|---|
| M7j（本份切片拿編號） | 88 | J1–J23 |
| M8 | 11 | 預期寫到桌面 App 自己的畫面、輸入框、手機 Service Worker 的行為、`/rewind` 指令、管理頁 |
| M9 | 4 | /goal、交辦子任務、多步驟任務 |
| M10 之後 | 1 | 從舊伺服器遷移 |
| 未排定 | 2 | `/btw` 支線、團隊管理員 |
| 合計 | 106 | |

本份跟別份分類不同的列，另外列在 D 段 Q7，請監督者一起確認。

### A.2 每一列

同一個理由的連號寫在一格；每個編號只出現一次。「來源」寫哪一份規劃交過來的。

| 編號 | 來源 | 分類 | 理由 |
|---|---|---|---|
| T-cards-n1-1、2、3 | phone（排 M8） | M7j J1 | 預期是任務狀態和交付報告，都是資料；👍 由替身手機照公開 API 送 |
| T-conn-batch-2 | M6c 留下 | M7j J2 | 即時通道 |
| T-server-n4-2、3、4 | phone（「整合列歸伺服器」） | M7j J3 | 推播訂閱、送出、內容上限 |
| T-server-n4-1 | 同上 | M10 之後 | 要從舊伺服器遷移 `vapid.pk8` |
| T-server-n4-5 | 同上 | M8 | 管理頁畫面 |
| T-safety-n2-1、6、7 | trust | M7j J4 | 誰收、什麼時候推 |
| T-account-push-1、8 | phone（排 M8） | M7j J4 | 預期是假推播的紀錄和替身電腦收到的卡片紀錄，都是資料 |
| T-account-push-2、4、9 | phone | M8 | 端對端，要手機的通知設定畫面、Android 通知按鈕 |
| T-account-push-3 | phone | M9 | 前提要「多步驟任務」（工作流） |
| T-account-push-5、7 | phone | M8 | 「連上後才送出」「開關顯示關閉中」是手機 Service Worker 和畫面的行為 |
| T-account-push-6 | phone | 未排定 | 團隊管理員 |
| T-safety-n1-9、10、11，T-safety-n2-8，T-trust-promises-10 | trust | M7j J5 | 卡片的鑰匙、不收的理由、「沒有用上」 |
| T-safety-always-7，T-safety-n1-14，T-approval-answers-7 | trust | M7j J6 | 信任設定存伺服器、同步到執行端 |
| T-safety-n1-12、16、17，T-cards-question-10，T-safety-n8-3 | trust | M7j J7 | 同時答、重送、執行端離線（`n1-17`、`n8-3` 看 Q2） |
| T-safety-n2-2、4、5，T-safety-reject-4，T-trust-verify-7 | trust | M7j J8 | 通知上的允許／拒絕、新的蓋掉舊的、不該推的不推 |
| T-deliver-confirm-4、5、8 | tools | M7j J9 | 裝置回報經伺服器 |
| T-deliver-confirm-6 | tools（排 M8） | M7j J9 | 「在桌面點開」由替身桌面照公開 API 回報；預期是證據和交付報告，都是資料 |
| T-deliver-confirm-2、3 | tools | M7j J10 | 顯示失敗叫醒 |
| T-files-n1-3、8 | tools | M7j J11 | 長輸出補上伺服器、別台讀得到 |
| T-files-n1-4 | tools 排 M7.5、brain 排 M7 工具區 | M7j J11 | 兩份不一致（Q6）；記憶的引用由 B16 接上 |
| T-deliver-send-4 | tools | M7j J11 | 伺服器的附件暫存每晚清 |
| T-activity-n1-1、2、6 | trust | M7j J12 | 細紀錄保留、清聊天和刪 Bot 不刪任務 |
| T-task-keep-2、3 | turn（交給工具區，工具區沒收） | M7j J12 | 同上 |
| T-conn-batch-1、3 | M6c 留下 | M7j J13 | 真的執行端成批回報、伺服器停機後補上 |
| T-trust-pipeline-6，T-files-background-9，T-activity-evidence-3 | trust、tools | M7j J13 | 伺服器停機、三台裝置的證據一樣 |
| T-computer-route-1，T-computers-default-5，T-computers-n2-2，T-files-workdir-4 | tools | M7j J14 | 從手機發起、改派 |
| T-computers-n6-1、2，T-files-workdir-2 | tools | M7j J15 | 換電腦時的工作資料夾 |
| T-computer-ops-4，T-files-background-10，T-computer-caps-5，T-sandbox-n4-5 | tools、trust | M7j J16 | 電腦連不上、執行端重開、管理員上限、伺服器被攻破時電腦自己擋 |
| T-computers-n7-1、2、3 | tools | M7j J17 | 伺服器記「最後在哪台打字」 |
| T-computers-n5-1、3，T-mailbox-route-3，T-mailbox-reassign-6 | tools 排 M7j、turn 排 M9、phone 排 M10+ | M7j J18 | 三份不一致（Q1） |
| T-safety-n10-4，T-server-n2-3，T-safety-n11-2、3、4 | trust、phone | M7j J19 | 連線數、相信代理 |
| T-server-n1-2、3，T-server-n2-4 | phone | M7j J20 | 代理、上傳大小 |
| T-server-n1-4 | phone | M8 | 管理頁的警告 |
| T-sync-referee-1，T-phone-cards-1，T-cards-question-9，T-task-resume-8 | turn、trust、phone（都寫「還不能測」） | M7j J21 | 形式驗證，工具已定 |
| T-safety-n1-1、2、3、5、6 | trust | M7j J22 | 形式驗證 |
| T-safety-n1-4、7、8 | trust | M7j J23 | 形式驗證 |
| T-safety-n1-13 | trust | 未排定 | 前提要 `/btw` 支線（R4）；turn 區把支線的列都排未排定 |
| T-safety-n8-4 | trust | M8 | 預期是「輸入框裡是 B」，那是桌面 App 的輸入框 |
| T-claims-mark-2 | trust | M8 | 端對端，預期是「在另一台桌面看」的畫面 |
| T-files-rewind-9 | trust、tools | M8 | 步驟是在手機打 `/rewind`，tools 區 D-15 已把 `/rewind` 排 M8 |
| T-computers-online-6 | tools | M8 | 預期是另一份安裝「顯示」的字和它的紀錄檔，要真的桌面客戶端（Q18） |
| T-activity-who-4 | trust | M9 | 前提是「進行中的目標」（/goal） |
| T-computers-default-3，T-computers-kinds-3 | tools | M9 | tools 區 D-1 已定：工具參數不能換電腦，這兩列改成交辦子任務；交辦工具 turn 區排 M9 |

### A.3 相鄰的列（不在 106 列裡）

**別區拿編號，但要這份先做好入口的**：

| 編號 | 誰驗 | 要這份的哪一段 |
|---|---|---|
| T-chat-window-4、T-chat-steps-7 | phone U6 | J2 的 `typing` |
| T-computers-online-3 | phone U20 | J14 的「這一輪在哪台跑」（Q11）；在線狀態本身已經有 |
| T-deliver-confirm-1、7，T-task-state-2 | phone U19 | J9 的裝置回報、J1 的狀態入口 |
| T-task-state-5、T-trust-verify-6、T-report-delivery-9 | phone U11 | J1 的 👍 和狀態入口 |
| T-task-state-1 等「任務狀態＝」的端對端列 | phone U10 | J1 的狀態入口 |
| T-server-privacy-1 | phone U4 | 管理者名稱：手機線已經在 `/v1/server/status` 加了 `admin_account`，這份不用做 |
| T-context-mask-1、T-context-must-2、T-chat-compact-11 | brain B11 | 接上伺服器（T13、T14）；「畫面」照 Q-28 讀 API |
| T-context-ref-1、2、3 | brain B12 | J11 的「向伺服器拿原文」；`ref-3` 用現有的永久刪除和 410 |
| T-memory-kinds-4、T-memory-carry-9 | brain B26 | 接上伺服器（T13–T16） |
| T-conn-push-1、2、3 | phone 排 M8 | J3、J4 的推播 |

**第 5 組伺服器的列，沒有任何一份規劃排到的**（建議另排，見 Q22）：`T-sync-seq-1`（性質，5 台裝置、伺服器停機兩次）、`T-server-n3-1`–`3`（伺服器紀錄的原因句）、`T-server-n18-1`–`4`（被代理擋住時改用長輪詢）、`T-server-n15-1`、`2`（`anybot --doctor`）、`T-server-n13-3`（停用帳號時例行任務不跑，M9）、`T-server-n8-3`、`T-server-n10-1`、`T-server-n9-1`、`T-account-bind-2`、`3`。

**形式驗證留在之後的**：`T-task-wake-3`（例行任務到點，M9）。

---

## B) 切片索引

### B.1 依賴順序

| 線 | 切片 | 內容 |
|---|---|---|
| A 狀態和裝置回報 | J1 → J9 → J10 | 狀態入口和 👍、交付確認、顯示失敗叫醒 |
| B 即時通道 | J2 | 伺服器半段先做；執行端半段等 T13 |
| C 推播 | J3 → J4 → J8 | 送出、誰收、通知上的回答 |
| D 卡片裁判 | J5 → J6 → J7 | 鑰匙、信任設定、同時答和離線 |
| E 原文和保存 | J11、J12 | 可以同時做 |
| F 執行端接上伺服器之後 | J13、J14 → J15、J16、J17、J18 | 都要 T14（`apps/agent`） |
| G 伺服器防護 | J19、J20 | 跟別的都不相干 |
| H 形式驗證 | J21 → J22 → J23 | J21 等 J5；J22、J23 等 J7、J8 |

線之間的依賴：

- J8 要先有 J5（鑰匙和來源）。
- J7 要先有 J6（一律允許由伺服器記，`T-approval-answers-7` 在 J6）。
- J9 要先有 tools 區 F10（`FileShared`）。
- J11 要先有 tools 區 F7、F8（`RawStore`、`read_result`）。
- J13–J18 要先有 turn 區 T13、T14；J14、J18 另外要 T18；J18 另外要 T8；J10 要 T12 的叫醒信。
- J22、J23 要先有 J7、J8。

**現在就能開工的**：J1、J2 的伺服器半段、J3、J5、J19、J20；J12 在 Q5 答了之後。

### B.2 切片清單

伺服器遷移用 0400–0449，每件 5 號；「預留」表示可能用不到。客戶端遷移這份沒有分配到號碼，J11、J16 要用（Q25）。

| # | 標題 | 驗的編號 | 依賴 | 可以同時做 | 套件 | 遷移 | 協定 | 熱檔 |
|---|---|---|---|---|---|---|---|---|
| J1 | 任務狀態唯讀入口＋👍 | T-cards-n1-1、2、3 | — | 全部 | protocol、domain、server、store-server | — | 是：`GET /v1/tasks/{task}/status`；`thumbs_up`、`thumbs_up_removed`（Q9） | `app.rs`、`known_task_event_kind.rs`、`task_events.rs` |
| J2 | 即時通道：正在打的字 | T-conn-batch-2 | 伺服器半段 —；執行端半段 T13 | 全部 | protocol、bus、server、sync、runner | — | 是：`POST /v1/streams/typing`、SSE `typing` | `app.rs`、`api_streams_sse_drive.rs`、`crates/bus` |
| J3 | Web Push 送出 | T-server-n4-2、3、4 | — | 全部 | channels、server、store-server、protocol | 0400–0404 | 是：推播訂閱入口 | `app.rs`、`Cargo.toml`（channels 加依賴） |
| J4 | 卡片推播：誰收、什麼時候推 | T-safety-n2-1、6、7，T-account-push-1、8 | J3；trust 區 S2（`CardShown`） | J5、J6 | server、channels、store-server、protocol | 0405–0409 | 是：`POST /v1/devices/activity`、訂閱的種類 | `app.rs` |
| J5 | 卡片的鑰匙和「沒有用上」 | T-safety-n1-9、10、11，T-safety-n2-8，T-trust-promises-10 | trust 區 S2 | J3、J4 | protocol、domain、server、store-server、locales | 0410–0414 | 是：`RefereeRequest` 加 `card_key`、`source`；新紀錄 `card_key_issued`；任務事件 `CardAnswerUnused` | `mailbox_card.rs`、`api_referee_card.rs`、`record_rules.rs` |
| J6 | 信任設定存伺服器、一律允許由伺服器記 | T-safety-always-7，T-safety-n1-14，T-approval-answers-7 | J5；trust 區 S3、S4（`TrustSettings`）；T13 | J9、J11 | domain、server、store-client、runner、protocol | 0415–0419（預留） | 是：帳號流的新紀錄（Q16） | `settings_registry.rs`、`record_rules.rs` |
| J7 | 同時答、重送、執行端離線 | T-safety-n1-12、16、17，T-cards-question-10，T-safety-n8-3 | J6；T7、T14、T15；Q2 | J9–J12 | server、domain、tests | — | — | `mailbox_card.rs` |
| J8 | 通知上的允許／拒絕 | T-safety-n2-2、4、5，T-safety-reject-4，T-trust-verify-7 | J4、J5；trust 區 S5（剛拒絕過）；T9、T10 | J9–J12 | server、channels、protocol、locales | — | 是：推播內容的格式、`CardAnswered.origin` 加 `Notification` | `mailbox_card.rs` |
| J9 | 交付確認：裝置回報 | T-deliver-confirm-4、5、6、8 | J1；tools F10；T13 | J6–J8 | protocol、domain、server、store-server、runner、context、locales | 0420–0424 | 是：`DeviceReport` 加欄位（Q10） | `known_task_event_kind.rs`、`record_rules.rs` |
| J10 | 顯示失敗叫醒 Bot | T-deliver-confirm-2、3 | J9；T12（叫醒信） | J11–J20 | server、domain、runner、locales | — | — | `mailbox_deliver.rs` |
| J11 | 長輸出和附件存伺服器、清理 | T-files-n1-3、4、8，T-deliver-send-4 | tools F7、F8、F10；T13、T14；Q4、Q6 | J12 | server、store-server、store-client、sync、runner、protocol | 0430–0434；客戶端（Q25） | 是：`/v1/raw`、`/v1/attachments` | `app.rs`、`migrations/client` |
| J12 | 細紀錄的保存期限 | T-activity-n1-1、2、6，T-task-keep-2、3 | J1；Q5、Q15 | J11 | server、store-server、domain | 0435–0439 | 是：設定鍵 `detail_retention_days` | `cleanup.rs`、`settings_registry.rs` |
| J13 | 伺服器停機與成批回報 | T-conn-batch-1、3，T-trust-pipeline-6，T-files-background-9，T-activity-evidence-3 | T14、T16、T17 | J14–J20 | tests（新測試檔），產品只修錯 | — | — | — |
| J14 | 從手機發起的路由與改派 | T-computer-route-1，T-computers-default-5，T-computers-n2-2，T-files-workdir-4 | T14、T18；Q11 | J13、J16、J17 | server、domain、runner、protocol | — | 是：`ComputerRef` 加 `device_id`（Q11） | `mailbox_deliver.rs`、`turn_records.rs` |
| J15 | 換電腦時的工作資料夾 | T-computers-n6-1、2，T-files-workdir-2 | J14；tools 區工作資料夾 | J16、J17 | protocol、server、runner、computers | — | 是：信裡帶原路徑 | `mailbox_api.rs` |
| J16 | 電腦連不上、執行端重開、管理員上限 | T-computer-ops-4，T-files-background-10，T-computer-caps-5，T-sandbox-n4-5 | T7、T14；tools F11、F12；trust 區底線 | J14、J15、J17 | runner、store-client、apps/agent、server、tests | 客戶端（Q25） | 是：伺服器設定 `background_limit` | `migrations/client` |
| J17 | 「我的電腦」＝最後打字的那台 | T-computers-n7-1、2、3 | T12、T13 | J14–J16 | server、store-server、runner、protocol、locales | 0440–0444 | 是：信裡帶使用者的電腦 | `mailbox_deliver.rs`、`fixed_blocks.rs` |
| J18 | 只有某台連得到的模型 | T-computers-n5-1、3，T-mailbox-route-3，T-mailbox-reassign-6 | T8、T14、T18；Q1 | J13–J17 | runner、apps/agent、server、store-server、domain、protocol、locales | 0445–0449 | 是：`POST /v1/computers/model-reach` | `mailbox_deliver.rs`、`mailbox_route.rs` |
| J19 | 連線數上限與相信代理 | T-safety-n10-4，T-server-n2-3，T-safety-n11-2、3、4 | — | 全部 | server、domain | 0425–0429（預留） | — | `serve_all.rs`、`req_source.rs` |
| J20 | 代理暗號以外的代理列、上傳大小 | T-server-n1-2、3，T-server-n2-4 | J19 | 全部 | server | — | — | `req_write.rs`、`limits.rs` |
| J21 | 形式驗證一：同步裁判 | T-sync-referee-1，T-phone-cards-1，T-cards-question-9，T-task-resume-8 | J5 | J6–J20 | tests/formal（新）、domain（只讀） | — | — | 根 `Cargo.toml`（加成員） |
| J22 | 形式驗證二：只放行、只記看到的 | T-safety-n1-1、2、3、5、6 | J7、J8、J21 | J23 | tests/formal | — | — | — |
| J23 | 形式驗證三：只答一次、按下放開、執行端離線 | T-safety-n1-4、7、8 | J7、J8、J21 | J22 | tests/formal | — | — | — |

---

## C) 每一段的藍圖

### C.0 每一段都一樣的

- **共用規矩**照 `target/agent-common.md`：五條鐵律；不跑 `make check`、雲端檢查；不重產 `api/openapi.yaml`、`api/asyncapi.yaml`、`api/docs/*`、`web/api-client/src/generated/schema.ts`、`tools/spec-trace/baseline.txt`，回報裡列出新入口和整列驗完的編號。
- **測試放哪裡**：
  - 只打伺服器的整合測試：`apps/server/tests/pg_<主題>.rs`（啟動真的 PostgreSQL，所以檔名 `pg_` 開頭）。
  - 伺服器加執行端（`apps/agent`、`apps/runner-service`）一起的整合測試：`tests/app-test/tests/pg_<主題>.rs`，用 harness 啟動真的執行檔，從 HTTP 打。
  - 純函式的單元測試放在該套件自己的 `*_tests.rs`。
  - 形式驗證放新套件 `tests/formal`（J21 建）。
- **替身**：整合列每個預期都是資料時，另一台裝置、另一個執行端可以用「照公開 API 收送」的替身（K118 ⑱、phone Q4）。替身只能打產品的公開 HTTP 入口，不能直接寫資料庫；回報寫明哪一列用了替身。
- **testenv**：要加控制 API 時，照 phone 區的規矩加在 `tests/testenv/src/` 的新檔，不改現有路由。
- **時間**一律走 `Clock`；測試快轉時鐘照 `docs/testing.md` 的做法。每個等待都有期限。
- **字**：使用者看得到的、模型看得到的字都放 `locales/`，用訊息代碼；伺服器只回代碼和參數。
- **遷移**只加不改，號碼在 B.2 那一格的範圍內。
- **熱檔**（`app.rs`、`lib.rs`、`openapi.rs`、`Cargo.toml`、`locales/*.ftl`）只在尾端加，新邏輯放新檔。

---

### J1：任務狀態唯讀入口＋👍

**1. 目標**：手機和測試可以從外面讀一件任務的狀態；對做完的任務的回答按 👍，任務變「完成（你已確認）」，拿掉 👍 不退回。

**2. 開工條件**：無（任務事件由替身執行端照 `/v1/streams/append` 寫，預期都是資料）。Q9 有答案。

**3. 只准改的範圍**：

- `crates/protocol`：新檔 `task_status_api.rs`；`known_task_event_kind.rs`、`task_events.rs`（照 Q9）。
- `crates/domain`：`task_engine_lifecycle.rs`（👍 的規則）、新檔 `thumbs.rs`。
- `crates/server/src/`：新檔 `api_task_status.rs`、`thumbs_check.rs`；`app.rs` 加一條路由。
- `crates/store-server`：查一件任務的紀錄（新檔 `streams_task.rs`）。
- `apps/server/tests/pg_task_status.rs`、`pg_thumbs.rs`；`locales/`。

**4. 定好的介面**：

- `GET /v1/tasks/{task}/status?bot={bot}`（登入憑證、裝置憑證都可以；只有那個 Bot 的擁有者）。
  - 回 `TaskStatusReply { status: TaskStatus, detail: Option<TaskDetail>, last_seq: u64 }`。
  - 算法：讀那條 Bot 流裡 `task_id` 是這件的紀錄，照 `task_seq` 排，轉成 `TaskEvent`，用 `anybot_domain::task_replay::replay`。網頁不另寫一份狀態的程式（phone Q3）。
  - Bot 已刪除（不是永久刪除）照樣讀得到（`activity-n1` S5、K33）。
  - 錯誤：沒有這件任務 → 404 `task-not-found`；`replay` 回缺號 → 500 `problem-internal`，不悄悄回某個狀態。
- 👍（phone Q12，已定）：
  - 紀錄種類 `thumbs_up`、`thumbs_up_removed`，內容 `{ answer_seq: u64 }`，`task_id` 帶那則回答所屬的任務；裝置用 `/v1/streams/append` 寫。
  - 伺服器收的時候檢查 `answer_seq` 是同一條流、同一件任務的一則 Bot 回答；不是 → 這一筆 `rejected`，代碼 `thumbs-target-invalid`。
  - `domain` 的規則：按 👍 的那一刻任務是「完成」→ `report.user_confirmed = true`，細項升成「你已確認」；不是完成 → 不改狀態（`cards-n1` S2）；`thumbs_up_removed` 不改任何東西（S3）。現在 `report_device` 不管任務做完沒都設 `user_confirmed`，要改（Q9）。
  - 交付報告記「你已確認」和時間（S1）：`TaskView.report` 加 `user_confirmed_at_ms`，取那筆 `thumbs_up` 的 `at_ms`。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 完成（已交付）的任務，對最終回答按 👍 | 完成（你已確認）；報告有「你已確認」和時間 | cards-n1 S1 |
| 進行中的任務，對中間的回答按 👍 | 進行中，沒變；之後做完也不是「你已確認」 | cards-n1 S2 |
| 按過 👍 再拿掉 | 仍是完成（你已確認） | cards-n1 S3 |
| `answer_seq` 指到使用者的話、別的任務、不存在 | 那一筆 rejected `thumbs-target-invalid` | 本段定 |
| 讀已刪除 Bot 的任務 | 照樣回狀態 | activity-n1 S5 |
| 讀別人的 Bot 的任務 | 403 `record-no-permission` | conn-auth |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_cards_n1_1` | T-cards-n1-1 | 狀態入口回 `completed/you_confirmed`；報告的 `user_confirmed_at_ms` 等於那筆 👍 的時間 | 整合 |
| `t_cards_n1_2` | T-cards-n1-2 | 按完還是 `running`；替身執行端接著寫完成後是 `completed/delivered`，不是 `you_confirmed` | 整合 |
| `t_cards_n1_3` | T-cards-n1-3 | 寫 `thumbs_up_removed` 之後還是 `you_confirmed` | 整合 |
| `task_status_reads_deleted_bot` | — | Bot 刪除後 200 | 整合 |
| `thumbs_up_rejects_non_answer_target` | — | rejected＋代碼 | 整合 |
| `thumbs_rule_only_counts_when_completed` | — | 純函式各種狀態 | 單元 |

**7. 不准做的**：不做 👍 的按鈕（phone U11）；不做表情（`cards-emoji`，M8）；不在 SSE 加狀態提示（K118 ⑧ 說可以，但沒人要用，等需要再加）；團隊成員的 👍（S4）不做，未排定。

**8. 驗收清單**：三個帶編號的測試綠；回報列出新入口和 Q9 的做法；`DeviceReportKind` 的改動列在回報給監督者重產 schema。

---

### J2：即時通道：正在打的字

**1. 目標**：Bot 正在打的字 1 秒內出現在正在看的裝置上；打字中途才打開的也看得到已經打出來的字；伺服器只轉不存；紀錄流裡那個回答只有完整的一筆（`conn-batch` S2、S3，K118 ⑭）。

**2. 開工條件**：伺服器半段（第 1–3 步）現在就能做，用替身執行端測；執行端半段（第 4 步）和 `t_conn_batch_2` 等 T13 在 `dev`。

**3. 只准改的範圍**：

- `crates/protocol`：新檔 `typing_api.rs`；`sse_api.rs` 加常數 `SSE_EVENT_TYPING`。
- `crates/bus`：加一條打字提示的廣播（新檔 `typing.rs`），不改 `Notification`。
- `crates/server/src/`：新檔 `api_streams_typing.rs`、`sse_typing.rs`；`api_streams_sse_drive.rs` 的 `Tail` 多收一條；`app.rs` 加路由。
- `crates/sync`：新檔 `typing_send.rs`（打 `POST /v1/streams/typing`）。
- `crates/runner`：模型串流時把「到目前為止的全部文字」交給 `TypingSink`（新檔 `typing.rs`）。
- 測試：`apps/server/tests/pg_streams_typing.rs`、`tests/app-test/tests/pg_conn_batch_typing.rs`。

**4. 定好的介面**：

- `POST /v1/streams/typing`（只有執行端：電腦憑證或執行服務憑證；要能寫那條 Bot 流）。
  - Body：`TypingRequest { stream: StreamSelector, task_id: String, text: String, thinking: String }`（`thinking` 的型別見 Q17）。
  - 回 204。不寫資料庫、不進紀錄流、不要 `Idempotency-Key`（不是寫入）。
  - 太大（超過 `MAX_WRITE_BODY`）→ 413。
- SSE 多一種事件 `typing`，資料 `TypingHint { stream, task_id, text, thinking, at_ms }`；只送給看得到那條流的連線。打的字不是紀錄，所以不違反 K118 ⑧（phone Q2，已定）。
- 伺服器對每條 SSE 連線、每件任務最多每秒送一次：收到新的時，距上次送超過 1 秒就馬上送，不然在滿 1 秒時送最新的那份（只留最新的一份在記憶體，不存）。
- 執行端：模型開始出字就送第一次，之後只要文字有變，每秒送一次；回答寫成紀錄後不再送。
- `trait TypingSink: Send + Sync { fn send(&self, task: &TaskId, text: &str, thinking: &str); }`，`runner` 只認這個 trait，`sync` 給實作，測試世界給收集器。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 假模型花 10 秒慢慢回 500 字；第 5 秒才連 SSE | 1 秒內收到 `typing`，`text` 是前 5 秒打出來的全部字 | conn-batch S3 |
| 之後 | 兩次 `typing` 至少隔 1 秒 | conn-batch S2 |
| 回答完 | 紀錄流裡 `ModelAnswer` 只有 1 筆、字完整；沒有任何一筆是打到一半的 | conn-batch S2 |
| 伺服器重開 | 沒有任何打字的資料留下（資料庫沒有、紀錄流沒有） | K118 ⑭ |
| 別人的 SSE | 收不到 | conn-auth |
| 手機的登入送 `typing` | 403 | 本段定（只有執行端會打字） |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_conn_batch_2` | T-conn-batch-2 | 真的執行端＋假模型；替身手機（SSE）第 5 秒連上；1 秒內收到；間隔 ≥ 1 秒；紀錄流 1 筆完整的回答 | 整合 |
| `typing_is_forwarded_not_stored` | — | 替身執行端送 3 次；SSE 收到；資料庫沒有新列 | 整合 |
| `typing_coalesces_to_one_per_second` | — | 1 秒內送 10 次，SSE 只收到最新的 1 次 | 整合 |
| `typing_only_reaches_owner` | — | 別的帳號收不到 | 整合 |
| `typing_rejects_browser_credential` | — | 403 | 整合 |

**7. 不准做的**：不寫手機畫面（phone U6）；不把打字寫進紀錄流或資料庫；不加 WebSocket（`conn-ws` S2）。

**8. 驗收清單**：`t_conn_batch_2` 綠；`api/asyncapi.yaml` 要加 `typing`，列在回報給監督者重產。

---

### J3：Web Push 送出

**1. 目標**：伺服器能存手機的推播訂閱、照規格加密送出，並實作 turn 區 T19 的 `NotifyOwner`。

**2. 開工條件**：無。Q14 有答案。

**3. 只准改的範圍**：

- `crates/channels/src/`：新模組 `web_push/`（`payload.rs`、`send.rs`、`vapid.rs`、`target.rs`），`Cargo.toml` 加 `sad-web-push`（workspace 已有）。
- `crates/store-server/src/`：新檔 `push_subscriptions.rs`、`push_key.rs`。
- `crates/server/src/`：新檔 `api_push.rs`、`push_notifier.rs`；`app.rs` 加路由。
- `crates/protocol`：新檔 `push_api.rs`。
- `migrations/server/0400_push_server_key.sql`、`0401_push_subscriptions.sql`。
- `apps/server/tests/pg_push_*.rs`；`crates/channels` 的單元測試。

**4. 定好的介面**：

- 推播的伺服器金鑰（VAPID，P-256）：第一次啟動產生，用現有的 `secret_seal` 加密存在 `push_server_key`（`server-n4` S1、`server-privacy` S3）。
- `GET /v1/push/server-key` → `{ public_key }`（base64url），手機訂閱時要。
- `POST /v1/push/subscriptions`（登入憑證），Body `{ endpoint, keys: { p256dh, auth } }` → `{ id }`。
  - `endpoint` 不是 https、超過 2048 字 → 400 `push-endpoint-invalid`。
  - 同一個 `endpoint` 先刪舊的。
  - 每帳號 16 支，第 17 支擠掉最舊的。
- `DELETE /v1/push/subscriptions/{id}`；`GET /v1/push/subscriptions/mine` → 這支裝置的訂閱有沒有（`account-push` S6 要用，畫面在 M8）。
- `channels::web_push`：
  - `PushMessage::compose(title, body_code, body_params, tag, badge) -> PushMessage`：照 `server-n4` S4 截短（見 Q24）。
  - `send(client: &ChannelHttpClient, sub: &Subscription, msg: &PushMessage, key: &VapidKey, clock) -> Result<SendOutcome, PushError>`：aes128gcm、`TTL: 86400`、`Urgency: high`、`Topic` 帶 tag、10 秒逾時、不跟轉址。
  - 推播服務的位址：要 https，不能是 `localhost`、`.local`，解析出來要是公開位址；不合 → 不送，`SendOutcome::Skipped`。
  - 回 404、410 → `SendOutcome::Gone`，呼叫的人刪掉那個訂閱。
  - HTTP 只在 `channels` 發（`spec/07` §3）。
- `server::push_notifier::WebPushNotifier` 實作 T19 的 `NotifyOwner`；T19 還沒合時先只做 `channels` 和訂閱入口，`NotifyOwner` 的實作留到 T19 合進 `dev` 再接（回報寫明）。
- 測試裡推播服務是 harness 的假推播；「公開位址」的檢查用從建構函式傳進去的解析器，測試傳一個把假推播的名字解析成公開位址的版本（不加旗標、不加環境變數）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 訂閱 http 位址、2049 字的位址 | 都 400 | server-n4 S2 |
| 同一位址訂兩次 | 只留一筆 | server-n4 S2 |
| 第 17 支 | 最舊的被擠掉 | server-n4 S2 |
| 假推播對某個訂閱回 410 | 那個訂閱被刪 | server-n4 S3 |
| 送出 | 假推播記到 `ttl=86400`、`urgency=high` | server-n4 S3 |
| 推播服務位址是 `https://localhost` | 沒送 | server-n4 S3 |
| 標題 100 字、內文 300 字、標記「a b/c」 | 標題 80 字、內文 200 字、標記「abc」 | server-n4 S4 |
| 金鑰存在資料庫 | 匯出找不到明文 | server-privacy S3 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_server_n4_2` | T-server-n4-2 | 四種情況照表 | 整合 |
| `t_server_n4_3` | T-server-n4-3 | 410 的被刪；`ttl`、`urgency`；localhost 的沒送 | 整合 |
| `t_server_n4_4` | T-server-n4-4 | 截短的結果 | 單元 |
| `push_server_key_is_sealed` | — | 匯出資料庫找不到私鑰的各種寫法 | 整合 |
| `push_send_does_not_follow_redirect` | — | 假推播回 302，不跟 | 整合 |

**7. 不准做的**：不做手機的訂閱和 Service Worker（phone、M8）；不做「誰收、什麼時候推」（J4）；不做從舊伺服器沿用金鑰（`T-server-n4-1`，M10）；不做管理頁的聯絡方式（`server-n4` S5，M8；送出時的 `sub` 欄位先用 `mailto:` 加伺服器設定裡已有的值，沒有就不帶，見 Q14）。

**8. 驗收清單**：三個帶編號的測試綠；新入口列在回報；`cargo deny` 對 `sad-web-push` 沒有新警告（監督者跑）。

---

### J4：卡片推播：誰收、什麼時候推

**1. 目標**：出卡片時，照規則決定推不推、推給哪幾支手機、帶不帶按鈕。

**2. 開工條件**：J3 在 `dev`；trust 區 S2（`CardShown` 的 `kind`、`buttons`）在 `dev`。Q12、Q13 有答案。

**3. 只准改的範圍**：

- `crates/domain`：新檔 `push_rules.rs`（純函式）。
- `crates/server/src/`：新檔 `push_card.rs`（收到 `CardShown` 時呼叫）、`api_device_activity.rs`；`mailbox_card.rs` 只加一個呼叫點。
- `crates/store-server`：新檔 `device_activity.rs`；`push_subscriptions.rs` 加種類。
- `crates/protocol`：`push_api.rs` 加型別。
- `migrations/server/0405_device_activity.sql`、`0406_push_subscription_kinds.sql`。
- `apps/server/tests/pg_push_card_*.rs`。

**4. 定好的介面**：

- `POST /v1/devices/activity`（任何登入、裝置憑證），Body `DeviceActivity { foreground: bool, last_input_at_ms: Option<u64>, open_bot: Option<BotId> }`；存一列，每台一列，只留最新的。
- `PUT /v1/push/subscriptions/{id}/kinds`，Body `{ kinds: Option<Vec<PushKind>> }`；`None`＝沒設過＝全部；`Some([])`＝都不要（`account-push` S1、R48）。`PushKind { NeedsYou, DoneWhileAway, Routine, FileReceived }`。
- `domain::push_rules::should_push(input: &PushDecisionInput) -> PushDecision`：
  - 你的任何一台桌面 `foreground` 而且 `last_input_at_ms` 在 90 秒內 → 不推（`safety-n2` S5、`account-push` S7）。
  - 有手機頁面 `foreground` 而且 `open_bot` 是這個 Bot → 不推（`safety-n2` S5）。
  - 其他 → 推給這個帳號每支「種類包含 NeedsYou」的訂閱（`account-push` S1、S5；團隊管理員未排定）。
- 卡片推播的內容（trust 區定的卡片種類）：
  - 每次問的卡、整個請求放得進通知 → 帶「允許」「拒絕」兩個按鈕（`safety-n2` S1）。
  - 放不進、改 hooks、需要處理、問題卡、計畫卡 → 不帶按鈕（`safety-n2` S2）。
  - 標題是 Bot 的名字；內文代碼和參數照 trust 區卡片的 `reason`；`tag`＝`bot-<id>`（同一個 Bot 新的蓋掉舊的，`safety-n2` S4、`account-push` S3）。
  - 推播內容帶 `card_id`、`card_key`（J5）。J4 先合時 `card_key` 欄位先不帶，J5 合了再加（回報寫明）。
- http 下手機根本訂不到（瀏覽器的規定），伺服器不另外判斷 http；`T-safety-n2-7` 驗的是「沒有訂閱就沒有推播，卡照樣看得到、答得了」。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 桌面閒置 91 秒，手機頁面沒開；出卡 `npm publish` | 假推播收到一則：標題 Bot1、內文「允許 Bot1 執行指令：npm publish」的代碼和參數、按鈕允許／拒絕 | safety-n2 S1、S5 |
| 桌面在最前面、10 秒前有輸入 | 沒有推播 | safety-n2 S5 |
| 手機沒有訂閱（http） | 沒有推播；替身手機讀紀錄看得到 `CardShown`，送允許被收下 | safety-n2 S5 |
| P 沒設過種類；P2 設成空的 | 推給 P，P2 沒有 | account-push S1 |
| 電腦 A 在前景、30 秒前有操作 | 沒有推播；替身 A 的紀錄裡有那張 `CardShown` | account-push S7 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_safety_n2_1` | T-safety-n2-1 | 照表第 1 列 | 整合 |
| `t_safety_n2_6` | T-safety-n2-6 | `/control/push/records` 空的 | 整合 |
| `t_safety_n2_7` | T-safety-n2-7 | 沒推；卡看得到、答得了 | 整合 |
| `t_account_push_1` | T-account-push-1 | 只有 P 的訂閱收到 | 整合 |
| `t_account_push_8` | T-account-push-8 | 沒推；A 有卡 | 整合 |
| `push_rules_*` | — | 純函式各種組合 | 單元 |

卡片由替身執行端寫 `CardShown`（預期都是資料）；「桌面閒置」由替身桌面送 `POST /v1/devices/activity`。

**7. 不准做的**：不做任務的通知（T19）；不做通知上的回答（J8）；不做手機、桌面送活動的程式（M8；這段只做入口）；不做團隊管理員（未排定）。

**8. 驗收清單**：五個帶編號的測試綠；新入口列在回報。

---

### J5：卡片的鑰匙和「沒有用上」

**1. 目標**：每張卡出現時伺服器發一把猜不到的鑰匙；回答要帶鑰匙和來源；不收的情況在主線聊天寫「沒有用上」和原因（`safety-n1` S1、S2、`safety-n2` S6）。

**2. 開工條件**：trust 區 S2（`CardShown.buttons`、卡片種類）在 `dev`。Q3、Q23 有答案。

**3. 只准改的範圍**：

- `crates/protocol`：`referee_api.rs` 加欄位；新檔 `card_key.rs`；`known_task_event_kind.rs` 加 `CardAnswerUnused`。
- `crates/domain`：新檔 `card_answer.rs`（純函式 `decide_card_answer`，J21–J23 也用它）；`record_rules.rs` 的 `DEVICE_FORBIDDEN_KINDS` 加 `card_key_issued`、`card_answer_unused`。
- `crates/server/src/`：`mailbox_card.rs`、`api_referee_card.rs`；新檔 `card_key_issue.rs`、`card_unused.rs`。
- `crates/store-server`：新檔 `card_keys.rs`。
- `migrations/server/0410_card_keys.sql`。
- `locales/`；`apps/server/tests/pg_card_key_*.rs`。

**4. 定好的介面**（照 Q3 的建議）：

- 伺服器收下一筆 `CardShown` 時，在同一筆交易裡：產生 16 位元組的亂數鑰匙；存 `card_keys(stream, task_id, card_id, key, kind, buttons, issued_at)`；寫一筆伺服器的紀錄 `card_key_issued { card_id, key }` 到同一條 Bot 流（擁有者的裝置都看得到，才能回答）。
- `RefereeRequest` 加：
  - `card_key: Option<String>`；
  - `source: Option<AnswerSource>`，`enum AnswerSource { DesktopWindow, PhonePage, Notification }`。
- `domain::card_answer::decide_card_answer(input: &CardAnswerInput) -> CardAnswerDecision`：
  - `CardAnswerInput { waiting: Option<WaitingCard>, answer: AnswerKind, card_key: Option<&str>, source: AnswerSource, credential: CredentialKind }`；
  - `CardAnswerDecision { Accept, Unused(UnusedReason), Silent }`；
  - `UnusedReason { PageTooOld, AppTooOld, NotWaiting, NotificationNotWaiting, WrongKind }`；
  - 沒帶鑰匙：登入憑證（手機頁面）→ `PageTooOld`；電腦憑證（桌面 App）→ `AppTooOld`。
  - 鑰匙不是現在在等的那張 → `NotWaiting`（來源是通知時 `NotificationNotWaiting`）。
  - 回答種類不合這張卡或這個來源（通知送一律允許、需要處理卡收到允許、按鈕不在 `buttons` 裡，trust 區 Q20）→ `WrongKind`。
  - 那張卡是直接操作打開時一起放行的開關卡 → `Silent`（不收、不寫，`safety-n1` S5，J6 用）。
- 有卡在等、決定是 `Unused` 時，伺服器寫任務事件 `CardAnswerUnused { card_id, source, reason }` 到主線；回答的人收到 409 和同一個代碼。
- 字（`safety-n1` S2、`safety-n2` S3）：
  - `card-unused-page-old`「這個頁面太舊了，沒有用上。請重新整理再回答；還是不行，就在電腦上回答。」
  - `card-unused-app-old`「那台電腦的 AnyBot 比較舊，回答沒有用上。請到設定 › 更新。」
  - `card-unused-stale`「那張卡已經不在等了，回答沒有用上。」
  - `card-unused-notification`「通知上的「允許／拒絕」沒有用上：那個請求已經不在等了。」
  - `card-unused-wrong-kind`：規格沒給字，見 Q23。
- 只帶 `answer=once`、沒帶鑰匙的連結（`safety-n2` S6）：網頁那邊不送；伺服器這邊沒帶鑰匙一律不收。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 卡 A 在等；替身桌面答 A；卡 B 出現；替身手機用 A 的鑰匙送一律允許 | B 還在等；一律允許清單沒變；主線有 `card-unused-stale` | safety-n1 S2、S3；trust-promises P9 |
| 手機頁面不帶鑰匙送允許這次 | 卡還在；主線 `card-unused-page-old` | safety-n1 S2 |
| 桌面 App 不帶鑰匙送允許這次 | 卡還在；主線 `card-unused-app-old` | safety-n1 S2 |
| 只帶 `answer=once` 的請求 | 卡還在等 | safety-n2 S6 |
| 沒有卡在等時的舊回答 | 409；不寫主線 | safety-n1 S2「有卡在等時」 |

「手機斷網、恢復後重送」由替身手機在「恢復」那一刻才送；斷網期間的排隊是手機程式的事（M8），預期裡沒有寫。

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_safety_n1_9` | T-safety-n1-9 | 照表第 1 列 | 整合 |
| `t_trust_promises_10` | T-trust-promises-10 | 同上，P9 的字 | 整合 |
| `t_safety_n1_10` | T-safety-n1-10 | 照表第 2 列 | 整合 |
| `t_safety_n1_11` | T-safety-n1-11 | 照表第 3 列 | 整合 |
| `t_safety_n2_8` | T-safety-n2-8 | 照表第 4 列 | 整合 |
| `card_key_is_random_and_unique` | — | 1,000 張卡沒有重複，長度 16 位元組 | 整合 |
| `decide_card_answer_*` | — | 純函式每種理由 | 單元 |

**7. 不准做的**：不改畫面；不做通知的推送（J4、J8）；不做一律允許清單的寫入（J6）；不碰頻道（`safety-n1` S6 本來就沒有入口）。

**8. 驗收清單**：五個帶編號的測試綠；`card_key_issued` 的格式寫進回報，phone 區和桌面照它送回答。

---

### J6：信任設定存伺服器、一律允許由伺服器記

**1. 目標**：一律允許清單、這段對話都允許、直接操作、一律放行、智慧模式秒數存在伺服器、同步到每個執行端；一律允許、這段對話都允許由伺服器在收下回答時照伺服器上那張卡寫（`safety-n1` S3）。

**2. 開工條件**：J5；trust 區 S3、S4 的 `TrustSettings` trait 在 `dev`；T13（執行端有本機副本）。Q16 有答案。

**3. 只准改的範圍**：

- `crates/domain`：`settings_registry.rs` 加鍵；新檔 `trust_lists.rs`（從紀錄算清單的純函式）。
- `crates/protocol`：新檔 `trust_records.rs`。
- `crates/server/src/`：`mailbox_card.rs` 收下回答後呼叫新檔 `trust_write.rs`。
- `crates/store-client`、`crates/runner`：`TrustSettings` 的正式實作（新檔 `trust_settings_replica.rs`），讀本機副本。
- `migrations/server/0415–0419`（預留，照 Q16 的做法可能用不到）。
- 測試：`tests/app-test/tests/pg_trust_settings.rs`。

**4. 定好的介面**（照 Q16 的建議）：

- 帳號流的紀錄：
  - `always_allow_added { entry: AlwaysEntry }`：伺服器寫（裝置不准）。
  - `always_allow_removed { key: ActionKey }`：裝置寫（使用者在設定移除）。
  - `conversation_allow_added { bot, conversation, entry }`：伺服器寫。
  - 清單＝照順序折出來（`domain::trust_lists::fold`）。
- `AlwaysEntry` 用 trust 區 S4 定的型別：`{ action, key, added_at, source_bot }`；`action`、`key` 取伺服器 `card_keys` 那張卡存的內容，不採用回答送來的任何文字。
- 開關（`direct_mode`、`yolo`、`smart_seconds`）走現有的設定同步，加進 `settings_registry`，範圍是帳號；`run_here` 照 R57 維持只在那台。
- 執行端每一步讀 `TrustSettings` 時讀本機副本（trust 區 S4：每一步都讀）。
- 直接操作打開時，伺服器把在等的開關卡標成放行（J5 的 `Silent`）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| Bot1 一輪在跑，清單有「在工作 Mac 跑 make」；替身手機移除；假模型再 `make` | 出卡 | safety-always S6 |
| Bot2 開關卡在等；設定直接操作＝開；之後手機對那張按拒絕 | 開關卡已放行；拒絕沒生效；主線沒有「沒有用上」 | safety-n1 S5 |
| 一般卡在等；桌面按一律允許、手機同時按拒絕 | 只有先到伺服器的生效；清單只在一律允許先到時新增 | approval-answers S7 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_safety_always_7` | T-safety-always-7 | 照表第 1 列，真的執行端讀副本 | 整合 |
| `t_safety_n1_14` | T-safety-n1-14 | 照表第 2 列 | 整合 |
| `t_approval_answers_7` | T-approval-answers-7 | 跑 50 次兩種先後，清單只在 always 先到時多一筆 | 整合 |
| `always_entry_comes_from_server_card` | — | 回答帶假的動作文字，清單記的是卡上的 | 整合 |
| `trust_lists_fold_*` | — | 加、移除、重複加 | 單元 |

**7. 不准做的**：不做設定畫面（M8）；不改 trust 區的把關順序；不讓裝置寫 `always_allow_added`。

**8. 驗收清單**：三個帶編號的測試綠；新的紀錄種類列在回報。

---

### J7：同時答、重送、執行端離線

**1. 目標**：兩處同時答只有一個生效；重送只算一次；執行端不在線時回答留在伺服器、上線後只交那張卡的；執行端當掉重開時在等的卡作廢（`safety-n1` S1、S7、S8；`safety-n8` S4；`cards-question` S9）。

**2. 開工條件**：J6；T7（接續）、T14（`apps/agent`）、T15（答案送回執行端）在 `dev`；Q2 有答案。

**3. 只准改的範圍**：`crates/server/src/mailbox_card.rs`、`mailbox_claim.rs`（只修錯和加作廢的判斷）；`crates/domain/src/card_answer.rs`；測試 `tests/app-test/tests/pg_card_race.rs`、`pg_card_offline.rs`。

**4. 定好的介面**：

- 不加新入口。收下回答和寫 `CardAnswered` 在同一筆交易，用 `card_keys` 那一列上鎖（`SELECT … FOR UPDATE`），所以兩個同時到的只有一個看到「在等」。
- 重送：同一個 `Idempotency-Key` 回一樣的結果（現有）；不同的 key、同一張卡 → 第二個 `card-already-answered`，不寫主線（卡已經不在等，`safety-n1` S2「有卡在等時」才寫）。
- 執行端不在線：回答照收，進那台的信箱（現有的 T15 路）；卡被別處作廢的，信箱裡那封不交（`safety-n1` S8）。
- 執行端重開時（T7 的接續），執行端寫「那張卡作廢」的事件（trust 區 / turn 區定的種類），伺服器之後收到的那張卡的回答 → `card-unused-stale`；已經在信箱、還沒交的那封作廢，主線寫 `card-unused-stale`（`safety-n8` S4）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 替身桌面和替身手機同一時間送允許這次和拒絕 | 只有一個生效；任務只往一個方向走；紀錄編號連續 | safety-n1 S1 |
| 手機答了；桌面再送 | 桌面那張的紀錄變成已答（`CardAnswered` 已在流裡）；桌面的答案 409 | cards-question S9 |
| 手機重送 3 次 | 卡只答一次；那一步只執行一次 | safety-n1 S7 |
| 工作 Mac 不在線（Q2：斷網）；手機允許；恢復 | 上線後那一步執行一次；證據有「你按的：允許這次」 | safety-n1 S8 |
| 工作 Mac 被關掉（Q2：程序結束）；手機允許；打開；接著做 | 手機的回答沒用上（主線 `card-unused-stale`）；接著做時重新出卡 | safety-n8 S4 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_safety_n1_12` | T-safety-n1-12 | 跑 50 次，每次只有一個生效 | 整合 |
| `t_cards_question_10` | T-cards-question-10 | 照表第 2 列 | 整合 |
| `t_safety_n1_16` | T-safety-n1-16 | 照表第 3 列 | 整合 |
| `t_safety_n1_17` | T-safety-n1-17 | 照表第 4 列，真的 `apps/agent` | 整合 |
| `t_safety_n8_3` | T-safety-n8-3 | 照表第 5 列，真的 `apps/agent` 被關掉再啟動 | 整合 |

**7. 不准做的**：不做桌面「卡馬上收掉」的畫面（`cards-question-10` 的那半由紀錄驗；畫面在 M8）；不做輸入框（`T-safety-n8-4`，M8）。

**8. 驗收清單**：五個帶編號的測試綠。

---

### J8：通知上的允許／拒絕

**1. 目標**：從通知按的回答帶鑰匙送到伺服器，只當「允許這次」或「拒絕」；不對就寫「沒有用上」；同一個 Bot 新的通知蓋掉舊的；不該推的不推。

**2. 開工條件**：J4、J5；trust 區 S5（剛拒絕過的直接拒絕，`rejected-just-now`）；`T-trust-verify-7` 另外要 T9、T10（驗收和「等驗收：電腦不在線」，Q20）。

**3. 只准改的範圍**：`crates/server/src/push_card.rs`、`mailbox_card.rs`；`crates/protocol`（`CardAnswered.origin` 加 `Notification`，跟 trust 區對齊）；`crates/channels/src/web_push/payload.rs`；`locales/`；測試 `apps/server/tests/pg_push_answer.rs`、`tests/app-test/tests/pg_trust_verify_offline.rs`。

**4. 定好的介面**：

- 推播內容（給手機 Service Worker 讀，phone 區照這個做）：`CardPushPayload { bot_id, bot_name, card_id, card_key, body_code, body_params, buttons: Vec<NotificationButton> }`，`NotificationButton { Once, Deny }`。
- Service Worker 按了之後打 `POST /v1/streams/referee`，`source = Notification`、`card_key`、`answer = once | deny`。
- 伺服器：來源是 `Notification` 時只收 `once`、`deny`，而且那張卡要是「每次問」的卡（J5 的 `WrongKind`，`safety-n1` ③）。
- 收下時 `CardAnswered { origin: Notification }`，證據的把關紀錄寫「你按的：允許這次（通知）」（trust 區的證據格式）；一律允許清單不動。
- 新的卡推播用同一個 `tag`，所以手機上只剩最新一則（`safety-n2` S4）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 卡已推播；替身手機照通知送允許 | 那一步執行一次；證據「你按的：允許這次（通知）」；清單沒有新增 | safety-n2 S1 |
| 卡在桌面被答掉後，照通知送允許 | 主線 `card-unused-notification` | safety-n2 S3 |
| Bot1 第一張被答、第二張出現 | 兩則推播的 `topic` 一樣（手機上只剩第二則） | safety-n2 S4 |
| 照通知送拒絕；假模型再呼叫一樣的 | 不出卡，直接拒絕 | safety-reject S3 |
| 做完的標準要讀家裡 Linux 的檔；Linux 關掉；快轉 23 小時、再 2 小時 | 23 小時：進行中（等驗收：家裡 Linux 不在線）；25 小時：報告那個條件「沒驗到」、原因「那台電腦一直不在線」、未驗收；沒有推播 | trust-verify S5 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_safety_n2_2` | T-safety-n2-2 | 照表第 1 列 | 整合 |
| `t_safety_n2_4` | T-safety-n2-4 | 照表第 2 列 | 整合 |
| `t_safety_n2_5` | T-safety-n2-5 | 照表第 3 列 | 整合 |
| `t_safety_reject_4` | T-safety-reject-4 | 照表第 4 列 | 整合 |
| `t_trust_verify_7` | T-trust-verify-7 | 照表第 5 列；狀態從 J1 讀；推播從 `/control/push/records` 讀 | 整合 |
| `notification_rejects_always_allow` | — | `WrongKind` | 整合 |

**7. 不准做的**：不寫 Service Worker（phone、M8）；不做 Android 離線排隊（`T-account-push-5`，M8）；不讓通知回答需要處理、問題、計畫卡。

**8. 驗收清單**：五個帶編號的測試綠；推播內容格式寫進回報。

---

### J9：交付確認：裝置回報

**1. 目標**：每台裝置回報 Bot 交出的檔「已顯示」「點開了」「顯示失敗」；每台每種只記第一次、最多 12 筆；之後每一輪的工具結果後面帶〔交付狀態：…〕；記進證據和交付報告（`deliver-confirm` S1–S4、S6）。

**2. 開工條件**：J1；tools 區 F10（`FileShared { call_id, name, size, digest }`）；T13（執行端接上伺服器）。Q10 有答案。

**3. 只准改的範圍**：

- `crates/protocol`：`known_task_event_kind.rs`、`task_events.rs`（照 Q10）。
- `crates/domain`：新檔 `delivery_reports.rs`（`record_report` 純函式）；`task_engine_lifecycle.rs`、`task_engine_evidence.rs`。
- `crates/server/src/`：新檔 `device_report_check.rs`，在 append 時呼叫。
- `crates/store-server`：新檔 `delivery_reports.rs`。
- `crates/runner`、`crates/context`：工具結果後面加交付狀態（新檔 `delivery_suffix.rs`）。
- `migrations/server/0420_delivery_reports.sql`；`locales/`；測試。

**4. 定好的介面**（照 Q10 的建議）：

- `DeviceReport { kind: DeviceReportKind, file: String, device: DeviceId, reason: Option<String> }`，`file` 是 `FileShared.digest`（附件的編號）；`DeviceReportKind { Shown, Opened, DisplayFailed }`（👍 照 Q9 搬走）。
- 裝置用 `/v1/streams/append` 寫；伺服器檢查：
  - `device` 要等於呼叫的裝置；不是 → rejected `deliver-report-wrong-device`。
  - 那個 `file` 要是這件任務的 `FileShared`；使用者自己附的檔沒有 `FileShared`，所以沒有記號（S2）→ rejected `deliver-report-unknown-file`。
  - `domain::delivery_reports::record_report(existing: &[ReportKey], new: ReportKey) -> RecordDecision`：同一台同一種已經有 → `Duplicate`（回 `duplicate`，不寫）；已經 12 筆 → `Full`（回 rejected `deliver-report-full`）；其他 → `Record`。用 `delivery_reports` 表的唯一鍵擋同時到的。
- 「斷線、沒打開 App 不算失敗」：伺服器不會自己產生 `DisplayFailed`，只有裝置回報才有（S2）。
- 工具結果後面的字：`deliver-status-suffix { items }`「〔交付狀態：…〕」，每個檔一段 `deliver-status-item { name, state, device }`；執行端從本機副本的任務事件算（S4）。
- 證據：每筆回報一條證據 `EvidencePayload::DeviceReport`；交付報告的宣稱「〈檔名〉：使用者在〈裝置〉點開過」，等級「已驗證」，判斷來源 `CheckSource::Program`（S6；報告格式屬 trust 區 S8）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| Bot 傳了 a.txt，替身手機回報顯示；下一輪假模型呼叫任何工具 | 模型收到的結果後面有〔交付狀態：…a.txt…已顯示…〕 | S4 |
| Bot 傳了 a.txt；手機斷網；快轉 1 小時 | 沒有 `DisplayFailed`；Bot 沒被叫醒 | S2、S5 |
| 一個任務裡傳了 a.txt，替身桌面回報點開；任務完成 | 證據包含〔a.txt 在桌面點開〕；報告「a.txt：使用者在桌面點開過」〔已驗證〕〔程式檢查〕 | S6 |
| 15 台各 3 次 | 每台每種只記第一次；總共 12 筆 | S2 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_deliver_confirm_4` | T-deliver-confirm-4 | 照表第 1 列，真的執行端 | 整合 |
| `t_deliver_confirm_5` | T-deliver-confirm-5 | 照表第 2 列 | 整合 |
| `t_deliver_confirm_6` | T-deliver-confirm-6 | 照表第 3 列（替身桌面，見 Q7） | 整合 |
| `t_deliver_confirm_8` | T-deliver-confirm-8 | `record_report` 跑 45 次，12 次 `Record` | 單元 |
| `device_report_rejects_other_device` | — | rejected | 整合 |
| `device_report_rejects_user_attachment` | — | rejected | 整合 |

**7. 不准做的**：不做記號的畫面（phone U19）；不做叫醒（J10）；不做 `show` 截圖、`send_to_device`（M9、M10）。

**8. 驗收清單**：四個帶編號的測試綠。

---

### J10：顯示失敗叫醒 Bot

**1. 目標**：有裝置顯示失敗、還沒有任何裝置顯示過 → 叫醒 Bot 一次，模型收到「［交付］…」；使用者上次說話之後最多兩次（`deliver-confirm` S5）。

**2. 開工條件**：J9；T12（伺服器寫叫醒、送信）。

**3. 只准改的範圍**：`crates/domain`（新檔 `delivery_wake.rs`）；`crates/server/src/`（新檔 `delivery_wake.rs`，在 J9 收下 `DisplayFailed` 後呼叫）；`crates/runner`（叫醒的開頭字）；`locales/`；測試 `tests/app-test/tests/pg_deliver_wake.rs`。

**4. 定好的介面**：

- `domain::delivery_wake::should_wake(history: &DeliveryHistory) -> bool`：這個檔還沒有任何 `Shown`；使用者最後一則訊息之後，這件任務因交付叫醒的次數 < 2；這個檔是 `send_file` 交的（`show` 的截圖、訊息頻道要的檔只記不叫醒）。次數從紀錄流裡的叫醒事件算，不另外存表。
- 叫醒走 T12 的叫醒（`Wake` 事件＋一封「一輪」的信），原因 `WakeReason::DeliveryFailed { file, ext, device, reason }`。
- 執行端把它寫成模型看得到的開頭：`deliver-wake { file_id, ext, device, reason }`「［交付］…」。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| Bot 傳了 broken.png；替身手機回報顯示失敗 | Bot 被叫醒一次；模型收到「［交付］…broken.png…」 | S5 |
| 又傳了兩次壞的；使用者都沒說話 | 總共只叫醒 2 次 | S5 |
| 使用者說了一句之後又壞 | 可以再叫醒 | S5 |
| 已有別台顯示過 | 不叫醒 | S5 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_deliver_confirm_2` | T-deliver-confirm-2 | 照表第 1 列 | 整合 |
| `t_deliver_confirm_3` | T-deliver-confirm-3 | 假模型收到的「［交付］」請求數＝2 | 整合 |
| `should_wake_*` | — | 純函式 | 單元 |

**7. 不准做的**：不改 T12 的叫醒規則本身。

**8. 驗收清單**：兩個帶編號的測試綠。

---

### J11：長輸出和附件存伺服器、清理

**1. 目標**：長輸出先存在跑這一輪的那台，跟紀錄一起補上伺服器；別台讀得到；伺服器連不上時照存照讀；沒被引用、過了保存期限才清（`files-n1` S2、S4，K103）；附件沒被引用超過 1 天每晚刪（`deliver-send` S6）。

**2. 開工條件**：tools 區 F7、F8、F10；T13、T14；Q4、Q5、Q6、Q25 有答案。

**3. 只准改的範圍**：

- `crates/protocol`：新檔 `raw_api.rs`、`attachment_api.rs`。
- `crates/server/src/`：新檔 `api_raw.rs`、`api_attachments.rs`、`raw_cleanup.rs`；`cleanup.rs` 加一個呼叫；`app.rs` 加路由。
- `crates/store-server`：新檔 `raw_outputs.rs`、`attachments.rs`、`raw_refs.rs`。
- `crates/store-client`：本機的長輸出表（新檔 `raw_local.rs`）。
- `crates/sync`：新檔 `raw_upload.rs`、`raw_fetch.rs`。
- `crates/runner`：`RawStore` 的正式實作（先本機、再伺服器）。
- `migrations/server/0430_raw_outputs.sql`、`0431_attachments.sql`；客戶端遷移照 Q25。
- 測試。

**4. 定好的介面**：

- `PUT /v1/raw/{id}?bot=`（執行端），本體是原文位元組；`id` 要等於內容算出來的 id（tools 區 Q11：「r」＋SHA-256 前 16 個十六進位字），不合 → 400 `raw-id-mismatch`；已經有 → 200（同內容同 id，不重寫）。
- `GET /v1/raw/{id}?bot=` → 原文；沒有或清掉了 → 404 `tool-read-result-missing`（字「找不到這份結果：可能超過 30 天被清掉了」，tools 區已定）。
- 執行端：`read_result` 先讀本機，本機沒有再打 `GET /v1/raw`；寫本機後放進匣子，跟紀錄一起補上（`files-n1` S2）。
- `POST /v1/attachments?bot=`、`GET /v1/attachments/{digest}?bot=`：附件存伺服器（`deliver-send` S6），最大 52,428,800 位元組（tools 區 Q12）。
- 引用（照 Q5 的建議）：`trait RawReferences { fn referenced(&self, ids: &[RawId]) -> … }`，伺服器的實作看「證據裡帶這個 id」和「還沒結束的任務的紀錄裡帶這個 id」；記憶的引用由 brain 區 B16 加一個實作（E 段）。
- 清理在每晚的整理（J12 的同一個排程）：沒被引用、超過保存期限（跟細紀錄一樣，`detail_retention_days`，預設 30 天）的長輸出刪掉；空間不夠時先刪沒被引用的；沒被任何聊天或頁面引用超過 1 天的附件刪掉。
- 存放位置照 Q4：PostgreSQL 的 `bytea`。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| Bot 的電腦那一輪拿到 id；同一個 Bot 在假電腦端「Linux」那一輪 `read_result(id)` | 讀得到 | files-n1 S2 |
| 伺服器停機時 shell 輸出 50,000 字，拿到 id、讀一段；恢復；「Linux」那一輪讀 | 停機時就讀得到；恢復後補上伺服器；Linux 讀得到，id 一樣 | files-n1 S2 |
| 甲沒被引用、乙被證據引用；快轉 31 天；讀甲、乙、`r-亂給` | 甲、亂給的回找不到；乙讀得到；證據仍有乙的頭尾和雜湊 | files-n1 S4 |
| 附件沒被引用（聊天已永久刪除）；快轉 2 天 | 那個附件被刪掉 | deliver-send S6 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_files_n1_3` | T-files-n1-3 | 照表第 1 列 | 整合 |
| `t_files_n1_8` | T-files-n1-8 | 照表第 2 列 | 整合 |
| `t_files_n1_4` | T-files-n1-4 | 照表第 3 列 | 整合 |
| `t_deliver_send_4` | T-deliver-send-4 | 照表第 4 列 | 整合 |
| `raw_put_rejects_wrong_id` | — | 400 | 整合 |

**7. 不准做的**：不改 `read_result` 的字和分段（tools F8）；不做檔案卡畫面；不做附件同步到手機本機（`sync-files`，M8）。

**8. 驗收清單**：四個帶編號的測試綠；回報寫清楚記憶引用留給 B16 的介面。

---

### J12：細紀錄的保存期限

**1. 目標**：任務和證據永久保存；逐步的細紀錄預設 30 天，每晚清掉更早的；清聊天、刪 Bot（不是永久刪除）不刪任務和證據（`activity-n1` S1、S2、S5；`task-keep` S1、S2）。

**2. 開工條件**：J1（從外面讀任務和證據）；Q5、Q15 有答案。

**3. 只准改的範圍**：`crates/domain`（新檔 `detail_retention.rs`：哪些紀錄算細紀錄、要清哪些欄位）；`crates/domain/src/settings_registry.rs`（加 `detail_retention_days`）；`crates/server/src/cleanup.rs`、新檔 `detail_cleanup.rs`；`crates/store-server`（新檔 `streams_expire.rs`）；`migrations/server/0435_records_expired.sql`；測試 `apps/server/tests/pg_detail_retention.rs`。

**4. 定好的介面**（照 Q5、Q15 的建議）：

- `domain::detail_retention::detail_fields(kind: &str) -> Option<DetailFields>`：哪些種類的哪些欄位是細紀錄（Q5 的清單）。
- 清理：細紀錄超過 `detail_retention_days`（帳號設定，7–365，預設 30）→ 清掉那些欄位、保留那一列和編號，標 `expired_at`（跟永久刪除一樣不刪列，K113 ②；但讀的時候是「超過保存期限」，不是「已永久刪除」）。
- 排程：`cleanup_loop` 每次醒來看「上次整理到現在是否跨過一個整理時間」（Q15）。
- 證據裡的摘要（前後各 6,000 字、雜湊、結束碼）在 `EvidenceAdded`，不屬細紀錄，不動。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 一個任務做完；快轉 400 天 | 任務、證據、截圖都在；細紀錄不在 | activity-n1 S1、S2 |
| 預設；快轉 29 天、再 2 天 | 29 天時細紀錄在；31 天時不在，證據摘要還在 | activity-n1 S2 |
| 一個任務做完；清除聊天；刪除 Bot | 任務狀態和證據還讀得到 | activity-n1 S5 |
| 一件任務 3 步；快轉 31 天 | 任務和證據清單還在；每一步的完整輸出不在 | task-keep S1 |
| 一件普通任務；快轉 3 年 | 任務和證據清單還在 | task-keep S1 |

截圖：M7 還沒有看畫面的工具，`activity-n1-1` 的截圖由替身執行端照公開 API 傳一個附件、在證據裡引用（預期是資料）。

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_activity_n1_1` | T-activity-n1-1 | 照表第 1 列 | 整合 |
| `t_activity_n1_2` | T-activity-n1-2 | 照表第 2 列 | 整合 |
| `t_activity_n1_6` | T-activity-n1-6 | 照表第 3 列 | 整合 |
| `t_task_keep_2` | T-task-keep-2 | 照表第 4 列 | 整合 |
| `t_task_keep_3` | T-task-keep-3 | 照表第 5 列 | 整合 |
| `detail_fields_*` | — | 每種紀錄 | 單元 |

**7. 不准做的**：不做設定畫面（`T-activity-n1-3`，M8）；不做例行任務的 1 年（M9）；不做遷移（M10）；不清聊天訊息本身（聊天照 `sync-append` 的規則）。

**8. 驗收清單**：五個帶編號的測試綠。

---

### J13：伺服器停機與成批回報

**1. 目標**：用真的執行端和伺服器，驗成批回報的間隔、伺服器停機時照做、恢復後補上，三台裝置的證據一樣。

**2. 開工條件**：T14、T16、T17 在 `dev`。

**3. 只准改的範圍**：`tests/app-test/tests/pg_conn_batch.rs`、`pg_server_down.rs`；產品程式只修測出來的錯（回報列出）。

**4. 定好的介面**：不加新介面。替身手機、替身桌面照公開 API 拉紀錄，用 `domain` 算證據清單比對。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 假模型一步呼叫 10 個很快的工具 | 相鄰兩次回報隔 0.2–0.5 秒；10 個動作各有把關、要做什麼、開始、完成、證據，沒少沒重複 | conn-batch S1 |
| 伺服器停機；A 跑完 3 步；恢復 | 3 步補進紀錄流 | conn-batch S4 |
| 伺服器停機；假模型改檔 a.txt；恢復 | 停機時照做；恢復後證據有〔改檔 a.txt〕；編號連續 | trust-pipeline S7 |
| Bot 的電腦上背景指令 #1 在跑；伺服器停機、恢復；`read_terminal(id=1)` | 照常「還在執行」 | files-background S6 |
| 兩台桌面和一支手機；跑任務；清除聊天；刪 Bot1 | 三處的證據清單一樣，編號連續；清除和刪除後任務的證據還在 | activity-evidence S2 |

**6. 要寫的測試**：`t_conn_batch_1`、`t_conn_batch_3`、`t_trust_pipeline_6`、`t_files_background_9`、`t_activity_evidence_3`（各驗表上一列，整合）。

**7. 不准做的**：不為了測試加旗標或環境變數；回報間隔用真時間量，測試放 `timing` 群組。

**8. 驗收清單**：五個帶編號的測試綠。

---

### J14：從手機發起的路由與改派

**1. 目標**：從手機發起的一輪照路由規則在對的電腦跑；指名的電腦不在線改派、寫改派那一行；手機看得到「這一輪在哪台跑」，給聊天上方的離線橫幅用（`computer-route` S1、`computers-default` S5、`computers-n2` S2、`files-workdir` S3、`computers-online` S3）。

**2. 開工條件**：T14、T18 在 `dev`；Q11 有答案。

**3. 只准改的範圍**：`crates/protocol/src/turn_records.rs`（照 Q11）；`crates/runner`（寫 `TurnStarted` 時帶）；`crates/server/src/mailbox_deliver.rs`（只修錯）；測試 `tests/app-test/tests/pg_route_phone.rs`。

**4. 定好的介面**：

- `ComputerRef` 加 `device_id: Option<DeviceId>`（`serde(default)`，舊紀錄沒有）。手機用「這件任務最後一筆沒結束的 `TurnStarted.computer.device_id`」對 `GET /v1/computers` 的 `online`、`offline_reason`，畫橫幅（phone U20）。
- 其他照 T18 的介面，不加新入口。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 在手機送「ls」；假模型 `shell(command="hostname")` | 證據的電腦＝Bot 的電腦 | computer-route S1 |
| 關掉電腦 B；在 P 送「在 B 上跑 ls」 | 在 X 跑，有改派那一行 | computers-default S5 |
| A 的交辦開關關；在 A 送「跑 ls」；在 P 送「在 A 上跑 ls」 | 第一個在 A 跑；第二個改派到 X | computers-n2 S2 |
| Bot 做事的電腦＝Linux，起始資料夾 ~/proj；關掉 Linux；在手機送「pwd」 | 改派到 Bot 的電腦，回 /home/bot/bot-1，沒出錯 | files-workdir S3 |
| 一輪在 A 跑 | `TurnStarted.computer.device_id`＝A 的裝置編號 | Q11 |

**6. 要寫的測試**：`t_computer_route_1`、`t_computers_default_5`、`t_computers_n2_2`、`t_files_workdir_4`（整合）；`turn_started_carries_device_id`（整合）。

**7. 不准做的**：不改 T18 的路由規則；不做橫幅畫面（phone U20）。

**8. 驗收清單**：四個帶編號的測試綠。

---

### J15：換電腦時的工作資料夾

**1. 目標**：交到別台時，家目錄底下的工作資料夾寫成 `~/…`、原路徑也一起送；收的那台照「原路徑存在 → 自己家目錄下同樣位置 → 家目錄」用；不失敗、不問（`computers-n6` S1–S3；`files-workdir` S1）。

**2. 開工條件**：J14；tools 區的工作資料夾設定在 `dev`。

**3. 只准改的範圍**：`crates/protocol/src/mailbox_api.rs`（信裡加 `WorkdirHint { original, home_relative }`）；`crates/domain`（新檔 `workdir_map.rs`，純函式）；`crates/server/src/mailbox_deliver.rs`；`crates/runner`、`crates/computers`（收的那台選資料夾）；測試。

**4. 定好的介面**：

- `domain::workdir_map::hint_of(original: &str, home: &str) -> WorkdirHint`。
- `computers` 裡選：`pick_workdir(hint: &WorkdirHint, exists: impl Fn(&str) -> bool, home: &str) -> String`。
- Bot 的電腦一律 `/home/bot/bot-<編號>`，不看提示（`files-workdir` S1）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 工作資料夾是 A 上的 `/Users/alice/Documents`；B 是 Linux、有 `~/Documents`；在 P 送「在 B 上列出工作資料夾」 | 在 B 的 `/home/alice/Documents` 跑 | computers-n6 S1、S2 |
| B 沒有 `~/Documents` | 在 B 的家目錄跑；沒失敗、沒問 | computers-n6 S3 |
| Linux 的起始資料夾＝~/proj；在 Linux 和手機各叫 `pwd` | Linux 回 ~/proj 的完整路徑；手機那一輪回 /home/bot/bot-1 | files-workdir S1 |

**6. 要寫的測試**：`t_computers_n6_1`、`t_computers_n6_2`、`t_files_workdir_2`（整合）；`pick_workdir_*`（單元）。

**7. 不准做的**：不做設定畫面（`T-files-workdir-3`，M8）；不做遷移（`T-files-workdir-6`，M10）。

**8. 驗收清單**：三個帶編號的測試綠。

---

### J16：電腦連不上、執行端重開、管理員上限

**1. 目標**：電腦連不上時模型收到原因、那一步記「沒執行」；執行端重開後背景指令找得回來或明說已結束；背景指令上限由管理員設定；伺服器被攻破塞進來的危險指令由電腦自己的底線擋。

**2. 開工條件**：T7、T14；tools 區 F11、F12；trust 區的底線（`sandbox-n4`）；phone 區 testenv 的 `/control/computer/unreachable`；Q19、Q25 有答案。

**3. 只准改的範圍**：`crates/runner`、`crates/store-client`（背景指令的名單存本機，客戶端遷移照 Q25）；`apps/agent`；`crates/server/src/`（新檔 `api_admin_limits.rs`，`/v1/server/status` 加上限）；`crates/protocol`；`tests/testenv/src/` 新檔（直接寫信箱的控制 API）；測試 `tests/app-test/tests/pg_agent_restart.rs`、`pg_admin_limits.rs`、`pg_mailbox_forged.rs`。

**4. 定好的介面**：

- `POST /v1/admin/limits`（伺服器管理員），Body `{ background_per_computer: u32 }`；存在現有的伺服器設定；`GET /v1/server/status` 多回 `limits.background_per_computer`，執行端讀這個值，沒設時用 `BACKGROUND_LIMIT_DEFAULT = 24`（tools 區 F12）。
- 背景指令的名單（編號、指令、最後的輸出）存本機副本；執行端重開後讀回：還在跑的照讀；已經停的回 `tool-terminal-ended-on-restart { id, last }`「背景指令 #N 在 AnyBot 重新啟動時結束了（最後的輸出：…）」（`files-background` S6）。
- 電腦連不上：照 `computer-ops` S4 的字（tools 區已定的代碼），那一步 `ToolNotRun`。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| Bot 的電腦開著；假模型 `shell(command="ls")`，同時讓 Bot 的電腦連不上 | 模型收到「Bot 的電腦現在連不上：…」；那一步記「沒執行」 | computer-ops S4 |
| 假電腦端 Linux 上背景指令 #1 在跑；關掉、打開；`read_terminal(id=1)` | 「背景指令 #1 在 AnyBot 重新啟動時結束了（最後的輸出：…）」 | files-background S6 |
| 管理員設上限＝2；假模型連續開 3 個背景指令 | 第 3 個收到上限已滿的字 | computer-caps S5 |
| 替身伺服器直接把「跑 rm -rf ~」的信放進家裡 Linux 的信箱；打開 Linux | 指令被拒，原因「一律不允許」；家目錄沒變 | sandbox-n4 S4 |

`T-sandbox-n4-5` 的「伺服器被攻破」：J16 在 `tests/testenv/src/` 加一個新檔的控制 API，直接把信寫進那台的信箱表（列的步驟就是這樣寫的）；不改產品的伺服器、不加產品入口。

**6. 要寫的測試**：`t_computer_ops_4`、`t_files_background_10`、`t_computer_caps_5`、`t_sandbox_n4_5`（整合）。

**7. 不准做的**：不做 Bot 的電腦的正式映像（M9）；不做「重新啟動這個 Bot」（M8）。

**8. 驗收清單**：四個帶編號的測試綠。

---

### J17：「我的電腦」＝最後打字的那台

**1. 目標**：伺服器記這個人最後在哪台電腦打字；手機的訊息不改；交給 Bot 的指示寫「使用者正在用：〔名字〕」或「不知道使用者的電腦是哪台」；執行服務的一輪不把伺服器或 Bot 的電腦當成使用者的電腦（`computers-n7` S1–S3）。

**2. 開工條件**：T12（`POST /v1/bots/{bot}/messages`）、T13。

**3. 只准改的範圍**：`crates/server/src/`（T12 的入口收到電腦憑證送的訊息時記一筆；新檔 `user_computer.rs`）；`crates/store-server`（新檔 `user_computer.rs`）；`crates/protocol/src/mailbox_api.rs`（信裡加 `user_computer: Option<String>`）；`crates/runner/src/fixed_blocks.rs`；`locales/`；`migrations/server/0440_user_computer.sql`；測試 `tests/app-test/tests/pg_user_computer.rs`。

**4. 定好的介面**：

- 表 `user_computer(account, device_id, typed_at_ms)`；只有電腦憑證送的訊息會更新（手機、執行服務不會）。
- 送「一輪」的信時帶那台的名字（不是編號），執行端寫進指示：`turn-user-computer { computer }`「使用者正在用：{ $computer }」；沒有 → `turn-user-computer-unknown`「不知道使用者的電腦是哪台」。
- 執行服務 X 跑的一輪照樣用同一個值；不寫 X 或伺服器。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 在電腦 A 送「hi」；在手機 P 送「在我的電腦上開計算機」 | 模型收到的請求有「使用者正在用：A」；這一輪指定 A | computers-n7 S1、S2 |
| alice 從沒在電腦上打過字；在 P 送「在我的電腦上跑 ls」 | 模型收到「不知道使用者的電腦是哪台」 | computers-n7 S2 |
| 在 P 送「我的電腦現在幾點」（在 X 跑） | 請求裡沒有把 X 或伺服器當成使用者電腦的說法 | computers-n7 S3 |

「這一輪指定 A」：T18 已經讓訊息裡指名的電腦算數（turn 區 Q12）；「我的電腦」換成那台的名字由 T18 的指名規則接（E 段）。

**6. 要寫的測試**：`t_computers_n7_1`、`t_computers_n7_2`、`t_computers_n7_3`（整合）。

**7. 不准做的**：不做遷移 `person-computer.txt`（`T-computers-n7-4`，M10）。

**8. 驗收清單**：三個帶編號的測試綠。

---

### J18：只有某台連得到的模型

**1. 目標**：執行端試連私有位址的模型、回報伺服器；用這種模型的 Bot 只排在連得到的那台；都不在線就卡住（模型連不到）、不改派、不偷換雲端模型；那台上線後 60 秒內自動接著做（`computers-n5` S1–S4、`mailbox-route` S1、`mailbox-reassign` S6）。

**2. 開工條件**：T8（`ModelUnreachable`）、T14、T18；Q1 有答案。

**3. 只准改的範圍**：`crates/runner`、`apps/agent`、`apps/runner-service`（試連，新檔 `model_probe.rs`；HTTP 試連放 `models`）；`crates/server/src/`（新檔 `api_model_reach.rs`、`model_reach_route.rs`）；`crates/domain/src/mailbox_route.rs`；`crates/store-server`（新檔 `model_reach.rs`）；`crates/protocol`；`locales/`；`migrations/server/0445_model_reach.sql`；`tests/harness`（假模型記請求來源）；測試 `tests/app-test/tests/pg_model_reach.rs`。

**4. 定好的介面**：

- 執行端在開始用之前和每 5 分鐘，對 Bot 設定裡私有位址（本機、區網、tailnet）的模型試連，1.5 秒連不上算連不到；打 `POST /v1/computers/model-reach`，Body `{ results: [{ base_url, reachable }] }`。
- 表 `model_reach(device_id, base_url, reachable, checked_at_ms)`。
- 路由：Bot 的模型是私有位址 → 候選只留「在線、而且最新一筆 `reachable = true`」的電腦；一台都沒有 → 不開始，寫 `ModelUnreachable`（任務＝卡住，細項模型連不到）、主線寫 `model-unreachable-no-computer { bot, host, model }`、推播（J3 的 `NotifyOwner`）。
- 自動接著做：那台上線、回報連得到時，伺服器把卡住的那一輪重新送進它的信箱；60 秒內開始（執行端上線後第一次回報就在 20 秒的請求間隔內）。
- 不經別台轉送、不經伺服器代連（伺服器不發模型請求，`spec/07` §3 本來就不准）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 小幫手用 `http://10.0.0.5` 的模型，只有 B 連得到；在 P 送一句 | 在 B 跑；模型收到的請求來自 B；伺服器沒有轉送 | computers-n5 S1、S2 |
| 同上 | 在 B 跑 | mailbox-route S1 |
| 關掉 B；在 P 送一句 | 沒有改派到 X；任務＝卡住（模型連不到） | mailbox-reassign S6 |
| 接著打開 B | 60 秒內這一輪在 B 開始 | computers-n5 S4 |

「模型收到的請求來自 B」：harness 的假模型現在沒有記請求從哪裡來，J18 在 `tests/harness` 加上（記連線的對方位址；B 和 X 用不同的來源位址，測試在建構時給），不改產品程式。

**6. 要寫的測試**：`t_computers_n5_1`、`t_computers_n5_3`、`t_mailbox_route_3`、`t_mailbox_reassign_6`（整合）。

**7. 不准做的**：不做 `T-computers-n5-2`（phone 區排 M10+）；不偷換雲端模型。

**8. 驗收清單**：四個帶編號的測試綠。

---

### J19：連線數上限與相信代理

**1. 目標**：同一個公開位址同時最多 48 條連線，私有網段、tailnet、本機不限；`X-Forwarded-For` 只看最後一個、最長 64 字；設了暗號只信帶暗號的；只准本機的介面帶代理標頭回「找不到」（`safety-n10` S3、`safety-n11` S1–S3、`server-n2` S2）。

**2. 開工條件**：無。Q21 有答案。

**3. 只准改的範圍**：`crates/domain/src/source_addr.rs`；`crates/server/src/`（`serve_all.rs` 接受連線處、`req_source.rs`；新檔 `conn_limit.rs`）；伺服器設定（代理暗號）；`migrations/server/0425–0429`（預留）；測試 `apps/server/tests/pg_conn_limit.rs`、`pg_proxy_trust.rs`。

**4. 定好的介面**：

- `ConnLimiter`：照連線本身的對方位址算（不看代理標頭）；私有網段、`100.64.0.0/10`（tailnet）、本機不算；第 49 條在接受後馬上關。計數照 Q21。
- `domain::source_addr::source_address(peer, forwarded, trusted_proxy, secret_ok)`：只取最後一個值；超過 64 字回 `peer`；有設暗號而 `secret_ok == false` 回 `peer`。
- 只准本機的介面：請求帶 `X-Forwarded-For` → 404。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 同一公開位址開 49 條；127.0.0.1 開 100 條 | 第 49 條被拒；本機的都通 | safety-n10 S3 |
| 同一公開位址開 49 條；192.168.x 開 100 條 | 第 49 條被拒；區網的都通 | server-n2 S2 |
| 相信 10.0.0.2；`X-Forwarded-For: 9.9.9.9, 1.2.3.4`；65 字的值 | 1.2.3.4；10.0.0.2 | safety-n11 S1 |
| 相信代理且設了暗號；同網路另一個程式帶標頭沒帶暗號 | 不信 | safety-n11 S2 |
| 只准本機的介面帶 `X-Forwarded-For` | 找不到 | safety-n11 S3 |

「公開位址」的連線在測試裡用 harness 的代理改對方位址做（現有的 `t_safety_n11_1` 怎麼造，照它）；不改正式程式的行為。

**6. 要寫的測試**：`t_safety_n10_4`、`t_server_n2_3`、`t_safety_n11_2`、`t_safety_n11_3`、`t_safety_n11_4`（整合，`ports` 群組）。

**7. 不准做的**：不開 `SO_REUSEPORT` 之類為測試改正式程式；不做安裝檢查的提醒（`T-safety-n11-5`，M10+）。

**8. 驗收清單**：五個帶編號的測試綠。

---

### J20：代理和上傳大小

**1. 目標**：`server-n1` S2 的代理列和 `server-n2` S3 的上傳大小。

**2. 開工條件**：J19。

**3. 只准改的範圍**：`crates/server/src/req_write.rs`、`limits.rs`、`req_source.rs`；測試 `apps/server/tests/pg_upload_limits.rs`、`pg_proxy_trust_b.rs`。

**4. 定好的介面**：沒登入的請求本體上限 256 KB；登入後 64 MB（`server-n2` S3）；慢的上傳不因為時間被切（2 KB/s 上傳 40 MB 要成功）。代理的規則同 J19。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 相信 10.0.0.9；從 10.0.0.9 送 `6.6.6.6, 1.2.3.4`；從 10.0.0.8 送同樣的 | 1.2.3.4；10.0.0.8 | server-n1 S2 |
| 相信 10.0.0.9＋暗號；從 10.0.0.9 送不帶暗號的 | 10.0.0.9 | server-n1 S2 |
| 沒登入送 257 KB；登入後送 65 MB；2 KB/s 上傳 40 MB | 前兩個被拒；慢的沒被切 | server-n2 S3 |

**6. 要寫的測試**：`t_server_n1_2`、`t_server_n1_3`、`t_server_n2_4`（整合；`n2-4` 用 harness 的限速，跟著假時鐘，見 `docs/testing.md`）。

**7. 不准做的**：不做管理頁的警告（`T-server-n1-4`，M8）。

**8. 驗收清單**：三個帶編號的測試綠。

---

### J21：形式驗證一：同步裁判

**1. 目標**：在 Rust 測試裡手寫窮舉（狀態機＋寬度優先搜尋），證明裁判的規則在所有先後順序下都成立。

**2. 開工條件**：J5（`domain::card_answer::decide_card_answer`）。Q8 有答案。

**3. 只准改的範圍**：新套件 `tests/formal`（加進根 `Cargo.toml` 的成員；只當開發用，依賴 `anybot-domain`、`anybot-protocol`）；`.config/nextest.toml`（只加這個套件的逾時，如果需要）。

**4. 定好的介面**：

- `tests/formal/src/explore.rs`：`fn explore<M: Model>(model: &M, budget: usize) -> Result<Stats, Counterexample<M>>`，用 `HashSet` 記看過的狀態；超過 `budget` 回錯誤，不悄悄當成通過。
- `trait Model { type State: Clone + Eq + Hash; type Step; fn init(&self) -> Self::State; fn steps(&self, s: &Self::State) -> Vec<Self::Step>; fn apply(&self, s: &Self::State, st: &Self::Step) -> Self::State; fn check(&self, s: &Self::State) -> Result<(), String>; }`。
- 轉移裡伺服器的決定一律呼叫 `decide_card_answer`、`card_is_waiting`（真的產品程式），環境（畫面多舊、回答多晚到、重送、斷線）由模型列舉。
- 重送用「在路上的回答」集合表示：送到可以發生很多次，狀態仍是有限的（Q8）。
- 反例要印出整串步驟（英文）。

**5. 行為表**：

| 模型 | 要成立的 | 出處 |
|---|---|---|
| 2 台裝置、1 個執行端、伺服器；卡片出現、換下一張、兩台按允許／拒絕、斷線重送 | 每張卡最多一個答案；答案只落在按的人看到的那張 | sync-referee S1、S2 |
| 上面再加「沒帶編號的回答」 | 沒帶編號的回答永遠不算 | phone-cards S1、S2 |
| 桌面、手機、通知三處同時答同一張卡，加換卡、停止 | 每張卡最多收下一個答案；答案一定是按下時看到的那張 | cards-question S9 |
| A 在線／離線／重連、按繼續、伺服器停機／恢復 | 同一件任務不會同時有兩台在跑；被停止的一輪不會被接回 | T-task-resume-8 |

`T-task-resume-8` 的轉移用 turn 區 T7 的接續規則和信箱的領信規則（`domain::mailbox_letter`）；T7 沒合時這一列先不做，回報寫明。

**6. 要寫的測試**：`t_sync_referee_1`、`t_phone_cards_1`、`t_cards_question_9`、`t_task_resume_8`（形式驗證）；`explore_finds_planted_bug`（把規則故意改錯，要找到反例）。

**7. 不准做的**：不引進 TLA+、`stateright` 等新工具（使用者已定）；不為了跑得完把規則簡化成跟產品不一樣。

**8. 驗收清單**：四個帶編號的測試綠；每個在 CI 上 30 秒內跑完（超過就照 Q8 拆）。

---

### J22：形式驗證二：只放行、只記看到的

**1. 目標**：`safety-n1` 永遠成立的 ①②③⑤⑥。

**2. 開工條件**：J7、J8、J21。

**3. 只准改的範圍**：`tests/formal`。

**4. 定好的介面**：模型照 `T-safety-n1-1` 的前提：1 個使用者、2 個 Bot、每個 Bot 依序 4 張卡（2 張允許請求、1 張需要處理、1 張問題卡）；能回答的地方：2 台桌面、1 支手機頁面、手機通知、1 個聊天頻道；畫面可以任意舊；回答可以任意晚到、重送；手機頁面和 1 台桌面可能是不送鑰匙的舊版；卡可以被答掉、停止、取消、換卡、作廢。頻道送來的東西在模型裡是一種步驟，伺服器沒有入口，所以永遠不改卡。

**5. 行為表**：

| 編號 | 要成立的 |
|---|---|
| T-safety-n1-1 | ① 只放行按下時看到的那張 |
| T-safety-n1-2 | ② 一律允許、這段對話都允許記的是按下時那張卡上寫的 |
| T-safety-n1-3 | ③ 通知的允許／拒絕不會變成我完成了／跳過，也不回答問題卡、計畫卡 |
| T-safety-n1-5 | ⑤ 找得到「桌面、手機頁面、通知的回答都各自交到一張卡上」的走法 |
| T-safety-n1-6 | ⑥ 頻道送來的東西不改卡片狀態 |

**6. 要寫的測試**：`t_safety_n1_1`、`2`、`3`、`5`、`6`（形式驗證）。`t_safety_n1_5` 是「找得到」：搜尋時遇到符合的狀態就成功，找完都沒有就失敗。

**7. 不准做的**：同 J21。

**8. 驗收清單**：五個帶編號的測試綠。

---

### J23：形式驗證三：只答一次、按下放開、執行端離線

**1. 目標**：`safety-n1` ④，加上按下與放開分兩步、主任務和支線各一張卡、執行端離線。

**2. 開工條件**：J7、J8、J21。

**3. 只准改的範圍**：`tests/formal`。

**4. 定好的介面**：在 J22 的模型上加：兩個地方同時送回答、重送（`n1-4`）；桌面按下與放開分成兩步、中間可以換卡，主任務和支線各一張卡（`n1-7`，支線在模型裡只是同一個 Bot 的第二件任務，各帶各的鑰匙；`/btw` 指令本身未排定）；執行端離線、回答先到伺服器（`n1-8`）。

**5. 行為表**：

| 編號 | 要成立的 |
|---|---|
| T-safety-n1-4 | ④ 一張卡最多被一個回答通過或拒絕 |
| T-safety-n1-7 | ①② 在按下放開分兩步、主線支線各一張時仍成立 |
| T-safety-n1-8 | ①④；執行端上線後收到的回答一定是那張卡的 |

**6. 要寫的測試**：`t_safety_n1_4`、`t_safety_n1_7`、`t_safety_n1_8`（形式驗證）。

**7. 不准做的**：同 J21。

**8. 驗收清單**：三個帶編號的測試綠。

---

## D) 要問的問題

### 第一批：擋住切片的

1. **Q1 `computers-n5` 三份規劃排法不一樣。**
   - tools 區排 M7j；turn 區把 `T-mailbox-route-3`、`T-mailbox-reassign-6` 排 M9、T18 也排除它；phone 區把 `T-computers-n5-2` 排 M10+。
   - 這個功能是 turn 區 T8「模型連不到」和 K109 ⑧「自動接著做」的另一半；只做一邊會有「卡住了，但沒人會接回」的狀態。
   - **建議**：`n5-1`、`n5-3`、`mailbox-route-3`、`mailbox-reassign-6` 在 M7j 的 J18 一起做；`n5-2` 照 phone 區排 M10+。擋住：J18。

2. **Q2 `T-safety-n1-17` 和 `T-safety-n8-3` 步驟一樣、預期相反。**
   - 兩列都是「關掉電腦〔工作 Mac〕；在手機按允許；打開」。`n1-17`（`safety-n1` S8）要上線後執行一次；`n8-3`（`safety-n8` S4）要回答沒用上、重新出卡。README 的「關掉／打開 電腦」只寫「讓某台電腦離線、上線」。
   - **建議**：`n1-17` 是「離線但執行端還活著」（用斷網做），回答留在伺服器、上線後交；`n8-3` 是「執行端被關掉」（程序結束再啟動），接著做時卡作廢。規格不用改，測試照這個區分，回報寫明。擋住：J7。

3. **Q3 卡片的鑰匙由誰發、怎麼送到裝置？**
   - `safety-n1` S1 說「伺服器…每張卡出現時發一把新的、猜不到的鑰匙」；現在的卡片只有執行端給的 `card_id`。
   - **建議**：伺服器收下 `CardShown` 時產生 16 位元組亂數，存 `card_keys`，同一筆交易寫一筆伺服器的紀錄 `card_key_issued { card_id, key }` 到那條 Bot 流（擁有者的裝置都看得到）；回答一律帶鑰匙。另一個做法是讓執行端用亂數產生 `card_id` 當鑰匙，比較簡單但跟 S1「伺服器發」不一樣。擋住：J5。

4. **Q4 長輸出和附件存在伺服器的哪裡？**
   - K118 ① 說啟動檔剛好兩行（資料庫、主鑰匙），伺服器沒有資料夾的設定；`blob` 套件現在是空殼。
   - **建議**：存在 PostgreSQL 的 `bytea`（長輸出每份最多 20 MB、附件最多 50 MB），`blob` 套件先不用；之後規模部署再換物件儲存。擋住：J11。

5. **Q5 K103 的「被引用」和 `activity-n1` 的「細紀錄」各指哪些？**
   - K103：被任務、證據、記憶引用的原始輸出就留。`task-keep-2` 又要 31 天後「每一步的完整輸出不在了」。如果每一步的完整輸出都算被任務引用，兩條就衝突。
   - **建議**：
     - 「被引用」＝證據裡帶這份長輸出的 id、還沒結束的任務的紀錄裡帶這個 id（`context-ref-2` 的進行中任務）、記憶裡帶這個 id（B16）。已經結束的任務只靠細紀錄帶著的 id 不算。
     - 「細紀錄」＝`ModelAsked` 的請求內容、`ToolIntent` 的完整參數、`ToolCompleted`／`ToolFailed` 的完整輸出欄位。`EvidenceAdded`（含前後各 6,000 字、雜湊、結束碼）、任務建立、狀態、卡片、交付報告、聊天訊息和 Bot 的回答都不算。
     - `activity-n1` S2 列的「插話、停止」指細紀錄裡的那份副本；聊天裡的訊息本身不因為 30 天被清。
     - 清的方法：清欄位、留列和編號，標「超過保存期限」（跟永久刪除的「已永久刪除」分開）。
   - 擋住：J11、J12。

6. **Q6 `T-files-n1-4` 在哪裡驗？** tools 區排 M7.5（要記憶的引用），brain 區寫「M7（工具區）」。**建議**：在 J11 做清理和引用的介面（`RawReferences`），任務和證據的引用 J11 做，記憶的引用由 B16 加一個實作；這一列的預期不需要記憶就能驗完，在 J11 拿編號。B16 合進來時要補一個「被記憶引用的不清」的測試。擋住：J11。

7. **Q7 跟別份分類不同的列，請一起確認。**
   - 從別處拉進 M7j（預期都是資料，可以用替身）：`T-deliver-confirm-6`（tools 排 M8）、`T-cards-n1-1`–`3`（phone 排 M8）、`T-account-push-1`、`8`（phone 排 M8）、`T-mailbox-route-3`、`T-mailbox-reassign-6`（turn 排 M9，見 Q1）、`T-task-keep-2`、`3`（turn 交給工具區，工具區沒收）。
   - 從 M7j 推出去：`T-computers-default-3`、`T-computers-kinds-3` → M9（tools 區 D-1 已定改成交辦）；`T-activity-who-4` → M9（/goal）；`T-safety-n1-13` → 未排定（`/btw`）；`T-safety-n8-4`、`T-claims-mark-2`、`T-files-rewind-9`、`T-computers-online-6` → M8；`T-account-push-3` → M9。
   - **建議**：照上面。

8. **Q8 形式驗證的狀態會不會太多？**
   - `T-safety-n1-1` 的模型（2 個 Bot × 4 張卡 × 5 個回答的地方 × 畫面任意舊 × 回答任意晚到、重送）直接展開可能很大。
   - **建議**：
     - 重送用集合表示（送到可以發生很多次，狀態仍有限）；看過的狀態用雜湊去重；兩個 Bot 對稱時只展開一種排列。
     - 不限制先後順序（規格寫窮舉所有順序）。
     - 每個測試要在 CI 30 秒內跑完；跑不完時把同一列拆成幾個子模型分別窮舉（例如每個 Bot 各自、再加跨 Bot 的那一段），回報寫明怎麼拆、為什麼仍然等於原本的模型。
   - 擋住：J21–J23。

9. **Q9 👍 用新的紀錄種類，還是現有的 `DeviceReport { kind: ThumbsUp }`？**
   - phone 區 Q12 已定紀錄種類 `thumbs_up`、`thumbs_up_removed`、內容 `{answer_seq}`。但 `protocol` 已經有 `DeviceReportKind::ThumbsUp`、`ThumbsDown`，`domain` 的 `report_device` 不管任務做完沒都設 `user_confirmed`，跟 `cards-n1` S2 不合。規格沒有 👎。
   - **建議**：照 Q12 用新種類（`KnownTaskEventKind::ThumbsUp { answer_seq }`、`ThumbsUpRemoved { answer_seq }`）；把 `DeviceReportKind` 的 `ThumbsUp`、`ThumbsDown` 拿掉（現在沒有任何程式寫它們）；`domain` 改成按的那一刻任務已完成才算。擋住：J1、J9。

10. **Q10 裝置回報要加哪些欄位？**
    - `deliver-confirm` S1–S3 要「哪台、時間、做了什麼」和「顯示失敗加原因」；現在 `DeviceReport` 只有 `kind`、`file`，紀錄的 `device_id` 是紀錄自己的編號，不是裝置。
    - **建議**：`DeviceReport { kind, file, device, reason }`，`file` 是 `FileShared.digest`；`DeviceReportKind` 加 `DisplayFailed`；伺服器檢查 `device` 等於呼叫的裝置；每台每種只記第一次由伺服器在收的時候擋（`duplicate`），表 `delivery_reports` 用唯一鍵擋同時到的。擋住：J9。

11. **Q11 手機怎麼知道「正在跑這個對話的那台」？**
    - `computers-online` S3 的橫幅要知道是哪台；`TurnStarted.computer` 是 `ComputerRef { name, platform }`，只有名字，改名後對不上。
    - **建議**：`ComputerRef` 加 `device_id: Option<DeviceId>`（`serde(default)`），執行端寫 `TurnStarted` 時帶；手機用它對 `GET /v1/computers`。擋住：J14、phone U20。

### 第二批：不擋開工，做到那裡要答

12. **Q12 「桌面在最前面、90 秒內有輸入」「手機頁面開著那個 Bot 的聊天」伺服器怎麼知道？** 規格沒給入口。**建議**：`POST /v1/devices/activity { foreground, last_input_at_ms, open_bot }`，客戶端在狀態變了時送、在前景時順著 20 秒的請求間隔送（`conn-heartbeat` S1）；存資料庫，每台一列（K118 ⑭「狀態放資料庫」）。擋住：J4。

13. **Q13 每支手機的推播種類存哪？** `account-push` S1 說每支手機自己選，`R48` 說沒設過＝全部、設成空的＝都不要。**建議**：存在那支手機的推播訂閱上（`kinds` 欄可以是空值），`PUT /v1/push/subscriptions/{id}/kinds` 改。擋住：J4。

14. **Q14 Web Push 用什麼函式庫？聯絡方式（`sub`）怎麼帶？**
    - **建議**：用 workspace 已經有的 `sad-web-push`（`default-features = false`，只用它的加密和簽章，HTTP 用 `channels` 自己的 reqwest）；harness 的假推播已經用它解密。寫程式的人在回報裡寫它的維護者、最近更新、RustSec 紀錄。
    - 聯絡方式：`server-n4` S5 要安裝時問、管理頁可改，那兩個畫面在 M8／M10；M7j 先讀伺服器設定裡的值，沒有就不帶 `sub`。
    - 擋住：J3。

15. **Q15 「每晚整理一次」的「每晚」怎麼算？** 伺服器沒有時區設定。**建議**：用伺服器的 `Clock` 每天 03:00 UTC 跑一次；伺服器啟動時如果上一次整理已經超過 24 小時，馬上補跑一次。測試快轉時鐘照樣觸發。擋住：J11、J12。

16. **Q16 一律允許清單、這段對話都允許怎麼存？** trust 區 Q5 定「伺服器存、同步到每個執行端」，沒說格式。整份清單當一個設定值，兩處同時改會互相蓋掉。**建議**：用帳號流的紀錄：伺服器寫 `always_allow_added`、`conversation_allow_added`，裝置寫 `always_allow_removed`；清單由紀錄依序折出來；開關（`direct_mode`、`yolo`、`smart_seconds`）走現有的設定同步。擋住：J6。

17. **Q17 `typing` 的 `thinking` 是開關還是字？** phone 區 Q2 的 Body 寫 `{stream, task_id, text, thinking}`。**建議**：`thinking: String`，到目前為止的思考內容（沒有時是空字串）；手機出字前看到 `thinking` 不是空的就顯示「思考中…」（`chat-steps` S5）。

18. **Q18 `T-computers-online-6`（同一個登入開兩份）放哪？** 預期是另一份「顯示」的字和它自己的紀錄檔。**建議**：排 M8，跟桌面客戶端一起；伺服器那半（後連上的留著，前一份收到 `computer-login-in-use`）也一起做，因為現在沒有客戶端會用到。

19. **Q19 `T-computer-ops-4` 的「斷網（Bot 的電腦）」怎麼做？** K119 ⑲ 說電腦介面現在只在程式裡，沒有網路可以斷。**建議**：用 phone 區 testenv 的 `/control/computer/unreachable` 讓假電腦端回「連不上」；這就是這一列要驗的流程（模型收到原因、那一步記沒執行）。

20. **Q20 `T-trust-verify-7` 的「等驗收：電腦不在線」由誰做？** trust 區寫「邏輯在 M7h」（turn 區 T9、T10），M7j 只補「沒有推播」和伺服器的在線狀態。**建議**：J8 等 T10 合了再拿這一列；T10 沒有「等驗收（電腦不在線）」的細項時，寫成描述性的測試、回報給 turn 區。

21. **Q21 48 條連線的計數放哪？** `safety-n10` S4 說「計數存在伺服器的資料庫、副本共用、重開不歸零」。但「同時開著的連線」重開後本來就全部斷了，存資料庫還要處理當掉時沒扣回去的數。**建議**：同時連線數放在每個伺服器程式自己的記憶體（個人部署只有一個副本）；S4 照舊管登入、註冊這些「5 分鐘幾次」的計數。這是對 S4 的解讀，請確認。擋住：J19。

22. **Q22 第 5 組沒人排的伺服器列**（A.3 列的 `T-sync-seq-1`、`T-server-n3-1`–`3`、`T-server-n18-1`–`4`、`T-server-n15-1`、`2` 等）。**建議**：`T-sync-seq-1` 加進 J13（同樣是伺服器停機和多台）；`server-n3`、`server-n18`、`server-n15` 另寫一份「伺服器維運」的規劃排 M8；其餘照 phone 區的分類。

23. **Q23 「沒有用上」怎麼寫進聊天？回答種類不合時寫什麼？**
    - **建議**：伺服器寫任務事件 `CardAnswerUnused { card_id, source, reason }`，畫面照 `reason` 的代碼顯示 J5 列的四句。
    - 回答種類不合（`safety-n1` S2 的「通知送來一律允許」「需要處理的卡收到允許」）規格沒給字。建議用「那張卡不能這樣回答，回答沒有用上。」（`card-unused-wrong-kind`），請確認。

24. **Q24 推播的「標題 80 字、內文 200 字」截的是哪一段？**
    - K91、K92 說後端只回代碼和參數，所以推播的字由手機的 Service Worker 照語言組；但 `T-server-n4-4` 是伺服器這邊組一則推播。
    - **建議**：伺服器截參數（Bot 名字當標題最多 80 字、內文的變動部分最多 200 字、問題 100 字、標記只留英數和 `-` 最多 40 字、圖示數字最多 999），`T-server-n4-4` 驗這個函式；Service Worker 組好字之後再截一次 200 字（phone 區做）。

25. **Q25 客戶端遷移的號碼。** 這份只分到伺服器的 0400–0449；J11（本機的長輸出）、J16（背景指令名單）要客戶端遷移。已用的：brain 0040–0054、turn 0200–0204。**建議**：J11 用客戶端 0060–0064、J16 用 0065–0069。

---

## E) 跟其他區的介面

**回合和任務（turn 區）**

- **這份要用的**：
  - T12 的 `POST /v1/bots/{bot}/messages` 和叫醒（J10、J17）；J17 在 T12 的入口記「最後打字的電腦」，要 T12 在收到電腦憑證送的訊息時呼叫一個鉤子。
  - T13 的領信和執行服務、T14 的 `apps/agent`（J2 的執行端半段、J6、J7、J9、J11、J13–J18）。
  - T15 的答案送回執行端（J7）；T16 離線時自己發起的一輪（J13）；T17 的全路徑（J13）。
  - T18 的路由（J14、J15、J17、J18）：J17 的「我的電腦」換成名字後走 T18 的「訊息裡指名電腦」；J18 在 T18 的候選清單上再濾「連得到模型」。
  - T19 的 `trait NotifyOwner`：J3 給 Web Push 的實作。
  - T7 的接續（J7 的 `n8-3`、J16、J21 的 `task-resume-8`）；T8 的 `ModelUnreachable`（J18）；T9、T10 的驗收（J8 的 `trust-verify-7`）。
- **這份給的**：`GET /v1/tasks/{task}/status`（J1），T19、T20 要讀狀態也可以用 `domain` 的同一個函式。

**信任和卡片（trust 區）**

- **這份要用的**：S2 的 `CardShown`（`kind`、`buttons`、`reason`）、S3、S4 的 `TrustSettings` 和 `AlwaysEntry`、S5 的「剛拒絕過」、S8 的交付報告格式、證據的把關紀錄格式。
- **這份給的**：
  - 卡片鑰匙和來源（J5）：`decide_card_answer` 放在 `domain`，trust 區 Q20 的「按鈕不在卡上就擋」做在這裡。
  - `CardAnswered.origin` 加 `Notification`（J8），證據寫「你按的：允許這次（通知）」，字由 trust 區的代碼出。
  - `TrustSettings` 的正式實作（J6）讀本機副本。
  - 形式驗證（J21–J23）取代 trust 區 Q8 的「暫不能測」。

**工具和交付（tools 區）**

- **這份要用的**：F7、F8 的 `RawStore`、`read_result`（J11）；F10 的 `send_file`、`FileShared`、附件（J9、J11）；F11、F12 的背景指令和 `BACKGROUND_LIMIT_DEFAULT`（J16）；工作資料夾設定（J15）；`computer-ops` S4 的字（J16）。
- **這份給的**：伺服器和客戶端的長輸出、附件存放（J11）；交付確認（J9）和報告那一段（`deliver-confirm` S6）；管理員的背景指令上限（J16）；執行端重開後背景指令找回（J16）。

**大腦和上下文（brain 區）**

- B11：J13、J14 之後，「畫面」照 Q-28 讀 API；這份不做 B11 的列。
- B12：J11 給「向伺服器拿原文」，`ref-3` 用現有的 `/v1/streams/record` 回 410 `record-source-purged`，執行端把它轉成「來源已永久刪除」。
- B16：要實作 J11 的 `RawReferences`（被記憶引用的長輸出不清），並補一個測試。
- 記憶伺服器（B16–B19，遷移 0350–0364）不在這份。

**手機畫面（phone 區）**

- 即時通道：`POST /v1/streams/typing`、SSE `typing`（J2）；`thinking` 照 Q17。
- 任務狀態：`GET /v1/tasks/{task}/status`（J1），U10、U11、U19 用。
- 👍：`thumbs_up`、`thumbs_up_removed`（J1）。
- 交付確認：裝置用 `/v1/streams/append` 寫 `DeviceReport { kind, file, device, reason }`（J9），U19 在顯示和點開時各送一次。
- 離線橫幅：`TurnStarted.computer.device_id`（J14）對 `GET /v1/computers`（已經有）。
- 推播：`GET /v1/push/server-key`、`POST /v1/push/subscriptions`（J3）、推播內容 `CardPushPayload`（J8）、`POST /v1/devices/activity`（J4）；Service Worker 和設定畫面在 M8。
- 卡片回答要帶 `card_key`、`source`（J5），鑰匙從 `card_key_issued` 紀錄拿。
- 管理者名稱（`admin_account`）手機線已經做了，這份不再做。

**監督者要決定的**

- D 的 Q1–Q11 擋住切片，請先答（照建議即可開工）。
- 不擋的可以先開工：J1、J2 的伺服器半段、J3、J5、J19、J20；Q5、Q15 答了之後 J12。
- 這份的切片會動 `api/openapi.yaml`、`api/asyncapi.yaml` 的新入口和 SSE 事件，照這一輪的規矩由監督者合併後重產。
