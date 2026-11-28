# M7／M7.5 規劃：大腦與上下文（記憶、上下文編譯器、搜尋、切詞）

這份是**規劃**，不是任務檔。給監督者（Claude）拿來切派工用；每一片要派出去前，再照 `docs/tasks/_template.md` 抄成獨立的任務檔，並照 D 節的答案改掉標了「待 Q-n」的地方。

- 規格來源：`spec/09` K95–K98、K103、K106、K107、K112 ⑨、K113 ②、K114、K119 ⑯ ⑰；`spec/10-behavior/3-chat-bots.md` 的 `chat-compact`、`chat-n15`、`context-*`、`memory-*`；第 6 組 `models-capabilities` S5–S8；`spec/07` §3；`spec/06`。
- 現況（`ws/m7-plan` 切出時的 `dev`）：`crates/context` 有編譯器純函式（M7b），`t_context_build_1`、`t_context_budget_1`、`t_context_build_4`、`t_context_state_2` 已驗；M7b review fixes 把 8 列純函式的編號放回基準檔，只留描述性測試。`crates/runner/src/context_input.rs` 的記憶、釘選、舊對話都還傳空的。`protocol`、`store-server`、`store-client`、`server` 都還沒有任何記憶的程式；`migrations/server` 到 `0124`，`migrations/client` 只有 `0001`。
- 回合引擎那邊的片名沿用 `docs/STATUS-huskyVer.md` 的順序，暫稱：M7e 卡片、M7f 停止／插話／接續、M7g 換模型、M7h 做完的標準、M7i 檔案工具、M7j 伺服器整合、M7k 畫面。別的規劃區改名時，照功能對過來。
- 編號規則照 K86：名字帶編號＝那一列的「預期」整欄都驗到；K119 ⑰：還沒接上伺服器時，「畫面出現」「推播」「按〔按鈕〕」不算驗到，步驟有伺服器停機、兩台裝置、斷網的，等接上伺服器。

---

## A. 範圍分類

分類：**M7**（M7 的剩下幾片做）／**M7.5**（大腦）／**之後**（寫出里程碑）／**還不能測**（寫原因）。

### A.1 這一區的列（102 列）

| 編號 | 分類 | 切片 | 理由（一行） |
|---|---|---|---|
| T-context-build-2 | M7 | B2 | 純函式性質測試已有描述性版本，補齊「快照更新」那一種、1,000 組就能收 |
| T-context-build-3 | M7 | B5 | 執行端層：40 步後清掉清單和快取再組，要能從同一份紀錄重開一輪 |
| T-context-budget-2 | M7 | B2 | 純函式；現有 `shrinking_usable_drops_old_results_then_chat_then_memory` 補齊整欄 |
| T-context-budget-3 | M7 | B8 | 執行端層；要 M7g 的換模型 |
| T-context-budget-4 | M7 | B7 | 執行端要把任務以外的舊對話餵進 `prior_chat`（現在是空的） |
| T-context-budget-5 | M7 | B5 | 執行端層，40 步跑完數少放的次數 |
| T-context-fixed-1 | M7.5 | B24 | 要真的釘選記憶和「開始新對話」的快照 |
| T-context-fixed-2 | M7 | B4 | 補齊固定段的第 3、4 塊（安全、任務和證據）後，執行端層可驗 |
| T-context-manifest-1 | M7 | B2 | 純函式；現有 `manifest_entries_match_request_blocks` 補齊整欄 |
| T-context-manifest-2 | 之後（M8） | — | 步驟要打開「這一步給模型看了什麼」畫面 |
| T-context-manifest-3 | M7 | B5 | 執行端層：假模型回輸入用量 1,234，清單補實際值和差 |
| T-context-mask-1 | M7 | B11 | 預期有「畫面上每一步的結果完整」，等 M7j |
| T-context-mask-2 | M7 | B3 | 純函式；現有 `mask_pairs_and_accumulation` 補齊整欄 |
| T-context-mask-3 | M7 | B1 | 錯誤位置要由程式找（現在由呼叫的人傳 `error_at`，找法沒做） |
| T-context-mask-4 | M7 | B5 | 執行端層：`read_file` 讀同一個檔兩次 |
| T-context-mask-5 | M7 | B5 | 執行端層：兩筆收起事件、清快取重組位元組一樣 |
| T-context-mask-6 | M7 | B1 | 8 種錯誤樣式的找法還不存在（`crates` 裡沒有 `Traceback` 這個字） |
| T-context-must-1 | M7 | B2 | 純函式；現有 `must_items_are_verbatim_or_the_build_is_refused` 補齊整欄 |
| T-context-must-2 | M7 | B11 | 預期有「畫面寫缺多少和兩個做法」，等 M7j |
| T-context-must-3 | M7 | B8 | 要 M7g 換模型、M7h 做完的標準 |
| T-context-must-4 | M7 | B7 | 要 `read_result` 工具（第 4 組 `files-n1`，工具區） |
| T-context-must-5 | M7 | B7 | 要插話（M7f）把補充、糾正送進進行中的任務 |
| T-context-ref-1 | M7 | B12 | 要 `read_result`（含 `find`）工具 |
| T-context-ref-2 | M7 | B12 | 要 K103 的保存規則（被引用就留） |
| T-context-ref-3 | M7 | B12 | 要伺服器的永久刪除接到 `read_result`，等 M7j |
| T-context-state-1 | M7 | B3 | 純函式；現有 `state_summary_is_stable_and_cites_event_seqs` 補 1,000 組和雜湊比對 |
| T-context-state-3 | M7 | B9 | 要 M7h 的做完的標準（「還沒驗到」） |
| T-context-state-4 | M7 | B3 | 純函式；現有 `long_plan_summary_counts_done_steps` 補齊整欄 |
| T-chat-compact-7 | M7.5 | B23 | 預期「帶了 2 筆記憶」要真的找回記憶 |
| T-chat-compact-8 | M7 | B6 | /compact 三種回覆，執行端層可驗 |
| T-chat-compact-10 | M7 | B6 | 自動收起和 /compact 混著做，紀錄流的聊天不變（性質） |
| T-chat-compact-11 | M7 | B11 | 預期有「畫面沒有出現」，等 M7j |
| T-chat-n15-1 | 之後（M8） | — | 要「匯出文字記錄」 |
| T-chat-n15-2 | 之後（做「重新產生」的那片） | — | 步驟是按「重新產生」，這個功能還沒排；S2 的計時本身在 B9 做描述性測試 |
| T-chat-n15-3 | M7 | B9 | 兩則訊息的前段一字不差，要舊對話正確進請求 |
| T-chat-n15-4 | M7 | B9 | 原生區塊只送回同一家同一個模型（K109 ②），和模型區共用 |
| T-chat-n15-5 | 之後（遷移工具，M10 之後） | — | 要舊版資料遷移 |
| T-models-capabilities-6 | M7 | B10 | K112 ⑨：分詞器套件在 M7 加；參考值來源見 Q-21 |
| T-memory-kinds-1 | M7.5 | B21 | `remember` 600 字只留 500 字 |
| T-memory-kinds-2 | M7.5 | B21 | 每個 Bot 只搜得到自己的 |
| T-memory-kinds-3 | 之後（做支線 /btw 的那片） | — | 要 /btw 支線 |
| T-memory-kinds-4 | M7.5 | B26 | 兩台裝置加斷網，等 M7j 之後 |
| T-memory-kinds-5 | 之後（遷移工具） | — | 舊版資料遷移 |
| T-memory-pin-1 | 還不能測 | — | 跟 K106「每段對話開始時凍結快照」衝突：沒開新對話，C 照 K106 仍會帶（Q-5） |
| T-memory-pin-2 | M7.5 | B24 | 工具清單沒有釘選工具 |
| T-memory-pin-3 | M7.5 | B28 | 大腦頁面基本版（範圍待 Q-19） |
| T-memory-pin-4 | 之後（M8） | — | 輸入框上的帶入記憶小卡片 |
| T-memory-pin-5 | M7.5 | B29 | 大腦頁面基本版 |
| T-memory-carry-1 | 之後（M8） | — | 小卡片 UI |
| T-memory-carry-2 | 之後（M8） | — | 小卡片 UI |
| T-memory-carry-3 | 之後（M8） | — | 預期有「展開看到 2 條和打開記憶頁」（訊息下面的 UI） |
| T-memory-carry-4 | 之後（M9） | — | 步驟要觸發例行任務；每一步重新找本身在 B23 做描述性測試 |
| T-memory-carry-5 | M7.5 | B23 | 勾選最多 20 條；空清單的意思待 Q-25 |
| T-memory-carry-6 | M7.5 | B23 | 矛盾一起帶並標記；矛盾怎麼認待 Q-12，應答詞清單待 Q-23 |
| T-memory-carry-7 | M7.5 | B23 | 勾的定住、程式找的每步重新決定 |
| T-memory-carry-8 | 之後（M8） | — | 工作中沒有小卡片（UI） |
| T-memory-carry-9 | M7.5 | B26 | 伺服器停機時用本機副本只比字詞，等 M7j 之後 |
| T-memory-about-1 | M7.5 | B24 | 「關於你」跨 Bot 帶 |
| T-memory-about-2 | M7.5 | B23 | 最新 40 條、最舊那條搜得到 |
| T-memory-about-3 | M7.5 | B21 | 搜尋結果標「關於你（共用）」 |
| T-memory-about-4 | M7.5 | B22 | `forget` 自己找不到才動「關於你」 |
| T-memory-about-5 | 還不能測 | — | 「整理建議」跟 K97、K106 衝突，`memory-page` S7 要重寫（Q-6） |
| T-memory-about-6 | 之後（訊息管道，M10 之後） | — | 要頻道 |
| T-memory-search-1 | 還不能測 | — | 規格標了「以 09 為準，做到時重寫」（Q-1）；還要語意搜尋 |
| T-memory-search-2 | M7.5 | B21 | 只看、不跳卡；不受重寫影響 |
| T-memory-search-3 | 還不能測 | — | 同上標了要重寫；「用意思找記憶」設定隨向量延後（Q-4） |
| T-memory-forget-1 | M7.5 | B22 | 「在記憶頁按還原」用頁面會打的 API 動作驗（整合列） |
| T-memory-forget-2 | M7.5 | B22 | 對到兩條回候選 |
| T-memory-forget-3 | M7.5 | B22 | 落到「關於你」 |
| T-memory-forget-4 | M7.5 | B22 | 做過的事動不到 |
| T-memory-page-1 | 之後（M8） | — | 四個入口在 Bot 選單、設定卡、小卡片、訊息下，加手機分頁和團隊成員 |
| T-memory-page-2 | M7.5 | B28 | 基本頁：切換和摘要（「語意搜尋狀態」顯示關閉，待 Q-4） |
| T-memory-page-3 | M7.5 | B28 | 基本頁：搜尋和篩選 |
| T-memory-page-4 | M7.5 | B28 | 基本頁：新增 |
| T-memory-page-5 | M7.5 | B29 | 帶過次數、清空最近刪除 |
| T-memory-page-6 | 還不能測 | — | S7 整理建議跟 K106 衝突（Q-6） |
| T-memory-page-7 | M7.5 | B28 | 一次 60 條 |
| T-memory-n1-1 | 還不能測 | — | S1 被 K97、K106 取代，規格要重寫（Q-1） |
| T-memory-n1-2 | 還不能測 | — | S2 被 K97、K106 取代（Q-1） |
| T-memory-n1-3 | M7.5 | B21 | S3 仍有效；救回後的狀態待 Q-8 |
| T-memory-n1-4 | M7.5 | B29 | 編輯成一樣就合併（K96 合併留轉址） |
| T-memory-n2-1 | M7.5 | B25 | 任務做完自動記一行 |
| T-memory-n2-2 | M7.5 | B25 | 要「查網頁」的工具；到時沒有就延後（見 B25） |
| T-memory-n2-3 | M7.5 | B25 | `forget` 動不到 |
| T-memory-n3-1 | 還不能測 | — | S2、S3 被取代；每晚整理現在要做什麼沒寫（Q-1、Q-26） |
| T-memory-n3-2 | 還不能測 | — | 驗的正是被取代的 S2 |
| T-memory-n3-3 | 還不能測 | — | 驗的正是被取代的 S2 |
| T-memory-n3-4 | 還不能測 | — | 驗的正是被取代的 S3 |
| T-memory-n3-5 | 還不能測 | — | 「整理建議」待重寫（Q-6），「180 天」和 K106「90 天候選」的關係沒寫 |
| T-memory-n3-6 | 還不能測 | — | 整理工作本身未定，補做一次沒東西可驗 |
| T-memory-n3-7 | 還不能測 | — | 同上；而且「伺服器資料庫備份」的功能屬伺服器區 |
| T-memory-n4-1 | 還不能測 | — | 標了要照 K107 重寫；向量延後（Q-4） |
| T-memory-n4-2 | 還不能測 | — | 伺服器不准發 HTTP、不准依賴 `models`（`07` §3），下載誰做沒定（Q-4） |
| T-memory-n4-3 | 還不能測 | — | 同上，還要遷移 |
| T-memory-n4-4 | 還不能測 | — | 向量延後（Q-4） |
| T-memory-n4-5 | 還不能測 | — | 「關閉」設定隨向量延後（Q-4） |
| T-memory-n4-6 | 還不能測 | — | 隨向量延後（Q-4） |
| T-memory-n4-7 | 之後（遷移工具） | — | 舊版 Ollama 選項遷移；也標了要重寫 |
| T-memory-n5-1 | 之後（訊息管道＋團隊成員） | — | 頻道和團隊成員都還沒有 |
| T-memory-n5-2 | 之後（團隊成員 `account-share`） | — | 團隊成員還沒有 |
| T-memory-n5-3 | 之後（團隊成員＋M8） | — | 端對端，團隊成員的手機 |
| T-memory-n5-4 | 之後（團隊成員） | — | 「記憶只有擁有者能看和改」的擋法在 B16 先做描述性測試 |

