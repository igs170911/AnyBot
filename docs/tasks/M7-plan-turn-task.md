# M7 剩下的工作：執行端的回合和任務引擎（規劃）

這份是規劃，不是程式。範圍：`turn-loop`、`task-*`、`plan-*`、`goal-*`，加上任務引擎讀裁判紀錄和卡片答案（`sync-referee`、`phone-cards`）、執行端領信（`mailbox-*`），和需要用到的 `server-brains`。

- 列的來源：`tools/spec-trace/baseline.txt` 裡還沒驗的列（已經有 `fn t_<id>` 的不算），共 **230 列**。
- 已經做完、不再列的：`t_goal_brake_9`、`t_goal_followup_13`、`t_mailbox_claim_1`、`t_phone_cards_3`、`t_server_brains_7`、`t_task_budget_1`–`6`、`8`、`9`、`t_task_done_gate_1`–`7`、`t_task_effects_1`–`4`、`t_task_entry_4`、`t_task_events_1`–`6`、`t_task_object_2`、`3`、`t_task_stall_4`、`t_task_state_6`–`9`、`t_turn_loop_1`、`2`、`3`、`9`、`11`、`12`。
- 驗收的層次照 `09` K119 ⑰：
  - 在執行端這一層，「任務狀態＝」（用 `anybot_domain` 的重播算）、證據、模型收到的請求、檔案、任務紀錄、交付報告都算驗到。
  - 「畫面出現」「推播送出」「按〔按鈕〕」在這一層不算。
  - 斷網、兩台裝置、伺服器停機的列，要等接上伺服器（M7j）。
  - 替身裝置、替身執行端只在每個預期都是資料時才准用（K118 ⑱）。
- 子段的名字沿用 `docs/tasks/M7d-turn-thin-slice.md`：
  - M7f：停止、插話、先記再做、接續。
  - M7g：換模型、預算。
  - M7h：做完的標準、驗收、自己接下一輪。
  - M7j：伺服器、信箱、執行服務、背景程式。
  - M7k：手機畫面。

---

## A) 範圍分類

### A.1 總數

| 分類 | 列數 | 說明 |
|---|---|---|
| M7 | 49 | 本區切片驗 34 列；交給手機畫面區（M7k）驗 13 列，引擎由本區提供；交給工具／檔案區 2 列 |
| M7.5 | 0 | 本區沒有大腦的列 |
| 之後 | 175 | M8：20；M9：47；目標與計畫（建議 M9，見 D 的 Q2）：76；M10 之後：16；未排定：16 |
| 還不能測 | 6 | 形式驗證的工具沒定（4 列）、實機（1 列）、執行服務的憑證怎麼交還沒定（1 列） |

### A.2 每一列

同一個理由的連號寫在一格；每個編號都出現一次。

| 編號 | 分類 | 理由 |
|---|---|---|
| T-goal-brake-1–8、10 | 之後：目標與計畫 | /goal 本身的煞車，要先有 /goal（Q2） |
| T-goal-brake-11 | M7（M7k） | 一般任務的煞車，引擎在 T10；預期有「沒有推播」「畫面沒有出現「目標暫停」」，要手機畫面和推播 |
| T-goal-done-1–3 | 之後：目標與計畫 | /goal 達成或做不到 |
| T-goal-evidence-1–2 | 之後：目標與計畫 | /goal 的驗收；另外要第 2 組的送出計數（claims-n1） |
| T-goal-followup-1–12、14–16 | 之後：M9 | 追進度 |
| T-goal-judge-1–7 | 之後：目標與計畫 | /goal 的驗收；judge-2、3、6 另外要多家供應商和資料政策（第 6 組 models-egress） |
| T-goal-n1-1–2 | 之後：M10 之後 | 命令列 `--ask`（第 5 組 install-n16） |
| T-goal-n2-1–3 | 之後：目標與計畫 | 撒花，端對端，跟 /goal 一起 |
| T-goal-n3-1–5 | 之後：目標與計畫 | 標準卡只出現在 /goal |
| T-goal-next-1 | 之後：目標與計畫 | /goal 的下一輪 |
| T-goal-next-2 | 之後：M8 | 引擎在 T10；預期是聊天的叫醒樣式（不是使用者氣泡）和任務頁件數 |
| T-goal-restart-1–2 | 之後：目標與計畫 | /goal 重開 |
| T-goal-set-1–8 | 之後：目標與計畫 | /goal 指令 |
| T-mailbox-claim-2 | M7 | T13：卡住不領信，10 秒改派 |
| T-mailbox-claim-3 | M7 | T15：背景服務關掉時答卡，回來後拿到 |
| T-mailbox-one-1–3 | 之後：M9 | 跨電腦子任務、交辦開關（computers-n2）、「叫另一台做」工具 |
| T-mailbox-reassign-1 | M7（M7k） | 端對端：改派那一行要在 P 和 A 的畫面出現 |
| T-mailbox-reassign-2 | 之後：M9 | 例行任務 |
| T-mailbox-reassign-3 | M7（M7k） | 端對端；報告的「做不到的部分」靠第 2 組 claims |
| T-mailbox-reassign-4 | 之後：M9 | 交辦開關 computers-n2 |
| T-mailbox-reassign-5 | 之後：M9 | Bot 的電腦休眠和叫醒（Docker） |
| T-mailbox-reassign-6 | 之後：M9 | 只有某台連得到的模型（computers-n5） |
| T-mailbox-route-1 | 之後：M10 之後 | 一次要例行任務（M9）、頻道、Bot 交辦一起觸發 |
| T-mailbox-route-2 | M7 | T18：Bot 設定的做事的電腦，和訊息裡指名電腦（Q12） |
| T-mailbox-route-3 | 之後：M9 | 只有 B 連得到的區網模型（computers-n5） |
| T-mailbox-route-4 | 之後：未排定 | 團隊成員 |
| T-mailbox-route-5 | 之後：M8 | 步驟詳情和任務頁的畫面 |
| T-mailbox-route-6、7 | M7 | T18：模型說的電腦不算、指定的電腦不在線就改派 |
| T-phone-cards-1 | 還不能測 | 形式驗證，工具沒定（Q6） |
| T-phone-cards-2 | M7（M7k） | 端對端：手機上的舊卡 |
| T-phone-cards-4 | 之後：未排定 | 支線 /btw |
| T-plan-approve-1–5 | 之後：目標與計畫 | /plan |
| T-plan-card-1–3 | 之後：目標與計畫 | /plan |
| T-plan-doing-1–4 | 之後：目標與計畫 | /plan |
| T-plan-done-1–4 | 之後：目標與計畫 | /plan |
| T-plan-mode-1–5 | 之後：目標與計畫 | /plan |
| T-plan-n1-1 | 之後：M10 之後 | 命令列 `--ask` |
| T-plan-restart-1–2 | 之後：目標與計畫 | /plan |
| T-plan-start-1–7 | 之後：目標與計畫 | /plan |
| T-server-brains-1 | 之後：M9 | 預期裡有例行任務到點 |
| T-server-brains-2 | 之後：M9 | 每人一個 Docker 容器 |
| T-server-brains-3 | 之後：M9 | Bot 電腦之間的隔離 |
| T-server-brains-4 | 之後：M9 | 每人的上限（server-limits） |
| T-server-brains-5 | 之後：M10 之後 | 遷移舊伺服器 |
| T-server-brains-6 | 還不能測 | 執行服務的憑證怎麼交還沒定（K118 ⑬，Q3） |
| T-sync-referee-1 | 還不能測 | 形式驗證（Q6） |
| T-sync-referee-2 | 之後：M9 | 例行任務 |
| T-sync-referee-3 | M7（M7k） | 端對端：伺服器停機時在 A 答卡；也要「刪除檔案」工具 |
| T-sync-referee-4 | 之後：M9 | 「執行例行任務」 |
| T-sync-referee-5 | M7（M7k） | 端對端：兩台 50 毫秒內答同一張卡 |
| T-task-badge-1–2 | 之後：M8 | 任務數字的畫面 |
| T-task-bot-1、2、5 | 之後：M8（建議） | 「查看任務」工具沒排進任何里程碑（Q15） |
| T-task-bot-3 | 之後：M10 之後 | 頻道 |
| T-task-bot-4 | 之後：M9 | 查看排程 |
| T-task-bot-6 | 之後：M10 之後 | MCP 外部 agent |
| T-task-budget-7 | M7 | T19：預期有「推播送出一次」和「做到哪」的報告 |
| T-task-cancel-1、3 | 之後：目標與計畫 | 前提要 /goal |
| T-task-cancel-2 | 之後：M9 | 追進度 |
| T-task-cancel-4 | 之後：M9 | 例行任務；刪 Bot 和還原另外要第 3 組的 bots |
| T-task-cancel-5 | 之後：未排定 | 支線 |
| T-task-done-gate-8、9 | M7 | T19：預期有「沒有推播」；9 的引擎在 T10、T11 |
| T-task-done-gate-10 | M7 | T9：提出標準工具 |
| T-task-entry-1 | M7 | T12：伺服器分辨開新任務還是插話 |
| T-task-entry-2 | 之後：M9 | 追進度 |
| T-task-entry-3 | 之後：M10 之後 | 頻道、Bot 交辦、例行任務 |
| T-task-entry-5 | 之後：M9 | 「叫另一台做」 |
| T-task-events-7 | M7 | T16：伺服器、執行端、兩台裝置的性質測試 |
| T-task-events-8 | M7 | T15：斷網時取消 |
| T-task-events-9 | M7 | T17：全路徑 |
| T-task-export-1–3 | 之後：未排定 | Bot 檔的匯出匯入（第 3 組 bots-n3） |
| T-task-handoff-1–5 | 之後：未排定 | Bot 之間交辦（team-message、team-ask） |
| T-task-journal-1、4 | M7 | T6 |
| T-task-journal-2 | M7 | T14：要真的程序被關掉 |
| T-task-journal-3 | M7（M7k） | 預期看手機畫面；遮秘密的規則屬第 2、6 組 |
| T-task-journal-5 | 之後：M10 之後 | 遷移 |
| T-task-journal-6 | M7 | T7 |
| T-task-keep-1、4、5、6 | 之後：M10 之後 | 遷移 |
| T-task-keep-2、3 | M7（工具／檔案區） | 完整輸出留 30 天屬原文的存放（files-n1、activity-n1） |
| T-task-list-1–4、6 | 之後：M8 | 任務頁 |
| T-task-list-5 | 之後：未排定 | 團隊 |
| T-task-list-7 | 之後：M9 | 例行任務 |
| T-task-notify-1、4、6 | 之後：目標與計畫 | /goal 達成的推播；6 另外要「在電腦前」的判斷（第 6 組） |
| T-task-notify-2、3 | 之後：M9 | 例行任務、多步驟任務 |
| T-task-notify-5 | 之後：M8 | 通知種類的設定（第 6 組 settings-n1） |
| T-task-notify-7 | M7 | T20 |
| T-task-object-1 | 之後：M8 | 任務頁 |
| T-task-object-4 | 之後：目標與計畫 | /goal |
| T-task-proof-1 | 之後：未排定 | ask_bot |
| T-task-proof-2、3 | 之後：M8 | 任務頁上的「證據：…」 |
| T-task-proof-4 | 之後：M10 之後 | 舊版證據等級的遷移對照 |
| T-task-resume-1、4、7 | M7 | T7 |
| T-task-resume-2、3、5、6 | M7（M7k） | 引擎在 T7、T15；預期有繼續卡的畫面、推播、「改到 Bot 的電腦」 |
| T-task-resume-8 | 還不能測 | 形式驗證（Q6） |
| T-task-resume-9 | 還不能測 | 實機：要真的 Mac 上的桌面 App（桌面功能在 M10 之後） |
| T-task-resume-10 | 之後：M10 之後 | 假連接器（MCP） |
| T-task-stall-1–3 | M7 | T20（Q16） |
| T-task-stall-5 | 之後：M9 | 追進度 |
| T-task-state-1、5 | M7（M7k） | 1 是端對端；5 是在手機按掉 👍 |
| T-task-state-2 | 之後：M8 | send_file 和手機顯示的回報（第 4 組 deliver） |
| T-task-state-3 | 之後：未排定 | 團隊成員 |
| T-task-state-4 | 之後：未排定 | update_bot 工具（第 3 組 bots-chat） |
| T-task-sync-1、2、4 | 之後：M8 | 任務頁畫面 |
| T-task-sync-3 | M7 | T17：三台的任務清單一樣（資料） |
| T-task-unattended-1–4、6 | 之後：M9 | 例行任務 |
| T-task-unattended-5 | 之後：未排定 | update_bot |
| T-task-unattended-7 | 之後：M10 之後 | 頻道 |
| T-task-wake-1、2、4、5 | 之後：M9 | 伺服器排程 |
| T-task-wake-3 | 還不能測 | 形式驗證（屬 M9） |
| T-turn-loop-4 | M7 | T8 |
| T-turn-loop-5 | M7 | T21 |
| T-turn-loop-6–8 | M7 | T4 |
| T-turn-loop-10 | M7 | T9 |
| T-turn-loop-13、15 | M7 | T5 |
| T-turn-loop-14 | M7 | T16 |

