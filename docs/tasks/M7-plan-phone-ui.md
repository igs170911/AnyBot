# M7 規劃：手機畫面（M7k）

這份是 M7 手機畫面這一區的規劃，不是可以直接派的任務檔。C 段每一件照 `docs/tasks/_template.md` 寫成藍圖，監督者答完 D 段的問題後，把一件抄成 `docs/tasks/M7k-<n>.md` 派出去。

- **範圍**：`web/ui`、`web/api-client`、`apps/web`、`tests/e2e`、`tests/testenv`，加上 `tests/harness` 裡給假模型、假電腦用的小改動。
- **目標**：做出最小的手機畫面，讓第一條從頭到尾的路可以用、也可以用 Playwright 測。測試用假模型、假電腦，流程是：送一句話 → 看逐字的回答和步驟 → 答核准卡 → 看任務狀態和交付報告。其他畫面都留到 M8。
- **片名**：別區的規劃把這一區叫 M7k，這份照用。別區切片的代號照它們自己的規劃寫：
  - turn／task 區（`M7-plan-turn-task.md`）：T0–T21。
  - trust／cards 區（`M7-plan-trust-cards.md`）：S1–S13。
  - tools／delivery 區（`M7-plan-tools-delivery.md`）：F1–F12。
  - 記憶區（`M7-plan-brain-context.md`）：B1–B29。
  - 伺服器整合那一段叫 M7j。
- **編號的規則**（`09` K86、K118 ⑱、K119 ⑰）：
  - 測試名稱帶編號，就要把那一列「預期」那一欄全部驗到。
  - 端對端的列要從瀏覽器操作真的網頁，連到真的伺服器和執行端。「模型收到的請求」「檔案內容」這類資料，從 testenv 的控制 API 讀，也算從外面驗。
  - 只驗到一部分的，用描述性的名字，編號留在基準檔。

---

## A. 範圍分類

**怎麼數的**：
- 從 `tools/spec-trace/baseline.txt`（2,501 個還沒有測試的編號）挑出跟畫面有關的列。挑的條件是下面任一項：
  - 種類是端對端或實機；
  - 種類是整合，而且預期寫到「畫面」；
  - 屬於這一區的段（`chat-`、`cards-`、`screen-`、`phone-`、`phonefit-`、`report-`、`bots-`、`profile-`、`appearance-`、`settings-`）；
  - 別區的規劃明寫交給 M7k 的列。
- 這樣一共挑出 **1,175 列**。同一段裡分類一樣的列合成一行。
- 「M7·他區」是這些列裡屬於執行端或伺服器那一層的整合、單元、性質列，由別區的規劃拿編號，這一區不驗。

已經有測試、不在基準檔的 UI 列只有 `T-phonefit-width-1`（`tests/e2e/smoke.spec.ts`）。`T-approval-answers-1` 也已經不在基準檔：它已經由執行端的 `crates/runner/tests/turn_approval.rs::t_approval_answers_1` 拿走了（見 D 的 Q11）。

**合計**

| 分類 | 列數 | 意思 |
|---|---|---|
| M7 | 34 | 這一區的切片會拿掉的編號；依賴的是別區規劃裡已經排進 M7 的切片 |
| M7·條件 | 36 | 也排在這一區，但要等 D 段的問題答了，或要等別區還沒定的切片（開關卡、插話、繼續卡、傳檔進聊天、M7j） |
| M7·他區 | 108 | 整合、單元、性質列，屬於執行端或伺服器那一層，由別區的規劃負責 |
| M8 | 458 | 桌面、設定、任務頁、Bot 管理、離線、推播、斜線指令 |
| M7.5 | 17 | 大腦（`memory-*`） |
| M9 | 163 | Bot 的電腦、看畫面、例行任務、排程、/goal、計畫、支線 |
| M10+ | 203 | 頻道、連接器、市集、保險庫、外部 agent、技能、頁面、安裝、遷移、模型供應商 |
| 還不能測 | 156 | 實機列，或形式驗證：要真的手機、真的沙盒、真的 macOS |
| **合計** | **1,175** | |

**每一段**

| 組 | 段 | 未驗列 | 分類（列數） | 在 M7 的編號（切片） | 理由 |
|---|---|---|---|---|---|
| 1 | `task-object` | 1 | M7·他區 1 | — | 整合列，執行端那層（turn／task 區） |
| 1 | `task-state` | 3 | M7 2、M7·條件 1 | 1（U10）、5（U11）、2（U19） | 1：手機送一句到完成（已驗收）；5：按掉 👍；2：要 send_file 和交付確認（tools F10、M7j） |
| 1 | `task-proof` | 3 | M7·他區 2、M8 1 | — | 整合列歸 turn 區；3 要任務頁（M8） |
| 1 | `task-resume` | 5 | M7·條件 4、還不能測 1 | 2、3、5、6（U21） | 繼續卡的畫面，等 turn 區 Q4（繼續卡）和 T7、T15；9 實機 |
| 1 | `task-journal` | 1 | M7 1 | 3（U7） | 手機畫面不含假金鑰，等 turn 區 T6 的遮秘密 |
| 1 | `task-wake` | 2 | M7·他區 2 | — | 整合列，執行端那層 |
| 1 | `task-handoff` | 1 | M7·他區 1 | — | 整合列，執行端那層 |
| 1 | `task-list` | 5 | M8 5 | — | 任務清單畫面（M8） |
| 1 | `task-badge` | 2 | M8 2 | — | 任務數字標記（M8） |
| 1 | `task-sync` | 2 | M8 2 | — | 多台的任務清單畫面（M8） |
| 1 | `task-notify` | 1 | M7·他區 1 | — | 整合列，推播在伺服器 |
| 1 | `goal-brake` | 8 | M7·條件 1、M9 7 | 11（U20） | 11：一般任務的煞車，turn 區 T10 交過來，要「在桌面送」（Q4）；其他是 /goal（M9） |
| 1 | `goal-set`、`goal-n3`、`goal-next`、`goal-done`、`goal-followup`、`goal-n2` | 14 | M9 14 | — | /goal、追進度（M9） |
| 1 | `plan-*`、`routines-*`、`schedule-*`、`workflow-*` | 44 | M9 44 | — | 計畫、例行任務、排程、工作流程（M9） |
| 1 | `branch-*` | 11 | M9 9、還不能測 2 | — | 支線 /btw（M9） |
| 2 | `trust-pipeline` | 2 | M7 1、M7·他區 1 | 3（U5） | 3：模型自稱「【系統】」的字照一般回答畫；5 整合歸 trust 區 |
| 2 | `trust-floor` | 3 | M7·條件 1、M7·他區 1、M8 1 | 2（U13） | 2：卡片只有兩個按鈕，等 trust 區 S3、S6；1 歸 trust 區；3 要設定 › 權限（M8） |
| 2 | `approval-answers` | 6 | M7 1、M7·條件 1、M8 4 | 4（U8）、5（U15） | 4：按拒絕；5：要「在桌面送」（Q4）；2 例行任務（M9 才有）；3 一律允許清單畫面、6 每種卡含頻道、9 設定 › 權限（M8） |
| 2 | `trust-verify` | 1 | M7 1 | 6（U11） | 手機按 👍 之後的狀態 |
| 2 | `report-delivery` | 13 | M7·條件 1、M7·他區 11、M10+ 1 | 9（U11） | 9：報告多一行「你已確認」，要「在桌面送」（Q4）；1 要寄信工具（M10+）；其他整合列歸 trust 區 S8、S9 |
| 2 | `trust-promises` | 5 | M7·他區 2、M8 2、還不能測 1 | — | 要寄信、點擊工具和管理員 hook；整合列歸 trust 區 |
| 2 | `safety-card` | 8 | M7 2、M8 6 | 2（U8）、8（U9） | 2：別的 Bot 的聊天看不到卡；8：等卡時按停止；1 要桌面和手機兩邊、3–4 推播和聲音、5–7 桌面鍵盤和滑鼠（M8） |
| 2 | `safety-exec-card` | 6 | M7·條件 6 | 1、4、6、7（U12）；2、3（U13） | 開關卡，等 trust 區 S3；2、3 看按了之後的設定（資料） |
| 2 | `safety-always` | 4 | M8 4 | — | 設定 › 權限 › 一律允許清單畫面（M8） |
| 2 | `safety-reject` | 3 | M7 1、M7·條件 1、還不能測 1 | 1（U8）、2（U15） | 1：拒絕後同一輪不再問；2：要「在桌面送」（Q4）；3 實機 |
| 2 | `safety-local` | 3 | M7·他區 1、M8 2 | — | 設定 › 電腦（M8）；4 整合歸 trust 區 |
| 2 | `safety-lock` | 4 | M7·他區 1、還不能測 3 | — | 管理員鎖定；多數實機 |
| 2 | `safety-timeout` | 3 | M7·條件 1、M7·他區 1、M8 1 | 7（U15） | 7：要「在桌面送」和時鐘快轉 2 小時（Q4）；1 歸 trust 區；8 要任務清單（M8） |
| 2 | `safety-gates` | 1 | M7 1 | 6（U9） | 按停止後 0.2 秒內變被拒 |
| 2 | `safety-n4`、`safety-n9` | 2 | M8 2 | — | 頻道開的一輪、瀏覽器送出（工具在 M10+；畫面 M8） |
| 2 | `safety-n7` | 1 | 還不能測 1 | — | 實機 |
| 2 | `safety-n8`、`safety-n11` | 2 | M7·他區 2 | — | 整合，trust 區、伺服器 |
| 2 | `sandbox-*` | 13 | M7·他區 1、還不能測 12 | — | 幾乎都要真的沙盒（實機） |
| 2 | `autonomy-direct`、`autonomy-yolo`、`autonomy-review`、`autonomy-patterns`、`autonomy-hooks`、`autonomy-remote`、`autonomy-n1` | 15 | M8 15 | — | 直接操作、一律放行、規則、hooks 的設定畫面，和斜線指令的輸入（M8） |
| 2 | `autonomy-smart` | 4 | M7 1、M8 2、還不能測 1 | 9（U8） | 9：一般卡的按鈕組裡沒有「這輪都准」；其他是智慧模式的設定（M8） |
| 2 | `autonomy-n3` | 1 | 還不能測 1 | — | 實機 |
| 2 | `claims-check` | 2 | M7·條件 2 | 1、2（U18） | 「未驗證」標記，等 trust 區 S11 和 tools 區 F10（傳檔進聊天） |
| 2 | `claims-link` | 2 | M7·條件 2 | 1、2（U18） | 連結和預覽，等 trust 區 S11、tools 區 F10 |
| 2 | `claims-mark` | 2 | M8 2 | — | 桌面滑過、兩台桌面（M8） |
| 2 | `claims-wake` | 1 | M7·條件 1 | 1（U18） | 核對的叫醒那一則，等 trust 區 S12 |
| 2 | `claims-n1` | 1 | 還不能測 1 | — | 實機 |
| 2 | `activity-*` | 9 | M8 9 | — | 稽核和操作紀錄畫面（M8） |
| 2 | `files-rewind` | 1 | M8 1 | — | `/rewind` 斜線指令（M8） |
| 2 | `replay-*` | 6 | M10+ 6 | — | 學習和重播（M10+） |
| 3 | `chat-layout` | 4 | M8 4 | — | 桌面版面（M8） |
| 3 | `chat-input` | 7 | M7·條件 1、M8 6 | 6（U14） | 6：工作中的提示字和「已排入」，等 turn 區 T3、T12；其他是輸入框細節（M8） |
| 3 | `chat-commands` | 6 | M8 6 | — | 斜線指令選單（M8） |
| 3 | `chat-steer` | 5 | M7·條件 2、M7·他區 3 | 1、3（U14） | 1、3：插話；其他整合列歸 turn 區 T3 |
| 3 | `chat-stop` | 9 | M7 3、M7·條件 1、M7·他區 4、M8 1 | 1、2、3（U9）；6（U14） | 1–3：停止鈕和停止後的畫面；6：插話放回輸入框；8 要電腦 A 的畫面（M8）；其他整合列歸 turn 區 T2 |
| 3 | `chat-followthrough`、`chat-stall`、`chat-n3`、`chat-n11` | 22 | M7·他區 22 | — | 整合，turn 區 |
| 3 | `chat-compact`、`context-*`、`chat-n15` | 11 | M7·他區 11 | — | 整合，記憶／上下文區 |
| 3 | `chat-actions` | 9 | M8 9 | — | ⋯ 選單、重新產生、還原（M8） |
| 3 | `chat-select` | 4 | M8 3、還不能測 1 | — | 選取文字（M8；3 實機） |
| 3 | `chat-steps` | 10 | M7 7、M7·他區 1、M8 1、還不能測 1 | 7（U6）；6、8（U7）；1（U8）；3、4、5（U10） | 步驟區；2 單元歸 trust 區；9 實機；10 團隊成員（M8） |
| 3 | `chat-attach` | 9 | M8 9 | — | 附檔（M8） |
| 3 | `chat-window` | 7 | M7 1、M8 6 | 4（U6） | 4：手機每秒最多更新 1 次；1–3 載入更早的、5 斷網、6 桌面（M8） |
| 3 | `chat-n1`、`chat-n2`、`chat-n4`、`chat-n5`、`chat-n6`、`chat-n8`、`chat-n16` | 42 | M8 42 | — | 新對話、清除、引用、匯出、叫醒訊息、Markdown、團隊成員的訊息（M8） |
| 3 | `chat-n9` | 4 | M8 4 | — | 新對話的開場（M8；開場的內容規格沒寫） |
| 3 | `bots-*` | 68 | M8 65、還不能測 3 | — | 側邊欄、表情、搜尋、選單、建立、刪除、匯出、主要 Bot、快速鍵（M8；M7 的 Bot 由 testenv 建） |
| 3 | `cards-question` | 14 | M7 6、M7·他區 4、M8 1、M9 1、M10+ 1、還不能測 1 | 3、4、7、14（U16）；1、2（U17） | 問題卡，等 trust 區 S13（`ask_person`）；5、6、10、11 歸 trust 區和 M7j；8 側邊欄（M8）；9 形式驗證；12、13 頻道、交辦 |
| 3 | `cards-emoji`、`cards-n1` | 9 | M8 9 | — | 表情；👍 的整合列（M8） |
| 3 | `team-*`、`skills-*` | 28 | M10+ 28 | — | 團隊、技能（M10+） |
| 3 | `profile-*` | 25 | M8 24、還不能測 1 | — | Bot 設定頁（M8） |
| 3 | `memory-*` | 17 | M7.5 17 | — | 大腦（M7.5，記憶區 B28、B29 用這一區的畫面基礎） |
| 4 | `computer-ops`、`computer-caps`、`computer-rewind` | 6 | M7·他區 5、還不能測 1 | — | 整合，tools 區；caps-7 實機 |
| 4 | `computer-route` | 2 | M7·條件 1、M8 1 | 6（U21） | 6：中斷後按繼續、改派，等 turn 區 Q4；2 要任務詳情（M8） |
| 4 | `computer-platforms`、`files-shell` | 4 | 還不能測 4 | — | 實機 |
| 4 | `box-*`、`screen-*` | 119 | M9 87、還不能測 32 | — | Bot 的電腦、看畫面（M9） |
| 4 | `browser-*`、`apps-*` | 20 | M10+ 2、還不能測 18 | — | 瀏覽器工具、macOS App（M10+；多數實機） |
| 4 | `files-terminal` | 3 | M7·他區 2、還不能測 1 | — | 互動終端機，整合歸 tools 區 |
| 4 | `files-monitor`、`files-workdir` | 3 | M7·他區 1、M8 2 | — | 監看、工作資料夾的畫面（M8） |
| 4 | `files-background` | 2 | M7·條件 1、M7·他區 1 | 6（U14） | 6：按停止之後背景指令還在跑，等 tools 區 F11、F12 |
| 4 | `pages-*`、`pageview-*`、`library-*`、`deliver-send`、`deliver-n1`、`deliver-n2` | 37 | M10+ 37 | — | 頁面、所有檔案、檔案卡的桌面那一半、預覽、傳到別的裝置 |
| 4 | `deliver-confirm` | 2 | M7·條件 2 | 1、7（U19） | 交付記號，等 tools 區 F10 和 M7j 的交付確認 |
| 4 | `deliver-open` | 1 | 還不能測 1 | — | 實機 |
| 4 | `computers-local` | 2 | M7·條件 1、還不能測 1 | 1（U12） | 1：「在你的電腦上執行」的卡，等 trust 區 S3；5 實機 |
| 5 | `sync-append`、`sync-id`、`sync-outbox`、`sync-settings`、`sync-files`、`sync-offline` | 14 | M7·他區 1、M8 13 | — | 刪除和還原、離線匣子、設定同步、離線（M8） |
| 5 | `sync-seq`、`sync-replica`、`mailbox-one`、`account-bind` | 4 | M7·他區 4 | — | 整合，伺服器、客戶端函式庫 |
| 5 | `sync-referee` | 3 | M7·條件 1、M8 2 | 5（U15） | 5：兩台同時答卡（Q4）；3 要電腦 A 的畫面、4 要例行任務（M8） |
| 5 | `conn-push`、`conn-ws` | 4 | M8 4 | — | Web Push、看畫面的 WebSocket（M8／M9） |
| 5 | `mailbox-route` | 1 | M8 1 | — | 步驟詳情和任務頁（M8） |
| 5 | `mailbox-reassign` | 3 | M7·條件 1、M7·他區 1、M8 1 | 3（U20） | 3：報告寫做不到的部分，等 M7j；1 要電腦 A 的畫面（M8） |
| 5 | `computers-online` | 1 | M7·條件 1 | 3（U20） | 手機的離線橫幅，等 M7j 的在線狀態 |
| 5 | `computers-default` | 2 | M7·條件 1、M8 1 | 4（U20） | 4：沒有這台電腦的訊息，等 turn 區 T18；1 對話詳細資訊（M8） |
| 5 | `computers-kinds`、`computers-allowed`、`computers-n1`、`computers-n8` | 6 | M7·他區 1、M8 5 | — | 設定 › 電腦、Bot 可以用的電腦、SSH、電腦名稱（M8） |
| 5 | `computers-n2` | 3 | M7·他區 1、M8 1、M9 1 | — | 截圖要看畫面的工具（M9）；整合歸 turn 區 |
| 5 | `computers-n5` | 1 | M10+ 1 | — | 只有某台連得到的模型（M10+） |
| 5 | `computers-pair`、`computers-n10` | 3 | 還不能測 3 | — | 實機 |
| 5 | `account-login` | 4 | M7 4 | 1、5、7、8（U4） | 登入、註冊畫面 |
| 5 | `account-n1`–`n13`、`account-share`、`account-can`、`account-push`、`account-computers`、`account-move`、`account-backup` | 44 | M8 43、還不能測 1 | — | http 警告、兩步驟、本機副本、離線、裝置、改密碼、搬家（M8） |
| 5 | `server-*` | 36 | M7·他區 5、M8 14、還不能測 17 | — | 伺服器管理頁（M8）、安裝（實機）；整合列歸伺服器 |
| 5 | `phone-pages`、`phone-look`、`phone-colors`、`phone-net`、`phone-app`、`phone-n1`、`phone-n2` | 26 | M8 25、還不能測 1 | — | 手機的每種頁面、樣式、斷網、安裝成 App、時區、秘密（M8） |
| 5 | `phone-cards` | 3 | M7·條件 1、M8 2 | 2（U15） | 2：別台先答之後手機上的舊卡（Q4）；1 形式驗證、4 支線（M8、M9） |
| 5 | `phone-claude` | 2 | M7 2 | 1（U7）、2（U22） | 1：長按顯示步驟；2：每個可以點的都 44×44，掃 M7 的每個畫面 |
| 5 | `phonefit-measure`、`phonefit-ruler` | 3 | M8 2、還不能測 1 | — | 量出來的調整、量尺（M8） |
| 5 | `phonefit-short`、`phonefit-keyboard` | 3 | 還不能測 3 | — | 實機 |
| 5 | `install-*`、`migrate-*` | 53 | M10+ 22、還不能測 31 | — | 安裝、遷移 |
| 6 | `channels-*`、`connectors-*`、`market-*`、`vault-*`、`agent-*`、`garden-*` | 93 | M10+ 90、還不能測 3 | — | 頻道、連接器、市集、保險庫、外部 agent、整理 |
| 6 | `models-*` | 35 | M7·他區 13、M8 7、M10+ 15 | — | 設定 › 模型（M8）；供應商、ChatGPT 方案（M10+）；整合列歸模型區 |
| 6 | `settings-*`、`appearance-*` | 90 | M8 83、還不能測 7 | — | 設定頁、外觀（M8） |