**小計（102 列）**：M7 **32** 列；M7.5 **31** 列；之後 **19** 列；還不能測 **20** 列。

### A.2 別區的列，但碰到 K96–K98、K106、K107、K113 ②（11 列，只列出來，不在這一區做）

| 編號 | 分類 | 理由 |
|---|---|---|
| T-files-n1-4 | M7（工具區） | 「被記憶引用就留」：B16 之後記憶的證據參照要算進引用（E 節） |
| T-sync-append-5 | 之後（M8） | 再確認畫面寫「Bot 從這些對話記住了 1 件事」；B16 先提供「某段對話學到幾筆」的查詢 |
| T-sync-scope-1 | 之後（技能、核准規則、金鑰都同步之後） | 記憶這一項在 B26 做描述性測試 |
| T-branch-btw-4 | 之後（支線 /btw） | 同 `T-memory-kinds-3` |
| T-safety-n4-2 | 之後（訊息管道） | 不受信任的一輪沒有記憶工具，B21 提供工具清單的開關 |
| T-safety-n4-3 | 之後（訊息管道） | 不受信任的一輪不帶記憶，B24 提供不帶的入口 |
| T-channels-untrusted-1 | 之後（訊息管道） | 同上 |
| T-trust-promises-7 | 之後（訊息管道） | 同上 |
| T-migrate-fields-8 | 之後（遷移工具） | 170 條合併 |
| T-migrate-drop-2 | 之後（遷移工具） | 記憶索引重建用 B19 的重建；文件搜尋另有歸屬 |
| T-migrate-readonly-2 | 之後（遷移工具） | 讀舊版 WAL |

---

## B. 切片索引

伺服器遷移每片給 5 個號，從 `0350` 起；客戶端從 `migrations/client/0040` 起，也是每片 5 個號。表裡沒寫的就是不用遷移。

| 切片 | 標題 | 規格編號（≤5） | 依賴 | 可並行 | 套件 | 遷移 | 協定改動 |
|---|---|---|---|---|---|---|---|
| B1 | 錯誤位置的找法 | T-context-mask-3、T-context-mask-6 | dev | B2、B3、B4、B10、B13 | context、runner | — | — |
| B2 | 純函式列收尾（一） | T-context-build-2、T-context-budget-2、T-context-manifest-1、T-context-must-1 | dev | B1、B3、B4、B10、B13 | context | — | — |
| B3 | 純函式列收尾（二） | T-context-mask-2、T-context-state-1、T-context-state-4 | dev（mask-2 跟 B1 碰同一檔，B1 先合） | B2、B4、B10、B13 | context | — | — |
| B4 | 固定段六塊和語言 | T-context-fixed-2 | dev；信任區給安全指示的訊息代碼、回合區給任務和證據指示的代碼 | B1–B3、B10、B13 | runner、messages（`locales/`） | — | — |
| B5 | 執行端：重組、收起、少放的節奏 | T-context-mask-4、T-context-mask-5、T-context-build-3、T-context-budget-5、T-context-manifest-3 | B1；M7f 的接續（build-3、mask-5） | B6、B10、B13 | runner（測試） | — | — |
| B6 | /context 和 /compact | T-chat-compact-8、T-chat-compact-10 | B5；回合區的斜線指令入口 | B7、B10 | runner、messages | — | — |
| B7 | 舊對話、長原話、補充 | T-context-budget-4、T-context-must-4、T-context-must-5 | B5；工具區的 `read_result`；M7f 插話 | B6、B8 | runner | — | — |
| B8 | 換模型時的上下文 | T-context-budget-3、T-context-must-3 | B5；M7g、M7h | B6、B7 | runner | — | — |
| B9 | 對話連續、原生區塊、模型的宣稱 | T-chat-n15-3、T-chat-n15-4、T-context-state-3 | B7（舊對話）；M7h | B8 | runner、context | — | — |
| B10 | token 估算的分詞器 | T-models-capabilities-6 | B13 的套件結果；Q-21 | B1–B9 | models | — | — |
| B11 | 接上伺服器才驗得到的上下文列 | T-context-mask-1、T-context-must-2、T-chat-compact-11 | M7j；B5、B8 | B12 | runner、server 測試 | — | — |
| B12 | 照編號讀回原文 | T-context-ref-1、T-context-ref-2、T-context-ref-3 | M7j；工具區 `read_result` 和 K103 保存 | B11 | runner、context | — | — |
| B13 | 探路（切詞套件、FTS5 trigram、pg_trgm／pgvector、字元分類、分詞器套件） | — | dev | 全部 | tests/harness（守門測試）、`docs/` | — | — |
| B14 | 切詞模組 | — | B13；Q-2、Q-3、Q-14 | B15 | domain 或新套件（Q-2） | — | — |
| B15 | 記憶事件、投影、准入（純邏輯） | — | Q-7–Q-13、Q-24 | B14 | domain、protocol、context（換掉重複的 `MemorySource`） | — | 記憶事件的內容型別 |
| B16 | 伺服器：記憶紀錄、投影表、API | — | B15 | B18 | protocol、store-server、server | 0350–0354 | 記憶 API 型別 |
| B17 | 伺服器搜尋：PG 全文、trgm、排序 | — | B13、B14、B16 | B18 | store-server、server | 0355–0359 | 搜尋 API 型別 |
| B18 | 客戶端副本索引：SQLite FTS5 | — | B14、B15 | B16、B17 | store-client | client 0040–0044 | — |
| B19 | 索引重建和一致性檢查 | — | B17、B18；Q-18 | B20 | store-server、store-client、apps/server（命令列，待 Q-18） | 0360–0364；client 0045–0049 | — |
| B20 | 10 萬筆效能關卡 | — | B17、B18；Q-17 | B19、B21 | store-server、store-client 測試、tests/harness、`.config/nextest.toml` | — | — |
| B21 | 記憶工具：記、找 | T-memory-kinds-1、T-memory-kinds-2、T-memory-search-2、T-memory-about-3、T-memory-n1-3 | B16、B17；工具區的工具清單 | B18–B20 | tools（工具說明）、runner、sync | — | — |
| B22 | 忘記和還原 | T-memory-forget-1、T-memory-forget-2、T-memory-forget-3、T-memory-forget-4、T-memory-about-4 | B21 | B23 | runner、server | — | — |
| B23 | 每一步重新找、帶進上下文 | T-memory-carry-5、T-memory-carry-6、T-memory-carry-7、T-memory-about-2、T-chat-compact-7 | B21、B6；回合區的 `BrainSource` 接口 | B22、B25 | runner、context、protocol | — | 送訊息帶勾選的記憶編號 |
| B24 | 釘選和「關於你」的快照 | T-memory-pin-2、T-memory-about-1、T-context-fixed-1 | B23、B4；回合區的「開始新對話」 | B25 | runner、context、server | — | — |
| B25 | 做過的事自動記一行 | T-memory-n2-1、T-memory-n2-2、T-memory-n2-3 | B16、M7h；查網頁的工具 | B24 | domain、runner | — | — |
| B26 | 離線和跨裝置 | T-memory-kinds-4、T-memory-carry-9 | B18、B21、M7j、M6c | B27 | runner、store-client、sync | — | — |
| B27 | Markdown 雙向同步 | — | B16；Q-16 | B26、B28 | domain（轉換）、store-client（讀寫檔） | client 0050–0054 | 匯回動作 |
| B28 | 大腦頁面（一）：清單、搜尋、新增 | T-memory-page-2、T-memory-page-3、T-memory-page-4、T-memory-page-7、T-memory-pin-3 | B16、B17、M7k；Q-6、Q-19 | B27 | web/ui、server | — | — |
| B29 | 大腦頁面（二）：帶過次數、清空、編輯合併、久沒用的候選 | T-memory-page-5、T-memory-pin-5、T-memory-n1-4 | B28、B22；Q-15 | B26 | web/ui、server | — | — |

**共 29 片**：M7 12 片（B1–B12），M7.5 17 片（B13–B29）。B13 探路沒有規格編號，但要排在 B14 之前，可以現在就跟 M7 的片並行。

**共用的熱檔案**（只准往後加，衝突時保留別人的）：`locales/en.ftl`、`locales/zh-TW.ftl`（B4、B6、B21、B22、B23、B25、B28）；`crates/messages` 產生的代碼檔（照產生流程重產）；`crates/context/src/input.rs` 的 `BuildInput`（B1、B9、B15、B23、B24）；`crates/runner/src/context_input.rs` 和執行端的建構函式（B6、B7、B23、B24）；`crates/protocol/src/lib.rs`（B15、B16、B17、B23）；伺服器的路由表（B16、B17、B22、B28）；根目錄 `Cargo.toml` 的依賴（B10、B14）；`deny.toml`（B10、B14 若要動，先問）；`.config/nextest.toml`（B20）。`openapi` 和 `tools/spec-trace/baseline.txt` 由監督者重產，片裡不手改。

---

## C. 切片藍圖

每片的第 9 節「驗證和回報」都照 `_template.md` §9，不再重寫。測試裡的中日韓文字一律用 Unicode 跳脫（K92）；給模型和使用者看的固定字一律是訊息代碼（K91、K119 ⑯），測試照代碼和參數比。

### B1 錯誤位置的找法

1. **目標**：工具結果超過上限時，錯誤位置改由程式照 `context-mask` S2 的 8 個樣式找出來，不再靠呼叫的人傳。
2. **開工條件**：`dev` 上有 `crates/context/src/mask.rs`（`render_view`）和 `StepRecord.error_at`。分支 `ws/b1-error-line`。
3. **只准改**：`crates/context/src/error_line.rs`（新）、`mask.rs`、`input.rs`（`error_at` 的型別）、`lib.rs` 的匯出；`crates/runner` 裡建 `StepRecord` 的那一處；測試 `crates/context/src/error_line_tests.rs`（新）、`mask_tests.rs`。
4. **定好的介面**：
   - `pub const ERROR_PATTERNS: [&str; 8] = ["error", "Error:", "ERROR", "FAILED", "panicked", "Traceback", "Exception", "fatal:"];`
   - `pub fn find_error_line(content: &str, exit_code: Option<i64>) -> Option<usize>`：回那一行開頭在第幾個字（字元，不是位元組）。
   - `StepRecord.error_at` 從 `Vec<usize>` 改成 `Option<usize>`（K114 ⑮ 只取一個）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 結束碼 1，有一行含 `Traceback` | 回那一行；看的內容三段各佔上限的 1/3 | S2、K114 ② |
| 2 | 結束碼 1，兩行 `Error:` | 只取靠近結尾的那一行 | S2、K114 ⑮ |
| 3 | 結束碼 0，有 `Error:` | `None`，頭尾各一半 | S2 |
| 4 | 結束碼 1，沒有樣式 | `None`，頭尾各一半 | S2 |
| 5 | 結束碼 1，只有小寫 `failed` | `None`（大小寫要一樣） | S2 |
| 6 | 結束碼沒有（`None`） | `None` | S2「只在結束碼不是 0 時找」 |
| 7 | 一行含好幾個樣式 | 照「那一行」算一次 | S2 |

6. **要寫的測試**：

| 測試 | 驗 | 寫死的預期 | 種類 |
|---|---|---|---|
| `t_context_mask_6` | 列 1–5 | 上限 8,000 字（K114 ②）；8 份各有一個樣式在中間、結束碼 1 → 看的內容有開頭、那一行前後、結尾三段，每段 ≤ 2,666 字；兩行 `Error:` 那份含後一行、不含前一行；另三份頭尾各 4,000 字 | 單元 |
| `t_context_mask_3` | 列 1、一行摘要 | 5,000 行、每行固定 20 字（共 100,000 字），第 3,000 行是 `ERROR`、結束碼 1 → 有第 1 行、第 5,000 行、第 3,000 行；省略字數＝100,000 減留下的原文字數；有原文編號；一行摘要含 `exit=1` | 單元 |
| `error_line_ignores_success_and_missing_exit` | 列 3、6 | `None` | 單元 |

7. **不准做的**：不改上限的數字（K114）；不加樣式、不做大小寫不分；不問模型。
8. **驗收清單**：
   1. `find_error_line` 和 8 個樣式照第 4 節的名字存在。
   2. 執行端建 `StepRecord` 時呼叫它，原本傳 `error_at` 的地方都改掉。
   3. 上面三個測試存在且綠；把任一個樣式從陣列拿掉，`t_context_mask_6` 會紅。
   4. `make check-fast` 過。

### B2 純函式列收尾（一）

1. **目標**：把 `build-2`、`budget-2`、`manifest-1`、`must-1` 四列的現有描述性性質測試補到「整欄預期都驗到、1,000 組」，改名成編號。
2. **開工條件**：`dev`；`docs/tasks/M7b-review-fixes-2.md` 讀過（為什麼放回基準檔）。
3. **只准改**：`crates/context/src/build2_run_tests.rs`、`budget_prop_tests.rs`、`manifest_run_tests.rs`、`build_tests.rs`、測試用的產生器（`fixtures.rs`、`compile_tests_support.rs`）。正式程式碼只有在測試抓到真的錯時才改，並寫進回報。
4. **定好的介面**：不新增公開介面。每個性質測試 `ProptestConfig::with_cases(1000)`，放進 nextest 的 `prop` 組。
5. **行為表**（每列逐條對預期欄）：