### A.3 相鄰的列（第 3 組，屬 `turn-loop` 第 ① ② 步；不算在 230 列裡）

| 編號 | 分類 | 理由 |
|---|---|---|
| T-chat-stop-4 | M7（T5） | 卡在等時按停止；要第 4 組的「刪除檔案」工具 |
| T-chat-stop-9 | M7（T2） | 按停止後沒有步驟開始（性質） |
| T-chat-steer-4 | M7（T3） | 收尾時才到的插話變成下一輪開頭 |
| T-chat-stop-1、2、3、6、8，T-chat-steer-1、3 | M7（M7k） | 端對端，或預期看畫面 |
| T-chat-stop-7 | M8 | 「取消任務」按鈕在任務頁 |
| T-chat-stop-5，T-chat-steer-2、5 | 之後 | ask_bot（未排定）、例行任務（M9） |

---

## B) 切片索引

### B.1 依賴順序

切片分成四條線，每條線裡照順序做；不同的線可以同時做。

| 線 | 切片 | 內容 |
|---|---|---|
| A | T0 → T1 → T2 → T3 → T4 → T5 → T9 → T10 → T11 → T21 | 回合的主迴圈；`step.rs`、`action.rs` 是熱檔，所以要照順序 |
| B | T6 → T7 | 先記再做、接續 |
| C | T8 | 換到備用模型 |
| D | T12 → T13 → T14 → T15 → T16 → T17，加上 T18、T19 → T20 | 伺服器和執行端 |

線之間的依賴：

- T7 要先有 T1。
- T13 要先有 T6。
- T19 要先有 T4、T10、T11。
- T21 要先有 T2、T4、T5、T7、T8。

協定一次在 T0 加好，之後的切片原則上不碰 `crates/protocol`；例外標在下表的「協定」欄。

### B.2 切片清單

遷移的號碼：server 和 client 共用一個號碼空間，每件 5 號；「預留」表示可能用不到。

| # | 標題 | 驗的編號 | 依賴 | 可以同時做 | 套件 | 遷移 | 協定 | 熱檔 |
|---|---|---|---|---|---|---|---|---|
| T0 | 協定和狀態引擎先行 | — | — | （第一件，單獨做） | protocol、domain | — | 是：結束原因、發起者、「開始驗收」 | protocol 全部、`api/*.yaml`、TS 客戶端 |
| T1 | 接回同一件任務；「驗收」發起者 | — | T0 | T6、T8、T12 | runner、tests/runner-test | — | — | `runner.rs`、`turn.rs`（測試世界） |
| T2 | 停止和取消（每個動作前後都看） | T-chat-stop-9 | T1 | T6、T8、T12 | runner、tests/runner-test | — | — | `record.rs`（trait）、`step.rs`、`action.rs` |
| T3 | 插話收進下一步 | T-chat-steer-4 | T2 | T6、T8、T12 | runner、tests/runner-test、harness（只用不改） | — | — | `step.rs`、`runner.rs` |
| T4 | 預算接進迴圈 | T-turn-loop-6、7、8 | T3 | T6–T8、T12 | runner、domain（只讀） | — | — | `step.rs`、`action.rs`、`checks.rs` |
| T5 | 卡片在這一輪裡等答案 | T-turn-loop-13、15，T-chat-stop-4 | T4 | T7、T8、T12 | runner、tests/runner-test | — | — | `approvals.rs`、`step.rs` |
| T6 | 先記再做：本機的進行中的回合 | T-task-journal-1、4 | T0 | T1–T5、T8、T12 | store-client、runner（新檔）、tools（只讀） | client 0200–0204 | — | `migrations/client` |
| T7 | 中斷後接著做（啟動時先整理） | T-task-resume-1、4、7，T-task-journal-6 | T1、T6 | T8、T12、T13 | runner、store-client | 預留 0230–0234 | 看 Q4：繼續卡 | `runner.rs` |
| T8 | 換到備用模型重組；模型連不到 | T-turn-loop-4 | T0 | T1–T7 | models、runner | — | 看 Q5：閘道的失敗型別 | models 的閘道 API |
| T9 | 做完的標準與任務層驗收（程式檢查） | T-turn-loop-10、T-task-done-gate-10 | T5 | T12–T18 | runner、tools（新工具）、trust（用） | — | — | `step.rs` 收尾、tools registry |
| T10 | 驗收沒過自己接下一輪＋一般任務的煞車 | — | T9 | T12–T18 | runner、domain | — | — | 收尾 |
| T11 | 模型判斷的項目（最小版） | — | T10 | T12–T18 | runner、models（用） | — | — | 收尾 |
| T12 | 伺服器收一句話：開任務、插話或排隊 | T-task-entry-1 | T0 | T1–T11 | protocol（API 型別）、domain、server、store-server | server 0205–0209（預留） | 是：送訊息的 API | `app.rs`、`api/openapi.yaml`、`mailbox_deliver.rs` |
| T13 | 執行端領信跑一輪＋執行服務 | T-mailbox-claim-2 | T6、T12 | T7–T11 | sync、runner、apps/runner-service | — | — | `crates/sync`、`apps/runner-service` |
| T14 | 使用者電腦的背景程式 `apps/agent` | T-task-journal-2 | T7、T13 | T9–T11、T18 | apps/agent、runner（用） | — | — | `apps/agent` |
| T15 | 裁判紀錄和卡片答案送回執行端 | T-mailbox-claim-3、T-task-events-8 | T5、T14 | T16、T18 | store-client（看得到變化）、runner、apps/agent | server 0210–0214（預留） | — | store-client 副本 |
| T16 | 離線時自己發起的一輪與編號 | T-turn-loop-14、T-task-events-7 | T14 | T15、T18 | apps/agent、server（收執行端寫的任務建立） | — | — | `api_streams_append.rs` |
| T17 | 全路徑：一輪的紀錄和多台同步 | T-task-events-9、T-task-sync-3 | T15、T16 | T18、T19 | tests（新測試檔），產品只修錯 | — | — | — |
| T18 | 這一輪在哪跑：路由規則接上 | T-mailbox-route-2、6、7 | T13 | T14–T17 | server、domain、protocol（設定鍵） | server 0215–0219（預留） | 是：Bot 設定「做事的電腦」 | `mailbox_deliver.rs`、設定 |
| T19 | 任務通知：該推的推一次、不該推的不推 | T-task-done-gate-8、9，T-task-budget-7 | T4、T11、T13 | T18 | server、domain（用） | server 0220–0224 | — | `app.rs` 啟動 |
| T20 | 卡住偵測：沒有動靜 15 分鐘 | T-task-stall-1、2、3，T-task-notify-7 | T19 | T17、T18 | runner、server | 預留 0225–0229 | — | `step.rs` |
| T21 | 一輪結束原因的性質測試 | T-turn-loop-5 | T2、T4、T5、T7、T8 | T12–T20 | runner、tests/runner-test | — | — | — |

共 22 件，驗 34 列本區的列，加 3 列相鄰的列。每一件都會改 `tools/spec-trace/baseline.txt`（拿掉驗完的編號），所以這個檔每件都會衝突，合 `dev` 時照常解。

---

## C) 每件的藍圖

### C.0 每件都一樣的部分

- 工作區、分支：從 `dev` 切 `ws/m7-tN-<短名>`。
- 驗證和回報（範本第 9 節）：
  - 本機只跑改到的套件的 clippy、相關的 `cargo nextest run`；送雲端前跑一次 `make check-fast`。
  - 雲端檢查：`python3 ~/.claude/skills/acp-agents/bin/cloud_check.py`（前景，timeout 50 分鐘）。
  - 回報照範本：驗收清單逐條寫在哪個 commit、哪個檔、哪一行。
- 字串：測試裡不准有中文。
  - 使用者或模型看得到的字一律用訊息代碼比對。
  - 要比對模型收到的那段字時，從 `locales/` 用代碼查出字串再比，或用 Unicode 跳脫寫法。
  - 下面表格裡寫的中文是規格原文，只是說明。
- 新的訊息代碼要同時加進 `locales/en.ftl`、`locales/zh-TW.ftl`。
- 帶編號的測試要驗完那一列的每一個預期（K86）。驗不完就用描述性的名字，編號留在基準檔，回報裡說明。

---

### T0 協定和狀態引擎先行

**1. 目標**：一次加好本區 M7 要用的事件種類和列舉，並且讓狀態引擎算得出來。之後的切片就不用同時去改 `crates/protocol`。

**2. 開工條件**：`dev` 最新。

**3. 只准改的範圍**：

- `crates/protocol/src/turn_records.rs`、`known_task_event_kind.rs` 和它們的測試。
- `crates/domain/src/task_engine*.rs`、`task_replay*.rs` 和它們的測試。
- 重新產生的 `api/openapi.yaml`、`api/asyncapi.yaml`、TS 客戶端。

**4. 定好的介面**：