**別區交給 M7k、但這份排到 M8 或之後的**（理由也寫在 E 段）：

| 編號 | 排到 | 理由 |
|---|---|---|
| `T-approval-answers-3`、`T-safety-always-*` | M8 | 「一律允許清單出現這一句」要設定 › 權限的畫面 |
| `T-report-delivery-1` | M10+ | 要寄信工具，也要桌面和任務頁 |
| `T-chat-stop-8`、`T-sync-referee-3`、`T-mailbox-reassign-1` | M8 | 預期寫到電腦 A 自己的畫面（桌面 App） |
| `T-computer-route-2`、`T-task-proof-3` | M8 | 要任務詳情、任務頁 |
| `T-autonomy-yolo-6`、`T-autonomy-remote-1`、`T-files-rewind-10` | M8 | 要在聊天裡打斜線指令（`chat-commands`） |
| `T-computers-n2-3` | M9 | 截圖要看畫面的工具 |
| `T-computers-n5-2` | M10+ | 只有某台連得到的模型 |

---

## B. 切片索引

| 片 | 標題 | 驗的編號 | 先要有 | 可以同時做 | 目錄 | 要的入口（API） | locales | 要重產 api-client |
|---|---|---|---|---|---|---|---|---|
| U1 | testenv：同源網頁、建測試資料、在頁面裡登入、每個 Bot 各用一份假模型腳本 | — | — | turn 區 T0–T5、trust 區、tools 區 | `tests/testenv`、`tests/harness`、`tests/e2e`、`Makefile` 的 e2e 目標 | 現有的 `/v1/account/*`、`/v1/admin/*`、`/v1/streams/append` | 不動 | 否 |
| U2 | testenv：執行端和假電腦接上 | — | U1；turn 區 T13、T14 | U3、U4 | `tests/testenv`、`tests/runner-test`（只加不改） | `/v1/computers/pairing-code`、`/v1/computers/pair`、`/v1/mailbox/claim` | 不動 | 否 |
| U3 | 網頁的基礎：API 呼叫、登入狀態、路由、紀錄同步的 hook | — | U1 | U2 | `web/api-client`、`web/ui`、`apps/web` | `/v1/streams/after`、`/v1/streams/notifications`、`/v1/server/status` | 不動 | 否（用現有的） |
| U4 | 登入和註冊畫面 | T-account-login-1、5、7、8 | U3；Q6、Q8 | U5 | `web/ui/src/login`、`tests/e2e` | `/v1/account/login`、`/v1/account/register`；**新**：註冊頁的管理者名稱（Q8） | 加 `login-*` | 是（Q8 的欄位） |
| U5 | Bot 清單、聊天、送出 | T-trust-pipeline-3 | U2、U3；turn 區 T12 | U4 | `web/ui/src/bots`、`web/ui/src/chat` | **新**：`POST /v1/bots/{bot}/messages`（T12） | 加 `chat-*`、`bots-*` | 是 |
| U6 | 逐字顯示和思考中 | T-chat-window-4、T-chat-steps-7 | U5；即時通道（Q2，M7j） | U7 | `web/ui/src/chat`、`tests/harness`（假模型照節奏送） | **新**：即時通道（Q2） | 加 `steps-thinking*` | 是 |
| U7 | 步驟區（長按顯示、狀態符號、在哪台、正在…） | T-phone-claude-1、T-chat-steps-6、T-chat-steps-8、T-task-journal-3 | U5；turn 區 T6（遮秘密） | U6 | `web/ui/src/steps` | 不用新的（讀紀錄） | 加 `steps-*` | 否 |
| U8 | 一般核准卡 | T-approval-answers-4、T-safety-reject-1、T-safety-card-2、T-chat-steps-1、T-autonomy-smart-9 | U7；trust 區 S2；turn 區 T5 | U10 | `web/ui/src/cards` | `/v1/streams/referee`（`card_answer`） | 加 `card-*`（畫面用的） | 視 S2 而定 |
| U9 | 停止 | T-chat-stop-1、2、3、T-safety-card-8、T-safety-gates-6 | U8；turn 區 T2；tools 區的「中斷指令」 | U10 | `web/ui/src/chat`（輸入框） | `/v1/streams/referee`（`stop`） | 加 `stop-*` | 否 |
| U10 | 交付報告卡和證據頁 | T-task-state-1、T-chat-steps-3、4、5 | U7；trust 區 S7、S8；turn 區 T9；Q3 | U9、U11 | `web/ui/src/report`、`web/ui/src/evidence` | 讀紀錄；**新**：任務狀態的讀取（Q3） | 加 `report-*`、`evidence-*` | 是（Q3） |
| U11 | 👍＝你已確認 | T-task-state-5、T-trust-verify-6；條件：T-report-delivery-9 | U10；👍 的入口（Q12）；Q4 | U12 | `web/ui/src/chat`、`web/ui/src/report` | **新**：👍 的寫入（Q12） | 加 `confirm-*` | 是 |
| U12 | 開關卡和「在你的電腦上執行」卡 | 條件：T-safety-exec-card-1、4、6、7、T-computers-local-1 | U8；trust 區 S3 | U11、U14 | `web/ui/src/cards` | 同 U8 | 加 `card-switch-*` | 視 S3 而定 |
| U13 | 開關卡按了之後 | 條件：T-safety-exec-card-2、3、T-trust-floor-2 | U12；trust 區 S4、S6 | U14 | `web/ui/src/cards`、`tests/e2e` | `/v1/settings`（讀） | 加 `card-direct-on` | 否 |
| U14 | 插話和停止的延伸 | 條件：T-chat-steer-1、3、T-chat-stop-6、T-chat-input-6、T-files-background-6 | U9；turn 區 T3、T12；tools 區 F11、F12 | U12、U13 | `web/ui/src/chat` | `POST /v1/bots/{bot}/messages` 的 `Steer` 結果 | 加 `composer-*` | 否 |
| U15 | 另一台裝置先動作 | 條件：T-phone-cards-2、T-sync-referee-5、T-approval-answers-5、T-safety-reject-2、T-safety-timeout-7 | U8、U9；**Q4** | U16 | `tests/e2e`（替身）、`web/ui/src/cards` | `/v1/streams/referee` 的錯誤代碼 | 用現有的 `card-*` | 否 |
| U16 | 問題卡（一）：作答流程 | T-cards-question-14、3、4、7 | U8；trust 區 S13 | U15 | `web/ui/src/question` | `/v1/streams/referee`（問題卡的回答，S13 定） | 加 `question-*` | 視 S13 而定 |
| U17 | 問題卡（二）：位置和標題 | T-cards-question-1、2 | U16 | U18 | `web/ui/src/question` | 同 U16 | 加 `question-title` | 否 |
| U18 | 「未驗證」標記和連結 | 條件：T-claims-check-1、2、T-claims-link-1、2、T-claims-wake-1 | U10；trust 區 S11、S12；tools 區 F10 | U19 | `web/ui/src/claims` | 讀紀錄 | 加 `claims-*` | 否 |
| U19 | 檔案卡和交付記號 | 條件：T-task-state-2、T-deliver-confirm-1、7 | U10；tools 區 F10；M7j 的交付確認 | U18 | `web/ui/src/files` | **新**：裝置回報「顯示了／點開了」（M7j） | 加 `file-card-*`、`deliver-*` | 是 |
| U20 | 改派、離線、找不到電腦的那幾行 | 條件：T-computers-default-4、T-computers-online-3、T-mailbox-reassign-3、T-goal-brake-11 | U10；turn 區 T10、T18；M7j 的在線狀態；Q4 | U21 | `web/ui/src/chat` | 讀紀錄；**新**：電腦在線狀態（M7j） | 用別區加好的代碼 | 視 M7j 而定 |
| U21 | 繼續卡 | 條件：T-task-resume-2、3、5、6、T-computer-route-6 | U8；turn 區 Q4、T7、T15 | U20 | `web/ui/src/cards` | 繼續卡的回答（turn 區 Q4） | 用 turn 區的 `turn-resume-*` | 視 Q4 而定 |
| U22 | 掃 M7 的每個畫面：44×44 和不出畫面 | T-phone-claude-2 | U4–U10 都在 `dev` | — | `tests/e2e` | — | 不動 | 否 |