| 編號 | 要補的預期 | 出處 |
|---|---|---|
| build-2 | 產生器要會產生「快照更新」（釘選版本換掉）；三種情況以外，沒有任何一次前段不同 | `context-build` S3、S4 |
| budget-2 | 把可以放的一步一步調小：先收舊結果，再是任務以外的舊對話，再是記憶；`context-must` 到最後都在；最近 2 步原文都在 | `context-budget` S2、S3 |
| manifest-1 | 「放了」⇔請求裡的塊（雙向）；`context-must` 每一項都在「放了」；沒放的都有原因；各塊估算加總＝總數 | `context-manifest` S1、S2 |
| must-1 | 可讀長度 2,000–200,000；結果只有兩種：全部原文（或 S2 完整參照，只限你的話）或不送、卡住「上下文預算不夠」 | `context-must` S1–S3 |

6. **要寫的測試**：`t_context_build_2`、`t_context_budget_2`、`t_context_manifest_1`、`t_context_must_1`（都是性質，各 1,000 組）。預期值是規則本身（例如「最近 2 步的原文位元組在請求裡」），不准呼叫被測函式算預期。
7. **不准做的**：不縮小產生器範圍讓它變綠；不改 K114 的數字。
8. **驗收清單**：
   1. 四個測試名稱照第 6 節，各 1,000 組。
   2. 回報逐列列出「原本缺哪條預期、補在哪一行」。
   3. 把 `budget` 少放順序裡的「舊對話」和「記憶」對調，`t_context_budget_2` 會紅。
   4. `make check-fast` 過。

### B3 純函式列收尾（二）

1. **目標**：同 B2，收 `mask-2`、`state-1`、`state-4`。
2. **開工條件**：B1 已合進 `dev`（`mask-2` 碰到 `error_at` 的型別）。
3. **只准改**：`crates/context/src/mask_prop_tests.rs`、`mask_accum_tests.rs`、`state_tests.rs`、產生器。
4. **定好的介面**：不新增。`state-1` 的事件產生器要產生改檔事件（路徑和雜湊），長度上限從 24 提高到能涵蓋 K114 ⑦ 縮短的情況。
5. **行為表**：

| 編號 | 要補的預期 | 出處 |
|---|---|---|
| mask-2 | 每個工具要求對得到結果；你的話、Bot 的回答一字不差；沒確認的對外動作、結果不知道的、600 字以下的沒被收；最近 2 步原文都在；兩次累積收起之間新累積 ≥ 可以放的 15% | `context-mask` S3–S5、K114 ③ |
| state-1 | 算兩次一樣；每一項對得到事件編號；改過的檔和雜湊跟事件裡的一致 | `context-state` S1、S2、S4 |
| state-4 | 300 步、280 做完：做完的只寫數量 280；沒做完的 20 個全列；目前步驟、還沒解決的問題完整；整份 ≤ 2,000 token | `context-state` S5、K114 ⑦ |

6. **要寫的測試**：`t_context_mask_2`（性質 1,000）、`t_context_state_1`（性質 1,000）、`t_context_state_4`（單元：數字 280、20 寫死）。
7. **不准做的**：同 B2。
8. **驗收清單**：
   1. 三個測試照名稱存在、組數對。
   2. 把「短的不收」的 600 改成 0，`t_context_mask_2` 會紅；把雜湊從摘要拿掉，`t_context_state_1` 會紅。
   3. `make check-fast` 過。

### B4 固定段六塊和語言

1. **目標**：系統提示的固定段照 `context-fixed` S1 補齊第 3 塊（安全的指示）和第 4 塊（任務和證據的指示），六塊順序固定、空的不出現、語言跟著使用者。
2. **開工條件**：`dev` 上有 `crates/runner/src/fixed_blocks.rs`（現在只有身分、指示、電腦三塊）。**信任區**提供第 3 塊的內容來源（`safety-n6` S2、`safety-n7` S1、`safety-reject` S4），**回合區**提供第 4 塊（`task-bot` S2、`report-delivery` S3）；兩塊的字要先在那兩區的規劃裡定下，這片只負責排。
3. **只准改**：`fixed_blocks.rs`、`locales/en.ftl`、`locales/zh-TW.ftl`（往後加）、`crates/messages` 重產的代碼；測試 `crates/runner/tests/turn_fixed_blocks.rs`。
4. **定好的介面**：代碼 `turn-fixed-safety`、`turn-fixed-task-evidence`（`turn-` 開頭，K119 ⑯）；`system_prompt` 依序排：身分（含指示）→ 電腦 → 安全 → 任務和證據 → 工具說明 → 釘選和「關於你」（B24 才有內容）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 語言設成繁體中文 | 固定段＝同一組代碼照 zh-TW 排出的字 | `context-fixed` S4 |
| 2 | 語言沒設定 | 照英文排 | S4、K92 |
| 3 | 同一個使用者再送一次 | 固定段位元組一樣 | S4、`context-build` S2 |
| 4 | 沒有釘選 | 第 6 塊不出現，其他順序不變 | S3 |
| 5 | 身分塊 | 只有時區，沒有現在幾點 | S1、`chat-n15` S3 |

6. **要寫的測試**：`t_context_fixed_2`（整合，執行端層：兩個發起者，一個 zh-TW、一個沒設定，各送「hi」；預期＝測試自己用 `Catalog` 照同一串代碼排出的字；第三次送位元組一樣）；`fixed_blocks_keep_order_without_pinned`（描述性，列 4）。
7. **不准做的**：不自己寫安全規則和報告規則的內容；不把現在時間放進固定段；不碰第 6 塊（B24）。
8. **驗收清單**：
   1. 兩個新代碼在 en、zh-TW 都有。
   2. 固定段的順序照第 4 節；`t_context_fixed_2` 綠。
   3. 把安全塊和任務塊對調，測試會紅。
   4. `make check-fast` 過。

### B5 執行端：重組、收起、少放的節奏

1. **目標**：五個要從執行端外面打的上下文列，用 `runner` 的公開介面、真的 HTTP 假模型、假電腦端驗到（K119 ⑰）。
2. **開工條件**：B1 已合；M7f 的「接續」已在 `dev`（`build-3`、`mask-5` 要用「同一份紀錄、新的執行端實例、接著做」來重組）。沒有接續的話，這兩列留到有了再收，其他三列照做。
3. **只准改**：`crates/runner/tests/turn_context_*.rs`（新）、`crates/runner-test`（或現有測試輔助）裡建立「沒有清單和快取的新執行端」的輔助函式。正式程式碼只修測試抓到的錯。
4. **定好的介面**：不加入口、不加旗標。清單存在 `raw` 裡（`step.rs` 的 `runner.raw.put`），屬衍生資料：測試用「新的 `raw` 存放＋同一份紀錄」代表「清掉清單和快取」。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| mask-4 | `read_file` 同一個檔兩次，內容沒變 | 第二次是「跟第 N 步的結果一樣」加編號；內容在請求裡只出現一次 | `context-mask` S6、K114 ⑭ |
| mask-5 | 發生過 2 次收起 | 任務事件有 2 筆收起，各記收了哪幾筆和原因；清快取重組，位元組一樣 | S7 |
| build-3 | 跑了 40 步 | 下一步的請求位元組，跟清掉清單、快照、快取後再組的一樣；紀錄流和記憶的筆數、內容沒變 | `context-build` S1 |
| budget-5 | 可讀長度 16,000；每步 1,500 字結果；40 步 | 每次少放之後，至少 3 步不用再少放 | `context-budget` S2、K114 ④ ⑫ |
| manifest-3 | 假模型回輸入用量 1,234 | 那一步的清單有估算、實際 1,234、兩者的差 | `context-manifest` S1 |

6. **要寫的測試**：`t_context_mask_4`、`t_context_mask_5`、`t_context_build_3`、`t_context_budget_5`、`t_context_manifest_3`（全部整合，執行端層）。`budget-5` 預期寫死：相鄰兩筆「因為放不下而少放」的事件之間，「組了上下文」至少 3 筆。
7. **不准做的**：不為測試在正式程式加「只組不送」的入口；不改 K114 數字；不用固定時鐘遮住真時間造成的錯。
8. **驗收清單**：
   1. 五個測試存在，都打真的假模型 HTTP。
   2. `build-3` 比對的是位元組（不是 JSON 等價）。
   3. 把「重複內容只放一次」拿掉，`t_context_mask_4` 會紅。
   4. `make check-fast` 過。

### B6 /context 和 /compact

1. **目標**：使用者打 /context 拿到最近一次組上下文的摘要；打 /compact 現在就收起最近 10 步以外的舊工具結果。
2. **開工條件**：B5 已合；**回合區**定好斜線指令從哪裡進執行端（信箱收到的文字由誰判斷是指令），這片接在那個入口後面。
3. **只准改**：`crates/runner/src/` 新增 `context_command.rs`（或回合區指定的位置）、`locales/*.ftl`、messages 代碼；測試 `crates/runner/tests/turn_compact.rs`（新）。
4. **定好的介面**：
   - 回覆代碼：`chat-context-summary`（參數 `estimate`、`usable`、`percent`、`collapsed`、`memories`）、`chat-compact-busy`、`chat-compact-none`、`chat-compact-done`（參數 `count`）。
   - /compact 寫一筆收起事件，原因「compact」（`domain` 現有的收起事件種類），收起範圍照 `context-mask` S3 第三點：不看累積門檻。
   - /compact 作用在哪一件任務：待 **Q-27**（建議：這個 Bot 最近一件任務）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | Bot 工作中打 /compact | `chat-compact-busy`，不寫事件 | `chat-compact` S4 |
| 2 | 對話裡沒有工具結果 | `chat-compact-none` | S4 |
| 3 | 跑過 30 步、每步一筆 700 字 | `chat-compact-done`，`count`＝20 | S4、K114 ① ③ |
| 4 | 打 /context | 回 `chat-context-summary`；`percent` 照估算 ÷ 可以放、四捨五入成整數 | S3 |
| 5 | 任何次數的自動收起和 /compact | 紀錄流裡聊天的訊息數和內容不變 | 永遠成立 |
| 6 | 任何時候 | 不叫模型寫摘要 | S2 |

6. **要寫的測試**：`t_chat_compact_8`（整合：三種情況依序回 busy、none、done(20)）；`t_chat_compact_10`（性質：隨機對話，隨機穿插自動收起和 /compact，比對前後聊天紀錄）；`context_summary_reports_rounded_percent`（描述性：32,000 ÷ 120,000 → 27，`T-chat-compact-7` 要等 B23 才有「帶了 2 筆記憶」）。
7. **不准做的**：不自動整理成摘要；/compact 不動聊天紀錄和原文；不在這片做 `memories` 的真值（B23）。
8. **驗收清單**：
   1. 四個代碼在 en、zh-TW 都有。
   2. 三個測試綠；把「不看累積門檻」改回看門檻，`t_chat_compact_8` 會紅。
   3. 回報寫 Q-27 用了哪個答案。
   4. `make check-fast` 過。

### B7 舊對話、長原話、補充

1. **目標**：執行端把「不屬於這件任務的舊對話」真的餵給編譯器；長原話變參照、Bot 讀得回來；任務中途的補充、糾正 80 步後仍一字不差。
2. **開工條件**：B5 已合；工具區的 `read_result` 在 `dev`；M7f 插話在 `dev`（只有 `must-5` 要）。
3. **只准改**：`crates/runner/src/context_input.rs`（`prior_chat`、`user_turns`）、讀聊天紀錄的那一處；測試 `crates/runner/tests/turn_context_chat.rs`（新）。
4. **定好的介面**：`prior_chat` 取這個 Bot 紀錄流裡、不屬於目前任務的聊天紀錄，照時間先後；編譯器怎麼少放已在 `crates/context`（K114 ④ ⑤）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| budget-4 | 400 則無關舊對話；新任務、可讀長度 16,000 | 有最近的舊對話原文，和一塊「較早的對話」參照；清單記著少放了哪幾則、原因「預算」 | `context-budget` S2、S5 |
| must-4 | 可讀長度 32,000；送一句 50,000 字開任務 | 那句是參照：開頭 1,000 字、總長度、編號；假模型 `read_result` 讀得到後面；清單記著換成參照 | `context-must` S2、K114 ⑤ |
| must-5 | 一件任務你先後說了 12 句，之後 80 步 | 第 81 步的請求 12 句都在、一字不差 | `context-must` S1 |

6. **要寫的測試**：`t_context_budget_4`、`t_context_must_4`、`t_context_must_5`（整合，執行端層）。`must-4` 的開頭寫死成原文前 1,000 字。
7. **不准做的**：不讓舊對話擠掉 `context-must`；不在這片做記憶。
8. **驗收清單**：
   1. `context_input.rs` 不再傳空的 `prior_chat`。
   2. 三個測試綠；把參照的開頭改成 500 字，`t_context_must_4` 會紅。
   3. `make check-fast` 過。

### B8 換模型時的上下文

1. **目標**：做到一半換模型，下一個請求照新模型的大小重新決定收起哪些（K114 ⑪），預算不歸零；卡在「上下文預算不夠」換大模型後接著做。
2. **開工條件**：B5 已合；M7g（換模型的事件和時機）、M7h（做完的標準）在 `dev`；`must-2` 的卡住狀態由 B11 驗，但這片的 `must-3` 用同一個前提。
3. **只准改**：`crates/runner/tests/turn_context_swap.rs`（新）；換模型後組上下文的那一處若有錯才改。
4. **定好的介面**：沿用 M7g 的換模型事件和 `crates/context` 的「取代之前的」收起事件。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| budget-3 | 30 步、128,000 → 換 16,000 跑一步 → 換回 128,000 跑一步 | 換小後收起更多、`context-must` 都在、估算加邊際 ≤ 新模型可以放的；預算用量沒歸零；換回後最近 10 步原文都在 | `context-budget` S4、K114 ⑪ |
| must-3 | 4,000 卡住後換成 32,000 | 從停下的那一步接著做；第一個請求有完整的標準（6,000 token） | `context-must` S4 |