- `TurnEndReason` 加這幾種：
  - `Stopped`
  - `Cancelled`
  - `OverBudget { item: BudgetItem, used: u64, limit: u64 }`
  - `CardTimedOut`
  - `ModelUnreachable`
  - `Interrupted`
- `TurnInitiator` 加 `Verification`（activity-who S1 的「驗收」）。
- `KnownTaskEventKind` 加 `VerificationStarted`（task-events S2 的「開始驗收」）。
- 不加 `TaskOpener`。不加任何新的 `TaskDetail`：缺哪個細項，停下來回報。
- 如果 Q4（繼續卡）在開工前已經有答案，照答案一起加；沒有就留給 T7。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| `TurnEnded{CardTimedOut}` | 主狀態等你，細項「等你（逾時）」 | turn-loop S10、task-unattended S1 |
| `TurnEnded{Interrupted}`，有 `OutcomeUnknown` | 等你（中斷，等你按繼續） | turn-loop S10、task-resume S3 |
| `TurnEnded{OverBudget{ToolCalls,2,2}}` | 卡住（超過預算：工具呼叫 2／2），跟從用量算出來的一致 | task-budget S3 |
| `TurnEnded{OverBudget{Steps,50000,50000}}` | 卡住（超過預算：步數 50,000／50,000） | chat-n11 S1 |
| `TurnEnded{Stopped}` 或 `{Cancelled}` | 照 `TaskStopped`、`TaskCancelled` 事件算；結束原因只是 S10 那一格的標籤，跟事件不一致的時候以事件為準 | task-events S7 |
| `TurnEnded{ModelUnreachable}`＋`ModelUnreachable` 事件 | 卡住（模型連不到） | task-state S2 |
| `VerificationStarted`，結果還沒全部出來 | 進行中（驗收中）；每一項都有結果之後，照完成閘門算 | turn-loop S8 |
| 舊紀錄裡沒有這些種類 | 狀態跟以前一樣 | task-events S5 |

**6. 要寫的測試**：

| 測試名稱 | 驗什麼 | 預期值 | 種類 |
|---|---|---|---|
| `turn_end_card_timed_out_waits_for_you` | 第 1 列 | `(WaitingForYou, 逾時那個細項)` | 單元 |
| `turn_end_over_budget_matches_usage` | 第 3、4 列 | 細項帶 `(ToolCalls,2,2)`、`(Steps,50000,50000)` | 單元 |
| `verification_started_sets_verifying` | 第 7 列 | `TaskDetail::Verifying` | 單元 |
| `new_kinds_round_trip` | 序列化 | 每一種序列化再讀回來都一樣 | 單元 |

既有的 `t_task_events_*`、`t_task_state_7` 要照樣過。

**7. 不准做的**：

- 不改 runner。
- 不改舊的列舉值的 serde 名稱。
- 不加「任務狀態變更」這種紀錄（K119 ⑬）。

**8. 驗收清單**：

1. 6 個結束原因、1 個發起者、1 個事件種類都加了，而且都有序列化往返的測試。
2. 第 5 節每一列都有對應的單元測試。
3. `api/*.yaml` 和 TS 客戶端重新產生，`make check-fast` 過。
4. 沒有新的 `TaskDetail`。

---

### T1 接回同一件任務；「驗收」發起者

**1. 目標**：執行端能在**已經存在**的任務上開新的一輪（task-entry S9、task-state S5、turn-loop S11）。現在每一句話都會開一件新的任務。

**2. 開工條件**：T0 在 `dev` 上。

**3. 只准改的範圍**：

- `crates/runner/src/runner.rs`、新檔 `crates/runner/src/continue_turn.rs`。
- `tests/runner-test/src/turn.rs`（加 `continue_task` 的幫手）。
- `crates/runner/tests/`。

**4. 定好的介面**：

```rust
pub struct ContinueTurn { pub task: TaskId, pub initiator: TurnInitiator, pub opening: Option<String> }
impl TurnRunner { pub async fn continue_task(&self, start: ContinueTurn) -> Result<TurnOutcome, TurnError> }
```

`TurnError` 加 `TaskNotFound`、`TaskClosed`。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 任務存在、沒有一輪在跑 | 寫 `TurnStarted`（第幾輪＝這件最大的號＋1）；不寫 `TaskCreated` | task-entry S9 |
| `opening` 有字、發起者是你 | 那句寫成你的訊息，掛在這件任務 | task-object S3 |
| 證據的短編號、第幾步 | 從這件任務已經有的最大號接著給 | turn-loop S12 |
| 任務是已取消 | `Err(TaskClosed)`，不寫任何東西 | task-cancel S1 |
| 這個 Bot 有一輪在跑 | `Err(Busy)` | turn-loop S13 |

**6. 要寫的測試**：

| 測試名稱 | 預期值 | 種類 |
|---|---|---|
| `continue_task_reuses_task_and_numbers` | 第一輪改 a.txt，接著的一輪改 b.txt：`TaskCreated` 1 筆；`TurnStarted` 第 1、2 輪；證據 E1、E2；最後＝完成（已驗收） | 整合（執行端這一層） |
| `continue_cancelled_task_is_refused` | `Err(TaskClosed)`；紀錄的筆數沒變 | 整合 |
| `continue_while_busy_is_refused` | `Err(Busy)` | 整合 |

**7. 不准做的**：

- 不決定「你的哪一句話算接回上一件」（Q1）。誰叫 `continue_task` 由呼叫的人決定。
- 不做自己接下一輪（T10）。

**8. 驗收清單**：

1. 第 4 節的介面照寫。
2. 3 個測試都過，預期值照上表寫死。
3. `on_user_message` 的行為沒有變，`t_turn_loop_*` 照樣過。

---

### T2 停止和取消

**1. 目標**：一輪會在 ①「每次問模型之前」和「每個動作的前後」看停止和取消。按停止之後，不再開始任何動作，也不再問模型（chat-stop S2、turn-loop S1 和 S10）。

**2. 開工條件**：T1 在 `dev` 上。

**3. 只准改的範圍**：

- `crates/runner/src/{record.rs, checks.rs, step.rs, action.rs}`，新檔 `halt.rs`。
- `tests/runner-test`（`MemoryRecord` 實作新的方法）。

**4. 定好的介面**：

- `Record` 加這個方法：

  ```rust
  fn append_unless_halted(&self, entry: RecordEntry, turn_started: u64)
      -> PinBoxFuture<'_, Result<Appended, RecordError>>;
  ```

  - `enum Appended { Stored(Stored), Halted(Halt) }`，`enum Halt { Stopped, Cancelled }`。
  - 看「這一輪開始之後有沒有 `TaskStopped`／`TaskCancelled`」，和「寫入」要在同一個臨界區。
- `ModelAsked`、`ToolStarted` 一律用這個方法寫。
- 停止和取消的紀錄**不是**執行端寫的：
  - 連得上伺服器時，由伺服器裁判寫（K113 ①）。
  - 離線、而且是這台自己發起的那一輪，由背景程式寫（T16）。
  - 測試裡用 `TurnWorld::append_task_stopped` 當寫的那一方。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| ① 之前已經停止 | 不寫 `ContextBuilt`、`ModelAsked`；寫 `TurnEnded{Stopped}`；主狀態＝已停止 | turn-loop S10 |
| 一步有 3 個讀檔，第 1 個做完之後停止 | 第 1 個完成；第 2、3 個各一筆 `ToolNotRun{Stopped}`；`ModelAsked` 只有 1 筆 | turn-loop S5 |
| 取消 | 同上，但寫 `TurnEnded{Cancelled}`、已取消 | task-cancel S1 |
| 停止的時候還沒有任何回答的字 | 回答寫代碼 `turn-reply-stopped`（「（已停止）」） | chat-stop S2 |
| 正在跑的指令 | 這一件不中斷它：讓它跑完，記下結果。中斷要等第 4 組的介面（見 E） | chat-stop S2 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_chat_stop_9` | T-chat-stop-9 | 假模型 5 步，每步 1 個讀檔；在隨機時間點停止，跑 1,000 次：每一筆 `ToolStarted`、`ModelAsked` 的時間和號碼都早於 `TaskStopped` | 性質（`prop` 組） |
| `stop_after_first_action_marks_rest_not_run` | 第 2 列 | 見上表 | 整合 |
| `cancel_ends_turn_cancelled` | 第 3 列 | `TurnEnded{Cancelled}`；已取消 | 整合 |
| `stop_without_text_writes_stopped_reply` | 第 4 列 | 回答的代碼＝`turn-reply-stopped` | 整合 |

**7. 不准做的**：

- 不中斷正在跑的指令。
- 不處理在等的卡（T5）。
- 不做插話放回輸入框（T3）。
- 不做目標和計畫的「等你」。

**8. 驗收清單**：

1. `append_unless_halted` 照寫，`MemoryRecord` 有實作，而且有「同一個臨界區」的單元測試。
2. `t_chat_stop_9` 在 `prop` 組。
3. 第 5 節的前 4 列各有測試。
4. 基準檔拿掉 `T-chat-stop-9`。

---

### T3 插話收進下一步

**1. 目標**：你自己起的那一輪在跑的時候，你再送的話，在下一步的 ② 收進來。來不及讀的，變成下一輪的開頭。按停止時還沒讀到的，交回給呼叫的人（chat-steer S1、S3、S4）。

**2. 開工條件**：T2 在 `dev` 上。

**3. 只准改的範圍**：

- `crates/runner/src/{runner.rs, step.rs}`，新檔 `steer.rs`。
- `tests/runner-test`。

**4. 定好的介面**：

- `TurnRunner::steer(&self, Steer { device: DeviceId, text: String }) -> Result<SteerQueued, TurnError>`。
- `TurnError` 加 `NoOwnTurn`（沒有你起的那一輪在跑）。
- `TurnOutcome` 加 `unread_steers: Vec<String>`。
- 這一輪正常結束時還有沒讀到的插話：執行端自己 `continue_task`，同一件任務，發起者＝你（那一句是哪台裝置送的）。這是照 Q8 的建議做的。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 第一步 shell 在跑時插話「B」 | 下一個請求包含「B」；`TaskCreated` 1 筆 | chat-steer S1 |
| 正在跑的一輪的發起者不是你 | `Err(NoOwnTurn)`；那一輪的請求不包含它 | chat-steer S2 |
| 最後的回答沒有工具要求、收尾時才到 | 這一輪的請求都不包含它；新的一輪的第一個請求以它開頭 | chat-steer S3 |
| 按停止時還沒讀到 | `unread_steers == ["B"]`；模型從沒收到「B」 | chat-steer S4 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_chat_steer_4` | T-chat-steer-4 | 假模型最後回「好」（`"\u{597d}"`），送完之後串流再撐 2 秒才關（harness 的 model_hold）；在這中間 steer「B」。第 1 輪所有請求都不含「B」；第 2 輪的第一個請求，最後一則使用者訊息以「B」開頭 | 整合 |
| `steer_reaches_next_step` | 第 1 列 | 見上表 | 整合 |
| `steer_during_verification_round_refused` | 第 2 列 | `Err(NoOwnTurn)` | 整合 |
| `stop_returns_unread_steers` | 第 4 列 | 見上表 | 整合 |