**順序和並行**：

1. U1 現在就可以開工，不用等別區。
2. U1 合進 `dev` 之後，U2 和 U3 可以同時做。U2 要等 turn 區 T13、T14。
3. U4 和 U5 可以同時做。U5 要等 U2，也要等 turn 區 T12。
4. U6、U7 可以同時做。
5. U8 要等 U7。
6. U8 之後，U9、U10 可以同時做。
7. 條件片的開工時機各看它自己的依賴。
8. U22 最後做。

**熱檔**：大部分切片都會改到下面這些檔，後合的負責解衝突，衝突裡別人的改動要保留。

- `locales/en.ftl`、`locales/zh-TW.ftl`，和 `make locale-refresh` 產生的 `web/ui/src/generated/*`。每片只加自己前綴的代碼，不改別片的。
- `tools/spec-trace/baseline.txt`。
- `web/ui/src/chat/ChatView.tsx`：每片只在它裡面加一行掛自己的元件。
- `tests/testenv/src/control.rs`。

---

## C. 切片藍圖

### C0. 每一片共用的規定

每份藍圖都隱含下面幾條，派工時抄進任務檔：

- **開工條件**：
  - 先 `git merge dev`；
  - 讀 `AGENTS.md` 和 `spec/10-behavior/README.md` 的用語；
  - 再讀藍圖寫的規格段。
- **畫面的程式**（`web/ui`）：
  - 一個元件一個檔；檔案不超過 300 行，函式不超過 60 行；不寫註解。
  - 元件和程式碼裡不准有中文。使用者看得到的字都放在 `locales/*.ftl`，畫面用 `formatMessage(code, args)` 查。
  - 每加一個代碼，`en.ftl`、`zh-TW.ftl` 兩邊都加：zh-TW 照規格的原字，en 寫意思一樣的英文。加完跑 `make locale-refresh`。
- **畫面不自己判斷規則**：
  - 卡片有哪些按鈕、步驟是什麼狀態、任務是什麼狀態，都照紀錄裡寫的畫。
  - 紀錄缺欄位時，停下來回報要哪個欄位，不在前端用字串去猜。
- **網頁跟伺服器同一個來源**（U1 之後）：
  - 每個請求都帶 `AnyBot-Client-Version`；帶 cookie 的寫入另外帶 `AnyBot-CSRF: 1`（K118 ④⑤）。
  - 伺服器沒有開 CORS，也不准為了測試開。
- **Vitest**：
  - 放在元件旁邊的 `*.test.tsx`，測純函式和元件的狀態。
  - 標題帶編號的，只准用在種類是「單元」的列（例如 `T-cards-question-14`）。
- **Playwright**：
  - 測試放在 `tests/e2e/<片名>.spec.ts`，所有手機情境的 project 都跑。
  - 測試標題開頭寫編號，例如 `T-chat-stop-3 ...`。
  - 比對的是訊息代碼和參數，用 `textFor`，U3 會讓它能帶參數。
  - 測試資料裡的中日韓文字用 `\u` 跳脫寫法。
  - 每個等待都要有期限。
- **一個測試一個帳號**：用 U1 的 `seedWorld()` 建。要動共用時鐘的、要用固定帳號名（`bob`）的測試，先拿 U1 的控制鎖。
- **不准做的（每片都一樣）**：
  - 不准為了測試在正式程式加旗標、環境變數、輸出模式。
  - 不准在伺服器開 CORS 或 `SO_REUSEPORT`。
  - 不准改 `spec/`。
  - 不准用 `#[allow`、`biome-ignore`、`@ts-ignore`。
  - 不准在前端寫死中文。
  - 不准把別片的編號放進測試名稱。
- **驗證和回報**：
  - 本機：跑 `make check-fast`，加上這一片的 Vitest 和 Playwright。用 `make e2e` 跑，或用 `pnpm --filter=@anybot/e2e exec playwright test <檔>`。
  - 雲端檢查：`python3 ~/.claude/skills/acp-agents/bin/cloud_check.py`，在前景跑，期限 50 分鐘，全綠才回報。
  - 回報：驗收清單逐條寫「做到了，在哪個 commit、哪個檔、哪一行」。跟藍圖不一樣的地方寫出來，附上理由。

---

### U1：testenv 同源網頁、建測試資料、在頁面裡登入、每個 Bot 各用一份假模型腳本

**1. 目標**：讓 Playwright 打開的網頁和伺服器在同一個來源（`http://127.0.0.1:<server_port>`）。每個測試能建自己的帳號和 Bot，各用自己的假模型腳本，互不干擾。

**2. 開工條件**：`dev` 最新。不用等別區。

**3. 只准改的範圍**：
- `tests/testenv/src/*`，和 `tests/testenv/Cargo.toml`（加 `tower-http` 的 `fs`）。
- `tests/harness/src/fake_model.rs`、`model_script.rs`。
- `tests/e2e/src/*`、`tests/e2e/playwright.config.ts`、`tests/e2e/smoke.spec.ts`（只改網址）。
- `Makefile` 的 `_e2e-body`。

**4. 定好的介面**：
- testenv 的伺服器在同一個埠上做兩件事：
  - 照常跑 `anybot_server::build_router(state)`；
  - `.fallback_service(ServeDir::new("apps/web/dist"))`，給網頁的檔案。
- `vite preview` 和 `webServer` 拿掉。`_e2e-body` 先跑 `pnpm --filter=@anybot/web build`，再 `cargo build -p testenv`。
- Playwright 的網址從 `testenv-ports.json` 讀。加 `tests/e2e/src/world.ts`：
  - `appUrl(path)`：給網頁的完整網址。
  - `seedWorld(request, opts)`：回 `{ account, password, bots: {name, id, model}[] }`。
  - `loginInPage(page, account, password)`：在頁面裡用 `fetch` 打 `/v1/account/login`，`credential: "cookie"`，帶兩個標頭。這樣瀏覽器自己會帶 `Origin`。
  - `withLock(request, name, fn)`：拿控制鎖再做事。
- 新的控制 API（都在 `/control/` 底下，只有 testenv 有）：
  - `POST /control/seed`：
    - 送 `{bots: string[], admin?: boolean}`。
    - 回 `{account, password, bots: [{name, id, model}]}`。
    - 帳號由 testenv 照真的 API 建：管理員先發邀請碼，再用 Bearer 註冊，請求不帶 `Origin`。
    - Bot 用帳號紀錄流的 `bots_create` 建。
    - 每個 Bot 的 `model` 是 `fake-<uuid>`。
  - `POST /control/model/script`：多一個 `model` 欄位，每個模型名稱各有一份腳本佇列。沒帶 `model` 的照舊用共用的那一份。
  - `GET /control/model/requests?model=`。
  - `POST /control/lock`：送 `{name}`，拿到鎖才回應，最多等 60 秒；`DELETE /control/lock/{name}` 放掉。
  - `POST /control/account/disable`：送 `{account}`，testenv 用管理員打真的 `/v1/admin/accounts/{account}/disable`。
- 假模型照請求裡的 `model` 欄位選腳本。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 網頁從伺服器的埠打開 | 不是 GET 的請求，`Origin` 跟伺服器自己的位址一樣，不會被擋 | K118 ④ |
| 2 | 兩個測試同時跑，各用自己的 Bot | 各自的假模型腳本不互相吃掉 | AGENTS.md「測試不穩就修測試」 |
| 3 | 註冊次數上限 | 建測試資料走管理員發邀請碼、Bearer 註冊；如果撞到 `account-register-rate`，停下來回報 | `account-login`、K118 ③ |

**6. 要寫的測試**（都是描述性的名字）

| 測試 | 驗什麼 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `testenv serves the web app on the server origin`（e2e） | 打開 `appUrl("/")` | 外殼出現；`location.origin` 等於 `http://127.0.0.1:${server_port}` | 端對端 |
| `seeded worlds keep separate model scripts`（e2e） | 建兩份資料，各排一步回答 | 兩個 Bot 的模型各收到 1 個請求 | 整合 |
| `control lock serializes holders`（Rust，testenv） | 兩個人搶同一個名字的鎖 | 第二個要等第一個放掉才拿到 | 單元 |
| `smoke T-phonefit-width-1 still passes on the server origin` | 原本的測試只改網址 | 照舊全綠 | 端對端 |

**7. 不准做的**：
- 不准在 `crates/server` 加「給網頁檔案」的程式。正式版網頁誰給，等 Q1。
- 不准用 Docker。
- 不准把 `/control` 掛在伺服器的埠上。
- 不准讓伺服器讀環境變數。

**8. 驗收清單**：
1. `vite preview` 拿掉，Playwright 的網址指到 testenv 的伺服器。
2. `phone-max-font`、`phone-wide-font`、`desktop-large`、`rtl-arabic`、`dark-mode`、`lang-en`、`lang-zh-TW` 的 smoke 全綠。
3. 五個新的控制 API 都有 Rust 測試。
4. 每個模型名稱各有一份假模型腳本，有測試。
5. `world.ts` 的四個函式有型別；每個等待都有期限。
6. `make e2e` 從頭跑得起來：先建網頁，再建 testenv。

---

### U2：testenv 執行端和假電腦接上

**1. 目標**：testenv 裡有真的執行端在領信、跑一輪、寫紀錄。電腦是 `tests/runner-test` 的假電腦。U5 以後的端對端測試，按一下送出，就會走完整條路。

**2. 開工條件**：U1 已合進 `dev`。turn 區 T13（執行服務領信跑一輪）、T14（`apps/agent`）已合進 `dev`，而且它們的程式能當函式庫用、電腦和模型的網址能從外面傳進去（見 E 段第 1 點）。

**3. 只准改的範圍**：`tests/testenv/src/*`、`tests/testenv/Cargo.toml`（加 `runner-test`、`anybot-runner` 當依賴）、`tests/runner-test/src/*`（只加，不改已有的行為）。

**4. 定好的介面**：
- `POST /control/seed` 多一個欄位 `computers`：
  - 格式是 `[{name, owner: "user"|"bot", sandbox: bool, run_here: "ask"|"allow"}]`。
  - 每台由 testenv 照真的 API 配對（`pairing-code` → `pair`），拿到 Bearer。
  - 每台起一個 T13／T14 的執行端，執行端用的電腦是一台 `FakeComputer`。
- 控制 API：
  - `GET /control/computer/file?computer=&path=`：回 `{content}`，找不到回 404。
  - `POST /control/computer/script`：送 `{computer, command, output, exit_code, duration_ms}`。
  - `POST /control/computer/unreachable`：送 `{computer, on}`。