6. **要寫的測試**：`t_context_budget_3`、`t_context_must_3`（整合）。
7. **不准做的**：不重算任務預算（K104）；不在步驟中間換。
8. **驗收清單**：
   1. 兩個測試綠。
   2. 把「換模型後從頭算」改成沿用舊的收起範圍，`t_context_budget_3` 會紅。
   3. `make check-fast` 過。

### B9 對話連續、原生區塊、模型的宣稱

1. **目標**：同一段聊天的第二則訊息，請求開頭跟第一則逐字一樣；原生內容區塊只原樣送回同一家同一個模型；模型自己說「完成了」不進狀態摘要。
2. **開工條件**：B7 已合（舊對話進請求）；M7h 在 `dev`；模型區已能存下原生區塊（`models-normalize` S7）。
3. **只准改**：`crates/runner/src/context_input.rs`、`crates/context`（訊息附送出時間、10 分鐘的「現在時間」）；測試 `crates/runner/tests/turn_chat_n15.rs`（新）。
4. **定好的介面**：你的訊息送出時附「送出時間」；距你最後一則超過 10 分鐘才回覆時，最後一則多附「現在時間」（`chat-n15` S1、S2）。字用訊息代碼，例如 `turn-sent-at`、`turn-now`（名字由這片定，寫進回報）。這些附加不寫回聊天紀錄。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| n15-3 | 送「A」；送「B」 | 第二個請求的開頭（系統說明加「A」那一輪）跟第一個逐字一樣；系統說明沒有時刻 | `chat-n15` S3 |
| n15-4 | 假模型扮 Claude，回帶簽章的思考區塊；再送「B」 | 第二個請求裡那個區塊跟收到的逐位元相同 | S4、K102、K109 ② |
| state-3 | 第 5 步回覆寫「測試都過了，任務完成」，沒跑過測試 | 第 6 步的狀態摘要沒有那句；做完的標準仍是還沒驗到 | `context-state` 永遠成立 |
| 補 | 超過 10 分鐘才回覆 | 多附「現在時間」；5 分鐘不附 | S2（`T-chat-n15-2` 的按鈕還沒有，先描述性） |

6. **要寫的測試**：`t_chat_n15_3`、`t_chat_n15_4`、`t_context_state_3`（整合）；`now_time_appears_only_after_ten_minutes`（描述性）。
7. **不准做的**：送出時間不放系統說明；換模型時不送原生區塊（`models-handoff` S3）；不把附加寫進聊天。
8. **驗收清單**：
   1. 三個編號測試綠、一個描述性測試綠。
   2. 把送出時間搬進系統說明，`t_chat_n15_3` 會紅。
   3. `make check-fast` 過。

### B10 token 估算的分詞器

1. **目標**：`TokenizerKind::O200k` 和 `TokenizerKind::TokenizerJson` 真的用分詞器算，不再都走公式（K107、K112 ⑨）。
2. **開工條件**：B13 的套件結果（`tiktoken-rs`、`tokenizers` 的授權、依賴、RustSec）已寫進探路報告；**Q-21** 已答（參考值從哪來、tokenizer.json 從哪來）。
3. **只准改**：`crates/models/src/estimate.rs`（可拆檔）、`crates/models/Cargo.toml`、根目錄 `Cargo.toml` 的依賴、`crates/models/tests/fixtures/`（小的 tokenizer.json 測試檔）。
4. **定好的介面**：`estimate_tokens(kind: &TokenizerKind, text: &str, tokenizer_json: Option<&[u8]>) -> Result<u64, EstimateError>`（實際名字沿用 `estimate.rs` 現有的入口，只加參數）；tokenizer.json 由呼叫的人以位元組傳進來（`models` 不准讀檔，`07` §3）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | OpenAI 系、內建表標 o200k | 用 o200k 算 | `models-capabilities` S5 |
| 2 | 本機模型給了 tokenizer.json | 用它算 | S5 |
| 3 | 其他 | 照 S5 公式 | S5 |
| 4 | tokenizer.json 壞掉 | 回錯誤（不悄悄改用公式） | `AGENTS.md`「算不出來的值要回錯誤」 |

6. **要寫的測試**：`t_models_capabilities_6`（單元：一段固定英文、一段固定中文（Unicode 跳脫）；三種算法各對一個寫死的參考值，參考值照 Q-21 的答案）；`broken_tokenizer_json_is_an_error`。
7. **不准做的**：參考值不准用被測的函式算出來；不在 `models` 裡讀檔或下載。
8. **驗收清單**：
   1. 兩個新依賴過 `make depcheck`（cargo-deny、machete）。
   2. 兩個測試綠；把 o200k 換成公式，`t_models_capabilities_6` 會紅。
   3. `make check-fast` 過。

### B11 接上伺服器才驗得到的上下文列

1. **目標**：三列預期裡有「畫面」，等 M7j 接上伺服器後，從伺服器外面驗。
2. **開工條件**：M7j 在 `dev`；B5、B8 已合；**Q-28**（整合列裡的「畫面」接上伺服器後怎麼算）已答。
3. **只准改**：`crates/server/tests/` 或監督者指定的整合測試位置（新檔）。
4. **定好的介面**：不新增；用前端會打的 API 和 SSE。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| mask-1 | 32,000；每步 700 字；跑到 10 步以外累積超過 15% | 下一個請求 10 步以外只剩一行摘要和編號；最近 10 步原文都在；畫面上每一步的結果完整 | `context-mask` S3、S8 |
| must-2 | 4,000 的假模型；標準 6,000 token | 假模型沒收到請求；任務卡住「上下文預算不夠」；畫面寫缺多少和兩個做法 | `context-must` S3、K114 ⑬ |
| compact-11 | 32,000；200 步、每步 700 字 | 假模型收到 201 個請求；沒有要它寫摘要的；畫面沒有「整理成摘要」 | `chat-compact` S2 |

6. **要寫的測試**：`t_context_mask_1`、`t_context_must_2`、`t_chat_compact_11`（整合，從伺服器外面打）。
7. **不准做的**：不為測試加輸出模式；「畫面」不准用 runner 內部狀態代替。
8. **驗收清單**：
   1. 三個測試照 Q-28 的方式驗到「畫面」那一欄。
   2. `must-2` 的「缺多少」寫死成標準 6,000 加系統提示、工具說明減 4,000 可以放的（K114 ⑬），由測試自己用估算規則的寫死數字算，不呼叫被測函式。
   3. `make check-fast` 過。

### B12 照編號讀回原文

1. **目標**：被收起、被省略、換成參照的內容都用 `read_result` 讀得回來；被引用的原文過了 60 天還在；永久刪除後回「來源已永久刪除」。
2. **開工條件**：工具區的 `read_result`（含 `find`，`files-n1` S3、S5）和 K103 的保存規則在 `dev`；M7j 在 `dev`（`ref-3` 的永久刪除是伺服器動作）。
3. **只准改**：`crates/context/src/read.rs`（純邏輯已有）、執行端接 `read_result` 的那一處；測試 `crates/runner/tests/turn_context_ref.rs`（新）或伺服器整合測試。
4. **定好的介面**：沿用工具區的 `read_result(id, from, chars, find)`；永久刪除的回覆代碼沿用 `sync-append` S6 的代碼（`read.rs` 已有「deleted」分支）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| ref-1 | 50,000 字結果已收起，第 31,000 字附近有「ERROR 42」 | `find` 回位置和前後文；`from=30000, chars=2000` 回「第 30,000–32,000 個字，共 50,000 個字」 | `context-ref` S1、`files-n1` S3、S5 |
| ref-2 | 進行中的任務有一筆被收起的結果；快轉 60 天 | 讀得到 | S3、K103 |
| ref-3 | 你的一則長訊息在請求裡是參照；永久刪除那段聊天 | 回「來源已永久刪除」 | S4、K113 ② |

6. **要寫的測試**：`t_context_ref_1`、`t_context_ref_2`、`t_context_ref_3`（整合）。數字照上表寫死。
7. **不准做的**：不自己做 `read_result` 工具（工具區做）；不改保存天數。
8. **驗收清單**：
   1. 三個測試綠。
   2. 把「被引用就留」拿掉，`t_context_ref_2` 會紅。
   3. `make check-fast` 過。

### B13 探路：切詞套件、FTS5 trigram、pg_trgm／pgvector、字元分類、分詞器套件

1. **目標**：先把會卡住 B10、B14、B17、B18 的未知數查清楚，寫成一份探路報告，加兩個留下來當守門的小測試。不寫產品程式。
2. **開工條件**：`dev`。可以跟 M7 的任何一片並行。
3. **只准改**：`docs/tasks/M7.5-probe-report.md`（新）；`tests/harness` 或 `crates/store-*/tests/` 裡的守門測試（新檔）；試驗用的暫時 crate 放在自己工作區的 `target/probe/`，不進 workspace、不 commit。
4. **要查的事和怎麼查**：

| # | 問題 | 怎麼查 | 離線已知 |
|---|---|---|---|
| 1 | sqlx 內建的 SQLite 有沒有 FTS5 和 trigram 分詞 | 守門測試：`select sqlite_version()`；`CREATE VIRTUAL TABLE … USING fts5(x, tokenize='trigram')`，查 3 字的路徑片段和中文片段；`fts5(x, tokenize='unicode61')` 存空白隔開的切好的字，`bm25()` 排序 | `libsqlite3-sys 0.40.0`（已在 lock）的 `build.rs` 有 `-DSQLITE_ENABLE_FTS5`，內建 SQLite 3.54.1（trigram 要 ≥ 3.34） |
| 2 | 測試用 PostgreSQL（`postgresql_embedded 0.22` theseus，`=17.6.0`）有沒有 `pg_trgm`、`vector`、`pg_bigm` | 守門測試：`select name from pg_available_extensions where name in ('pg_trgm','vector','pg_bigm')`；`CREATE EXTENSION pg_trgm` | 不知道；pgvector 不是 PostgreSQL 內附的，很可能沒有 |
| 3 | 測試資料庫的字元分類 | `SHOW lc_ctype`、`select datctype from pg_database`；`to_tsvector('simple', '<CJK 字以空白隔開> ERR_42 a/b/c')` 的詞位有沒有中文；`gin_trgm_ops` 對中文片段 `LIKE '%…%'` 有沒有用到索引（`EXPLAIN`） | `initdb` 只設了 `encoding("UTF8")`，沒設 locale，跟著執行環境；K107 要求用 UTF-8 的字元分類建立 → 可能要在 `create_database` 用 `TEMPLATE template0 LC_CTYPE …` |
| 4 | jieba-rs 能不能用 | 暫時 crate 編一次（macOS 本機、雲端三系統）；`cargo deny check`（授權、RustSec）；記下編譯時間、執行檔增加多少；記下幾個固定句子的 `cut` 結果（B14 的預期值要用） | `jieba-rs 0.11.0`（MIT，github.com/messense/jieba-rs，edition 2024）；預設功能 `default-dict` 拉 `include-flate 0.3.5`（Apache-2.0，SOFe）→ `libflate 2.3.2`、`zstd 0.14.3`／`zstd-sys 2.1.2+zstd.1.5.7`（C）、`proc-macro-error3 3.1.1`；還有 `jieba-macros 0.11.0`、`ordered-float 5`、`rustc-hash 2`、`bytecount`；辭典約 5 MB 編進去。這些都還不在 `Cargo.lock` |
| 5 | NFKC 用哪個 | 比較 `unicode-normalization 0.1.26`（unicode-rs，(MIT OR Apache-2.0) AND Unicode-3.0）和 `icu_normalizer 2.4.0`（ICU4X，Unicode-3.0）；兩個都已在 lock，`deny.toml` 已允許 Unicode-3.0 | 建議前者（API 簡單），報告寫理由 |
| 6 | `tiktoken-rs`、`tokenizers`（B10 用） | 暫時 crate 試編；記下授權、維護者、依賴樹（`tokenizers` 預設帶 C 的 onig 等，看關掉預設功能行不行）、資料檔大小 | 本機 registry 沒有這兩個套件，全部要上網查 |
| 7 | 每個新套件誰維護、多久沒更新、有沒有 RustSec | crates.io、GitHub、`cargo deny check advisories` | registry 裡檔案的時間被統一成 2006，離線看不出發佈日期 |

5. **報告格式**：每一題寫「結論、證據（指令和輸出）、對哪一片有什麼影響」。0.0.x 版或單一作者的套件另外列出來（`AGENTS.md` 第三節）。
6. **要寫的測試**（守門，描述性名字）：`pg_brain_extensions_report`（列出可用的擴充，`pg_trgm` 不可用就紅，訊息寫清楚）、`sqlite_fts5_trigram_available`。
7. **不准做的**：不改 `deny.toml`、不把套件加進 workspace（B10、B14 才加）；不用 Docker；不下載 PostgreSQL 以外的資料庫程式。
8. **驗收清單**：
   1. 報告七題都有結論和證據。
   2. 兩個守門測試存在且結果跟報告一致。
   3. 報告最後列出「需要使用者決定的」（例如 `pg_trgm` 沒有時改用 `pg_bigm` 還是換測試用的 PostgreSQL 版本）。

### B14 切詞模組