**7. 不准做的**：

- 不做伺服器分辨插話（T12）。
- 不做「已排入：…」的畫面（M7k）。
- 插話不寫成任務事件：它是你的訊息紀錄，掛在這件任務。

**8. 驗收清單**：

1. 介面照寫。
2. 4 個測試都過。
3. 基準檔拿掉 `T-chat-steer-4`。

---

### T4 預算接進迴圈

**1. 目標**：③「看預算」和「每個動作開始之前」都會看預算。到上限本身不算超過。超過了，就不再開新的動作、也不再問模型（turn-loop S6、task-budget S3）。

**2. 開工條件**：T3 在 `dev` 上。

**3. 只准改的範圍**：

- `crates/runner/src/{checks.rs, step.rs, action.rs}`，新檔 `budget_gate.rs`。
- `BotProfile`。
- `tests/runner-test`。

**4. 定好的介面**：

- `BotProfile.budget_overrides: Vec<(BudgetItem, BudgetLimitValue)>`：Bot 設定的上限。任務建立後，每一項各寫一筆 `BudgetLimit`。這是 Q13 的建議做法；型別用 domain 現有的。
- `Proceed` 加 `OverBudget { item, used, limit }`。
- `const TURN_STEP_CAP: u64 = 50_000`。
- 用量照 `anybot_domain::task_budget` 從事件算，不另外計數。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 工具呼叫上限 2，一步要 3 個讀檔 | 前 2 個執行；第 3 個 `ToolNotRun{OverBudget}`；不再問模型；卡住（工具呼叫 2／2） | turn-loop S6 |
| 上限 2，兩步各讀一個檔，再回答 | 完成，不是卡住 | task-budget S3 |
| token 上限 1,000，回答的用量 900＋300，附一個 edit | 回答完整記下；edit 是 `ToolNotRun{OverBudget}`；卡住（token 1,200／1,000） | task-budget S3 |
| 驗收自己跑的工具 | 不算工具呼叫 | turn-loop S8 |
| 步數不限，到 50,000 步 | `TurnEnded{OverBudget{Steps,50000,50000}}` | chat-n11 S1 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_turn_loop_6` | T-turn-loop-6 | 讀 a、b、c：a、b 有 `ToolCompleted`；c 是 `ToolNotRun{OverBudget}`；模型的請求共 1 個；`(Stuck, 超過預算(ToolCalls,2,2))` | 整合 |
| `t_turn_loop_7` | T-turn-loop-7 | 主狀態＝`Done` | 整合 |
| `t_turn_loop_8` | T-turn-loop-8 | 回答那一筆裡有那個 edit 要求；工作資料夾沒有 a.txt；`(Stuck, 超過預算(Tokens,1200,1000))`；請求共 1 個 | 整合 |
| `default_steps_stop_before_301st_request` | task-budget-7 的執行端這一半 | 請求剛好 300 個；`(Stuck, 超過預算(Steps,300,300))` | 整合 |

**7. 不准做的**：

- 不推播（T19）。
- 不產生「做到哪」的報告（第 2 組）。
- 不跑 50,000 步的測試（T-chat-n11-1 留給第 3 組，或看 Q19）。

**8. 驗收清單**：

1. 第 5 節每一列都有測試，或者明確引用已經有的測試。
2. 3 個帶編號的測試過。
3. 基準檔拿掉 `T-turn-loop-6`、`7`、`8`。

---

### T5 卡片在這一輪裡等答案

**1. 目標**：你起的那一輪遇到卡片，**不結束、一直等**。沒人在看的那一輪，等滿 15 分鐘收工。答案進來之後在同一步接著做。被拒絕，後面的動作就不做（turn-loop S5、S10、S13）。

現在 M7e 的做法是先用 `PendingApproval` 結束這一輪。

**2. 開工條件**：

- T4 在 `dev` 上。
- 信任／核准區的「一律允許：在這台跑 make」規則在 `dev` 上（T-turn-loop-15 要用）。
- 第 4 組的「刪除檔案」工具在 `dev` 上（T-chat-stop-4 要用；沒有就不用這個編號）。

**3. 只准改的範圍**：

- `crates/runner/src/{approvals.rs, step.rs, action.rs}`，新檔 `card_wait.rs`。
- `tests/runner-test`。

**4. 定好的介面**：

- 新的 trait `TaskWatch { fn changed(&self, task: TaskId) -> PinBoxFuture<'_, ()>; }`。
- `RunnerParts` 加 `watch: Arc<dyn TaskWatch>`。
- 答案讀紀錄裡的 `CardAnswered`，寫的人可以是任何一方。
- 沒人在看的那一輪：答案和 `clock.sleep(15 分鐘)` 比誰先到。逾時就寫 `TurnEnded{CardTimedOut}`，不寫 `CardAnswered`。
- 卡片在等的時候看到停止：執行端寫 `CardAnswered{Deny}`，再照 T2 停下（照 Q9 的建議）。
- `resolve_approval` 留著，當成離線時本機寫答案的那一條路。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 讀檔、make（要問，按拒絕）、edit | 讀檔執行；make 被拒；edit 是 `ToolNotRun{PreviousRefused}`、沒有卡；下一個請求有 3 個結果 | turn-loop S5 |
| 第一個讀檔跑 10 秒時，加一條一律允許 make | make 直接做，把關紀錄寫依那一條；`TurnStarted` 寫 M1 和思考模式；沒有 `ModelSwapped` | turn-loop S13 |
| 你起的那一輪等卡，假時鐘快轉 20 分鐘 | 沒有 `TurnEnded`；等你（等你允許） | turn-loop S10 |
| 發起者是驗收，等卡 15 分鐘 | `TurnEnded{CardTimedOut}`；等你（逾時）；卡還在等（`card_is_waiting` 是真） | task-unattended S1 |
| 卡在等時按停止 | `CardAnswered{Deny}`；那個動作沒執行 | chat-stop S3 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_turn_loop_13` | T-turn-loop-13 | 假電腦宣告成 Linux；請求 #2 剛好有 3 個工具結果，第 3 個的原因是 `PreviousRefused`；edit 沒有 `CardShown` | 整合 |
| `t_turn_loop_15` | T-turn-loop-15 | make 那筆 `GateResult` 寫依一律允許那一條；沒有 make 的 `CardShown`；`ModelSwapped` 0 筆；只有 M1 收到請求 | 整合 |
| `t_chat_stop_4` | T-chat-stop-4 | 刪除檔案 a.txt 那張卡在等時停止：有 `CardAnswered{Deny}`；a.txt 還在 | 整合 |
| `own_card_waits_past_fifteen_minutes` | 第 3 列 | 見上表 | 整合 |
| `unattended_card_times_out` | 第 4 列 | 見上表 | 整合 |

**7. 不准做的**：

- 不改卡片的內容和四種答案（信任區）。
- 不推播。
- 不做逾時之後「重跑這一次」（M9）。

**8. 驗收清單**：

1. `TaskWatch` 照寫。
2. 你自己起的那一輪不再產生 `TurnEnded{PendingApproval}`；舊的測試要改的，列出來。
3. 5 個測試都過。
4. 基準檔拿掉這次驗完的編號。

---

### T6 先記再做：本機的進行中的回合

**1. 目標**：執行端的紀錄寫進本機的 SQLite。先記再做的那一筆，就是匣子裡要送上去的那一筆：同一張表、同一筆交易（K119 ⑲）。這一筆確定存進磁碟，才去執行。本機那份不遮秘密，送上去的那份要遮（task-journal S1、S4）。

**2. 開工條件**：T0 在 `dev` 上。

**3. 只准改的範圍**：

- `crates/store-client/src/` 新檔 `journal.rs`。
- `migrations/client/0200_journal.sql`（0200–0204）。
- `crates/runner/src/` 新檔 `journal_record.rs`。
- `crates/tools`：只讀 registry。
- 相關的測試。

**4. 定好的介面**：

- `Store::journal_append(&JournalEntry) -> Result<JournalRow, StoreError>`：
  - `JournalEntry { stream, record_id, kind, content (遮過的), local_detail: Option<String> (沒遮的), task, turn, effect: Option<ToolEffect> }`。
  - 寫進 `outbox`，加上新欄位 `local_detail`、`turn`、`effect`。
  - 這張表用 `synchronous=FULL`。
- `Store::journal_open() -> Result<Vec<JournalRow>, StoreError>`：還沒有 `TurnEnded` 的那幾輪。
- `Store::journal_trim(task, turn)`：一輪結束後清掉 `local_detail`，只留還沒讀到的插話。
- `JournalRecord` 實作 `Record`：
  - `append` 就是 `journal_append`。
  - `conversation`、`task_events` 讀本機副本加上匣子。
- 遮秘密用信任區給的函式（見 E）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 一筆 `ToolIntent` | 匣子裡剛好多一列；別的表沒有寫 | task-journal S1、K119 ⑲ |
| 參數裡有〔假金鑰〕 | `local_detail` 有；`content` 和送出去的內容都沒有 | task-journal S4 |
| 每個工具的等級 | 剛好一個；沒標的預設不可逆 | task-journal S3 |
| 一輪正常結束 | `local_detail` 清掉 | task-journal S5 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_task_journal_1` | T-task-journal-1 | registry 裡每個工具剛好一個 `ToolEffect`；`read_file`＝ReadOnly；`edit_file`、`shell`＝Irreversible；註冊一個沒標的工具＝Irreversible | 單元 |
| `t_task_journal_4` | T-task-journal-4 | 在 500 個隨機時間點中止那一輪的 future（模擬關掉電腦 A），每次重新打開 store：假電腦每執行過一個動作，本機都有它的 `ToolIntent`，而且早於執行 | 性質（`prop`） |
| `journal_row_is_outbox_row` | 第 1 列 | `outbox_pending` 多 1 列 | 單元 |
| `journal_secret_stays_local` | 第 2 列 | 假金鑰在測試執行時才組出來 | 單元 |
| `journal_trim_after_normal_end` | 第 4 列 | `local_detail` 是 NULL | 單元 |

**7. 不准做的**：

- 不做接續（T7）。
- 不改 sync 送出的流程。
- 不在 runner 裡直接寫 SQL。

**8. 驗收清單**：

1. 遷移在 0200–0204 裡面。
2. 同一筆交易有測試。
3. 5 個測試都過。
4. 基準檔拿掉 `T-task-journal-1`、`4`。

---

### T7 中斷後接著做

**1. 目標**：執行端啟動時，在跑任何別的東西之前，先整理中斷的那幾輪（task-resume S1、S2、S3、S5）：

- 唯讀的：重做。
- 可重複的：先對帳。
- 不可逆的：標「可能已經執行了」，等你按繼續。

**2. 開工條件**：

- T1、T6 在 `dev` 上。
- Q4（繼續卡的協定）、Q7（「關掉電腦」在這一層怎麼算）有答案。

**3. 只准改的範圍**：

- `crates/runner/src/` 新檔 `recover.rs`、`recover_plan.rs`。
- `runner.rs`。
- `tests/runner-test`。

**4. 定好的介面**：

- `TurnRunner::recover(&self) -> Result<Vec<Recovered>, TurnError>`。
- `Recovered { task, turn, plan: RecoveryPlan }`。
- `enum RecoveryPlan { AutoContinue, NeedsContinue { maybe_done: Vec<CallId> }, AlreadyStopped, Clean }`。
- 怎麼處理照 `anybot_domain::task_effects::resume_action`。
- 新的訊息代碼：
  - `turn-resume-step-aborted`：「（這個步驟被中止了，沒有執行完。）」
  - `turn-resume-maybe-done`：「（AnyBot 在執行這個步驟時被關掉了…）」
  - `turn-resume-auto-line`：「這一輪被中斷了，已經自動接著做。」
- 繼續卡照 Q4 的答案。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| read_file 進行中被關掉 | 進行中（中斷後自動接著做）；下一個請求包含 `turn-resume-step-aborted`；沒有繼續卡 | task-resume S1、S2 |
| shell 跑完、結果還沒記下 | `OutcomeUnknown`；`TurnEnded{Interrupted}`；等你（中斷，等你按繼續）；繼續卡列出那一步 | task-resume S3 |
| 已經按過停止的那一輪 | `AlreadyStopped`；已停止；之後沒有任何請求 | task-resume S5 |
| 正常結束之後才關掉 | `Clean`；沒有 `turn-resume-auto-line`；沒有繼續卡；沒有請求 | task-journal S6 |

**6. 要寫的測試**：

「關掉、打開」的做法照 Q7 的建議：丟掉 runner，用同一個 SQLite 檔重新建一個。

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_task_resume_1` | T-task-resume-1 | 見上表第 1 列 | 整合 |
| `t_task_resume_4` | T-task-resume-4 | shell(sleep 30) 時按停止，然後關掉、打開：已停止；重開後模型收到的請求是 0 個 | 整合 |
| `t_task_resume_7` | T-task-resume-7 | 每個腳本 1–5 個 edit_file；在 500 個時間點關掉再打開，不按繼續：每個 edit 在假電腦上最多執行 1 次；每個開始了、沒有結果的 edit，都在繼續卡的清單上 | 性質（`prop`） |
| `t_task_journal_6` | T-task-journal-6 | 見上表第 4 列 | 整合 |
| `irreversible_unknown_needs_continue` | 第 2 列 | 見上表 | 整合 |

**7. 不准做的**：

- 不做按繼續之後的流程：那要伺服器交回原本那台（T15，和 M7k 的畫面）。
- 不做 K51 的離線改派（屬 T-task-resume-6，M7k）。
- 不做連接器的對帳。

**8. 驗收清單**：

1. `recover` 在 `TurnRunner` 能接任何信之前就跑完（有測試）。
2. 5 個測試都過。
3. 基準檔拿掉 4 個編號。

---

### T8 換到備用模型重組；模型連不到

**1. 目標**：

- 主模型重試用完、有備用模型時，執行端照備用模型重組一次上下文，再寫一筆「換了模型」和一筆「組了上下文」（turn-loop S4、S11，K109 ⑤）。
- 模型連不到時，任務是卡住（模型連不到）（K109 ⑧）。

**2. 開工條件**：

- T0 在 `dev` 上。
- Q5 有答案。

**3. 只准改的範圍**：

- `crates/models`：閘道的失敗型別、指定模型的呼叫。
- `crates/runner/src/{step.rs, ending.rs}`，新檔 `fallback.rs`。
- 測試。

**4. 定好的介面**（照 Q5 的建議）：

- 閘道重試用完時回 `GatewayFail::Exhausted { fallback: Option<ModelChoice> }`。閘道自己不去問備用模型。
- 執行端換的順序：
  1. 寫 `ModelSwapped{from,to,reason}`。
  2. 用 M2 的能力重新編譯上下文，寫 `ContextBuilt`。
  3. 指定 M2 再問閘道一次。
- `ModelReach` trait：`fn any_online_computer_reaches(&self, model: &ModelName) -> bool`。它是假的時，在測試裡從建構函式傳進去。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| M1 回 503 到重試用完，M2 回「好」 | M2 的請求之前剛好一筆 `ModelSwapped` 和一筆新的 `ContextBuilt`（清單寫 M2 和它的預算）；M1 失敗的那幾次沒有工具被執行 | turn-loop S4 |
| M2 也失敗 | `TurnEnded{ModelFailed}`；失敗（出錯：…） | turn-loop S11 |
| 連不到，而且沒有在線的電腦連得到 | `ModelUnreachable` 事件＋`TurnEnded{ModelUnreachable}`；卡住（模型連不到） | task-state S2 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_turn_loop_4` | T-turn-loop-4 | M1 可讀長度 128,000、M2 16,000；這件任務先跑 30 步（第一輪），再用 `continue_task`。M2 收到的請求，估算加安全邊際 ≤ M2 可以放的量（用 context 的預算函式算上限，不用被測的函式算預期） | 整合 |
| `fallback_also_fails_marks_failed` | 第 2 列 | 見上表 | 整合 |
| `unreachable_model_marks_stuck` | 第 3 列 | 見上表 | 整合 |

**7. 不准做的**：

- 不做主動換模型（models-handoff 的其他列）。
- 不做 /context、/compact。

**8. 驗收清單**：

1. 閘道的型別照 Q5 的答案。
2. 3 個測試都過。
3. 基準檔拿掉 `T-turn-loop-4`。

---

### T9 做完的標準與任務層驗收（程式檢查）

**1. 目標**：任務有明確的做完的標準時，在最後的回答之後、「一輪結束」之前做任務層驗收（turn-loop S8、S9）。Bot 可以用提出標準的工具自己寫標準，不出標準卡（task-done-gate S2）。

**2. 開工條件**：

- T5 在 `dev` 上。
- 信任區的 `check_criterion` 在 `dev` 上（見 E）。
- Q10 有答案。

**3. 只准改的範圍**：

- `crates/runner/src/` 新檔 `task_verify.rs`，和收尾那一段。
- `crates/tools`：新增工具 `propose_done_criteria`。
- 測試。

**4. 定好的介面**：

- 工具 `propose_done_criteria { items: Vec<Criterion> }`：
  - 執行端寫 `DoneWhenChange{by: Bot, change: SetItems 或 AddItem}`。
  - 工具等級＝ReadOnly；不出卡。
- 收尾的順序：
  1. 寫 `VerificationStarted`。
  2. 每一項呼叫 `anybot_trust::check_criterion`，它會過把關，證據標「驗收」。
  3. 每一項寫 `CriterionPassed` 或 `CriterionFailed{source: Program}`。
  4. 寫 `TurnEnded`。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 標準「out.csv 存在」，工具呼叫上限 1 | 順序：回答 → `VerificationStarted` → `CriterionPassed` → `TurnEnded`；驗收的讀檔有 `GateResult`，證據標驗收；工具呼叫用量＝1；完成（已驗收） | turn-loop S8 |
| 沒有標準；Bot 提出 FileExists(hello.txt) 再建檔 | 沒有 `CardShown`；完成（已驗收）；交付報告列出那一項，判斷來源＝Program | task-done-gate S2 |
| 這一輪停止、出錯，或在等你 | 不做驗收 | turn-loop S8 |
| 沒有明確標準 | 不寫 `VerificationStarted`（行為跟現在一樣） | turn-loop S8 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_turn_loop_10` | T-turn-loop-10 | 測試照伺服器會寫的樣子，先寫 `TaskCreated`、`DoneWhenChange{User, SetItems[FileExists("out.csv")]}`、`BudgetLimit{ToolCalls,1}`，再 `continue_task`；預期見上表第 1 列 | 整合 |
| `t_task_done_gate_10` | T-task-done-gate-10 | 見上表第 2 列 | 整合 |
| `stopped_turn_skips_verification` | 第 3 列 | 見上表 | 整合 |

**7. 不准做的**：

- 不做模型判斷（T11）。
- 不做自己接下一輪（T10）。
- 不做 /goal 的標準卡。

**8. 驗收清單**：

1. 工具名和參數照 Q10。
2. 3 個測試都過。
3. 基準檔拿掉 2 個編號。

---

### T10 驗收沒過自己接下一輪＋一般任務的煞車

**1. 目標**：不是 /goal 的任務，驗收有沒過的項目時，自己接下一輪，不等你。發起者＝驗收。最多自己接 3 輪；連續兩輪檢查說差同一件事就停，結果是未驗收（驗收沒通過）（task-done-gate S3、goal-next S2、goal-brake S10）。

**2. 開工條件**：

- T9 在 `dev` 上。
- Q11 有答案（誰決定開下一輪）。

**3. 只准改的範圍**：

- `crates/domain/src/` 新檔 `next_round.rs`。
- `crates/runner/src/` 新檔 `after_turn.rs`。
- 測試。

**4. 定好的介面**：

- `anybot_domain::next_round::decide(events: &[TaskEvent], kind: TaskKind) -> NextRound`：
  - `enum NextRound { Continue { gaps: Vec<GapRecord> }, StopUnverified, Done }`。
  - 用現有的 `goal_brake::same_gap`、`numbers_stable`。
  - 一般任務的上限是 3 輪。
- 離線時的路：執行端寫 `Wake{reason: Verification}`，再 `continue_task(Verification, 開頭＝代碼 task-verify-not-passed{items})`。
- 連得上伺服器時的路：照 Q11（建議：向伺服器送裁判動作 Wake）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 標準 out.csv；第 1 輪只回「好」，第 2 輪建檔 | 第 2 輪的發起者＝Verification；開頭的代碼帶 out.csv；任務只有 1 件；最後＝完成（已驗收）；沒有任何卡 | goal-next S2 |
| 每輪都只回「好了」 | 你起的那一輪之後自己接 1 輪就停（同一項、查到的值沒變）；未驗收（驗收沒通過）；沒有 `GoalPaused`；報告那一項標沒通過 | goal-brake S10 |
| 之後你再送一句 | 同一件回到進行中，輪數重新算 | goal-brake S10 |
| 每輪理由都不一樣 | 自己接滿 3 輪就停；未驗收 | goal-brake S10 |

**6. 要寫的測試**（都用描述性的名字；編號在 T19、M7k、M8 才驗）：

| 測試名稱 | 預期值 | 種類 |
|---|---|---|
| `non_goal_self_continue_reaches_done` | 見上表第 1 列 | 整合 |
| `non_goal_same_gap_twice_stops_unverified` | 見上表第 2、3 列 | 整合 |
| `non_goal_self_rounds_cap_three` | 用不同的值，例如每輪換一個檔名：自己接的輪數＝3 | 整合 |
| `next_round_decide_table` | 純函式的表格測試 | 單元 |

**7. 不准做的**：

- 不做 /goal 的 100 輪和「目標暫停」。
- 不做推播（T19）。
- 不做模型判斷（T11）。

**8. 驗收清單**：

1. `decide` 是純函式，不看時鐘。
2. 4 個測試都過。
3. 回報列出 `T-task-done-gate-9`、`T-goal-next-2`、`T-goal-brake-11` 還差哪幾個預期。

---

### T11 模型判斷的項目（最小版）

**1. 目標**：標「模型判斷」的項目，交給驗收的模型判。M7 先用 Bot 自己那家，照 goal-judge S2 的「沒有就用 Bot 自己那家」。最多查 3 次，而且只能看（goal-judge S4、S5）。

**2. 開工條件**：

- T10 在 `dev` 上。
- Q14（驗收模型怎麼交答案、沒回應怎麼算）有答案。

**3. 只准改的範圍**：

- `crates/runner/src/` 新檔 `judge.rs`、`judge_request.rs`。
- `RunnerParts` 加 `judge: TurnModel`。
- 測試。

**4. 定好的介面**：

- 驗收的模型用工具 `submit_verdict { verdict: passed | not_yet | impossible, reason }` 交答案（Q14 的建議）。
- 能用的查詢工具只有 `read_file`；`list_dir`、`web_fetch` 等第 4 組做好再加。
- 拒絕的代碼：`judge-tool-not-allowed{tool}`。
- 結果寫 `CriterionPassed` 或 `CriterionFailed{source: Model}`。
- 驗收的請求不算 Bot 的步數（task-budget S1）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 驗收的模型呼叫 edit_file | 被拒，代碼 `judge-tool-not-allowed{edit_file}`；檔案不存在 | goal-judge S4 |
| 連續 4 次 read_file | 第 4 次沒有執行；算沒有回應 | goal-judge S4 |
| 一直回「還沒」，每輪理由不一樣 | T10 自己接 3 輪；未驗收；沒有等你 | task-done-gate S3 |
| 程式能查的項目 | 驗收的模型收到的請求是 0 個 | turn-loop S8 |

**6. 要寫的測試**（描述性的名字）：

| 測試名稱 | 預期值 | 種類 |
|---|---|---|
| `model_criterion_self_continues_three_rounds` | 見上表第 3 列；請求裡帶 `task-verify-not-passed` 的剛好 3 個 | 整合 |
| `judge_write_tool_refused` | 見上表第 1 列 | 整合 |
| `judge_fourth_lookup_not_run` | 見上表第 2 列 | 整合 |
| `program_criterion_never_asks_judge` | 見上表第 4 列 | 整合 |

**7. 不准做的**：

- 不挑別家供應商，不看資料政策（models-egress，之後做）。
- 不做 goal-evidence 的標記。

**8. 驗收清單**：

1. `submit_verdict` 照 Q14。
2. 4 個測試都過。

---

### T12 伺服器收一句話：開任務、插話或排隊

**1. 目標**：伺服器收下你的一句話，自己決定是新的任務、插話，還是排隊。任務建立由伺服器寫（task-events S7、task-entry S1–S2、K119 ⑬ ⑳）。

**2. 開工條件**：

- T0 在 `dev` 上。
- Q1 有答案。

**3. 只准改的範圍**：

- `crates/protocol`（API 型別）。
- `crates/domain/src/` 新檔 `message_entry.rs`。
- `crates/server/src/` 新檔 `api_bot_messages.rs`，`app.rs` 加路由，`mailbox_deliver.rs` 改信的內容。
- `crates/store-server`（查詢）。
- `migrations/server/0205–0209`（預留）。
- `apps/server/tests/`。
- `api/openapi.yaml`。

**4. 定好的介面**：

- `POST /v1/bots/{bot}/messages`，要帶 `Idempotency-Key`。
  - Body：`PostMessage { text, computer: Option<ComputerId> }`。
  - 回：`PostMessageReply { task_id, outcome: MessageOutcome }`，`enum MessageOutcome { NewTask, Steer, Queued }`。
- `anybot_domain::message_entry::classify(&BotTurnView) -> MessageOutcome`，是純函式。
- 「一輪」那封信的內容加上 `task` 和 `message_seq`。
- 插話的信，種類是 `steer`。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 沒有一輪在跑 | 寫 `TaskCreated{opened_by: You}`（伺服器的裝置）＋你的訊息＋一封「一輪」的信 | task-entry S1 |
| 你起的那一輪在跑 | 只寫你的訊息（掛在那件），送插話的信給做事的那台；不開新任務 | task-entry S2 |
| 正在跑的那一輪不是你起的 | 排隊：照 Q1 決定新的任務還是那件；細項＝排隊中；那一輪的 `TurnEnded` 到了才交信 | turn-loop S13 |
| 同一個 `Idempotency-Key` 重送 | 回一樣的結果，不多寫 | M6a |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_task_entry_1` | T-task-entry-1 | 手機 P（替身）送 "A"；替身執行端領信，寫 `TurnStarted{You(P)}` 和 shell 的 `ToolStarted`；P 再送 "B"，回 `Steer`。`TaskCreated` 剛好 1 筆，`opened_by == You` | 整合（pg） |
| `pg_message_queued_behind_verification_round` | 第 3 列 | `Queued`；信在 `TurnEnded` 之後才交 | 整合 |
| `pg_message_idempotent` | 第 4 列 | 兩次的回覆一樣，紀錄的筆數一樣 | 整合 |
| `classify_table` | `classify` | 表格測試 | 單元 |

**7. 不准做的**：

- 不從字裡認出電腦（T18）。
- 伺服器不跑 Bot 的腦（`spec/07`：server 不准依賴 runner）。

**8. 驗收清單**：

1. API 文件、範例、錯誤碼都有。
2. 4 個測試都過。
3. 基準檔拿掉 `T-task-entry-1`。

---

### T13 執行端領信跑一輪＋執行服務

**1. 目標**：做一個共用的 `RoundWorker`：收到信的提示 → 去領 → 領到了就用 `continue_task` 跑一輪 → 紀錄走 T6 的匣子送上去。`apps/runner-service` 用它在伺服器上跑手機發起的那一輪（server-brains S1、mailbox-claim S1–S3）。

**2. 開工條件**：T6、T12 在 `dev` 上。

**3. 只准改的範圍**：

- `crates/sync/src/` 新檔 `mailbox_client.rs`：領信的 HTTP，只准在 sync 裡發。
- `crates/runner/src/` 新檔 `round_worker.rs`。
- `apps/runner-service/src/{lib.rs, main.rs}`。
- `apps/server/tests/`。

**4. 定好的介面**：

- `trait Mailbox { fn claim(&self, letter) -> …; fn hints(&self) -> … }`，由 sync 實作。
- `RoundWorker::new(RoundWorkerParts { mailbox, store, runners: Arc<dyn RunnerFactory>, clock }).run_until(shutdown)`。
- `RunnerService::start(ServiceParts { server_url, credential: Secret, computers: Arc<dyn BotComputerSource>, clock })`。
- `main.rs` 先不接憑證的來源，等 Q3。測試用函式庫，憑證從建構函式傳進去。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 手機的一句話 | X 領信，在 Bot 的電腦（假電腦端）上跑完；紀錄的裝置＝X | server-brains S1 |
| B 在線但是不領信，10 秒後 | 改派到 X；X 跑完；B 之後去領舊的那封，回「已經改派」，B 什麼都沒寫 | mailbox-claim S2、S3 |
| 領信被拒（別台領走、已經領過） | 不跑；記一筆原因的代碼 | mailbox-claim S1 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_mailbox_claim_2` | T-mailbox-claim-2 | 見上表第 2 列（伺服器的時鐘用假時鐘快轉 10 秒） | 整合（pg） |
| `pg_runner_service_runs_phone_round` | 第 1 列 | 任務＝完成（已驗收）；檔案在假電腦端 | 整合 |
| `pg_claim_refused_does_nothing` | 第 3 列 | 見上表 | 整合 |

**7. 不准做的**：

- 不讀環境變數、參數或檔案拿憑證（K118 ⑬）。
- 不開 Docker（M9）。
- 不碰伺服器的資料庫（`spec/07`）。

**8. 驗收清單**：

1. `apps/runner-service` 不依賴 `server`、`store-server`（工具會擋）。
2. 3 個測試都過。
3. 回報寫明 `main.rs` 在等 Q3。

---

### T14 使用者電腦的背景程式 `apps/agent`

**1. 目標**：`apps/agent` 成形。

- 啟動時先 `recover`（T7），才去領信。
- 跑交給這台的那一輪，操作這台的電腦。
- 程序真的被殺掉、重開之後照常（task-journal S2）。

**2. 開工條件**：

- T7、T13 在 `dev` 上。
- 工具／檔案區的 `LocalComputer` 在 `dev` 上（見 E）。

**3. 只准改的範圍**：

- `apps/agent/src/` 新檔 `lib.rs`，和 `main.rs`。
- `apps/agent/tests/`。
- `tests/harness`：只用 launch，不改。

**4. 定好的介面**：

- `Agent::start(AgentParts { server_url, credential: Secret, store_path, computer: Arc<dyn GatedComputer>, clock })`。
- `Agent::send_local(bot, text)`：
  - 連得上時，打 T12 的 API，`origin` 是這台。
  - 離線的那一半在 T16。
- 裝置的憑證放在 store-client 管的本機資料庫裡。只有 store-client 准寫檔（`spec/07`）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 最後一筆寫到一半就被殺掉 | 正常開機；前面的步驟照樣在紀錄裡；寫一半的那一步沒有執行 | task-journal S2 |
| 啟動 | `recover` 跑完之前不去領信 | task-resume S1 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_task_journal_2` | T-task-journal-2 | 真的 agent 程序。用 PID 殺掉之後，截掉 WAL 最後一個 frame，模擬寫到一半（回報寫明是模擬）；重開之後程序活著；前面的步驟都在；被截掉的那一步的檔案不存在 | 整合 |
| `agent_recovers_before_claiming` | 第 2 列 | 紀錄裡 recover 的結果早於第一次領信 | 整合 |

**7. 不准做的**：

- 不做桌面視窗跟背景程式之間的介面（桌面或手機畫面區）。
- 不按名字殺程序：只准用自己啟動的 PID。

**8. 驗收清單**：

1. 測試裡每個等待都有期限。
2. 2 個測試都過。
3. 基準檔拿掉 `T-task-journal-2`。

---

### T15 裁判紀錄和卡片答案送回執行端

**1. 目標**：伺服器寫的停止、取消、卡片答案，經過同步進到執行端的本機副本，叫醒在等的那一輪。

- 卡片答案的信，執行端不在時留著，回來再拿（mailbox-claim S5）。
- 斷線時的事實，連上之後排在取消後面（task-events S7）。

**2. 開工條件**：T5、T14 在 `dev` 上。

**3. 只准改的範圍**：

- `crates/store-client`：副本實作 `TaskWatch`。
- `crates/runner/src/round_worker.rs`：處理卡片答案的信。
- `apps/agent`。
- `migrations/server/0210–0214`（預留）。
- 測試。

**4. 定好的介面**：

- `Store` 實作 `anybot_runner::TaskWatch`：寫進這件任務的副本時發通知。
- 卡片答案的信只是「答案已經是任務事件」的提示，執行端讀副本。
- `append_unless_halted`（T2）在副本加匣子上實作。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| A 在等卡；關掉 A 的背景服務 10 秒，期間 P 按允許這次，再打開 | A 回來之後拿到允許，這一輪接著做，動作有執行 | mailbox-claim S5 |
| A 斷網時記了 3 筆工具事件；P 取消；恢復 | 伺服器上取消在前、那 3 筆在後；3 筆都留著；A、P 的主狀態都是已取消 | task-events S7 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_mailbox_claim_3` | T-mailbox-claim-3 | 見上表第 1 列（真的 agent 程序，用 PID 關掉） | 整合（pg） |
| `t_task_events_8` | T-task-events-8 | 見上表第 2 列（用 netctl 斷網；P 是替身） | 整合（pg） |

**7. 不准做的**：

- 不改伺服器的裁判規則（M6b 已經有）。
- 不做手機畫面。

**8. 驗收清單**：

1. 2 個測試都過。
2. 基準檔拿掉 2 個編號。

---

### T16 離線時自己發起的一輪與編號

**1. 目標**：這台自己發起、離線時跑的那一輪：

- 由執行端先寫任務建立（K119 ⑬）。
- 證據的短編號、第幾步，連上之後不重排（turn-loop S12）。
- 伺服器照收到的順序給任務編號。

**2. 開工條件**：T14 在 `dev` 上。

**3. 只准改的範圍**：

- `apps/agent`（`send_local` 離線的那一半）。
- `crates/server/src/api_streams_append.rs`：收離線時送出的、執行端寫的 `TaskCreated`。
- 測試。

**4. 定好的介面**：

- 執行端寫 `TaskCreated` 只准在「這台發起、離線」的時候，那一筆標 `offline_sent`。其他時候伺服器回錯誤代碼。
- 短編號存在紀錄裡（現在就是這樣），伺服器不改。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| A 斷網；讀 a、讀 b、回「好」；恢復 | 斷網時是 E1、E2；同步完還是 E1、E2；第幾步不變；任務編號連續、不重複 | turn-loop S12 |
| 隨機 1,000 組：事實、斷線、兩台裝置的裁判動作 | 任務編號從 1 開始、連續、跟紀錄流的順序一致；每台算出來的狀態跟伺服器一樣；暫時的狀態被換掉 | task-events S7 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_turn_loop_14` | T-turn-loop-14 | 見上表第 1 列；斷的只是 A 到伺服器那一段，假模型照樣連得到 | 整合（pg） |
| `t_task_events_7` | T-task-events-7 | 見上表第 2 列 | 性質（`prop`，pg） |

**7. 不准做的**：

- 不做伺服器停機時在 A 答卡的畫面（M7k）。

**8. 驗收清單**：

1. 伺服器拒絕「連得上時，執行端寫的 `TaskCreated`」，有測試。
2. 2 個測試都過。

---

### T17 全路徑：一輪的紀錄和多台同步

**1. 目標**：用真的伺服器、執行端和兩台裝置跑完整的一輪，證明：

- 每件事只有一筆紀錄。
- 欄位照 task-events S2。
- 三台最後的任務清單一樣。

**2. 開工條件**：T15、T16 在 `dev` 上；Q18 有答案。

**3. 只准改的範圍**：

- 新的測試檔。
- 產品程式只准修這兩個測試抓到的錯，回報裡逐條寫出來。

**4. 定好的介面**：沒有新介面。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 桌面送「建檔」；假模型 edit_file(a.txt, "", "hi")；回「好了」 | 這一輪的每件事只有一筆；每一筆帶 S2 的欄位；沒有「任務狀態變更」；`TaskCreated` 的裝置＝伺服器，其他＝執行端；桌面和手機各自算出來都是完成（已驗收） | task-events S2、S7 |
| 兩台電腦、一支手機；隨機斷網、恢復、送訊息、狀態變化 300 次，最後全部恢復 | 三台的任務清單（件數、主狀態、細項）完全一樣 | task-sync S1 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_task_events_9` | T-task-events-9 | 見上表第 1 列 | 整合（pg） |
| `t_task_sync_3` | T-task-sync-3 | 見上表第 2 列 | 性質（`prop`，pg） |

**7. 不准做的**：

- 不為了讓測試過去改產品的行為。

**8. 驗收清單**：

1. 2 個測試都過。
2. 基準檔拿掉 2 個編號。

---

### T18 這一輪在哪跑：路由規則接上

**1. 目標**：伺服器交「一輪」的信時，照 mailbox-route 的順序選電腦。Bot 設定的「做事的電腦」生效。選到的那台不在線就改派到 Bot 的電腦，並寫改派那一行。模型說的電腦不算。

**2. 開工條件**：

- T13 在 `dev` 上。
- Q12 有答案。
- 工具區的「工具帶電腦參數」和 computers-default S3 的拒絕在 `dev` 上。

**3. 只准改的範圍**：

- `crates/server/src/mailbox_deliver.rs`、設定。
- `crates/domain/src/mailbox_route.rs`（加上「從字認出電腦」的純函式，如果 Q12 這樣定）。
- `crates/protocol`（設定鍵）。
- `migrations/server/0215–0219`（預留）。
- 測試。

**4. 定好的介面**：

- Bot 設定 `work_computer: Option<ComputerId>`。
- 改派那一行：聊天紀錄，代碼 `mailbox-reassigned-offline{computer}`。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 做事的電腦＝B；A 送一句；A 再送「在 A 上跑 ls」 | 第一輪在 B；第二輪在 A | mailbox-route S2、S3 |
| 沒指定；P 送一句；模型呼叫 shell(電腦＝B) | 這一輪還是在 X；那一步被拒絕（原因照 computers-default S3） | 永遠成立 |
| 做事的電腦＝B，B 關著；P 送一句 | 在 X 跑；有改派那一行 | mailbox-route S6 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_mailbox_route_2` | T-mailbox-route-2 | 見上表第 1 列 | 整合（pg） |
| `t_mailbox_route_6` | T-mailbox-route-6 | 見上表第 2 列 | 整合（pg） |
| `t_mailbox_route_7` | T-mailbox-route-7 | 見上表第 3 列 | 整合（pg） |

**7. 不准做的**：

- 不做 computers-n5（模型只有某台連得到）。
- 不做團隊成員。
- 不做例行任務。

**8. 驗收清單**：

1. 3 個測試都過。
2. 基準檔拿掉 3 個編號。

---

### T19 任務通知

**1. 目標**：伺服器收到任務紀錄就重算狀態。

- 進入卡住、或不是核准卡的等你，推播一次。
- 完成、未驗收不推播（task-state S8、task-notify S1、S2、S5）。

**2. 開工條件**：

- T4、T11、T13 在 `dev` 上。
- 推播發送的介面（見 E、Q17）在 `dev` 上。
- 「做到哪」的報告（信任區）在 `dev` 上。

**3. 只准改的範圍**：

- `crates/server/src/` 新檔 `task_notify.rs`。
- `migrations/server/0220_task_notifications.sql`。
- 測試。

**4. 定好的介面**：

- `trait NotifyOwner { fn notify(&self, account, task, title_bot, body_code, params) }`，推播區提供。
- 表 `task_notifications(task_id, kind, sent_at_ms)`：同一件任務的同一種只推一次，新的取代舊的。
- 核准卡的推播不在這裡做（第 2 組）。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| edit x；shell make 結束碼 2；回「做好了」 | 未驗收（驗收沒通過）；沒有推播 | task-done-gate S6 |
| 模型判斷一項，驗收永遠回還沒、理由每輪不同；每輪 edit a.txt 一次 | 你起的那一輪之後自己接 3 輪；剛好 3 個請求帶 `task-verify-not-passed`；未驗收；沒有推播；從頭到尾沒有等你 | task-done-gate S3 |
| 沒設任何上限；每步讀一個不同的檔 | 請求剛好 300 個，第 301 個沒送；卡住（步數 300／300）；推播剛好 1 則；交付報告是一份「做到哪」 | task-budget S3 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_task_done_gate_8` | T-task-done-gate-8 | 見上表第 1 列；harness 的 fake push 紀錄是 0 筆 | 整合（pg＋執行服務） |
| `t_task_done_gate_9` | T-task-done-gate-9 | 見上表第 2 列 | 整合 |
| `t_task_budget_7` | T-task-budget-7 | 見上表第 3 列 | 整合 |

**7. 不准做的**：

- 不做通知種類的設定、「在電腦前」的判斷（第 6 組，M8）。
- 不做目標的推播。

**8. 驗收清單**：

1. 同一種推一次，有測試。
2. 3 個測試都過。
3. 基準檔拿掉 3 個編號。

---

### T20 卡住偵測：沒有動靜 15 分鐘

**1. 目標**：進行中的任務 15 分鐘沒有任何動靜，就是卡住（沒有動靜 15 分鐘），推播一次。有工具還在跑時不算卡住。一有動靜就回到進行中（task-stall S1–S4）。

**2. 開工條件**：

- T19 在 `dev` 上。
- Q16 有答案（誰判斷、模型「停在思考中」算不算動靜）。

**3. 只准改的範圍**（照 Q16 的建議）：

- `crates/runner/src/` 新檔 `stall_watch.rs`。
- `crates/server/src/task_notify.rs`（加這一種）。
- 測試。

**4. 定好的介面**：

- 執行端的看門狗用 `Clock`。動靜＝紀錄、模型串流的字、正在跑的工具。滿 15 分鐘寫 `Stalled{minutes: 15}`，之後每分鐘更新 N。
- 伺服器看到 `Stalled` 就推播，代碼 `task-stalled{minutes}`。這則推播沒有允許／拒絕的按鈕。

**5. 行為表**：

| 情況 | 預期 | 出處 |
|---|---|---|
| 假模型回「好的，我」之後停在思考中；快轉 15 分鐘 | 卡住（沒有動靜 15 分鐘）；推播「…沒有動靜 15 分鐘」 | S1 |
| shell(sleep 1200)；快轉 16 分鐘 | 進行中；沒有卡住的推播 | S2 |
| 接著上面第 1 列，模型繼續出字「…寫好了」 | 回到進行中，再到完成 | S4 |
| 卡住的推播 | 沒有允許／拒絕 | task-notify S7 |

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_task_stall_1` | T-task-stall-1 | 見上表第 1 列 | 整合 |
| `t_task_stall_2` | T-task-stall-2 | 見上表第 2 列 | 整合 |
| `t_task_stall_3` | T-task-stall-3 | 見上表第 3 列 | 整合 |
| `t_task_notify_7` | T-task-notify-7 | 見上表第 4 列；推播的 payload 的 actions 是空的 | 整合 |

**7. 不准做的**：

- 不做追進度中的那一種（M9）。
- 不改 chat-stall 的 2 分半規則（第 3 組）。

**8. 驗收清單**：

1. 4 個測試都過。
2. 基準檔拿掉 4 個編號。

---

### T21 一輪結束原因的性質測試

**1. 目標**：把 turn-loop S10 那張表的每一種結束原因放進同一個性質測試，驗「每一輪剛好一筆一輪結束」和「那筆之後沒有請求、也沒有動作開始」。

**2. 開工條件**：

- T2、T4、T5、T7、T8 在 `dev` 上。
- Q19 有答案（50,000 步那一格怎麼驗）。

**3. 只准改的範圍**：`crates/runner/tests/`、`tests/runner-test`。

**4. 定好的介面**：沒有新介面。

**5. 行為表**：照 turn-loop S10 的 11 格。

- 你起的那一輪等卡：在時間範圍內沒有 `TurnEnded`、狀態是等你，這就算對。
- 當機的那一格：重開之後用 `recover`。

**6. 要寫的測試**：

| 測試名稱 | 驗哪一列 | 預期值 | 種類 |
|---|---|---|---|
| `t_turn_loop_5` | T-turn-loop-5 | 1,000 輪，每輪在隨機時間點發生 S10 的其中一種：剛好一筆 `TurnEnded`（等卡那一格除外），原因跟那一格一樣；之後沒有 `ModelAsked`、`ToolStarted`；任務狀態跟那一格一樣 | 性質（`prop`） |

**7. 不准做的**：

- 不為了跑得快改產品的常數；照 Q19。

**8. 驗收清單**：

1. 11 格都有被抽到，有統計斷言。
2. 基準檔拿掉 `T-turn-loop-5`。

---

## D) 要問的問題（每題附建議）

### 第一批：擋住切片的

1. **Q1 你的一句話什麼時候接回上一件，什麼時候開新的？**
   - 衝突：
     - task-entry S1 說：沒有你起的一輪在跑，就開新的。
     - goal-brake S10、turn-loop S11、task-budget S4、T-goal-brake-11、T-plan-done-4、T-chat-stop-7 說：送一句話就在同一件接著做。
   - 建議：這個 Bot 最近的一件是未驗收、卡住、失敗、等你或已停止，而且沒有一輪在跑 → 接回那件。完成、已取消 → 開新的。
   - 擋住：T12（排隊那一格）、T10 的「再試」。

2. **Q2 /goal、/plan 放哪個里程碑？**
   - `docs/plan.md` 的 M7–M10 都沒有寫到它們；M7d 也明說不做。
   - 建議：放 M9，跟追進度、伺服器叫醒一起，因為下一輪和叫醒用同一套。不是 /goal 的任務自己接下一輪（T10、T11）照樣在 M7 做。
   - 影響：A 裡 76 列。

3. **Q3 執行服務的憑證怎麼交到它手上？**（K118 ⑬：不放命令列參數、環境變數、檔案）
   - 建議：伺服器和執行服務兩個容器共用一個只有它們掛的 volume，裡面放一個 Unix socket。執行服務啟動時連上去，伺服器交一次憑證。socket 本身不含秘密。
   - 擋住：T13 的 `main.rs`、T-server-brains-6。

4. **Q4 繼續卡和「按繼續」的協定。**
   - 建議：
     - 繼續卡＝`CardShown`，多一個卡片種類 `resume`，帶「可能已經執行」的動作清單和放回的話。
     - 「按繼續」＝裁判動作 `Wake{reason: resume}`。task-entry S9 把「中斷後接著做」列成一種叫醒。
     - 過時的卡（S4）由伺服器用「之後你有沒有再說話」判。
   - 擋住：T7（T-task-resume-7）、M7k 的 resume 列。

5. **Q5 換到備用模型時，閘道怎麼交回執行端？**（M7d 第 5 點）
   - 建議：閘道重試用完時，回 `Exhausted{fallback: Some(M2)}`，自己不去問 M2。執行端寫換模型、重組，再指定 M2 問一次。
   - 擋住：T8。

6. **Q6 形式驗證用什麼工具？**（T-sync-referee-1、T-phone-cards-1、T-task-resume-8、T-task-wake-3）
   - TLA+ 不在可以安裝的清單裡。
   - 建議：在 `tests/` 底下用 Rust 自己寫一個窮舉（BFS 列出所有先後順序），狀態機用 domain 的純函式。不引進第三方 model checker。

7. **Q7 「關掉電腦 A、打開電腦 A」在執行端這一層怎麼算？**
   - 建議：
     - 整合列：丟掉 runner，用同一個 SQLite 檔重建一個，算關掉和打開（K119 ⑰ 的延伸）。
     - 「寫到一半」（T-task-journal-2）用真的 agent 程序，用 PID 殺掉。
   - 擋住：T7 能不能用編號。

### 第二批：設計細節

8. **Q8** 收尾時才到的插話，變成下一輪的開頭（chat-steer S3）。那一輪算同一件任務嗎？建議：算同一件，因為它是插話進來的（task-entry S2）。

9. **Q9** 按停止時，那張在等的卡「當成拒絕收掉」。`CardAnswered{Deny}` 由誰寫？建議：發卡的執行端寫。之後的答案，伺服器照 `card_is_waiting` 拒掉。

10. **Q10** 「提出標準」工具的英文名和參數。建議：`propose_done_criteria { items: Vec<Criterion> }`，用 protocol 現有的 `Criterion`。

11. **Q11** 驗收沒過自己接下一輪，是執行端直接接，還是向伺服器送叫醒？叫醒是裁判動作（task-events S7）。建議：
    - 連得上：執行端送 `RefereeAction::Wake{verification}`，伺服器寫叫醒並交信。
    - 離線、這台發起：執行端自己寫。
    - 煞車由 domain 的純函式判。

12. **Q12** 從訊息的字認出電腦（mailbox-route S2，K118 ⑰ 說屬執行端）。但路由要在交信之前決定，交信的是伺服器。建議：寫成 domain 的純函式，伺服器交信前呼叫。規則：帳號裡的電腦名稱整個詞出現在字裡、而且剛好只對到一台，才算指名。

13. **Q13** 「這件任務的上限」在測試和產品裡怎麼設？建議：Bot 設定的上限在任務建立時寫成 `BudgetLimit` 事件。之後你改這件的上限，走裁判（再寫一筆 `BudgetLimit`）。

14. **Q14** 驗收的模型怎麼交答案；一般任務的驗收模型沒回應時算什麼（goal-brake S10 沒列 S5）。建議：
    - 用 `submit_verdict` 工具交答案，只回文字算沒有回應。
    - 一般任務沒有回應 → 未驗收（有沒驗到的動作）。

15. **Q15** 「查看任務」工具（task-bot-1、2、5）放哪個里程碑？建議：M8，跟任務頁同一份資料。

16. **Q16** 誰判斷「沒有動靜 15 分鐘」？T-task-stall-1 的「停在思考中」會不會先被 chat-stall 的 2 分半判失敗？建議：
    - 執行端判斷：它看得到工具在跑和模型的串流。伺服器只負責推播。
    - 思考的字算動靜，所以 chat-stall 不會觸發。

17. **Q17** 伺服器發 Web Push 的程式放哪？`spec/07` 只准 models、mcp、sync、channels、computers 發 HTTP。建議：放 `channels` 的 `web_push` 模組，由手機畫面區做，本區只用 `NotifyOwner`。

18. **Q18** T-task-events-9 的前提「假電腦端（宣告成 Bot 的電腦）」，跟「在桌面送」照 mailbox-route S4 本來在 A 跑，兩者衝突。建議：這一句指名 Bot 的電腦，由執行服務 X 跑。

19. **Q19** T-turn-loop-5 有「一輪滿 50,000 步」那一格，1,000 輪裡跑 50,000 步太慢。建議：一輪的步數上限從建構函式傳進去，產品的預設是 50,000，性質測試用 50。另外用 T-chat-n11-1 真的跑一次 50,000。

### 第三批：沿用規格裡已經在問的、本區以外的

20. **Q20** 規格「要問使用者」的第 4 題（例行任務執行的任務留 1 年還是永久）、第 5 題（計畫只叫醒一次、追進度檢查的逾時、工作流的佇列上限）。都屬 M9 或之後，不擋 M7。

21. **Q21** chat-followthrough（說了就做）、chat-n11 S2–S5（每 25 步的提醒、重複動作的煞車）是 turn-loop 每一步的規則，但不在任何規劃區。建議：歸本區，在 T21 之後另開 3 件。

---

## E) 跟其他規劃區共用的介面

### 信任／核准區

- **本區要用的：**
  - `anybot_trust::answer_ask`（已經有）。
  - 「一律允許：在這台跑某指令」的規則：T5 的 T-turn-loop-15 要用。
  - `check_criterion(gated, &Criterion) -> CriterionOutcome`：跑 FileExists、FileContains、CommandExitZero，過把關，證據標驗收（trust-verify）。T9 要用。
  - 寬鬆核對（claims-check）在收尾的掛點。
  - 「做到哪」的報告（report-delivery S1）：T19 要用。
  - 遮秘密的函式 `mask_secrets(&str) -> String`：T6 要用。
- **本區提供的：**
  - 卡片在一輪裡等的語意：你起的那一輪一直等；沒人在看的那一輪 15 分鐘後 `CardTimedOut`，卡留著（T5）。
  - 停止時由執行端寫 `CardAnswered{Deny}`（Q9）。

### 工具／檔案／交付區

- **本區要用的：**
  - 每個工具的 `ToolEffect` 跟著 registry 維護：T6 的 T-task-journal-1 只讀它。
  - 「刪除檔案」工具：T-chat-stop-4、T-sync-referee-3 要用。
  - 能中斷正在跑的指令：`GatedComputer` 加取消的入口，或回傳一個可以 kill 的 handle（chat-stop S2）。現在的方法是同步的，中斷不了。
  - `anybot_computers::LocalComputer: GatedComputer`：T14 要用。
  - 工具帶 `computer` 參數，以及 computers-default S3 的拒絕代碼：T18 要用。
  - 可重複工具的冪等鍵和對帳查詢：task-resume S1；M7 只有介面，沒有工具用到。
- **交給他們驗的：** T-task-keep-2、3（完整輸出 30 天）。

### 記憶／上下文區

- **本區要用的：**
  - 每一步的編譯（已經有）。
  - 「用新模型的能力重新編譯」的入口：T8 要用。
  - 插話在請求裡長什麼樣：T3 只負責把它記成你的訊息。
  - 給驗收模型的「最近約 24,000 字＋證據清單」組法：T11 要用。
- **本區提供的：** `ModelSwapped`、`ContextBuilt` 的寫入時機（T8）。

### 手機畫面區（M7k）

- **本區提供的：**
  - `POST /v1/bots/{bot}/messages` 和 `MessageOutcome`：「已排入」照 `Steer` 顯示（T12）。
  - 新的訊息代碼：`turn-reply-stopped`、`turn-resume-*`、`task-verify-not-passed`、`mailbox-reassigned-offline`、`task-stalled`、`judge-tool-not-allowed`。
  - 繼續卡（Q4）。
  - 任務狀態照 domain 算。
- **本區要用的：**
  - Web Push 的發送，`NotifyOwner`（Q17）。
  - 桌面視窗到 `apps/agent` 的本機介面，`Agent::send_local` 由它呼叫。
- **交給他們驗的：** A 裡標「M7（M7k）」的 13 列，和相鄰的端對端列。

### 監督者要決定的

- D 的 Q1–Q7 擋住 T7、T8、T10、T12、T13。
- 不擋的切片可以先開工：T0 → T1 → T2 → T3 → T4 → T5、T6，和 T9（等 Q10）。