- `duration_ms` 在 testenv 裡是真的等那麼久，畫面才來得及在指令跑的時候操作。`CommandScript.with_duration` 如果只動假時鐘，就加一個真的等的選項。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 送一句話給一個 Bot | 信箱給執行端一輪；執行端寫一輪開始、模型的回答、一輪結束 | K119 ⑬、`turn-loop` S1 |
| 2 | 假電腦宣告沒有指令沙盒 | 照 `computer-caps` 宣告 | `computer-caps` |
| 3 | 指令 `duration_ms=10000` | 真的過 10 秒才有結果 | `chat-stop` 的前提 |

**6. 要寫的測試**

| 測試 | 驗什麼 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `seeded computer runs a scripted turn`（e2e，直接打 API） | 排一步 `edit_file(hello.txt, "hi")`，再回 `"ok"` | `/control/computer/file` 回 `"hi"`；紀錄流有 `turn_ended` | 整合 |
| `user machine and bot machine both claim letters` | 兩台各收一輪 | 各自的 `logged()` 有一筆 | 整合 |

**7. 不准做的**：不准在 testenv 自己寫回合的迴圈，一律用 T13、T14 的程式。不准改 `crates/runner` 的行為來配合測試。

**8. 驗收清單**：
1. `seed` 能建使用者的電腦和 Bot 的電腦，兩種都有。
2. 三個新的控制 API 有 Rust 測試。
3. 有一個端對端測試，從送一句話到檔案寫進假電腦，不靠內部函式。
4. 執行端停掉時，用 PID 收掉，不按名字殺程式。

---

### U3：網頁的基礎（API 呼叫、登入狀態、路由、紀錄同步的 hook）

**1. 目標**：之後每片都用得到的地基。內容是：一個帶好標頭的 API 呼叫、登入狀態、三個路由（登入、Bot 清單、聊天）、從紀錄流讀資料的 hook。

**2. 開工條件**：U1 已合進 `dev`。

**3. 只准改的範圍**：`web/api-client/src/*`、`web/ui/src/{app,session,sync,routes}/*`、`apps/web/src/*`、`tests/e2e/src/ftl.ts`（讓 `textFor` 能帶參數）。

**4. 定好的介面**：
- `web/api-client`：
  - `createAnyBotClient({ clientVersion })`：用 `openapi-fetch`，`baseUrl` 是 `location.origin`。
  - 每個請求帶 `AnyBot-Client-Version`；不是 GET 的請求加 `AnyBot-CSRF: 1`。
  - 錯誤一律換成 `{ code, params }`，從 `ProblemJson` 拿。
- `web/ui`：
  - `SessionProvider` 和 `useSession()`：狀態只有 `unknown`、`signedOut`、`signedIn` 三種。
  - `AppRoutes`：`/login`、`/`（Bot 清單）、`/bots/:botId`（聊天）。用 hash 路由還是 history 路由，看伺服器的 fallback 回不回 `index.html`。
  - `useRecords(stream)`：
    - 先用 `/v1/streams/after` 從 0 要起。
    - 再開 `/v1/streams/notifications`，收到「有新的」就用 `after=max_seq` 去要。
    - 回 `{ records, maxSeq }`；同一個 `seq` 只留一筆。
  - `useAccountBots()`：從帳號紀錄流的 `bots_create`、`bots_delete`、`bots_restore` 算出 Bot 清單。
- `textFor(code, lang, args?)`：用跟網頁一樣的方式把 `{ $x }` 換成值。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 每個請求 | 帶 `AnyBot-Client-Version`；用 cookie 的寫入帶 `AnyBot-CSRF: 1` | K118 ⑤、④ |
| 2 | SSE 收到「有新的」 | 用自己記的最後一號去要；提示漏了、重複了，結果一樣 | `conn-sse` S2、S5 |
| 3 | 同一筆紀錄來兩次 | 畫面上只出現一次 | `chat-window` 永遠成立 |
| 4 | 還沒登入就打開 `/` | 跳到 `/login` | `account-login` S1 |

**6. 要寫的測試**

| 測試 | 驗什麼 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `mergeRecords keeps one copy per seq`（Vitest） | 把 `[1,2,3]` 和 `[2,3,4]` 合起來 | 結果是 `seq` `[1,2,3,4]` | 單元 |
| `client adds version and csrf headers`（Vitest，假 fetch） | 一個 POST 請求 | 兩個標頭都在；GET 請求沒有 `AnyBot-CSRF` | 單元 |
| `bots list follows create and delete records`（Vitest） | 建 A、建 B、刪 A | 清單只剩 B | 單元 |
| `signed out visitor lands on login`（e2e） | 打開 `appUrl("/")` | 畫面出現 `login-submit` 代碼的字 | 端對端 |

**7. 不准做的**：
- 不准把 cookie 或金鑰存進 `localStorage`：cookie 是 HttpOnly 的。
- 這一片還不做本機副本（M8 的 `account-n4`）。
- 不准做畫面的樣式細節。

**8. 驗收清單**：
1. 每個請求都經過 `createAnyBotClient`，有測試。
2. `useRecords` 在提示漏掉、重複時結果一樣，有測試。
3. `textFor` 能帶參數，原本的呼叫方式照樣能用。
4. Biome、`tsc --noEmit`、Vitest 全綠。

---

### U4：登入和註冊畫面

**1. 目標**：手機上可以登入、註冊；錯誤照規格的字顯示。

**2. 開工條件**：U3 已合進 `dev`；Q6（網址欄）、Q8（管理者名稱從哪裡拿）已經答了。

**3. 只准改的範圍**：`web/ui/src/login/*`、`locales/*`、`tests/e2e/login.spec.ts`；Q8 要的伺服器欄位由監督者另外派。

**4. 定好的介面**：
- `LoginPage` 的欄位：網址（照 Q6）、帳號、密碼、「建立新帳號」勾選。勾了之後多兩樣：邀請碼欄位；不是第一個帳號時，再多隱私說明和「我知道了」勾選。
- `useLogin()`：
  - 先打 `/v1/server/status`，確認網址是 AnyBot 伺服器。
  - 再送 `register` 或 `login`（`credential: "cookie"`）。
  - 勾了建立新帳號、伺服器回 `account-exists`，就改成登入；登入再失敗，顯示 `account-exists-reset`。