1. **目標**：一個伺服器和客戶端共用的 Rust 切詞模組，照 K107：NFKC 加小寫；中文切單字加重疊兩字詞，再加 jieba-rs 的詞；英數保留完整識別碼，再加拆開的子詞；帶版本號。
2. **開工條件**：B13 報告；**Q-2**（放哪個套件）、**Q-3**（辭典的依賴）、**Q-14**（查詢怎麼組、識別碼怎麼拆）已答。
3. **只准改**：Q-2 答案指定的位置（建議新套件 `crates/terms`；不准的話是 `crates/domain/src/terms*.rs`）；根目錄 `Cargo.toml`；依賴檢查工具若要登記新套件，照 `tools/` 現有寫法加一行。
4. **定好的介面**：
   - `pub const TOKENIZER_VERSION: u32 = 1;`
   - `pub fn index_terms(text: &str) -> Vec<String>`：存進索引的字（去重、照固定順序，同樣輸入同樣輸出）。
   - `pub fn query_terms(text: &str) -> TermQuery`：查詢用的（組法照 Q-14）。
   - `pub fn is_exact_key(token: &str) -> bool`：判斷是不是編號、錯誤碼（排序最前面用，規則照 Q-14）。
   - 純函式，不讀檔、不碰網路；jieba 實例用 `std::sync::LazyLock` 建一次。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 全形英數、相容字 | NFKC 後再小寫 | K107 |
| 2 | 一段中文 | 每個單字、每個重疊兩字詞、jieba 的詞都在 | K107 |
| 3 | 識別碼（例 `ERR_CONN_RESET`） | 完整的一個，加拆開的子詞 | K107 |
| 4 | 繁簡 | 不互通（「臺灣」和「台湾」各自照字面切，不互相轉換） | K107「繁簡互通先不做」 |
| 5 | 空字串、只有標點 | 空的 | 邊界 |
| 6 | 改了切法 | `TOKENIZER_VERSION` 要加一（B19 靠它重建） | K107 |

6. **要寫的測試**（描述性；`spec/10` 還沒有切詞的列，見 Q-1）：`index_terms_cjk_unigrams_bigrams_and_words`（預期寫死：單字、兩字詞照字面列；jieba 的詞照 B13 報告記下的輸出）、`index_terms_identifier_keeps_whole_and_parts`、`index_terms_nfkc_lowercase`、`query_terms_single_cjk_char`、性質測試 `index_terms_is_deterministic`（1,000 組）。
7. **不准做的**：不做繁簡轉換；不用 Tantivy、zhparser、pg_jieba（K107）；不在 `domain` 以外的產品套件各自寫一份切法。
8. **驗收清單**：
   1. 介面照第 4 節；依賴方向檢查過（Q-2 的答案已寫進 `spec/07`，由使用者改，不是 agent 改）。
   2. 五個測試綠；把兩字詞拿掉，第一個測試會紅。
   3. `make depcheck`、`make check-fast` 過。

### B15 記憶事件、投影、准入（純邏輯）

1. **目標**：K106 的記憶事件、從事件算出「現在有哪些記憶」、K97 的准入檢查和寫入前掃描，全部是 `domain` 裡的純函式；伺服器和每台裝置算出一樣的結果。
2. **開工條件**：**Q-7**（種類）、**Q-8**（狀態和升級）、**Q-9**（怎麼認「使用者說的」）、**Q-10**（事件種類補不補）、**Q-11**（掃到東西怎麼辦）、**Q-12**（重複和矛盾）、**Q-13**（候選帶不帶）、**Q-24**（准入在哪算）已答。
3. **只准改**：`crates/domain/src/memory_*.rs`（新，每檔 ≤ 300 行）、`crates/protocol/src/memory_records.rs`（新）、`crates/protocol/src/lib.rs`（往後加）、`crates/context/src/input.rs`（`MemorySource` 改用 `domain` 的型別，不要兩份）。
4. **定好的介面**（名字照寫；欄位照 K106）：
   - `protocol`：`MemoryEventKind`（`Added`、`Superseded`、`Retracted`、`Confirmed`、`Pinned`、`Voided`，加 Q-10 的答案）；紀錄種類字串 `memory_added` 等，放帳號那條紀錄流（`StreamSelector::Account`，第 5 組 `sync-append` 的表）；`MemoryEventContent { memory_id, version, prev_version, kind, content, source, evidence_refs, scope, tags, valid_from_ms, valid_to_ms, replaced_by, reason }`。
   - `domain`：`MemoryState`（候選、有來源支持、已驗證、過時、有矛盾、封存、被取代）、`MemorySource`（使用者說的、工具證據、模型推論、別的 Bot 轉述、匯入）、`MemoryScope { Bot(BotId), AboutYou }`、`MemoryKind`（照 Q-7）。
   - `pub fn project(events: &[MemoryEventRecord]) -> MemoryBook`：照伺服器給的順序重播；任何交錯都接受（比照 K113 ①）。
   - `pub fn admit(proposal: &MemoryProposal, book: &MemoryBook) -> Admission`：格式（內容最多 500 字，多的截掉，`memory-kinds` S1）、來源、重複、矛盾、敏感資料、範圍。
   - `pub fn scan(text: &str) -> Vec<ScanFinding>`：提示注入、金鑰、看不見的字元；樣式清單照 Q-11（金鑰的樣式跟信任區 `task-journal` 遮金鑰的規則共用一份）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 使用者在聊天裡親口說的（照 Q-9 認得出來） | 直接轉正，來源「使用者說的」，連到那句的紀錄 | K106 |
| 2 | 模型推論、別的 Bot 轉述 | 進候選 | K97、K106 |
| 3 | 候選要升級 | 只有使用者確認、或新的工具證據 | K106 |
| 4 | 矛盾 | 兩筆都留、寫原因，不覆蓋；狀態照 Q-12 | K97、K106 |
| 5 | 600 字的內容 | 留前 500 字 | `memory-kinds` S1 |
| 6 | 掃到金鑰／注入／看不見的字 | 照 Q-11 | K106 |
| 7 | 同樣的事件序列 | 算兩次結果一樣 | K106「由事件算出來」 |
| 8 | 被取代、合併 | 舊的留著、記 `replaced_by`（轉址），引用不斷 | K96 |

6. **要寫的測試**（描述性；規格列待 Q-1）：`user_said_memory_is_confirmed`、`model_inference_starts_as_candidate`、`candidate_upgrades_only_by_confirm_or_tool_evidence`、`contradiction_keeps_both_with_reason`、`content_is_cut_to_500_chars`、`scan_rejects_secret_shaped_text`、性質 `projection_is_deterministic_under_any_interleaving`（1,000 組）。
7. **不准做的**：不碰資料庫、檔案、非同步（`07` §3 `domain`）；不自動刪任何記憶（K106）；不寫每晚整理（Q-26）。
8. **驗收清單**：
   1. 型別和函式照第 4 節；`context` 不再有自己的 `MemorySource`。
   2. 七個測試綠；把「候選只能由確認或工具證據升級」拿掉，第三個測試會紅。
   3. 回報逐條寫 Q-7–Q-13、Q-24 用了哪個答案。
   4. `make check-fast` 過。

### B16 伺服器：記憶紀錄、投影表、API

1. **目標**：伺服器收記憶事件（帳號紀錄流）、維護可以重建的投影表，提供記憶的讀、新增、動作 API；只限擁有者。
2. **開工條件**：B15 已合。
3. **只准改**：`migrations/server/0350_memories.sql`–`0354_*`；`crates/store-server/src/memory_*.rs`（新）；`crates/server/src/memory_*.rs`（新）和路由表（往後加）；`crates/protocol/src/memory_api.rs`（新）。
4. **定好的介面**：
   - 表：`memories`（投影：`memory_id`、`version`、`scope_bot_id`（「關於你」為 NULL）、`kind`、`state`、`source`、`content`、`tags`、`valid_from_ms`、`valid_to_ms`、`replaced_by`、`pinned`、`retracted_at_ms`、`account_id`、`source_seq`）；`memory_versions`（每一版，給版本差異和 Markdown 比對）。兩張都是衍生的，正本是 `records`。
   - API（名字照寫，OpenAPI 由監督者重產）：`GET /api/v1/bots/{bot_id}/memories`、`GET /api/v1/memories/about`（篩選：種類、標籤、狀態、範圍、時間範圍；一頁 60 條，`memory-page` S8）、`POST /api/v1/memories`（使用者新增）、`POST /api/v1/memories/{memory_id}/actions`（確認、釘選、取消釘選、忘記、還原、編輯，種類照 Q-10）、`GET /api/v1/memories/{memory_id}/history`、`GET /api/v1/records/{record_id}/learned-memories/count`（給 `sync-append` S6 的再確認畫面）。
   - 使用者的動作是對伺服器的動作，由伺服器寫成事件（比照 K113 ①）；執行端寫的「記下」照 Q-24。
   - 不是擁有者 → 錯誤代碼 `memory-owner-only`（`memory-n5` S4）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 收到記憶事件 | 同一筆交易更新投影 | K106 |
| 2 | 團隊成員或別的帳號讀寫 | `memory-owner-only` | `memory-n5` S4 |
| 3 | 永久刪除一段對話、不勾「一起刪」 | 記憶留著，來源顯示「來源已永久刪除」 | K113 ② |
| 4 | 勾了 | 那筆記憶內容清掉，衍生的索引一起清 | K113 ② |
| 5 | 投影表整個清掉 | 從 `records` 重播得回一樣的 | K98、K106 |

6. **要寫的測試**（`pg_` 開頭，nextest `pg` 組；描述性）：`pg_memory_events_project_into_table`、`pg_memory_owner_only`、`pg_memory_projection_rebuilds_from_records`、`pg_purged_chat_keeps_learned_memory_by_default`。
7. **不准做的**：伺服器不依賴 `runner`、`models`（A1）；不做搜尋（B17）；不做每晚整理。
8. **驗收清單**：
   1. 遷移號在 0350–0354 之內。
   2. 四個測試綠；把擁有者檢查拿掉，第二個會紅。
   3. 回報列出新增的 API 和錯誤代碼。
   4. `make check-fast` 過。

### B17 伺服器搜尋：PG 全文、trgm、排序

1. **目標**：K98、K107 的伺服器搜尋：精確編號、過濾、中英全文（切好的字 + `to_tsvector('simple', …)` + GIN）、三個字以上片段（`pg_trgm`）、精確命中排最前、再用 RRF（k=60）合併字詞和最近使用。
2. **開工條件**：B13（`pg_trgm` 可用、字元分類的做法）、B14、B16 已合；**Q-4**（向量延後）、**Q-14**、**Q-15** 已答。
3. **只准改**：`migrations/server/0355_memory_search.sql`–`0359_*`；`crates/store-server/src/memory_search*.rs`；`crates/server/src/memory_search*.rs`；`crates/protocol/src/memory_api.rs`（往後加）；測試資料庫建立方式若要指定字元分類，改 `tests/harness/src/postgres.rs`（照 B13 結論）。
4. **定好的介面**：
   - 欄位：`memories.terms tsvector`（存 `index_terms` 用空白接起來的結果）、`memories.terms_version int`、GIN 索引；`content` 上 `gin_trgm_ops`；`memory_id` 一般索引。
   - `GET /api/v1/memories/search?q=&scope=&kind=&tag=&state=&from_ms=&to_ms=&limit=`：回 `[{ memory_id, version, content, kind, state, scope, date_ms, label }]`；「關於你」的標 `label = about_shared`（`memory-about` S3）。
   - 排序：精確命中的編號、錯誤碼（`is_exact_key`）最前；其餘 RRF k=60 合併字詞名次和最近使用名次；候選 90 天沒被用到也沒人確認的排到後面（K106）；不做重排、不做向量（Q-4）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 查一個記憶編號 | 只回那一筆，第一名 | K98 |
| 2 | 查錯誤碼 `E1234` | 含它的排最前 | K107 |
| 3 | 查一兩個字的中文詞 | 找得到（單字、兩字詞索引） | K98、K107 |
| 4 | 查路徑片段 `log/app` | 用 trgm 找得到 | K107 |
| 5 | 過濾種類、狀態、範圍、時間 | 只回符合的 | K98 |
| 6 | 任何查詢 | `EXPLAIN` 不出現對 `memories` 的 Seq Scan | K98「搜尋不掃全部記憶」 |
| 7 | 別的 Bot 的記憶 | 不回 | `memory-kinds` S2 |

6. **要寫的測試**（`pg_` 開頭）：`pg_search_exact_id_first`、`pg_search_error_code_first`、`pg_search_short_cjk_word`、`pg_search_path_fragment_trgm`、`pg_search_filters`、`pg_search_uses_indexes`、`pg_search_rrf_k60_order`（三筆寫死的名次，手算 RRF 值：1/(60+r)）。
7. **不准做的**：不用 zhparser、pg_jieba、ParadeDB；不做繁簡；不加向量欄位（Q-4）。
8. **驗收清單**：
   1. 遷移在 0355–0359。
   2. 七個測試綠；把精確命中的優先拿掉，`pg_search_error_code_first` 會紅。
   3. `make check-fast` 過。

### B18 客戶端副本索引：SQLite FTS5

1. **目標**：客戶端（桌面 App、執行端共用的 `store-client`）把本機副本裡的記憶事件投影成表，建 FTS5（切好的字）和一張 trigram 表，伺服器連不上時只比字詞（K107）。
2. **開工條件**：B14、B15 已合；B13 確認 FTS5 trigram 可用。
3. **只准改**：`migrations/client/0040_memory_replica.sql`–`0044_*`；`crates/store-client/src/memory_*.rs`。
4. **定好的介面**：`memory_rows`（投影）、`memory_terms`（`fts5(terms, tokenize='unicode61')`，存 `index_terms` 的結果）、`memory_trigram`（`fts5(content, tokenize='trigram')`）、`memory_index_meta(tokenizer_version)`；`pub async fn search_local(&self, query: &LocalMemoryQuery) -> Result<Vec<LocalMemoryHit>, StoreError>`，排序規則跟 B17 一樣（精確命中最前，再 RRF k=60），`bm25()` 當字詞名次。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 副本收到記憶事件 | 投影和兩張索引表一起更新 | `02` §3.1 |
| 2 | 查一兩個字的中文 | 找得到 | K98 |
| 3 | 查三個字以上片段 | trigram 找得到 | K107 |
| 4 | 同樣的資料、同樣的查詢 | 前幾名跟伺服器一致（同一套排序規則） | K107「兩端共用」 |

6. **要寫的測試**（描述性）：`local_memory_projection_follows_replica`、`local_search_short_cjk_word`、`local_search_trigram_fragment`、`local_search_exact_key_first`。
7. **不准做的**：不做向量（K107「向量只在伺服器」）；不另外寫一份切法。
8. **驗收清單**：
   1. 遷移在 client 0040–0044。
   2. 四個測試綠。
   3. `make check-fast` 過。

### B19 索引重建和一致性檢查

1. **目標**：兩端的記憶索引都可以整個刪掉、從正本重建，重建後檢查編號和版本一致；切詞版本變了自動重建（K98、K107）。
2. **開工條件**：B17、B18 已合；**Q-18**（誰觸發、要不要命令列）已答。
3. **只准改**：`migrations/server/0360_memory_index_meta.sql`–`0364_*`；`migrations/client/0045_*`–`0049_*`（若要）；`crates/store-server/src/memory_reindex*.rs`、`crates/store-client/src/memory_reindex*.rs`；Q-18 答要命令列時，`apps/server` 的子命令。
4. **定好的介面**：`pub async fn rebuild_memory_index(&self) -> Result<ReindexReport, StoreError>`；`pub async fn check_memory_index(&self) -> Result<IndexCheck, StoreError>`（回缺的、多的、版本不一致的編號）；`memory_index_meta(tokenizer_version, rebuilt_at_ms)`。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 刪掉索引再重建 | 同一組查詢回一樣的結果 | K98 |
| 2 | 索引少一筆、多一筆、版本舊一筆 | 檢查回報三種各一個編號 | K98 |
| 3 | 存的切詞版本 ≠ 程式的 | 照 Q-18 觸發重建 | K107 |
| 4 | 重建中途失敗 | 回錯誤；精確編號查詢照常（只查投影） | K98「向量、重排壞掉時精確和全文照常」的精神，細節待 Q-18 |

6. **要寫的測試**：`pg_memory_index_rebuild_matches`、`pg_memory_index_check_reports_drift`、`local_memory_index_rebuild_matches`、`tokenizer_version_change_triggers_rebuild`。
7. **不准做的**：重建不改 `records`；不刪記憶。
8. **驗收清單**：
   1. 四個測試綠；把版本比對拿掉，最後一個會紅。
   2. `make check-fast` 過。

### B20 10 萬筆效能關卡

1. **目標**：K98、K107 的效能關卡：10 萬筆中英混合的記憶，精確編號和全文查詢 p95 ≤ 100 毫秒，SQLite 和 PostgreSQL 各一組，進 CI。
2. **開工條件**：B17、B18 已合；**Q-17**（查詢數、暖機、資料產生器的種子、CI 機器的變異怎麼處理）已答。
3. **只准改**：`tests/harness/src/brain_corpus.rs`（新，產生測試資料，測試輔助套件可以寫 `target/`）；`crates/store-server/tests/pg_brain_perf.rs`、`crates/store-client/tests/brain_perf.rs`（新）；`.config/nextest.toml`（放進 `timing` 組）；`Makefile` 若要新目標。
4. **定好的介面**：`brain_corpus::generate(seed: u64, rows: usize) -> Vec<CorpusRow>`；資料要有（K98）：中英混合、錯誤碼、路徑、一兩個字的中文短詞、重複、過時、矛盾、無關的。查詢兩類：精確編號、全文。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | PG，10 萬筆，熱快取 | 精確編號 p95 ≤ 100 ms；全文 p95 ≤ 100 ms | K98、K107 |
| 2 | SQLite，10 萬筆，熱快取 | 同上 | K107 |
| 3 | 資料產生器同一個種子 | 產生一樣的資料 | 可重現 |

6. **要寫的測試**：`pg_brain_perf_100k_p95`、`brain_perf_100k_p95_sqlite`（描述性名字；規格沒有這一列，見 Q-1）。100 ms、100,000 寫死。
7. **不准做的**：不為了過關放寬 100 ms；不縮小筆數；不在本機跑完整 `make check`（交給雲端）。
8. **驗收清單**：
   1. 兩個測試在 `timing` 組、進 `make check`。
   2. 報告附本機一次的 p50、p95、p99。
   3. 雲端檢查全綠。

### B21 記憶工具：記、找

1. **目標**：Bot 有 `remember`、`search_memory` 兩個工具，接到 B16、B17（連得上伺服器時用伺服器搜尋）。
2. **開工條件**：B16、B17 已合；**工具區**的工具清單和分類（`task-journal` S3：`search_memory` 是唯讀）在 `dev`；Q-8、Q-9 已答。
3. **只准改**：`crates/tools` 的工具說明（`remember(fact, about_person)`、`search_memory(query, kind)`）；`crates/runner/src/memory_tools*.rs`（新）；`crates/sync` 的伺服器記憶 API 客戶端；`locales/*.ftl`（`tool-memory-*` 開頭）；測試 `crates/runner/tests/turn_memory_tools.rs` 或伺服器整合測試。
4. **定好的介面**：工具名和參數照 `memory-*` 的列；回覆代碼 `tool-memory-saved`、`tool-memory-results`（每條附日期，`memory-search` S1 沒被重寫的部分）、`tool-memory-none`。`remember` 寫一筆記憶事件到帳號紀錄流（Q-24 決定是「提議」還是直接「新增」）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| kinds-1 | `remember`(fact=600 字) | 記憶清單那條是前 500 字 | `memory-kinds` S1 |
| kinds-2 | A 記了「喜歡貓」；B `search_memory`(query="貓") | 找不到 | S2 |
| search-2 | 呼叫 `search_memory` | 沒有跳卡 | `memory-search` S2 |
| about-3 | `search_memory`(query="女兒") | 那條標「關於你（共用）」 | `memory-about` S3 |
| n1-3 | 「住台北」在最近刪除；`remember`(fact="住台北") | 救回原本那條（同一個編號），條數只多 1 | `memory-n1` S3；救回後的狀態照 Q-8 |
| 補 | 不受信任的一輪 | 工具清單沒有記憶工具 | `memory-n5` S1（列在之後驗） |

6. **要寫的測試**：`t_memory_kinds_1`、`t_memory_kinds_2`、`t_memory_search_2`、`t_memory_about_3`、`t_memory_n1_3`（整合：假模型呼叫工具；「記憶頁那條」用頁面會打的 `GET …/memories` 驗，`10-behavior/README.md`：整合＝打前端會用的動作）；`untrusted_turn_has_no_memory_tools`（描述性）。
7. **不准做的**：不做釘選工具（`memory-pin` S2）；不做 `forget`（B22）；不做向量。
8. **驗收清單**：
   1. 五個編號測試、一個描述性測試綠。
   2. 把 500 字截斷拿掉，`t_memory_kinds_1` 會紅。
   3. 新代碼在 en、zh-TW 都有。
   4. `make check-fast` 過。

### B22 忘記和還原

1. **目標**：`forget` 照 `memory-forget` S1–S4（對到 1 條才刪、2 條以上回最多 10 個候選、自己找不到才找「關於你」、做過的事動不到）；最近刪除 30 天內可以還原。
2. **開工條件**：B21 已合；Q-10（還原算哪種事件）已答。
3. **只准改**：`crates/runner/src/memory_tools*.rs`、`crates/server/src/memory_*.rs`（還原動作）、`locales/*.ftl`；測試。
4. **定好的介面**：`forget(what)`；回覆代碼 `tool-memory-forgot`、`tool-memory-forget-candidates`（參數：最多 10 條、各附全文開頭）、`tool-memory-not-found`；還原是 B16 的 `actions` 端點；忘記＝`Retracted` 事件，不用核准。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| forget-1 | 「住台北」；`forget`("台北")；快轉 29 天按還原 | 進最近刪除、不再帶；還原後回來；沒有跳卡 | S1 |
| forget-2 | 「住台北」「台北的公司」；`forget`("台北") | 兩條都在；模型收到 2 個候選 | S2、R29 |
| forget-3 | 自己沒有「小美」，「關於你」有 | 「關於你」那條進最近刪除 | S3 |
| forget-4 | 只有一條做過的事含「報表」 | 什麼都沒刪，回找不到 | S4 |
| about-4 | 自己沒有含「小美」的；`forget`("小美") | 「關於你」那條進最近刪除 | `memory-about` S4 |

6. **要寫的測試**：`t_memory_forget_1`–`t_memory_forget_4`、`t_memory_about_4`（整合；「不再帶」看下一個模型請求；「按還原」打 `actions` 端點；29 天用假時鐘快轉）。
7. **不准做的**：不猜第一條（R29）；30 天後的真正刪除照 K113 ② 只清內容，這片不做排程（Q-26）。
8. **驗收清單**：
   1. 五個測試綠；把「2 條以上不刪」拿掉，`t_memory_forget_2` 會紅。
   2. `make check-fast` 過。

### B23 每一步重新找、帶進上下文

1. **目標**：執行端每一步問模型前，照目前步驟重新找記憶（K97、`memory-carry` S4），放進第三段，最多佔可以放的 20%、放不下最少留 3 筆（K114 ⑥）；使用者勾的記憶送出時定下；「關於你」帶最新 40 條。
2. **開工條件**：B21、B6 已合；**回合區**提供 `BrainSource` 接口的插座（見 E 節）；Q-12、Q-13、Q-23、Q-25 已答。
3. **只准改**：`crates/runner/src/brain_source*.rs`（新）、`context_input.rs`（`memories`）、`crates/context`（候選和推論的標記、矛盾的標記）、`crates/protocol`（送訊息帶 `memory_ids`）、伺服器收訊息的那一處（存勾選）；測試。
4. **定好的介面**：
   - `pub trait BrainSource { async fn retrieve(&self, ctx: &RetrieveContext) -> Result<Vec<MemoryRecord>, BrainError>; async fn snapshot(&self, conversation: &ConversationRef) -> Result<PinnedSnapshot, BrainError>; }`，從執行端的建構函式傳進來（真的實作打伺服器 API，B26 加離線退回本機副本）。
   - 送訊息的請求加 `memory_ids: Vec<String>`（最多收前 20 條，`memory-carry` S5）。
   - `MemoryRecord` 加 `marks`：候選、推論、有矛盾（K106、K97）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| carry-5 | 手機送帶 21 條勾選的；再送空清單 | 第一則只帶前 20 條；第二則不包含任何記憶（空清單的意思照 Q-25） | `memory-carry` S5 |
| carry-6 | 「住台北」和矛盾的「上個月搬到高雄」；送「台北有什麼好吃」；送「謝謝」 | 第一則兩筆都帶、標有矛盾、各出現一次；「謝謝」那一輪沒拿這句去找 | S6、K97 |
| carry-7 | 勾 2 條送出；回覆開始後新增一條很相關的 | 勾的 2 條不變；新的出現在下一步 | S7 |
| about-2 | 「關於你」41 條；送「A」 | 帶最新 40 條、不帶最舊的；`search_memory` 找得到最舊的 | `memory-about` S2 |
| compact-7 | 最近一次約 32,000、可以放 120,000、收起 3 筆、帶 2 筆記憶；打 /context | 包含約 32,000、120,000、27%、3、2；沒有「自動整理」 | `chat-compact` S3 |
| 補 | 找回來的加起來 | ≤ 這一步可以放的 20%；放不下最少 3 筆 | K114 ⑥ |

6. **要寫的測試**：`t_memory_carry_5`、`t_memory_carry_6`、`t_memory_carry_7`、`t_memory_about_2`、`t_chat_compact_7`（整合）；`retrieved_memory_stays_under_twenty_percent`、`every_step_retrieves_again`（描述性，`T-memory-carry-4` 要等例行任務）。
7. **不准做的**：程式找的記憶不附在你的訊息後面（`chat-n15` S1）；不記「這段聊天帶過了」；不做小卡片 UI（M8）。
8. **驗收清單**：
   1. 五個編號測試、兩個描述性測試綠。
   2. 把「每一步重新找」改成一輪找一次，`t_memory_carry_7` 會紅。
   3. 回報寫 Q-12、Q-13、Q-23、Q-25 用了哪個答案。
   4. `make check-fast` 過。

### B24 釘選和「關於你」的快照

1. **目標**：釘選的記憶和「關於你」在每段對話開始時凍結成快照，放進固定段第 6 塊（K106、`context-fixed` S1）；Bot 沒有釘選工具。
2. **開工條件**：B23、B4 已合；**回合區**有「開始新對話」的界線（`chat-n1`），並交給 `BrainSource::snapshot`；Q-5 已答。
3. **只准改**：`crates/runner/src/brain_source*.rs`、`fixed_blocks.rs`（第 6 塊）、`crates/context`（快照版本進 `PinnedSnapshot.version`）；測試。
4. **定好的介面**：`PinnedSnapshot { version, items }`（已有）；版本＝凍結當下記憶事件的最後一個紀錄號；第 6 塊的代碼 `turn-fixed-pinned`。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| pin-2 | 看模型收到的工具清單 | 沒有釘選記憶的工具 | `memory-pin` S2 |
| about-1 | A `remember`(fact="女兒叫小美", about_person=true)；對 B 送「A」 | B 的請求包含「女兒叫小美」（候選的話帶標記，K106） | `memory-about` S1 |
| fixed-1 | Bot 的電腦；一筆釘選；送「hi」；拿掉釘選、開始新對話再送 | 第一次固定段依序六塊；第二次少釘選那塊，其他五塊順序不變 | `context-fixed` S1、S3 |
| 補 | 對話中途改釘選 | 這段對話不變，下一段才生效 | K106（`T-memory-pin-1` 衝突見 Q-5） |