- 帳號欄位在送出前轉成小寫。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 填網址、帳號 `Bob`、密碼、邀請碼，勾建立新帳號、勾我知道了，按登入 | 建立帳號 `bob`，並且登入 | `account-login` S1、S3 |
| 2 | 勾建立新帳號，帳號 `alice`，密碼錯 | 顯示 `account-exists-reset` | S5、K15 |
| 3 | alice 被停用，用對的密碼登入 | 顯示 `account-login-disabled` | S4、K116 ③ |
| 4 | 已經有 alice，打開註冊 | 顯示「這台伺服器的管理者（alice）看得到你的對話。」 | S7、`server-privacy` S1 |
| 5 | 密碼錯 | 顯示 `account-login-credentials` | S4 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-account-login-1 registering Bob creates bob and signs in` | 行為 1 | 進到 Bot 清單；`/control` 查得到帳號 `bob` | 端對端（拿 `fixed-account-bob` 鎖；測試前用控制 API 刪掉 `bob`，見 Q9） |
| `T-account-login-5 existing account with wrong password` | 行為 2 | `textFor("account-exists-reset", lang)` | 端對端 |
| `T-account-login-7 disabled account is told so` | 行為 3 | `textFor("account-login-disabled", lang)` | 端對端 |
| `T-account-login-8 register page names the admin` | 行為 4 | `textFor("login-privacy-note", lang, { admin: "alice" })` | 端對端 |
| `login lowercases the account name`（Vitest） | S3 | 送出的帳號是 `"bob"` | 單元 |

**7. 不准做的**：
- 這一片不做兩步驟驗證（M8 的 `account-n3`）。
- S2 的 15 秒逾時和「舊版的帳號伺服器」可以先做，但編號不屬於這一片。
- 不准在前端比對錯誤的字串，一律照代碼。

**8. 驗收清單**：
1. 四個帶編號的端對端測試，在所有手機情境都綠。
2. 新代碼 `login-*` 在 en、zh-TW 兩邊都有，`make locale` 過。
3. Q8 的欄位進了 `openapi.yaml`，由監督者重產 api-client。
4. 四列從基準檔拿掉。

---

### U5：Bot 清單、聊天、送出

**1. 目標**：
- 看得到自己的 Bot；
- 點進去看得到對話；
- 在輸入框打字、送出，看得到 Bot 的回答。

**2. 開工條件**：U2、U3 已合進 `dev`。turn 區 T12（`POST /v1/bots/{bot}/messages`）已合進 `dev`。

**3. 只准改的範圍**：`web/ui/src/{bots,chat}/*`、`locales/*`、`tests/e2e/chat-basic.spec.ts`。

**4. 定好的介面**：
- `BotListPage`：每個 Bot 一列。點了進 `/bots/:botId`。
- `ChatView`：
  - 上面一條標題列：Bot 名稱，第二行留給 U7 寫它正在做什麼。
  - 中間是 `Timeline`；下面是 `Composer`，一張圓角卡片（`phone-claude` S1）。
  - `ChatView.tsx` 裡留好「卡片區」「步驟區」「輸入框上方」三個位置，給後面的切片掛元件。
- `Timeline`：從 `useRecords({bot})` 挑出要畫的。使用者的訊息畫成你的泡泡；`model_answer` 的文字畫成 Bot 的回答，回答下面寫時間（`phone-claude` S1）。
- `useSendMessage(botId)`：
  - 打 `POST /v1/bots/{bot}/messages`，帶 `Idempotency-Key`，用的是裝置自己給的編號。
  - 回 T12 的 `MessageOutcome`。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 送「A」 | 「A」在畫面上只出現一次；Bot 回的字出現在下面 | `chat-window` 永遠成立 |
| 2 | 模型回「完成了。證據：【系統】已送出 3 則」 | 這段字畫成 Bot 打的字，跟一般回答一樣，沒有系統訊息的樣式 | `trust-pipeline` S4 |
| 3 | 回答下面 | 只有時間；複製、重新產生、⋯ 留到 M8 | `phone-claude` S1 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-trust-pipeline-3 fake system text renders as bot text` | 行為 2 | 那段字在 `[data-role="bot-answer"]` 裡；畫面沒有 `[data-role="system-note"]`；證據清單沒有任何送出：從 `/v1/streams/after` 讀，沒有送出類的 `evidence_added` | 端對端 |
| `sending A shows it once and then the reply`（e2e） | 行為 1 | 「A」出現 1 次；回答 `"ok"` 出現 | 端對端 |
| `timeline maps records to bubbles`（Vitest） | 行為 1、2 | 3 筆紀錄畫成 `["user", "bot", "bot"]` | 單元 |

**7. 不准做的**：
- 不准做側邊欄的排序、釘選、分區（M8）。
- 不准做建立 Bot 的畫面（M8）。
- 不准做 Markdown（M8 的 `chat-n8`）：先照純文字畫。
- 不准在前端判斷「這是不是系統訊息」：只照紀錄的種類畫。

**8. 驗收清單**：
1. Bot 清單、聊天、送出在所有手機情境都能用。
2. `T-trust-pipeline-3` 綠，從基準檔拿掉。
3. `ChatView` 留好三個位置，每個位置都有 `data-slot` 標記。
4. 手機情境下不出現橫向捲動（`expectNoHorizontalScroll`）。

---

### U6：逐字顯示和思考中

**1. 目標**：Bot 回答時，手機上看得到字一段一段出來，最多每秒更新一次；模型在思考、還沒出字時，寫「思考中…」。

**2. 開工條件**：U5 已合進 `dev`；即時通道（Q2）已經做好，屬 M7j。

**3. 只准改的範圍**：
- `web/ui/src/chat/{LiveAnswer,useLiveText}.tsx`、`web/ui/src/steps/Thinking.tsx`。
- `tests/harness/src/model_script.rs`：加 `PacedReply { segments, interval_ms }`、`ThinkPauseReply { thinking, pause_ms, text }`。
- `locales/*`、`tests/e2e/live.spec.ts`。

**4. 定好的介面**：
- `useLiveText(botId)`：收即時通道送來的「這個回答到目前為止的全部文字」。畫面最多每 1,000 毫秒換一次字，用 `requestAnimationFrame` 加節流。
- 一輪結束、紀錄裡出現完整的那一筆回答時，直接換成完整的字。
- `Thinking`：即時通道送的是思考、還沒有字時，顯示 `steps-thinking`（「思考中…」）。之後「思考過程」收在步驟區，可以展開（`chat-steps` S5）。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 假模型 2 秒內分 50 段送出 | 那 2 秒內畫面更新不超過 3 次；最後的字完整 | `chat-window` S4、`conn-batch` S2 |
| 2 | 假模型先送思考，2 秒後才出字 | 出字前出現「思考中…」；之後「思考過程」可以展開 | `chat-steps` S5 |
| 3 | 打字中途才打開聊天 | 看得到目前已經打出來的字 | `conn-batch` S3 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-chat-window-4 phone updates at most once per second while streaming` | 行為 1 | 50 段，每段是一個字：`"\u4e00"`、`"\u4e8c"`…；用 `MutationObserver` 數回答那一格的變化，2 秒內 ≤ 3 次；最後的字等於 50 段接起來 | 端對端 |
| `T-chat-steps-7 thinking shows before text and folds into steps` | 行為 2 | 出字前 `textFor("steps-thinking")` 看得到；出字後按 `steps-show`，`steps-thinking-process` 可以展開，內容是思考的字 | 端對端 |
| `useLiveText throttles to one paint per second`（Vitest，假時間） | 行為 1 | 1 秒內 20 次輸入，只畫 1 次 | 單元 |

**7. 不准做的**：
- 正在打的字不准寫進本機副本或紀錄（K118 ⑭）。
- 不准用 WebSocket（`conn-ws` 只給看畫面）。
- 不准為了測試把節流關掉。

**8. 驗收清單**：
1. 兩列帶編號的測試綠，從基準檔拿掉。
2. 假模型多了兩種步驟，在 Rust 那邊有測試。
3. 怎麼數「畫面更新」寫進測試檔的函式名（Q17）。

---

### U7：步驟區（長按顯示、狀態符號、在哪台、正在…）

**1. 目標**：照 `phone-claude` S2、`chat-steps` S1、S4、S6 做出步驟區。

**2. 開工條件**：U5 已合進 `dev`；Q5（長按多久）已經答了；`T-task-journal-3` 要等 turn 區 T6 的遮秘密合進 `dev`。

**3. 只准改的範圍**：`web/ui/src/steps/*`、`locales/*`、`tests/e2e/steps.spec.ts`。

**4. 定好的介面**：
- `useTurnSteps(records, taskId)`：純函式，回 `StepRow[]`。每列有 `{ actionId, status, title, computer?, resultHead?, evidenceIds }`。
  - `status` 照事件決定：`ToolIntent`、`GateResult`、`CardShown`、`ToolStarted`、`ToolCompleted`、`ToolFailed`、`ToolNotRun`、`CardAnswered`。
  - 不照模型說的話決定（`chat-steps` 永遠成立）。
- `StepsPanel`：
  - 預設收起。
  - 長按回答（Q5）或按 ⋯ ›「顯示步驟（N）」才打開，代碼 `steps-show`，參數 `{count}`。
  - 一列一個工具。符號：`●` 等你允許、`…` 執行中、`✓` 完成、`–` 被拒絕或已停止、`×` 失敗。
  - 動作說明最多 90 字，多的截掉加「…」。
- `StepDetail`：展開後寫「在：{電腦名稱}」，代碼 `steps-on-computer`；那一行本身不寫在哪台跑。
- `WorkingLine`：工作中，輸入框上方寫它正在做什麼，例如 `steps-working-command`（「正在跑指令…」）。最多每秒重畫 20 次。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 一輪做了 3 步 | 聊天裡沒有步驟；長按回答後出現「顯示步驟（3）」 | `phone-claude` S1、S2 |
| 2 | 這一輪在電腦 A 跑 | 步驟那一行沒有電腦名稱；展開後寫「在：A」 | `chat-steps` S4 |
| 3 | 假模型呼叫要跑 10 秒的指令 | 輸入框上方寫「正在跑指令…」 | `chat-steps` S6 |
| 4 | 指令帶著假金鑰 | 手機畫面和伺服器上的步驟都沒有那把金鑰 | `task-journal` S3 |
| 5 | 動作說明超過 90 字 | 截在 90 字，加「…」 | `chat-steps` S1 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-phone-claude-1 steps stay hidden until long press` | 行為 1 | 長按前畫面沒有 `[data-role="step-row"]`；長按後 `textFor("steps-show", lang, {count: 3})` 看得到 | 端對端 |
| `T-chat-steps-6 computer name only in the expanded step` | 行為 2 | 那一行不含 `"A"`；展開後 `textFor("steps-on-computer", lang, {computer: "A"})` | 端對端（假電腦的名字是 `A`） |
| `T-chat-steps-8 working line names the running command` | 行為 3 | 指令跑的時候 `textFor("steps-working-command")` 看得到 | 端對端 |
| `T-task-journal-3 fake key never reaches the phone or the server steps` | 行為 4 | 假金鑰在測試跑的時候才組出來：`crypto.randomUUID()`；`page.content()` 和 `/v1/streams/after` 的內容都不包含它 | 端對端 |
| `step status follows recorded events`（Vitest） | 行為 5、`chat-steps` S1 | 事件組合對應到 `["●", "…", "✓", "–", "×"]`；91 字的說明變成 90 字加 `"…"` | 單元 |

**7. 不准做的**：
- 這一片不做證據頁（U10），也不做步驟結果的 6,000 字（U10）。
- 不做團隊成員的看法（M8 的 `chat-steps` S8）。
- 「顯示步驟」不准放在聊天裡讓它一直看得到。

**8. 驗收清單**：
1. 四列帶編號的測試綠，從基準檔拿掉。
2. `useTurnSteps` 是純函式，有 Vitest。
3. 桌面瀏覽器按右鍵也打得開步驟區（`phone-claude` S2），寫在描述性的測試裡。

---

### U8：一般核准卡

**1. 目標**：Bot 要你同意時，卡片出現在那個 Bot 的輸入框上方；四個按鈕照紀錄畫；按了，答案送到伺服器裁判。

**2. 開工條件**：
- U7 已合進 `dev`。
- trust 區 S2 已合進 `dev`：`CardShown` 帶 `kind`、`buttons`、`reason`、`always_label`、`notes`。
- turn 區 T5 已合進 `dev`。
- U2 的 testenv 要能把電腦設成「在這台電腦上執行＝每次詢問」。

**3. 只准改的範圍**：`web/ui/src/cards/*`、`web/ui/src/bots/BotListRow.tsx`（加「需要你允許」）、`locales/*`、`tests/e2e/cards.spec.ts`。

**4. 定好的介面**：
- `usePendingCard(records)`：找出這一輪最後一張還在等的卡，回 `PendingCard | null`。
- `ApprovalCard`：
  - 淡黃底、細黃框。
  - 標題「允許 {Bot} {動作}嗎？」。
  - 下面一個灰底細節框：指令、路徑，在使用者的電腦上做時加「在：{電腦}」。
  - 按鈕照 `buttons` 的順序畫；「一律允許」按鈕上的字就是 `always_label`。
  - 沒有一律允許的卡，多一行灰字 `card-asks-every-time`（「這類操作每次都會問你」）。
- `useAnswerCard()`：
  - 打 `POST /v1/streams/referee`，送 `{action: "card_answer", card_id, task_id, answer, stream: {bot: {id}}}`。
  - 錯誤代碼 `card-already-answered`、`card-not-waiting`、`card-answer-stale` 照代碼顯示。
- Bot 清單：別的 Bot 有卡在等時，那一列寫 `bots-needs-approval`（「需要你允許」）。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 按拒絕 | 模型下一個請求含「使用者拒絕了這個操作。請不要再試同樣的操作」 | `approval-answers` S4 |
| 2 | 按拒絕 `make deploy`，模型再呼叫一次 | 第二次不出卡片；模型的請求含「這個操作他剛才已經拒絕過了」 | `safety-reject` S1 |
| 3 | 在看 Bot2 的聊天，Bot1 出卡 | Bot2 的畫面沒有卡；Bot 清單上 Bot1 寫「需要你允許」 | `safety-card` S3 |
| 4 | 三步：結束碼 0、結束碼 1、刪除檔案（按拒絕） | 三行依序 `✓`、`×`、`–`；等卡時那一行是 `●`，指令跑的時候是 `…` | `chat-steps` S1 |
| 5 | 一般卡 | 按鈕是允許這次、這段對話都允許、一律允許、拒絕；沒有「這輪都准」 | `autonomy-smart` S7、`approval-answers` S6 |
| 6 | 按允許這次，再呼叫同一個指令 | 第二次又出卡（描述性的測試，編號已經不在這一區，見 Q11） | `approval-answers` S1 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-approval-answers-4 deny reaches the model` | 行為 1 | `/control/model/requests` 最後一個請求，含 `textFor` 查出的那句（代碼照 trust 區 S5：`gate-user-denied`） | 端對端 |
| `T-safety-reject-1 denied make deploy is not asked again` | 行為 2 | 第二次卡片數＝0；請求含 `gate-denied-again` 的字 | 端對端 |
| `T-safety-card-2 other bot shows no card and list flags Bot1` | 行為 3 | Bot2 畫面沒有 `[data-role="approval-card"]`；Bot1 那一列有 `textFor("bots-needs-approval")` | 端對端 |
| `T-chat-steps-1 step marks follow the run` | 行為 4 | 依序看到 `●`、`…`，最後 `["✓", "×", "–"]` | 端對端 |
| `T-autonomy-smart-9 general card has exactly four buttons` | 行為 5 | 按鈕的字依序對到 `card-allow-once`、`card-allow-conversation`、`always_label`、`card-deny`；沒有 `card-allow-turn` | 端對端 |
| `allow once asks again next time`（e2e，描述性） | 行為 6 | 第二次又看到卡片 | 端對端 |

**7. 不准做的**：
- 不准在前端決定有哪些按鈕：只照 `buttons`。
- 不做鍵盤 Enter、Esc（M8 的 `safety-card` S5）。
- 不做推播和聲音（M8）。
- 不准用整個視窗變暗的彈出視窗（`safety-card` S1）。

**8. 驗收清單**：
1. 五列帶編號的測試綠，從基準檔拿掉。
2. 卡片在 `phone-max-font`、`phone-wide-font` 下沒有橫向捲動，每個按鈕都 ≥44×44。
3. 三個錯誤代碼都有 Vitest。
4. 卡片用的字都從紀錄或 `locales` 來，前端沒有寫死的字。

---

### U9：停止

**1. 目標**：照 `chat-stop` S1–S3 做送出鈕的兩種樣子，和停止之後的畫面。

**2. 開工條件**：U8 已合進 `dev`；turn 區 T2（停止）、tools 區的「中斷正在跑的指令」已合進 `dev`。

**3. 只准改的範圍**：`web/ui/src/chat/{Composer,SendButton,useStopTurn}.tsx`、`locales/*`、`tests/e2e/stop.spec.ts`。

**4. 定好的介面**：
- `SendButton`：
  - Bot 工作中、輸入框空著時是「■」，提示 `stop-hint`（「停止」）。
  - 框裡有字時是「↑」，提示 `stop-steer-hint`（「補充給正在進行的工作」）。
- `useStopTurn(botId, taskId)`：打 `POST /v1/streams/referee`，送 `{action: "stop", ...}`。
- 回答還沒有任何字就停了：回答寫 `turn-reply-stopped`（「（已停止）」），代碼由 turn 區 T2 加。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 工作中，輸入框空著 → 打「x」不送 | 先是「停止」；打字後變「補充給正在進行的工作」 | `chat-stop` S1 |
| 2 | 第一步 10 秒指令、第二步寫檔；第 3 秒按停止 | 指令被中斷；寫檔沒做：檔案不存在、證據清單沒有寫檔；第一步是「–」；畫面沒有錯誤 | S2 |
| 3 | 假模型不回應；按停止 | 回答寫「（已停止）」 | S2 |
| 4 | 卡片在等時按停止 | 這一步標「被拒」，這一輪停下（Q7） | `safety-card` S7、`chat-stop` S3 |
| 5 | 卡片在等時按停止 | 0.2 秒內這一步變被拒 | `safety-gates` S2 第 10 關、S3 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-chat-stop-1 button flips between stop and steer` | 行為 1 | `aria-label` 先等於 `textFor("stop-hint")`，打字後等於 `textFor("stop-steer-hint")` | 端對端 |
| `T-chat-stop-2 stop interrupts the command and skips the write` | 行為 2 | `/control/computer/file?path=b.txt` 回 404；`evidence_added` 沒有寫檔；第一列是 `"–"`；畫面沒有 `[role="alert"]` | 整合（從瀏覽器操作） |
| `T-chat-stop-3 silent model shows stopped reply` | 行為 3 | `textFor("turn-reply-stopped")` | 端對端 |
| `T-safety-card-8 stop while a card waits marks it denied` | 行為 4 | 那一列是 `"–"`，旁邊有 `steps-denied` 的字；紀錄有 `card_answered{deny}` 和 `turn_ended` | 端對端 |
| `T-safety-gates-6 stop flips the waiting step within 200 ms` | 行為 5 | 從按下到那一列變被拒 ≤ 200 毫秒，用 `performance.now()` 量 | 端對端 |

**7. 不准做的**：
- 按停止不准取消任務（`chat-stop` S6）：這一片不做「取消任務」的按鈕。
- 不做插話（U14）。
- 停止的請求不准進本機匣子（`account-n6`）。

**8. 驗收清單**：
1. 五列帶編號的測試綠，從基準檔拿掉。
2. 0.2 秒的測試在所有手機情境都穩，連跑 20 次不紅，次數寫進回報。
3. 「被拒」怎麼畫照 Q7 的答案。

---

### U10：交付報告卡和證據頁

**1. 目標**：一輪做完後，聊天裡出現交付報告卡，第一屏是結論。步驟可以點進證據頁。任務狀態看得到。

**2. 開工條件**：
- U7 已合進 `dev`。
- trust 區 S7（證據欄位）、S8（報告逐句對證據）已合進 `dev`。
- turn 區 T9 已合進 `dev`。
- Q3（任務狀態怎麼讀）已經答了。

**3. 只准改的範圍**：`web/ui/src/{report,evidence}/*`、`web/ui/src/steps/StepDetail.tsx`、`locales/*`、`tests/e2e/report.spec.ts`。

**4. 定好的介面**：
- `ReportCard`：讀 `DeliveryReportRecord`。
  - 第一屏：目標、結果（主狀態和細項，`report-status`，參數 `{status, detail}`）、摘要、四級各幾句。
  - 下面三個收起的區塊：「步驟」「證據」「驗收」（`report-delivery` S2）。
- `StepResult`：
  - 收起時標題寫「結果：」加第一行，最多 60 字。
  - 展開最多 6,000 字；超過的寫 `steps-full-in-evidence`（「完整內容在證據裡」），並連到 `EvidenceSheet`。
- `EvidenceSheet`：從底部拉上來的唯讀頁（Q14），列出證據的種類、檔案和前後雜湊、指令和結束碼、判斷來源。
- `useTaskStatus(taskId)`：照 Q3 讀。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 送「建檔」；`edit_file(hello.txt, "hi")`；回「好了」 | 任務狀態是完成（已驗收）；中間沒有卡片，也沒有要你按的東西；`hello.txt` 內容是 `hi` | `task-state` S1 |
| 2 | 指令輸出 10,000 字，展開那一步 | 顯示前 6,000 字和「完整內容在證據裡」；點了打開那一步的證據 | `chat-steps` S2 |
| 3 | 寫檔 `(a.txt, "x")`，展開步驟 | 證據清單有 `a.txt` 和前後雜湊；判斷來源是「程式檢查」 | `chat-steps` S3 |
| 4 | 結束碼 1 的測試，模型說「全部測試都通過了」 | 那一步還是 `×`；證據清單有結束碼 1 | `chat-steps` 永遠成立 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-task-state-1 phone run ends verified with no cards` | 行為 1 | 報告卡 `textFor("report-status", lang, {status: "done", detail: "verified"})`；整輪沒出現過 `[data-role="approval-card"]`；`/control/computer/file?path=hello.txt` 是 `"hi"`；Q3 的狀態是 `done/verified` | 端對端 |
| `T-chat-steps-3 long output truncates and links evidence` | 行為 2 | 結果的字數＝6000；`steps-full-in-evidence` 看得到；點了之後 `EvidenceSheet` 標題是那一步的 `actionId` | 端對端 |
| `T-chat-steps-4 write evidence lists file and hashes` | 行為 3 | 證據頁有 `a.txt`、兩個 64 字的十六進位雜湊、`evidence-source-program` | 整合（從瀏覽器操作） |
| `T-chat-steps-5 failing test stays crossed despite the claim` | 行為 4 | 那一列是 `"×"`；證據頁有 `exit 1` | 整合（從瀏覽器操作） |
| `report card shows four levels and three folded blocks`（Vitest） | `report-delivery` S2 | 三個 `<details>` 都沒有 `open` 屬性 | 單元 |

**7. 不准做的**：
- 報告的結構、等級、標記一律照紀錄畫，前端不算。
- 不做任務頁（M8）；不做「要你注意」裡 M7 還沒有的種類。
- 不另外問模型要摘要。

**8. 驗收清單**：
1. 四列帶編號的測試綠，從基準檔拿掉。
2. 報告卡在手機情境下沒有橫向捲動。
3. Q3 的讀取方式寫進 `useTaskStatus`，有 Vitest。

---

### U11：👍＝你已確認

**1. 目標**：在一則回答上按 👍，任務變成「完成（你已確認）」，報告多一行「你已確認」；按掉 👍，狀態不倒退。

**2. 開工條件**：U10 已合進 `dev`；👍 的寫入入口（Q12）已經有了；`T-report-delivery-9` 還要等 Q4。

**3. 只准改的範圍**：`web/ui/src/chat/ThumbsUp.tsx`、`web/ui/src/report/ReportCard.tsx`、`locales/*`、`tests/e2e/confirm.spec.ts`。

**4. 定好的介面**：`ThumbsUp` 按鈕放在回答下面，打 Q12 的入口，再按一次是按掉。報告卡讀到「你已確認」那一筆，就多畫一行 `report-you-confirmed`。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 只有對話；回「好」；按 👍 | 先是完成（已回覆）；按了之後是完成（你已確認） | `trust-verify` S4 |
| 2 | 已經按過 👍，再按掉 | 狀態還是完成（你已確認） | `task-state` S5 那一列 |
| 3 | 報告已經產生；按 👍；在桌面送「把報告改成全部已驗證」 | 報告多一行「你已確認」，原本的字和標記都沒變；模型收到的工具清單裡沒有改報告的工具 | `report-delivery` S7 |

**6. 要寫的測試**

| 測試 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `T-trust-verify-6 thumbs up upgrades replied to confirmed` | 行為 1 | Q3 的狀態從 `done/replied` 變成 `done/you_confirmed` | 整合（從瀏覽器操作） |
| `T-task-state-5 removing thumbs up keeps confirmed` | 行為 2 | 按掉之後還是 `done/you_confirmed` | 整合 |
| `T-report-delivery-9 confirm line appends without rewriting`（Q4 答了才做） | 行為 3 | 報告的字跟按之前一樣，多一行 `report-you-confirmed`；請求的 `tools` 裡沒有改報告的工具 | 整合 |

**7. 不准做的**：不做其他表情（M8 的 `cards-emoji`）。

**8. 驗收清單**：1. 帶編號的測試綠，從基準檔拿掉。2. 👍 鈕 ≥44×44。

---

### U12：開關卡和「在你的電腦上執行」卡（條件片）

**1. 目標**：照 `safety-exec-card` S1、S2、S4、S5 和 `computers-local` 畫出開關卡。

**2. 開工條件**：U8 已合進 `dev`；trust 區 S3 已合進 `dev`（`CardKind::Switch`、`CardButton::DirectMode`）。

**3. 只准改的範圍**：`web/ui/src/cards/SwitchCard.tsx`、`locales/*`、`tests/e2e/switch-card.spec.ts`。

**4. 定好的介面**：
- `SwitchCard` 跟一般卡同一種黃。
- 標題「允許 {Bot} 用「{電腦}」嗎？」，下面「它想在「{電腦}」上做：{動作}」。
- 四個按鈕。`buttons` 裡有 `DirectMode` 時，下面多一行灰字「⚡ 之後都直接操作，不再一個個問」。
- `notes` 裡有「沒有指令沙盒」那一條時，多一行字。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | Bot1 的可以用的電腦沒勾「工作 Mac」；讀檔 `~/Documents/a.txt` | 出現兩行字；四個按鈕加「⚡」那一行 | S1、S2 |
| 2 | 管理員政策 `AllowDirectMode=false` | 沒有「⚡」那一行 | S2 |
| 3 | 開關卡已經一律允許；`rm old.log` | 出一般卡（刪除類） | S4 |
| 4 | 假電腦宣告沒有指令沙盒；按一律允許；呼叫 `dir` | 卡片有「這台電腦沒有指令沙盒…」；`dir` 又出卡，只有允許這次和拒絕 | S5 |
| 5 | 「Linux」每次詢問；送 `ls` | 出現「允許 {Bot} 在你的電腦上執行任務嗎？」；按一律允許後設定變成一律允許 | `computers-local` |

**6. 要寫的測試**：
- `T-safety-exec-card-1`、`T-safety-exec-card-4`、`T-safety-exec-card-6`、`T-safety-exec-card-7`、`T-computers-local-1`，一列一個，都是端對端。
- 預期值是上表的代碼和參數，`{bot: "Bot1", computer: "\u5de5\u4f5c Mac"}`。
- 設定的值從 `/v1/settings` 讀。

**7. 不准做的**：不准在前端判斷要不要出「⚡」。

**8. 驗收清單**：1. 五列綠，從基準檔拿掉。2. 跟一般卡共用顏色的元件，沒有複製一份。

---

### U13：開關卡按了之後（條件片）

**1. 目標**：驗按了開關卡之後的設定，和「已打開「直接操作」」那一行。

**2. 開工條件**：U12；trust 區 S4（答案寫回）、S6（`/always-approve`）。

**3. 只准改的範圍**：`web/ui/src/cards/*`、`locales/*`、`tests/e2e/switch-after.spec.ts`。

**4. 定好的介面**：卡片答完後，紀錄裡出現「直接操作打開了」，聊天就畫 `card-direct-on` 那一行。設定的值從 `/v1/settings` 讀。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 按一律允許 | Bot1 的可以用的電腦勾上「工作 Mac」；之後讀同一台的檔不再跳開關卡 | `safety-exec-card` S2 |
| 2 | 按「⚡」 | 這一步做了；直接操作打開；畫面出現「已打開「直接操作」」 | S2 |
| 3 | 沒有沙盒；已經打 `/always-approve`（這個前提由 testenv 先設好）；直接操作開；呼叫 `ls` | 卡片只有允許這次、拒絕；快轉 3600 秒還在等 | `trust-floor` S1 第 5 條 |

**6. 要寫的測試**：`T-safety-exec-card-2`、`T-safety-exec-card-3`、`T-trust-floor-2`，都是端對端。`-2`、`-3` 的設定照資料比對（`/v1/settings`）。`T-trust-floor-2` 要拿 `clock` 鎖。

**7. 不准做的**：不做設定 › 權限的畫面（M8）。

**8. 驗收清單**：三列綠，從基準檔拿掉；快轉時鐘的測試拿了鎖。

---

### U14：插話和停止的延伸（條件片）

**1. 目標**：工作中送的話變成插話，畫面寫「已排入」；按停止時，還沒被讀到的插話放回輸入框。

**2. 開工條件**：U9；turn 區 T3（插話收進下一步）、T12（`MessageOutcome::Steer`）；tools 區 F11、F12（背景指令）。

**3. 只准改的範圍**：`web/ui/src/chat/{Composer,QueuedNote}.tsx`、`locales/*`、`tests/e2e/steer.spec.ts`。

**4. 定好的介面**：
- `useSendMessage` 收到 `Steer` 時，輸入框下面寫 `composer-queued`（「已排入：「{text}」」）。
- 停止的時候，還沒被讀到的插話照 T3 的事件放回輸入框。
- 提示字：
  - 平常是 `composer-placeholder`（「傳訊息給 {bot}」）；
  - 框下小字是 `composer-hint-keys`（「Enter 傳送 · Shift+Enter 換行」）；
  - 工作中是 `composer-hint-working`（「工作中也可以繼續打字，Bot 會在下一步看到…」）。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 送「A」；工作中再送「B」 | 「已排入：「B」」；模型下一個請求含「B」 | `chat-steer` |
| 2 | 送「A」、送「B」；在模型讀到之前按停止 | 輸入框裡是「B」；模型從沒收到「B」 | `chat-steer`、`chat-stop` S5 |
| 3 | 看框下小字；送「A」；工作中看小字；再送「B」 | 依序出現三段字；提示字是「傳訊息給 {Bot 名稱}」 | `chat-input` S6 |
| 4 | 背景指令 #1 在跑；按停止；`read_terminal(id=1)` | #1 還在執行 | `files-background` S4 |

**6. 要寫的測試**：`T-chat-steer-1`、`T-chat-steer-3`、`T-chat-stop-6`、`T-chat-input-6`、`T-files-background-6`，一列一個。`T-files-background-6` 讀 `/control/computer` 的背景程式清單。

**7. 不准做的**：插話不准進本機匣子之外的地方；插話不准開新任務（T12 決定）。

**8. 驗收清單**：五列綠，從基準檔拿掉。

---

### U15：另一台裝置先動作（條件片，要 Q4）

**1. 目標**：驗「在電腦 A 按」「在桌面送」之後，手機上看到的結果。

**2. 開工條件**：U8、U9；Q4 答「可以」。

**3. 只准改的範圍**：`tests/e2e/src/other-device.ts`、`tests/e2e/other-device.spec.ts`、`web/ui/src/cards/*`（顯示錯誤代碼）。

**4. 定好的介面**：
- `otherDevice(request, account)`：
  - 用 Bearer 登入一台「電腦 A」的替身，只走公開的 API。
  - 方法有 `answerCard(cardId, answer)`、`send(botId, text)`。
- 回報寫明哪幾列用了替身。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 卡 1 在等；A 按允許這次；還顯示卡 1 的手機 P 按拒絕 | 卡 1＝允許；P 的聊天寫「手機上的回答沒有用上：那張卡已經不在等了」 | `phone-cards` S1 |
| 2 | A、P 在 50 毫秒內各按允許這次、拒絕 | 只有先到的算；另一台出現「那張卡已經回答過了（在 A 上）」 | `sync-referee` S2 |
| 3 | 卡片在等；按停止；在桌面送「再試」；呼叫同一個指令 | 第一張算拒絕；第二次照樣出卡 | `approval-answers` S5 |
| 4 | 承 `safety-reject-1`；在桌面送「好，現在可以」；`make deploy` | 出卡 | `safety-reject` S2 |
| 5 | 在桌面送的那一輪，一張卡在等；快轉 2 小時 | 卡片還在；這一輪沒有收工；任務狀態是等你，沒有逾時細項 | `safety-timeout` S5 |

**6. 要寫的測試**：
- `T-phone-cards-2`、`T-sync-referee-5`、`T-approval-answers-5`、`T-safety-reject-2`、`T-safety-timeout-7`。
- `-5` 那列先讓 A 按（替身），再讓 P 按，兩下間隔 <50 毫秒。P 上看到 `card-already-answered`，參數 `{device: "A"}`。

**7. 不准做的**：替身不准讀資料庫，也不准呼叫 testenv 的內部函式。

**8. 驗收清單**：五列綠，從基準檔拿掉；回報寫明用了替身。

---

### U16：問題卡（一）：作答流程

**1. 目標**：照 `cards-question` S3、S4、S7 做作答的流程。

**2. 開工條件**：U8；trust 區 S13 已合進 `dev`（`ask_person` 工具、`QuestionShown` 和 `QuestionAnswered` 事件）。

**3. 只准改的範圍**：`web/ui/src/question/*`、`locales/*`、`tests/e2e/question.spec.ts`。

**4. 定好的介面**：
- `questionReducer(state, event)`：純函式，事件有選、填、上一題、下一題、送出。
- `QuestionCard`：一次一題，有單選、複選、填寫三種。
- 回答照 S13 的格式送。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 第 1 題沒答，第 2、3 題有；在第 3 題按送出 | 沒送出，跳回第 1 題 | S4 |
| 2 | 單選、複選（選「甲」「乙」）、填寫（預填「台北」，改成「台中」按 Enter），按送出 | 單選點了直接下一題；模型收到「甲、乙」和「台中」 | S3 |
| 3 | 3 題填寫；第 1 題不填 → 填了按下一題 → 在第 2 題按上一題 | 不填時「下一題」按不下去；「上一題」回到第 1 題，剛填的字還在 | S4 |
| 4 | 答到第 2 題；換一張同編號的新卡 | 新卡從第 1 題開始 | S7 |

**6. 要寫的測試**：
- `T-cards-question-14`：Vitest，標題開頭寫編號，因為那一列的種類是單元。
- `T-cards-question-3`、`-4`、`-7`：端對端。字用 `"\u7532"`、`"\u4e59"`、`"\u53f0\u5317"`、`"\u53f0\u4e2d"`。

**7. 不准做的**：不做側邊欄的提醒（M8 的 `cards-question-8`）；在輸入框打字不算回答（S13 的 `-6`，trust 區驗）。

**8. 驗收清單**：四列綠，從基準檔拿掉；`questionReducer` 不超過 60 行，有需要就拆成每個事件一個函式。

---

### U17：問題卡（二）：位置和標題

**1. 目標**：問題卡在最新一則之後；往上捲時不會被拉回去；左上寫「（Bot）問你 · 1／1」。

**2. 開工條件**：U16。

**3. 只准改的範圍**：`web/ui/src/question/*`、`web/ui/src/chat/Timeline.tsx`（捲動）、`locales/*`、`tests/e2e/question-place.spec.ts`。

**4. 定好的介面**：`useStickToCard()`：卡出現時捲到卡；使用者往上捲時不拉回；按「↓」回到卡。標題 `question-title`，參數 `{bot, index, total}`。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | `ask_person`（3 題）；往上捲；按「↓」 | 卡在最新一則之後；聊天捲到卡；往上捲時沒被拉回；按「↓」回到卡 | S1 |
| 2 | `ask_person`（7 題）；再呼叫 1 題、題目 301 字 | 第一次工具回錯誤，沒出卡；第二次出卡，題目 300 字；左上「（Bot）問你 · 1／1」 | S2 |

**6. 要寫的測試**：`T-cards-question-1`（端對端）、`T-cards-question-2`（整合，從瀏覽器操作；工具回錯誤這一點從 `/control/model/requests` 看）。

**7. 不准做的**：題目的 300 字不准在前端截：照紀錄畫。

**8. 驗收清單**：兩列綠，從基準檔拿掉。

---

### U18：「未驗證」標記和連結（條件片）

**1. 目標**：回覆下面照 `ClaimChecked` 畫「未驗證」和原因；點連結時照 `claims-link` 處理。

**2. 開工條件**：U10；trust 區 S11（`ClaimChecked.reasons`）、S12（核對的叫醒）；tools 區 F10（傳檔進聊天、`FileShared`）。

**3. 只准改的範圍**：`web/ui/src/claims/*`、`web/ui/src/chat/AnswerLinks.tsx`、`locales/*`、`tests/e2e/claims.spec.ts`。

**4. 定好的介面**：
- `ClaimMark`：讀 `ClaimChecked`，原因照 `reasons` 的代碼畫。
- `AnswerLinks` 把回答裡的連結分成三種：
  - 聊天裡有的檔 → 打開預覽；
  - `file://` 這類手機打不開的位置 → `claims-link-unreachable`；
  - 其他 → 照一般連結。
- 「回覆沒對上」那一則，照 S12 的發起者「核對」畫小標。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 傳 `nope.md` 失敗；回「完成了，檔案已傳給你。」 | 回覆下面出現「未驗證」；原因是說已傳但那一步失敗；判斷來源標規則＋程式檢查；沒有任何模型收到核對的請求 | `claims-check` S1、S2、S5 |
| 2 | 回「報告已傳給你」，這一輪沒放檔 | 「未驗證」，原因「這一輪沒有放任何檔案」 | S2 |
| 3 | 傳 `spec.md`；回 `[下載](sandbox:/home/bot/spec.md)`；在手機點 | 沒有「未驗證」；打開 `spec.md` 的預覽 | `claims-link` S1、S2 |
| 4 | 回 `[r](file:///Users/x/r.md)`，沒放檔；點 | 「未驗證」；出現「這個連結指到 Bot 電腦上的位置，手機打不開；聊天裡也沒有這個檔案」 | S3 |
| 5 | 承 1；等這一輪結束 | 聊天出現標「回覆沒對上」的一則；模型的請求含「［核對］你上一則回覆…」，不含 `nope.md` | `claims-wake` S1 |

**6. 要寫的測試**：`T-claims-check-1`、`T-claims-check-2`、`T-claims-link-1`、`T-claims-link-2`、`T-claims-wake-1`，都是端對端。

**7. 不准做的**：不做桌面滑過（M8 的 `claims-mark`）；不在前端判斷宣稱是不是真的。

**8. 驗收清單**：五列綠，從基準檔拿掉。

---

### U19：檔案卡和交付記號（條件片）

**1. 目標**：Bot 傳的檔在手機上畫成檔案卡；手機顯示了、點開了，要回報給伺服器；記號從 ✓ 變 ✓✓ 再變強調色。

**2. 開工條件**：U10；tools 區 F10；M7j 的交付確認（裝置回報「顯示了／點開了」的入口）。

**3. 只准改的範圍**：`web/ui/src/files/*`、`locales/*`、`tests/e2e/deliver.spec.ts`。

**4. 定好的介面**：
- `FileCard` 寫「名字 · 類型 · 大小 · 點一下開啟」（tools 區 E 段）。
- 顯示和點開時，各打一次 M7j 的回報。
- `DeliveryMarks` 照紀錄畫記號，點記號列出「手機、時間、顯示／點開」。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 送「給我圖」；`send_file(dot.png)`；手機顯示；點開 | 顯示前是完成（已驗收）；顯示後是完成（已交付）；點開後是完成（你已確認） | `task-state` S1 |
| 2 | Bot 傳了 `p.png`；打開聊天；打開圖 | 記號 ✓ → ✓✓ → 強調色；點記號列出「手機、時間、顯示／點開」 | `deliver-confirm` |
| 3 | 使用者自己附了 `b.txt` | 沒有交付記號 | `deliver-confirm` |

**6. 要寫的測試**：`T-task-state-2`、`T-deliver-confirm-1`、`T-deliver-confirm-7`，都是端對端。狀態照 Q3 讀。

**7. 不准做的**：不做預覽頁的「檔案核對」（M8 之後）；不做使用者自己附檔（M8 的 `chat-attach`）：`-7` 的附檔用替身寫進紀錄，要先問 Q4。

**8. 驗收清單**：三列綠，從基準檔拿掉。

---

### U20：改派、離線、找不到電腦的那幾行（條件片）

**1. 目標**：照紀錄畫「改到別台做」「A 目前離線」「沒有這台電腦」，和報告裡「做不到的部分」。

**2. 開工條件**：U10；turn 區 T10（一般任務的煞車）、T18（路由）；M7j 的在線狀態；`T-goal-brake-11` 還要等 Q4。

**3. 只准改的範圍**：`web/ui/src/chat/{OfflineBanner,ReassignLine}.tsx`、`web/ui/src/report/*`、`tests/e2e/route.spec.ts`。

**4. 定好的介面**：
- `OfflineBanner`：照 M7j 給的電腦在線狀態，在聊天上方畫 `computers-offline-banner`，參數 `{computer}`。
- `ReassignLine` 畫 `mailbox-reassigned-offline`，代碼 turn 區已經加了。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 送「在工作機上跑 ls」，沒有這台 | 出現「沒有「工作機」這台電腦。可以用：…」 | `computers-default` S4 |
| 2 | 小幫手正在 A 跑一輪；A 斷網；快轉 61 秒 | 聊天上方出現「「A」目前離線：…。連上後自動更新」 | `computers-online` S3 |
| 3 | 關掉 A；送「打開我 Mac 上的 Keynote 檔改標題」 | 在 Bot 的電腦跑；報告寫「做不到的部分：…（要「A」上線）」；沒有標已驗收 | `mailbox-reassign` S3 |
| 4 | 做完的標準「out.csv 存在」；模型每輪只回「好了」 | 自己接 1 輪就停；未驗收（驗收沒通過）；沒有推播；沒有「目標暫停」；報告那一項標沒通過；在桌面送「再試」，回到進行中 | `goal-brake` S10 |

**6. 要寫的測試**：`T-computers-default-4`、`T-computers-online-3`、`T-mailbox-reassign-3`、`T-goal-brake-11`。推播照 `/control/push/records` 是空的；快轉時鐘要拿 `clock` 鎖。

**7. 不准做的**：前端不判斷電腦在不在線：只照伺服器給的狀態。

**8. 驗收清單**：四列綠，從基準檔拿掉。

---

### U21：繼續卡（條件片）

**1. 目標**：一輪中斷後，出現繼續卡；按繼續接著做，或改到 Bot 的電腦。

**2. 開工條件**：U8；turn 區 Q4（繼續卡的事件和按鈕）、T7、T15 已合進 `dev`。

**3. 只准改的範圍**：`web/ui/src/cards/ResumeCard.tsx`、`locales/*`（用 turn 區加的 `turn-resume-*`）、`tests/e2e/resume.spec.ts`。

**4. 定好的介面**：
- `ResumeCard` 列出可能已經做過的那一步，有一個「繼續」按鈕。
- 按了之後照 Q4 的回答送出；卡片過時的時候，照錯誤代碼顯示。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | `shell(echo x >> log.txt)` 跑完、還沒記下時關掉 A；打開 A | 等你（中斷，等你按繼續）；推播送出；`log.txt` 是「x」一行；出現繼續卡，列著 shell 那一步 | `task-resume` |
| 2 | 承 1，在手機按繼續 | 模型的請求含「它可能已經執行了」；`log.txt` 還是只有一行 | 同上 |
| 3 | 承 1，在桌面送「別的事」；再按舊卡的繼續 | 畫面說明那張卡已經過時；模型沒有收到接續的請求 | 同上 |
| 4 | `edit_file(b.txt)` 進行中關掉 A，不再打開；快轉到離線；按繼續 | 先是等你（做事的電腦不在線）；按了之後出現「改到 Bot 的電腦」；在 Bot 的電腦接著做 | 同上、K51 |
| 5 | 在「Linux」上跑 10 秒指令；關掉；快轉 1 分鐘；按繼續 | 那一輪標「中斷」；按繼續之後在 Bot 的電腦接著做，出現改派的那一行 | `computer-route` |

**6. 要寫的測試**：`T-task-resume-2`、`-3`、`-5`、`-6`、`T-computer-route-6`。`-5` 要先問 Q4；關掉 A 用 `/control/computer/unreachable`，或停掉 A 的執行端（用 PID 收）。

**7. 不准做的**：不准在前端判斷那一步做過了沒有。

**8. 驗收清單**：五列綠，從基準檔拿掉。

---

### U22：掃 M7 的每個畫面：44×44 和不出畫面

**1. 目標**：一個測試走過 M7 的每個畫面，量每一個可以點的東西。

**2. 開工條件**：U4–U10 都已合進 `dev`；Q10 答「可以」。

**3. 只准改的範圍**：`tests/e2e/sweep.spec.ts`、`tests/e2e/src/act.ts`（加 `TAP_TARGETS_JS`）。

**4. 定好的介面**：
- `visitM7Screens(page)` 依序打開這些畫面：登入、Bot 清單、聊天、步驟區、核准卡、報告卡、證據頁。
- 每一頁跑 `expectNoHorizontalScroll` 和 `expectTapTargets(page, 44)`。

**5. 行為表**

| # | 情況 | 預期 | 規格出處 |
|---|---|---|---|
| 1 | 量每一個可以點的東西 | 寬、高都至少 44px | `phone-claude` S3 |
| 2 | 每一頁 | 沒有橫向捲動；每個看得到的元素都在畫面寬度以內 | `phonefit-width` |

**6. 要寫的測試**：
- `T-phone-claude-2 every tap target is at least 44 by 44`。
- 掃到的畫面清單寫死在測試裡。M8 加畫面時，往清單裡加。

**7. 不准做的**：不准為了過關，只量看得到的一部分。

**8. 驗收清單**：一列綠，從基準檔拿掉；在 `phone-max-font`、`phone-wide-font` 下都量過。

---

## D. 要問的問題（每題附建議）

**第一批：擋住最多片的**

1. **Q1 正式版的手機網頁由誰給？**
   - `09` K113 ③ 寫了「伺服器自己給的網頁和登入頁」，但沒寫怎麼給。
   - 伺服器讀不了任意檔案（`07` §3），網頁的建置結果要麼編進伺服器，要麼由反向代理給。
   - 建議：
     - M7 只在 testenv 用 `ServeDir` 給（U1），讓測試同源；
     - 正式版怎麼給排在 M8，到時候在 K 表加一條，我傾向編進伺服器。
   - 擋住：U1 的範圍。
2. **Q2 即時通道（正在打的字、思考）走哪條線？誰做？**
   - `conn-batch` S2、S3 和 K118 ⑭ 只說「即時通道」：執行端每次送到目前為止的全部文字，伺服器只轉不存。
   - 現在沒有任何入口，四份規劃也都沒排。
   - 建議：
     - 執行端打 `POST /v1/streams/typing`，送 `{stream, task_id, text, thinking}`；
     - 伺服器在現有的 SSE 上多一種提示 `typing`，帶那段文字。打的字不是紀錄，所以不違反「SSE 不送紀錄的內容」（K118 ⑧）；
     - 由 M7j 做。
   - 擋住：U6。
3. **Q3 任務狀態在手機上去哪裡看？測試怎麼讀？**
   - K119 ⑬ 說狀態由伺服器和每台裝置各自從事件算；SSE 可以帶一個算好的值當提示。
   - 但狀態的程式在 Rust 的 `domain`，網頁用不到；任務頁又在 M8。
   - 建議：
     - 伺服器加一個唯讀入口 `GET /v1/tasks/{task}/status?bot=`，用 `domain` 的同一個函式算，回 `{status, detail}`；
     - 手機在報告卡上顯示；
     - 測試裡「任務狀態＝…」從這個入口讀，這是資料，也是從外面讀。
     - 網頁不重寫一份狀態的程式。
   - 擋住：U10、U11、U19。
4. **Q4 端對端列裡，另一台裝置的「動作」可以用替身嗎？**
   - 例如「在電腦 A 按允許」「在桌面送『再試』」。
   - K118 ⑱ 只准整合、性質列用替身。
   - 建議：預期沒寫那一台自己的畫面或行為時，可以用照公開 API 收送的替身，回報寫明；預期寫到那一台畫面的（例如 `T-chat-stop-8`），等 M8 的桌面。
   - 擋住：U15 的五列，和 `T-report-delivery-9`、`T-goal-brake-11`、`T-task-resume-5`、`T-deliver-confirm-7`。

**第二批：畫面的細節**

5. **Q5 長按多久算長按？**
   - 規格沒寫。
   - 建議 500 毫秒；按著不放超過 500 毫秒才算，中間手指移動超過 10px 就不算。
6. **Q6 網頁是伺服器自己給的時候，登入頁的網址欄怎麼處理？**
   - `account-login` S1 要填網址；`T-account-login-1` 的步驟有「填網址」。
   - 但網頁連不到別台伺服器：沒有 CORS，cookie 也是 SameSite=Strict。
   - 建議：
     - 欄位留著，預填 `location.origin`；
     - 改成別的網址時，照 S2 檢查：檢查失敗回「這個網址不是 AnyBot 伺服器」，檢查通過就提示「請在那台伺服器的網址打開」。
     - 測試的「填網址」就填 testenv 的網址。
7. **Q7 卡片在等時按停止，那一步怎麼畫？**
   - `chat-steps` S1 用「–」表示「被拒絕或已停止」；`T-safety-card-8` 寫「標『被拒』」。
   - 建議：符號用「–」，旁邊加一個字「被拒」（代碼 `steps-denied`）；按停止時另外寫「已停止」。
8. **Q8 註冊頁要的管理者名稱從哪裡拿？**
   - `server-privacy` S1 要寫「管理者（alice）」，但沒登入的頁面現在拿不到。
   - 建議：`/v1/server/status` 加一個欄位 `admin_account`，只在註冊開著時給，伺服器那邊由監督者另外派。
9. **Q9 要用固定帳號名的測試怎麼處理？**
   - 例如 `T-account-login-1` 的 `bob`。
   - 七個 project 共用一個 testenv，第二個 project 會撞到「帳號已存在」。
   - 建議：
     - testenv 加 `/control/account/delete`，只在測試裡用；
     - 用 `fixed-account-bob` 這把控制鎖讓這幾個測試排隊；
     - 同一個 project 跑完再放掉。
10. **Q10 `T-phone-claude-2` 這種「量每一個」的列，只掃 M7 現有的畫面可以帶編號嗎？**
    - 建議可以，比照 `T-phonefit-width-1`；M8 加畫面時把它加進同一個清單。

**第三批：跟別區對齊**

11. **Q11 `T-approval-answers-1` 的測試要不要改名？**
    - 這列是端對端，但已經被執行端的 `crates/runner/tests/turn_approval.rs::t_approval_answers_1` 拿走了。照 K86，端對端要從瀏覽器驗。
    - 建議：
      - 那個 Rust 測試改成描述性的名字，編號放回基準檔，由 U8 用瀏覽器驗；
      - 由 trust 區下一片做。
      - 這一區不碰 `crates/runner`。
12. **Q12 👍 用哪個入口寫？**
    - `cards-n1`、`report-delivery` S7 都說「按 👍 加『你已確認』」，但沒說是紀錄還是動作。
    - 建議：
      - 當成裝置能直接送的一筆紀錄：`POST /v1/streams/append`，種類 `thumbs_up`，內容 `{answer_seq}`；
      - 按掉是 `thumbs_up_removed`；
      - 狀態照 `task-state` S5 只升不降。
      - 種類由 trust 區定。
13. **Q13 Bot 用哪個模型，由哪個設定決定？**
    - testenv 建 Bot 時要指到假模型，現在沒有設定的名稱。
    - 建議：照 turn 區 T18 加的 Bot 設定，或模型區的設定鍵；U1 先用 Bot 的設定欄位 `model`，值是 `fake-<uuid>`，名稱定了再改。
14. **Q14 「完整內容在證據裡」點了打開的證據頁，在手機上放哪？**
    - 建議：從底部拉上來的唯讀頁，關掉後回到原本捲動的位置；完整的稽核頁在 M8 的 `activity-phone`。
15. **Q15 Web Push 由誰發？**
    - turn 區 Q17 建議由手機畫面區做。
    - 建議不要：發推播是伺服器的事，`channels` 套件裡的程式，不在 `web/` 底下；由 M7j 做。這一區只用 `/control/push/records` 驗。

**第四批：可以晚點答的**

16. **Q16 `chat-n9` 新對話的開場要寫什麼？** 規格沒寫內容，M8 前要定。
17. **Q17 `T-chat-window-4` 的「畫面更新」怎麼數？**
    - 建議數回答那一格的文字節點真的變了幾次，用 `MutationObserver`，同一個畫面幀裡變好幾次只算一次。
18. **Q18 `AnyBot-Client-Version` 送什麼版本？**
    - 網頁要送它自己的版本，照 K118 ⑤。
    - 建議送 `apps/web/package.json` 的 `version`，建置時寫進去；testenv 的伺服器版本是 `testenv-9.9.9`，要確定 `0.x` 的網頁不會被當成太舊，由 U3 確認，不行就停下來回報。

---

## E. 跟其他規劃區共用的介面

**turn／task 區（T0–T21）**

1. **執行端要能當函式庫用**（T13、T14），U2 要用：
   - `apps/runner-service`、`apps/agent` 的主程式只是殼。
   - 「領信 → 跑一輪 → 寫紀錄」的程式放在函式庫裡，電腦（`GatedComputer`）、模型閘道的網址、時鐘都從建構函式傳進去。
   - 不准用旗標或環境變數。
2. **送一句話**：`POST /v1/bots/{bot}/messages`，帶 `Idempotency-Key`。回 `MessageOutcome`，含 `task_id` 和「新任務、插話、排隊」哪一種（T12）。U5、U14 用。
3. **停止**：用現有的 `/v1/streams/referee`，`action: stop`。停止時執行端寫 `CardAnswered{Deny}`（T5、Q9）。U9 只照紀錄畫。
4. **代碼**：`turn-reply-stopped`、`turn-resume-*`、`mailbox-reassigned-offline`、`task-stalled` 由 turn 區加進 `locales`，這一區只用。
5. **任務狀態**：照 D 的 Q3 加一個唯讀入口，請 turn 區或 M7j 做，用 `domain` 的同一個函式。
6. **交過來、但這一區排到 M8 或之後的**：`T-chat-stop-8`、`T-sync-referee-3`、`T-mailbox-reassign-1` 要電腦 A 的畫面。這三列的引擎那一半照做，用描述性的名字。

**trust／cards 區（S1–S13）**

1. **卡片紀錄要帶畫面要的全部欄位**（S2）：`card_id`、`task_id`、`kind`、`buttons`（照順序）、`always_label`（「一律允許：…」那一句）、`reason`（哪一層要問的）、`notes`（例如沒有沙盒、會取消哪條規則）、`computer`（電腦的顯示名稱）、動作的細節。畫面一律照它畫，自己不決定按鈕。
2. **開關卡**（S3）：`CardKind::Switch`、`CardButton::DirectMode`。直接操作打開之後，要有一筆紀錄，畫面才畫得出「已打開『直接操作』」（U13）。
3. **報告和證據**（S7–S9）：`DeliveryReportRecord` 加了 `claims`、`level_counts`、`progress`、`ReportAttention.kind` 之後，U10 照畫。證據的指令輸出前後各 6,000 字，給 `T-chat-steps-3` 用。
4. **核對**（S11、S12）：`ClaimChecked.reasons` 用訊息代碼。U18 照代碼畫。
5. **問題卡**（S13）：`QuestionShown` 帶題目、種類、預填、卡片編號；`QuestionAnswered` 的格式定了之後，U16 照送。
6. **`T-approval-answers-1`**：見 D 的 Q11。
7. **交過來、但這一區排到 M8 或之後的**：`T-approval-answers-3`（一律允許清單畫面）、`T-report-delivery-1`（寄信工具）、`T-autonomy-yolo-6`、`T-autonomy-remote-1`（斜線指令）。

**tools／delivery 區（F1–F12）**

1. 能中斷正在跑的指令（`chat-stop` S2）：`T-chat-stop-2`、`T-safety-gates-6` 要用。
2. `FileShared`（F10）帶名字、類型、大小、檔案編號，U19 的檔案卡照畫；預覽頁在 M8。
3. 背景指令的清單從 `/control/computer` 讀得到（F11、F12），U14 的 `T-files-background-6` 要用。
4. **交過來、但這一區排到 M8 或之後的**：`T-computer-route-2`（任務詳情）、`T-files-rewind-10`（斜線指令）、`T-computers-n2-3`（看畫面，M9）、`T-computers-n5-2`（M10+）。

**記憶區（B1–B29）**

- B28、B29 的大腦頁面用 U3 的路由、`createAnyBotClient`、`useRecords`，和 C0 的規定。
- 頁面掛在 `/brain` 底下，不改 `ChatView`。
- 記憶頁的入口和「帶了 N 條記憶」在 M8。

**伺服器整合（M7j）**

1. 即時通道（Q2）：`POST /v1/streams/typing`，加 SSE 的 `typing` 提示。
2. 電腦在線狀態給手機讀（`computers-online` S3），U20 用。
3. 交付確認：裝置回報「顯示了／點開了」（`deliver-confirm`），U19 用。
4. 註冊頁的管理者名稱（Q8）。
5. Web Push 的發送（Q15）。

**testenv（這一區做，別區可以用）**

- U1、U2 做好之後，別區要寫「從外面打」的整合測試，可以直接用 `POST /control/seed`、`/control/computer/*`、各自一份的假模型腳本，和控制鎖。
- 別區要加控制 API 時，加在 `tests/testenv/src/control.rs` 的新檔，不改這一區的路由。