6. **要寫的測試**：`t_memory_pin_2`、`t_memory_about_1`、`t_context_fixed_1`（整合）；`pinned_change_waits_for_next_conversation`（描述性）。
7. **不准做的**：Bot 不准有釘選工具；不在對話中途換快照（K106）。
8. **驗收清單**：
   1. 三個編號測試、一個描述性測試綠。
   2. 把快照改成每步重讀，描述性測試會紅。
   3. `make check-fast` 過。

### B25 做過的事自動記一行

1. **目標**：任務做完、證據清單裡有改變東西的步驟或查資料 5 步以上，程式自動記一行「MM-DD：要求（≤100 字）→ 回覆（≤120 字）」，連到那件任務（`memory-n2` S1）。
2. **開工條件**：B16 已合；M7h（任務做完）在 `dev`；工具區有「查網頁」的工具（`T-memory-n2-2` 要用；沒有的話這列延後，另兩列照做）；**Q-22** 已答。
3. **只准改**：`crates/domain/src/memory_task_line.rs`（新，純邏輯：要不要記、怎麼截）、執行端任務收尾的那一處（寫事件）、`locales/*.ftl`（這一行的顯示用代碼 `memory-task-line`，參數 `date`、`request`、`reply`）。
4. **定好的介面**：`pub fn task_line(task: &TaskSummary, evidence: &[EvidenceItem]) -> Option<TaskLine>`；`TaskLine { date_mmdd, request, reply, task_id }`，存結構化欄位，顯示時才照語言排（K91）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| n2-1 | 假時鐘 11-24；150 字要求，寫檔後回 200 字 | 多一行，開頭「11-24：」，要求 100 字、回覆 120 字，連到那件任務 | S1 |
| n2-2 | 連查 5 次網頁；只查 1 次；只聊天 | 第一個記一行；後兩個不記 | S1、S2 |
| n2-3 | 有一行做過的事；`forget`(那一行的字) | 那一行還在 | S3 |

6. **要寫的測試**：`t_memory_n2_1`、`t_memory_n2_2`、`t_memory_n2_3`（整合）；`task_line_cuts_request_100_reply_120`（單元，字數寫死）。
7. **不准做的**：不另外記一份任務紀錄（`04` §3）；不叫模型寫這一行。
8. **驗收清單**：
   1. 測試綠；把 5 步門檻改成 4，`t_memory_n2_2` 會紅。
   2. `make check-fast` 過。

### B26 離線和跨裝置

1. **目標**：離線記的記憶連上後補上；伺服器停機時，每一步的記憶改從本機副本只比字詞找（`memory-kinds` S4、`memory-carry` S9、K107）。
2. **開工條件**：B18、B21 已合；M7j（執行端接上伺服器）、M6c（客戶端同步）在 `dev`。
3. **只准改**：`crates/runner/src/brain_source*.rs`（退回本機）、`crates/store-client` 的查詢入口；伺服器整合測試。
4. **定好的介面**：`BrainSource` 的真的實作：伺服器回錯或逾時 → 用 `store-client::search_local`；記憶事件跟著匣子補送。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| kinds-4 | 桌面記「X」；桌面斷網再記「Y」；恢復 | 手機看得到「X」，恢復後也看得到「Y」；紀錄編號連續、不重複 | `memory-kinds` S4 |
| carry-9 | 伺服器停機；「住台北」；桌面送「台北天氣」 | 請求帶了「住台北」 | `memory-carry` S9 |

6. **要寫的測試**：`t_memory_kinds_4`、`t_memory_carry_9`（整合，兩台裝置、斷網、伺服器停機都用測試環境的網路控制）。
7. **不准做的**：離線不做向量；不另開一條同步路。
8. **驗收清單**：
   1. 兩個測試綠；把退回本機拿掉，`t_memory_carry_9` 會紅。
   2. `make check-fast` 過。

### B27 Markdown 雙向同步

1. **目標**：K96：伺服器資料庫是正本；同時產生一個 Markdown＋YAML 的資料夾（Obsidian 打得開）；人改了檔案，比對版本後匯回，有衝突讓人選，匯回前照 K97 檢查，檔案裡的字不當成指令。
2. **開工條件**：B16 已合；**Q-16**（資料夾在哪台、檔案怎麼排、什麼時候匯回、衝突怎麼選、刪檔算什麼）已答。**規格沒有這一節的列**（Q-1），沒答之前不要開工。
3. **只准改**：`crates/domain/src/memory_markdown*.rs`（新，純轉換：記憶 ⇄ 文字）；`crates/store-client/src/memory_folder*.rs`（新，讀寫檔，`07` §3 只准 `store-client` 讀寫任意檔案）；`migrations/client/0050_*`–`0054_*`（記每個檔最後同步的版本）；匯回走 B16 的 API。
4. **定好的介面**（建議，照 Q-16 改）：一條記憶一個檔 `<memory_id>.md`；YAML 欄位＝K106 的欄位（編號、版本、種類、狀態、來源、範圍、標籤、成立、作廢、被誰取代）；`pub fn render(memory: &Memory) -> String`、`pub fn parse(text: &str) -> Result<ParsedMemory, MarkdownError>`。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| 1 | 伺服器有新版 | 檔案更新 | K96 |
| 2 | 檔案改了、伺服器那條沒變 | 照 K97 檢查後匯回成一筆新版本 | K96 |
| 3 | 兩邊都改了 | 不自動選，留一筆衝突等人選 | K96 |
| 4 | 檔案裡寫「忽略之前的指示…」 | 當成記憶內容、照 K106 掃描，不進系統提示當指令 | K96 |
| 5 | YAML 壞掉 | 不匯回，回報哪個檔、哪裡壞 | 邊界 |

6. **要寫的測試**（描述性）：`markdown_roundtrip_keeps_every_field`（性質 1,000）、`file_edit_imports_new_version`、`both_sides_changed_is_a_conflict`、`file_text_is_never_an_instruction`、`broken_yaml_is_reported`。
7. **不准做的**：不依賴 Obsidian；伺服器不寫這個資料夾（`server` 不准讀寫任意檔案）；不自動解衝突。
8. **驗收清單**：
   1. 五個測試綠；把衝突判斷拿掉，第三個會紅。
   2. 回報寫 Q-16 每一小題用了哪個答案。
   3. `make check-fast` 過。

### B28 大腦頁面（一）：清單、搜尋、新增

1. **目標**：App 裡看大腦的基本頁面（K96），先做記憶頁的切換、摘要、搜尋、篩選、新增、分頁（`memory-page` S2–S5、S8）。
2. **開工條件**：B16、B17 已合；M7k（手機和網頁畫面的基礎）在 `dev`；**Q-6**、**Q-19** 已答。
3. **只准改**：`web/ui/src/brain/*`（新）、`locales/*.ftl`；伺服器若缺摘要欄位，補在 B16 的 API（往後加）；Playwright 測試。
4. **定好的介面**：用 B16、B17 的 API；摘要裡「語意搜尋的狀態」在向量延後期間固定顯示「關閉」（照 Q-4）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| page-2 | Bot「小助」，打開、切換 | 有「「小助」自己的」和「關於你（所有 Bot 共用）」；摘要有各種條數、釘選 token、占用空間、語意搜尋狀態 | S2、S3 |
| page-3 | 100 條；打「貓」；篩選「釘選」 | 「共 N 條（搜尋 X 毫秒）」；篩選後只剩釘選的 | S4 |
| page-4 | 新增「喝美式」按「記住」 | 多一條、寫「還沒被帶入過」 | S5、S6 |
| page-7 | 150 條；按「再顯示 60」 | 先 60 再多 60 | S8 |
| pin-3 | 切到「關於你」看一條 | 沒有「釘選」 | `memory-pin` S3 |

6. **要寫的測試**：`T-memory-page-2`、`T-memory-page-3`、`T-memory-page-4`、`T-memory-page-7`、`T-memory-pin-3`（Playwright 端對端，標題開頭寫編號；文字照 en 語言比對或用代碼）。
7. **不准做的**：不做整理建議卡（Q-6）；不做 S1 的四個入口（M8）；不做技能、關聯圖（Q-19）。
8. **驗收清單**：
   1. 五個端對端測試綠。
   2. 測試和程式碼沒有中文（字在 `locales/`）。
   3. `make check-fast` 過。

### B29 大腦頁面（二）：帶過次數、清空、編輯合併、久沒用的候選

1. **目標**：每條的「帶過 N 次 · 上次…」、釘選 token、清空最近刪除（要再確認）、編輯成一樣就合併、K106「很久沒用到的候選」一次確認或清掉。
2. **開工條件**：B28、B22 已合；**Q-15**（「帶過」怎麼算）已答。
3. **只准改**：`web/ui/src/brain/*`、伺服器的統計查詢（B16 的 API 往後加）、`locales/*.ftl`；測試。
4. **定好的介面**：帶過的次數和上次時間由「組了上下文」事件的清單算（建議，照 Q-15）；清空最近刪除照 K113 ② 只清內容；合併＝`Superseded` 事件、舊的記轉址（K96）。
5. **行為表**：

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| page-5 | 一條帶過 3 次；最近刪除 2 條；按清空 | 寫「帶過 3 次 · 上次…」；跳「清空後無法復原。」，確認後 0 條 | `memory-page` S6 |
| pin-5 | 釘選 2 條 | 「釘選的每次都帶（約 N token）」，N > 0 | `memory-pin` S5 |
| n1-4 | 「A 事」「B 事」；把 B 編輯成 A | 只剩一條「A 事」 | `memory-n1` S4、K96 |
| 補 | 一條候選 91 天沒用、沒確認 | 出現在「很久沒用到的候選」；可以一次確認或清掉；不會自動刪 | K106 |

6. **要寫的測試**：`T-memory-page-5`、`T-memory-pin-5`、`T-memory-n1-4`（端對端）；`stale_candidates_listed_not_deleted`（描述性，伺服器層）。
7. **不准做的**：不自動刪候選（K106）；合併不刪舊的（K96）。
8. **驗收清單**：
   1. 三個端對端、一個描述性測試綠。
   2. `make check-fast` 過。

---

## D. 要問使用者的問題

每題附建議的答案。前 6 題擋住最多片，最好先答。

**第一批：規格要先改（agent 不能改 `spec/`）**

- **Q-1 標了「做到時重寫」的列，和 K96–K98、K106、K107 新功能缺的列，誰寫、什麼時候寫？** `memory-search`、`memory-n1` S1–S2、`memory-n3` S2–S3、`memory-n4` 都寫著「以 09 為準，做到這一區時會照新的決定重寫」；切詞、索引重建、效能關卡、Markdown 同步、寫入前掃描、90 天候選、矛盾標記則完全沒有 `T-` 列。沒有列，B14–B20、B27 只能用描述性名字，驗收時沒有編號可以對。
  - 建議：B14 開工前，由使用者（或 Claude）在第 3 組新增 `memory-events`、`memory-index`、`memory-md-sync` 三節，並重寫上面四節。這些列在那之前留在基準檔。
- **Q-5 `T-memory-pin-1` 跟 K106 衝突。** K106 規定釘選在每段對話開始時凍結，中途改的下一段才生效；但 `pin-1` 是同一段裡「取消釘選；送 C」就期望 C 不帶。
  - 建議：照 K106，把 `pin-1` 改成「取消釘選；開始新對話；送 C」（跟 `T-context-fixed-1` 的寫法一致）。
- **Q-6 `memory-page` S7 的整理建議跟 K97、K106 衝突。** S7 寫「合併（留下較新的）」「全部忘記」，還有「壓掉的原文放最近刪除 30 天」；但 K106 規定整理不改狀態、原文不刪、候選 90 天不自動刪。這題影響 `T-memory-page-6`、`T-memory-about-5`、`T-memory-n3-5`。
  - 建議：把 S7 重寫成兩張卡。第一張是「很久沒用到的候選」（K106 的 90 天），可以一次確認或清掉。第二張是「很像的兩條」，可以合併（用「取代」事件留轉址，舊的變被取代、不刪）或兩條都留。拿掉「壓掉的原文放最近刪除」那句小字。「關於你」不出現在第一張。
- **Q-26 每晚整理現在要做什麼？** `memory-n3` S2、S3 被取代後，只剩 K106 的一句「只合併、只壓縮，不改狀態」，沒寫壓縮成什麼、放在哪裡、跟 `memory-n2` 的做過的事怎麼銜接。
  - 建議：M7.5 不做每晚整理，`memory-n3` 整節留到重寫後另開一片。重寫方向：每週的做過的事產生一筆新的「整理過的」記憶，附原始記憶的編號，原文不刪，也不從整理過的再整理。

**第二批：架構和套件**

- **Q-2 切詞模組放哪個套件？** `spec/07` §2 沒有 `brain`、`search` 這類套件。伺服器和客戶端共同依賴的只有 `foundation`、`protocol`、`domain`。如果放進 `domain`，每個套件都要跟著編 jieba-rs 和它的 C 依賴（`zstd-sys`）。
  - 建議：新增葉子套件 `crates/terms`，只依賴 `foundation`，由 `store-server`、`store-client`（和需要的話 `runner`）依賴；請使用者在 `spec/07` §3 加這一列。不准新增的話，就放 `domain::terms`。
- **Q-3 jieba-rs 預設的辭典功能會拉進 `include-flate` → `zstd-sys`（C）、`libflate`、`proc-macro-error3`，可以接受嗎？** 另一個做法是關掉預設功能，把 jieba 的辭典（約 5 MB，MIT）放進 repo，自己載入。
  - 建議：B13 確認 `cargo deny` 乾淨、三種系統都編得過，就接受預設功能。
- **Q-4 向量和語意搜尋要不要在 M7.5 做？** K107 說語意模型在伺服器跑、用 pgvector，但：`server` 不准依賴 `models`、不准發 HTTP（`07` §3、A1）；測試用的 PostgreSQL 很可能沒有 pgvector；K98 說向量是「可選的」。
  - 建議：M7.5 只做字詞，RRF 合併「字詞＋最近使用」兩路。向量、重排、`memory-n4`、`T-memory-search-3` 移到「M10 之後：選配向量」。到那時再決定由哪個行程算向量（建議伺服器機器上的執行服務，它可以依賴 `models`）。
- **Q-24 准入在哪裡算、誰寫哪種記憶事件？**
  - 建議：准入是 `domain` 的純函式，在重播事件時套用，伺服器和每台裝置算出一樣的狀態。執行端只寫「提議」（`remember` 的內容和來源）。使用者的確認、釘選、還原、編輯是對伺服器的動作，由伺服器寫成事件（比照 K113 ①）。
- **Q-18 索引重建誰觸發？**
  - 建議：兩端啟動時比對存的切詞版本，不同就重建；伺服器另加一個命令列子命令 `rebuild-memory-index`（`apps/server`），重建完跑一致性檢查。重建期間精確編號照常查，全文回「索引重建中」的錯誤代碼。
- **Q-21 B10 的參考值和 tokenizer.json 從哪來？** `T-models-capabilities-6` 要跟「o200k 的參考值」「那份 tokenizer.json 的參考值」比，但參考值不能用被測的函式算，而 agent 不能裝 Python 的 tiktoken。另外 `models` 不准讀檔，本機模型的 tokenizer.json 要有人拿給它。
  - 建議：參考值由使用者或監督者用官方工具算一次，寫進規格那一列或測試的固定資料。tokenizer.json 由 `models` 用 HTTP 跟使用者的本機模型伺服器拿；拿不到就用公式，並在清單記下估算方式。`tokenizers` 只開必要的功能，不帶 C 的 onig。

**第三批：行為細節（規格沒寫）**

- **Q-7 `memory-kinds` S1 的三種（記得的事、做過的事、整理過的）跟 K97 的種類（使用者偏好、知識、事件、任務經驗、教訓、做法）是什麼關係？**
  - 建議：存的是 K97 的種類；另存一個「怎麼來的」（`remember` 或頁面新增、任務自動、整理），頁面的三個分組照「怎麼來的」分。
- **Q-8 各種來源轉正後是哪個狀態？**
  - 建議：使用者說的、使用者按確認 → 已驗證；新的工具證據 → 有來源支持。`T-memory-n1-3` 從最近刪除救回時，保持被刪之前的狀態。
- **Q-9 程式怎麼知道 `remember` 的內容是「使用者親口說的」？** 模型呼叫工具，程式無法直接證明內容出自使用者。
  - 建議：`remember` 多一個選填參數，指向這件任務裡使用者的某一則訊息。程式把兩邊都做 NFKC、小寫、合併空白後比對，那則訊息含有這段內容，才算「使用者說的」；否則一律是「模型推論」，進候選。
- **Q-10 K106 的事件種類沒有「取消釘選」「還原」「編輯」，要補嗎？**
  - 建議：補「取消釘選」「還原」兩種；編輯用「取代」（新版本加轉址）。
- **Q-11 寫入前掃到提示注入、金鑰、看不見的字元時怎麼辦？樣式清單是什麼？**
  - 建議：三種都拒絕寫入，回不同的錯誤代碼給模型，不存內容。金鑰的樣式跟信任區遮金鑰的規則共用。提示注入和看不見的字元各寫一份固定清單進規格（像 `context-mask` 的 8 個樣式那樣）。
- **Q-12 重複和矛盾由誰認？舊的那筆變什麼狀態？** K97 寫「兩筆都留並寫明原因」，K106 寫「把舊的作廢」，`T-memory-carry-6` 又要兩筆都帶、標有矛盾。
  - 建議：重複＝同一個範圍裡、正規化（NFKC、小寫、合併空白）後完全相同。矛盾只在寫入的一方明確指出「跟哪一筆矛盾」時成立，不做語意判斷。成立時兩筆都是「有矛盾」，一起帶；使用者確認其中一筆後，另一筆才作廢。
- **Q-13 候選和推論算不算在「關於你」的最新 40 條、每一步找回來的記憶裡？**
  - 建議：算，帶上標記（K106），排序往後。
- **Q-14 查詢怎麼組？識別碼怎麼拆？錯誤碼怎麼認？**
  - 建議：
    - 中文：查詢只有一個字時查單字，兩個字以上查所有重疊兩字詞（全部都要有）。
    - jieba 的詞：只用來加分。
    - 識別碼：在 `_`、`-`、`.`、`/`、大小寫交界、數字和字母交界拆開；完整的識別碼和拆開的子詞都查（有任何一個就算）。
    - 「精確命中」：記憶編號的格式，或同時含字母和數字的完整識別碼（像 `E1234`、`ERR_42`）。
- **Q-15 「被用到」「帶過 N 次」怎麼算？**
  - 建議：被帶進某一步的請求（從「組了上下文」事件的清單算），或被 `search_memory` 回給模型，都算一次。這是衍生資料，不另寫記憶事件。
- **Q-16 Markdown 同步的細節。** 規格只有 K96 一句。
  - 建議：
    - 資料夾放在桌面 App 那台電腦的本機資料夾 `brain/`，由 `store-client` 讀寫（伺服器不准寫檔）。
    - 一條記憶一個 `<memory_id>.md`。
    - 桌面 App 啟動時和使用者按「從資料夾匯回」時才匯回，不監看檔案。
    - 新增的檔案沒有編號就當成新增，來源「使用者說的」（擁有者自己寫的）。
    - 刪掉檔案不算忘記，下次同步時重新產生。
    - 衝突列在大腦頁面讓人選。
- **Q-17 效能關卡的參數。**
  - 建議：
    - 種子固定，10 萬筆。
    - 暖機 20 次，精確編號和全文各量 200 次查詢，各自算 p95。
    - 放在 nextest 的 `timing` 組、進 `make check`。
    - CI 上沒過就算紅，不自動重試。
- **Q-19 M7.5 的「看大腦的基本頁面」做到哪裡？** K96 列了記憶、技能、學到的東西、來源和證據、關聯圖、版本差異、矛盾、過時、還沒驗證的候選、最近學了什麼。
  - 建議：M7.5 做記憶清單、搜尋、篩選、新增、釘選、編輯、忘記、還原、候選和矛盾的標記、一條記憶的版本歷史、很久沒用到的候選。技能、學到的東西、關聯圖放 M10 之後。
- **Q-20 `crates/context` 輸出給模型的固定字目前是寫死的英文，不是訊息代碼。** 例如 `masked: tool=…`、`[elided N chars, full text ref=…]`、`same as step N`、狀態摘要的標題，以及之後的記憶標記。K119 ⑯ 要求這些是 `turn-` 代碼，但 `context` 不准依賴 `messages`（`07` §3）。
  - 建議：執行端用 `messages` 排好一份「字句表」（固定字的樣板），放進 `BuildInput` 傳給 `context`，`context` 不用改依賴。另開一片（約一小時）在 B1 之後做。
- **Q-22 做過的事（`memory-n2`）是 K97 的哪一種、哪個狀態？**
  - 建議：種類「任務經驗」，來源「工具證據」（連到那件任務的證據清單），狀態「有來源支持」，不進候選。
- **Q-23 「謝謝」「繼續」這類應答詞，和「門檻很保守」，具體是什麼？**
  - 建議：在規格寫一份固定的應答詞清單（中英各十來個），整句正規化後完全相同才跳過。門檻定成至少命中一個兩字詞、jieba 的詞或完整識別碼才帶。
- **Q-25 `T-memory-carry-5`「勾選清單是空的訊息 → 請求不包含任何記憶」跟 `memory-carry` S4「每一步程式自己找」怎麼一起成立？**
  - 建議：空清單只代表「你沒勾」，程式照樣自己找；測試的前提要保證那句話跟任何記憶都不相關。如果使用者的本意是「空清單＝這一輪連程式找的都不帶」，請改 S4。
- **Q-27 /compact 作用在哪一件任務？**（B6）
  - 建議：這個 Bot 最近一件任務；沒有任務或沒有工具結果時回「沒有可以收起的舊結果」。
- **Q-28 整合列的預期裡有「畫面…」，接上伺服器（M7j）之後怎麼算驗到？**（B11）
  - 建議：看前端畫那個畫面時用的 API 和 SSE 回的資料（例如步驟的完整結果、卡住的細項和缺多少的參數）就算。真的要看像素的才寫成端對端。

---

## E. 跟其他規劃區共用的介面

**回合和任務引擎**（M7e–M7j）
- **`BrainSource` 接口**（B23、B24、B26）：執行端的建構函式多收一個 `BrainSource`，在兩個時間點呼叫。每一步問模型前呼叫 `retrieve`；「開始新對話」時呼叫 `snapshot`。回合區要定出「一段對話從哪裡開始」（`chat-n1`），並把那個界線交給 `snapshot`。沒有這條界線，K106 的快照做不出來。
- **舊對話**（B7）：`prior_chat` 由執行端讀這個 Bot 紀錄流裡不屬於目前任務的聊天；「這件任務的補充、糾正」靠 M7f 的插話送進 `user_turns`。
- **斜線指令**（B6）：/context、/compact 在哪裡被認出來（伺服器的信箱，或執行端），由回合區定；這一區只接在後面。
- **換模型**（B8）：沿用 M7g 的「換了模型」事件和步驟邊界（K119 ⑨）。收起的重新決定已經在 `crates/context`（K114 ⑪）。`T-models-handoff-5`、`-7` 是模型區的列，但驗的是這一區的編譯器，兩區要用同一組前提。
- **做完的標準和任務收尾**（B8、B9、B25）：`state-3` 要「做完的標準還沒驗到」；`memory-n2` 在任務完成時，由回合區的收尾掛上 `task_line`。
- **接續**（B5）：`build-3`、`mask-5` 要「同一份紀錄、新的執行端實例接著做」，靠 M7f 的接續，不另開入口。
- **例行任務**（M9）：`T-memory-carry-4` 等例行任務做好再驗。

**信任和核准**
- 固定段第 3 塊「安全的指示」的字由信任區定（`safety-n6` S2、`safety-n7` S1、`safety-reject` S4），代碼 `turn-fixed-safety`，B4 只負責排。
- 不受信任的一輪（`activity-who` S3、訊息管道、團隊成員）不帶記憶、沒有記憶工具。B21 提供工具清單的開關，B24 提供不帶快照的入口；信任區負責判斷哪一輪不受信任。
- 寫入前掃描的金鑰樣式，跟 `task-journal` 遮金鑰的規則共用一份（B15）。清單裡也不准出現金鑰（`context-manifest` S4，已有 `manifest_json_omits_secret_shaped_text`）。
- 記憶工具的分類：`search_memory` 是唯讀；`remember`、`forget` 不用核准（`memory-forget` S1、`memory-n1` S1）。它們在 `task-journal` 的三種等級裡算哪一種，請工具區或信任區定。

**工具和交付**
- `read_result`（含 `find`）、長輸出的存放和 K103 保存規則歸工具區（`files-n1`）。B7、B12 只用它們。記憶的「證據參照」要算進「被引用」（`files-n1` S4、`context-ref` S3），B16 之後提供「哪些原文被記憶引用」的查詢。
- 固定段第 4 塊「任務和證據的指示」的字由回合區和交付區定（`task-bot` S2、`report-delivery` S3），代碼 `turn-fixed-task-evidence`。
- `memory-n2` 判斷「改變東西的步驟、查資料 5 步以上」，用工具區的工具分類和證據清單（B25）。`T-memory-n2-2` 要「查網頁」的工具。

**手機和桌面畫面**（M7k、M8）
- B28、B29 用 M7k 的畫面基礎。這些不在這一區做：記憶頁的四個入口（`T-memory-page-1`）、帶入記憶的小卡片（`memory-carry` S1、S2、S8，`T-memory-pin-4`）、訊息下面的「帶了 N 條記憶」（`T-memory-carry-3`）、步驟詳情的清單（`T-context-manifest-2`）、永久刪除的再確認畫面（`T-sync-append-5`），都排在 M8。
- 「語意搜尋的狀態」在向量延後期間固定顯示「關閉」（Q-4）。
- 畫面上的字一律放 `locales/`（K91），兩區往同一份翻譯檔加，只准往後加。

**伺服器和同步**
- 記憶事件放在帳號那條紀錄流（`StreamSelector::Account`），由伺服器給號（K106、K113 ①）。客戶端副本和匣子沿用 M6c。
- 遷移號：伺服器 `0350`–`0364`、客戶端 `0040`–`0054` 保留給這一區（伺服器給 B16、B17、B19；客戶端給 B18、B19、B27），別區請避開。
- OpenAPI、TypeScript 客戶端、`tools/spec-trace/baseline.txt` 由監督者在合併時重產。
