# 01b · 資料檔的詳細格式（state.json、chats/ 以外）

> 來源：看過舊版的一方盤點 OLD 的資料格式（2026-11-24），不含程式結構。
> 用途：遷移工具的輸入規格。`01` 列出「有哪些檔」，這份寫到**欄位級**：每個表、每個欄位、每種紀錄、加密的位元組排法、保留天數。新版的格式可以全新，但這裡標「搬」的，意思要能完整轉過去。
> 互查：`01-data-inventory.md`（清單與分類）、`01a-data-state.md`（`state.json`、`chats/`，另一位負責）。

## 0. 範圍與共通規則

- **不含** `state.json`、`chats/`（含 `chats/threads/`）、`*.json.damaged-<時間>`：見 `01a`。
- 「資料夾」= `01` §A 的本機資料夾；雲端大腦是容器裡的 `/data`（§6）。
- 分類沿用 `01`：**搬／重建／丟／原地／待決定**。`01` 已經決定的照抄；跟 `01` 不同的在 §8 列出。

### 0.1 時間與編號的慣例

| 東西 | 單位 |
|---|---|
| App 寫的時間（記憶、紀錄、用量、保險庫、傳送、檢查點…） | Unix 毫秒，整數 |
| 帳號伺服器 `data.json` 的時間 | **Unix 秒**，整數 |
| `lan-cert.json` 的 `until`、管理員的 `brains-since` | Unix 秒 |
| Bot 編號（`bot-<id>`、`bot` 欄） | 正整數；**`0` 是「關於你（共用）」**，不是一個 Bot |
| 檔名裡的日期時間 | 本機時區（除非另外寫） |

### 0.2 權限

秘密與資料庫檔案建立時就是 `0600`（只有本人能讀寫），放它們的資料夾 `0700`。Windows 沒有這層。

### 0.3 SQLite 共通

| 項目 | 值 |
|---|---|
| journal | `WAL`（旁邊會有 `-wal`、`-shm` 兩個檔） |
| synchronous | `NORMAL` |
| 忙碌等待 | 5 秒 |
| 版本機制 | **沒有**（不用 `user_version`）。新欄位用「加欄位、失敗就算了」補上；要從 `pragma table_info` 看欄位在不在。 |
| 全文索引 | FTS5（`trigram` 斷詞需要 SQLite 3.34 以上） |
| 向量欄 | 小端序 32 位元浮點數一個接一個（長度 = 維度 × 4 bytes）；維度看當時的嵌入引擎 |

> **遷移注意**：舊資料夾唯讀（`00` 決定 8）。WAL 模式的資料庫直接唯讀打開，可能讀不到還在 `-wal` 裡的最新寫入、或因為要寫 `-shm` 而打不開。遷移工具要把 `.db`、`.db-wal`、`.db-shm` **三個一起**複製到暫存處再開。

### 0.4 加封值（`enc:v1:`）

`state.json` 的金鑰欄位，以及本檔的 `providers/chatgpt.json`、`connectors/oauth.json`、`brain-queue-*.json` 裡的秘密，用保險庫的同一把鑰匙加封成字串：

| 部分 | 內容 |
|---|---|
| 前綴 | ASCII `enc:v1:` |
| 其後 | 小寫十六進位，解碼後 = 12 bytes nonce ＋ 密文 ＋ 16 bytes 驗證標籤 |
| 演算法 | AES-256-GCM，鑰匙 = 保險庫鑰匙（§3.1） |
| AAD | ASCII `anybot-state-v1` |
| 明文 | UTF-8 原字串 |

- 寫入當下拿不到鑰匙（例如鑰匙圈鎖住）時，**照明文存**。所以遷移時同一個欄位可能是加封的、也可能是明文，兩種都要吃。
- 打不開的加封值舊版原樣保留、不使用。

---

## 1. SQLite

### 1.1 `memory.db`（記憶）— 搬（向量重建）

#### 表 `memories`（一列 = 一條記憶）

| 欄位 | 型別 | 意思 |
|---|---|---|
| `id` | INTEGER PRIMARY KEY | 記憶編號。被別列的 `superseded_by` 引用 |
| `bot` | INTEGER NOT NULL | 屬於哪個 Bot；`0` = 關於你（所有 Bot 共用） |
| `text` | TEXT NOT NULL | 內容。`fact` 寫入時截到 500 字 |
| `kind` | TEXT NOT NULL，預設 `fact` | `fact`（記得的事）、`event`（做過的事，每回合一行）、`rollup`（舊的做過的事壓成的週／月一行） |
| `created` | INTEGER NOT NULL | 建立時間（毫秒）。`rollup` 是那週週一或那月 1 號 00:00（使用者時區） |
| `vec` | BLOB，可 NULL | 嵌入向量；NULL = 還沒算。**重建** |
| `pinned` | INTEGER NOT NULL 預設 0 | 1 = 釘選（每次聊天都帶） |
| `uses` | INTEGER NOT NULL 預設 0 | 被用到幾次 |
| `last_used` | INTEGER NOT NULL 預設 0 | 最後一次被用到（毫秒；0 = 沒用過） |
| `deleted` | INTEGER NOT NULL 預設 0 | 0 = 在用；>0 = 放進「最近刪除」的時間（毫秒） |
| `superseded_by` | INTEGER NOT NULL 預設 0 | 被合併／壓縮進哪一列（`id`）；0 = 沒有 |
| `span` | TEXT NOT NULL 預設 `''` | 只有 `rollup` 用：`week:YYYY-MM-DD`（那週的週一）或 `month:YYYY-MM` |

- 唯一鍵：`(bot, text)`。同一個 Bot 不會有兩條一字不差的記憶；被刪的同句話再記一次，會把舊那列救回來（`deleted`、`superseded_by` 歸 0）。
- `event` 的文字格式：`MM-DD：<使用者的要求，最多 100 字> → <Bot 的回覆，最多 120 字>`（日期沒有年，年份看 `created`）。
- `rollup` 的文字格式：`M/D–M/D 做過的事（N 件）：甲 ×2、乙…還有 K 件` 或 `YYYY 年 M 月做過的事（N 件）：…`，一行最多約 400 字。

#### 索引

| 名稱 | 欄位 |
|---|---|
| `memories_bot` | `(bot, created DESC)` |
| `memories_alive` | `(bot, deleted, kind)` |
| （自動） | 唯一鍵 `(bot, text)` |

#### 全文索引（都可重建）

| 虛擬表 | 種類 | 斷詞 | 內容 |
|---|---|---|---|
| `memories_fts` | FTS5，外部內容 `content='memories'`、`content_rowid='id'`，欄 `text` | `trigram` | `memories.text` 的索引 |
| `memories_words` | FTS5 獨立表，欄 `words`，`rowid` = `memories.id` | `unicode61 remove_diacritics 0` | 由 `text` 衍生的搜尋用詞（空白分隔），舊版自己算 |

#### 觸發器

| 名稱 | 時機 | 做什麼 |
|---|---|---|
| `memories_ai` | 新增後 | 把 `text` 加進 `memories_fts` |
| `memories_ad` | 刪除後 | 從 `memories_fts` 移除 |
| `memories_au_text` | `text` 更新後 | `memories_fts` 先移除舊的再加新的 |
| `memories_words_ad` | 刪除後 | 刪掉 `memories_words` 同 rowid 那列 |

#### 表 `meta`（鍵值）

| 欄位 | 型別 |
|---|---|
| `key` | TEXT PRIMARY KEY |
| `value` | TEXT NOT NULL |

| 鍵 | 值 | 意思 |
|---|---|---|
| `embed_engine` | `m2v:minishlab/potion-multilingual-128M`（本機模型）、`ollama:<模型名>`；空 = 關閉 | 現在的向量是哪個引擎算的。換引擎時舊版把所有 `vec` 清成 NULL 重算 |
| `last_tidy` | `YYYY-MM-DD`（使用者時區） | 每晚整理最後跑的那天 |

#### 保留與整理（每晚，使用者時區 03:00 之後一次）

| 規則 | 數字 |
|---|---|
| 「最近刪除」放多久才真的刪 | 30 天（`deleted` 早於 30 天前的列刪除） |
| `event` 壓成每週一行 | 超過 30 天、而且那整週都過了 30 天 |
| 每週一行再壓成每月一行 | 超過 180 天、而且那整月都過了 180 天 |
| 被壓縮的原列 | 進「最近刪除」（`superseded_by` 指向新的那行），30 天內還能還原 |
| 很久沒用的 `fact` | 180 天沒建立也沒用到：**只提示**整理，不自動刪（釘選的、關於你的不提示） |
| 壓實 | 每天一次（程式啟動 10 分鐘後，之後每 24 小時）：兩個全文索引 optimize、WAL checkpoint |

#### 備份 `backups/memory-YYYYMMDD.db` — 丟

- 每晚整理之後做一份完整一致的拷貝（SQLite 的 `VACUUM INTO`），檔名用本機日期；同一天已經有就不再做。
- 只留**最新 7 份**（照檔名排序刪最舊的）。
- 格式跟 `memory.db` 一模一樣。

#### 跟 `state.json` 的關係

`state.json` 每個 Bot 的記憶清單是同步用的**副本**（最多 200 條，超過丟最舊的）；`memory.db` 才是完整的。舊版每次啟動會把副本裡有、資料庫沒有的補進 `memory.db`（當 `fact`，但「最近刪除」裡的不會被救回）。**遷移以 `memory.db` 為準**，副本只在 `memory.db` 壞掉時拿來補（互查 `01a`）。

### 1.2 `docs.db`（文件索引快取）— 重建

| 表 | 欄位 | 型別 | 意思 |
|---|---|---|---|
| `docs` | `path` | TEXT NOT NULL | 檔案絕對路徑 |
| | `mtime` | INTEGER NOT NULL | 檔案修改時間（毫秒）；沒變就不重新索引 |
| | `idx` | INTEGER NOT NULL | 這個檔的第幾段（0 起） |
| | `text` | TEXT NOT NULL | 這一段的文字（約 800 字一段，前後重疊約 120 字） |
| | `vec` | BLOB | 向量，可 NULL |
| | 主鍵 | `(path, idx)` | 一般 rowid 表 |
| `docs_fts` | `text` | FTS5，外部內容 `content='docs'`、`content_rowid='rowid'`，斷詞 `trigram` | |
| `meta` | `key`、`value` | 同 `memory.db` | 只有 `embed_engine` |

- 觸發器：`docs_ai`（新增後加索引）、`docs_ad`（刪除後移除）、`docs_au`（更新後先移除再加）。
- 只索引這些副檔名：`md txt csv log rs py js ts html json`；32 bytes 到 2,000,000 bytes 的檔；不跟捷徑、略過隱藏檔、`node_modules`、`target`。
- 壓實：每天一次 optimize；空頁超過 16 MiB 就 `VACUUM`。
- 整個檔刪掉重建不會掉資料（要掃的資料夾清單在 `state.json`）。

### 1.3 `oplog/audit.db`（操作紀錄）— 待決定（`01`）

#### 表 `audit`（一列 = 時間軸上的一件事）

| 欄位 | 型別 | 意思 |
|---|---|---|
| `id` | INTEGER PRIMARY KEY | 順序 |
| `bot` | INTEGER NOT NULL | Bot 編號 |
| `op` | TEXT NOT NULL | 這一輪的編號，格式 `turn-<Bot 編號>-<開始的毫秒>`；`queue` 列可能是空字串 |
| `at` | INTEGER NOT NULL | 發生時間（毫秒） |
| `end_at` | INTEGER | `tool` 列：結束時間 |
| `kind` | TEXT NOT NULL | 見下表 |
| `name` | TEXT | `model` 列：`assistant` 或 `compaction`；`tool` 列：工具名 |
| `detail` | TEXT | `turn` 列：使用者那則訊息全文；`tool` 列：參數（JSON 文字）；`queue` 列：中途打的字 |
| `status` | TEXT | 見下表 |
| `call_id` | TEXT | `tool` 列：模型給的呼叫編號；`queue` 列：中途訊息編號（`q-<毫秒>-<序號>`） |
| `prompt_at` | INTEGER | `turn` 列：那則訊息送出的時間（毫秒）。**後來才加的欄位**：舊列是 NULL，極少數檔案可能整欄不存在 |

| `kind` | 一列代表 | `status` 可能的值 |
|---|---|---|
| `turn` | 一輪開始 | NULL（進行中或被中斷）、`completed`、`aborted`、`failed` |
| `model` | 問模型一次（同一步重試會有多列） | NULL |
| `tool` | 一個步驟 | `running`、`done`、`failed`、`denied`、`waiting` |
| `queue` | 使用者在 Bot 做事時打的字 | `waiting`、`consumed`、`cancelled` |
| `abort` | 使用者按了停止 | NULL |

- 索引：`audit_bot_at (bot, at DESC)`、`audit_call (bot, op, call_id)`。
- 保留：**30 天**。每次打開、以及每晚整理時刪掉 `at` 早於 30 天前的列。
- 手機的「稽核時間軸」讀最近 30 天內、每個 Bot 最新 20 輪。
- 「誰發起的」不另存：由 `turn` 列的訊息開頭判斷——`［例行任務］` 或 `⏰ ` 開頭是例行任務，`［來自 Bot］` 開頭是別的 Bot，其他是本人。
- 更早的版本這個檔裡還有 `memories`、`docs` 兩個表；OLD 最後一版第一次啟動會搬走並刪掉（決定 4：遷移不用處理）。

---

## 2. JSONL 與文字紀錄

### 2.1 `oplog/bot-<id>.jsonl`（進行中回合的工作紀錄）— 丟

一行一筆 JSON，每行寫完都同步到磁碟。用 `"type"` 分種類；最後一行可能被斷電切斷（讀不懂就跳過）。**一輪結束時檔案縮到只剩還沒送到 Bot 的中途訊息**（通常是空的）。

| `type` | 欄位 | 意思 |
|---|---|---|
| `operation_started` | `op`、`at`、`source_leaf`、`prompt`（可無） | 一輪開始。`source_leaf` = 起點那則訊息的時間；`prompt` = `{"at": 毫秒, "text": 訊息全文}` |
| `step_attempt` | `op`、`step`（`assistant`／`compaction`）、`attempt`（第幾次） | 要問模型了 |
| `tool_started` | `op`、`call_id`、`name`、`args`（JSON 文字）、`replay`（`safe`／`never`）、`at`（舊檔可能 0） | 一個步驟要開始了。`replay`：當機後能不能自動重做（看東西可以；對外送出、點擊、寫檔、發布不行） |
| `tool_finished` | `op`、`call_id`、`status`（`waiting`／`running`／`done`／`denied`／`failed`）、`at` | 步驟結束 |
| `queue_enqueued` | `op`（可為 null）、`id`、`text`、`at`、`attachments`（沒有就省略） | 使用者在 Bot 做事時打的字（附件的格式同聊天附件，見 `01a`） |
| `queue_consumed` | `id` | 那則中途訊息送到 Bot 了 |
| `queue_cancelled` | `id` | 那則中途訊息被丟掉（使用者按停止） |
| `abort_requested` | `op` | 使用者按停止 |
| `operation_finished` | `op`、`outcome`（`completed`／`aborted`／`failed`） | 一輪結束 |

- 每一行同時寫進 `audit.db`（§1.3）。
- 前提（`01`）：遷移前所有 Bot 閒置。閒置時檔案只會有 `queue_enqueued`（使用者打了還沒送到的字）；**遷移工具應檢查：檔案不是空的就停下來**。

### 2.2 `checkpoints/`（改檔前的存檔點，`/rewind` 用）— 丟

| 路徑 | 內容 |
|---|---|
| `checkpoints/bot-<id>.jsonl` | 一行一筆：Bot 改某個檔之前的樣子。每次有變動整檔重寫 |
| `checkpoints/blobs/<SHA-256 十六進位 64 字>` | 檔案改之前的內容，以內容的雜湊命名（同內容只存一份） |

每一行的欄位：

| 欄位 | 意思 |
|---|---|
| `at` | 存的時間（毫秒） |
| `turn` | 開始那一輪的使用者訊息的時間（毫秒），`/rewind` 用它分組 |
| `computer` | 電腦名稱（顯示用） |
| `place` | 在哪台電腦、哪個起點，格式是只有一個鍵的物件：`{"Local":{"workdir":路徑}}`（使用者的電腦）、`{"Box":{"dir":…}}`（Bot 的電腦）、`{"Sealed":{"root":…}}`（沒有 Docker 時 Bot 自己的資料夾）、`{"Ssh":{"target":…,"dir":…}}`、`{"Device":{"host":主機編號,"dir":…}}` |
| `path` | 檔案路徑 |
| `before` | 改之前內容的 blob 名稱；`null` = 那時檔案還不存在（還原時刪掉） |
| `too_big` | `true` = 超過 2 MiB 沒存，無法還原 |

- 保留 **7 天**：讀的時候只看 7 天內的；blob 在程式啟動時，修改時間超過 8 天的刪掉。
- 單檔超過 2 MiB 不存。指令（shell、git…）改的檔、別台 AnyBot 上的檔不在裡面。

### 2.3 `usage/YYYY-MM.jsonl`（用量）— 搬

一個月一個檔（呼叫當下的本機年月），一次模型呼叫一行；供應商沒回報用量就不寫。**沒有保留期限，永遠留著**。

| 欄位 | 型別 | 意思 |
|---|---|---|
| `at` | 整數 | 呼叫時間（毫秒） |
| `provider` | 字串 | **顯示名稱**，不是代號。OLD 最後一版的值：`xAI Grok`、`OpenAI`、`ChatGPT 方案`、`Claude`、`Google Gemini`、`OpenRouter`、`OpenCode Go`、`Ollama`、`自訂（OpenAI 相容）` |
| `model` | 字串 | 模型名稱 |
| `session` | 字串（可無） | 誰、做什麼，見下表 |
| `input` | 整數（可無，預設 0） | 輸入 token（不含快取讀到的） |
| `output` | 整數 | 輸出 token |
| `cache_read` | 整數 | 從提示快取讀的 token |
| `cache_write` | 整數 | 寫進提示快取的 token |
| `cost_micros` | 整數 | 費用，百萬分之一美元；只有供應商有給才有（例如 OpenRouter） |

| `session` | `/usage` 歸到 |
|---|---|
| `anybot-bot-<Bot 編號>` | 那個 Bot 的「回合」 |
| `anybot-check-<Bot 編號>` | 那個 Bot 的「發布前檢查」 |
| `anybot-goal-<Bot 編號>` | 那個 Bot 的「目標檢查」 |
| `anybot-btw-<編號>` | 「支線」（編號是支線 Bot 的；待確認） |
| 其他或空 | 「其他」（Bot 0） |

> **遷移注意**：`provider` 是畫面上的名稱，遷移要有一張「名稱 → 新版供應商代號」的對照表。較早版本寫的行可能用過別的名稱（待確認）。

### 2.4 `sent.jsonl`（已送帳本）— 搬（建議；`01` 原為待確認）

**用途（解 `01` 的待確認）**：Bot 用瀏覽器對外送出文字（回覆、發文、傳訊息）時，系統（不是 Bot）在送出後回頭讀頁面確認，確認到了就記一行。下次要送之前先查這本帳：

- 同一個帳號已經回覆過同一則貼文（60 天內）→ 拒絕再送，告訴 Bot 那次是什麼時候。
- 頁面上看不到貼文網址時：同一頁、同樣的字、24 小時內 → 視為重複。
- Bot 也能查這本帳，看最近送過什麼（最多列 60 則）。

不搬的話，升級後例行任務可能對同一則貼文再回一次。

| 欄位 | 意思 |
|---|---|
| `at` | 送出時間（毫秒） |
| `bot` | Bot 的**名字**（不是編號） |
| `account` | 從哪個帳號送的：`<電腦名稱> · <瀏覽器設定檔>`。電腦名稱預設 `你的電腦`，設定檔預設 `預設設定檔` |
| `target` | 回覆的對象（貼文網址）；沒有就省略 |
| `page` | 在哪個網頁送的；沒有就省略 |
| `text` | 送出的字，最多存 300 字 |
| `link` | 送出後那則東西自己的網址；沒有就省略 |
| `seen` | 送出後在頁面上找到了這些字 |

- 保留 **60 天**：寫新的一行時，若第一行已超過 60 天，整檔重寫只留 60 天內的。
- 所有 Bot、所有帳號共用一本。

### 2.5 `outputs/`（太長的步驟結果全文）— 搬

| 項目 | 值 |
|---|---|
| 檔名 | `r<12 個十六進位>.txt`；編號 = `r` ＋ 全文 SHA-256 的前 12 個十六進位字（同一段文字永遠同一個編號） |
| 內容 | 結果全文，UTF-8；超過 **20 MiB** 截斷 |
| 保留 | **7 天**（看檔案修改時間，程式啟動時清） |
| 聊天裡怎麼引用 | 步驟結果中間被省略的地方寫 `…（中間省略 N 個字；完整內容用 read_result 讀：id=r…）…` |

> 過了 7 天的引用本來就打不開，舊版會說「可能超過 7 天被清掉了」。遷移只要搬還在的檔，聊天裡的編號照舊。

### 2.6 `logs/`（紀錄檔與當機報告）— 丟

| 檔名 | 內容 | 保留 |
|---|---|---|
| `anybot-YYYY-MM-DD.log` | 每天一個。每行 `YYYY-MM-DD HH:MM:SS.mmm 等級 來源: 訊息`，多行訊息的續行縮排 4 格；秘密先遮掉 | **7 天**（看檔名的日期）；一天最多 **20 MiB**，到了寫一行「今天的紀錄超過 20 MB…」之後不再寫進檔案 |
| `crash-YYYYMMDD-HHMMSS.mmm.txt` | 當機報告：`AnyBot <版本>`、`時間：`、`執行緒：`、`位置：`、`訊息：`、空行、呼叫堆疊（已遮秘密） | **不會自動刪** |
| `crash-….seen.txt` | 已經在下次啟動告訴過使用者的報告（改名） | 不會自動刪 |

- 等級由環境變數 `ANYBOT_LOG` 決定（`off`／`error`／`warn`／`info`（預設）／`debug`／`trace`）。
- macOS 的背景服務另外有 `~/Library/Logs/anybot-serve.log`（服務設定寫的，不在資料夾）。

---

## 3. 秘密

### 3.1 保險庫的鑰匙（系統鑰匙圈／`vault.key`）— 搬

**一把鑰匙**：32 bytes 隨機數，存成 **64 個小寫十六進位字**（ASCII，沒有換行）。用來：

1. 解開 `vault.bin`（§3.3）；
2. 解開所有 `enc:v1:` 加封值（§0.4）：`state.json` 的金鑰欄位、ChatGPT 權杖、連接器權杖。

#### 去哪裡找（舊版讀的順序）

| 順序 | 條件 | 位置與名稱 |
|---|---|---|
| 1 | 有設環境變數 `ANYBOT_VAULT_KEY_FILE` | 那個檔，內容 64 個十六進位字 |
| 2 | macOS | 鑰匙圈「一般密碼」：服務 `AnyBot vault key`、帳號 `anybot`、標籤 `AnyBot vault key`；密碼 = 64 個十六進位字 |
| 2 | Linux（有 `secret-tool`、Secret Service） | 屬性 `application=anybot`、`kind=vault-key`；標籤 `AnyBot vault key`；秘密 = 64 個十六進位字 |
| 3 | 以上都找不到 | `vault.key`（在 `vault.bin` 旁邊）；Windows 一律走這條 |

- 寫鑰匙時也是同樣順序，前一個失敗才寫下一個。
- `vault.bin`、`vault.key` 所在的資料夾可以用環境變數 `ANYBOT_VAULT_DIR` 改到別處（只影響這兩個檔，不影響其他資料）。
- 找到的不是 32 bytes：舊版停下來不覆蓋（「保險庫金鑰損毀…」）。

> **遷移注意**：Linux 上鑰匙圈在第一次啟動時鎖著，舊版可能先寫了 `vault.key`，之後鑰匙圈才可用——理論上兩處都可能有東西。遷移工具要把**每個候選位置都試一次，用真的能解開 `vault.bin`（或任一加封值）的那把**。也要讀使用者設過的 `ANYBOT_VAULT_DIR`、`ANYBOT_VAULT_KEY_FILE`（決定 7）。

#### 更早版本留在鑰匙圈的網站登入（只讀，不再寫）

| 平台 | 怎麼存的 |
|---|---|
| macOS | 「網際網路密碼」，標籤 `AnyBot 登入`，伺服器 = 網站主機名，帳號 = 使用者名稱，密碼 = 網站密碼 |
| Linux | Secret Service 屬性 `application=anybot`、`server=<主機名>`、`user=<使用者名稱>`，秘密 = 網站密碼（標籤待確認） |

這些會被匯入 `vault.bin`（見 §3.5）。匯入後舊版不刪它們。

### 3.2 `vault.key` — 搬

見 §3.1：純文字，64 個小寫十六進位字，`0600`。只在沒有系統鑰匙圈時出現（Windows、沒有 Secret Service 的 Linux、**雲端大腦**）。

### 3.3 `vault.bin`（保險庫）— 搬

#### 位元組排法

| 位移 | 長度 | 內容 |
|---|---|---|
| 0 | 12 | nonce（每次存檔重新產生） |
| 12 | N | 密文 |
| 12+N | 16 | GCM 驗證標籤 |

- AES-256-GCM，鑰匙見 §3.1，**AAD = ASCII `anybot-vault-v1`**（跟加封值的 AAD 不同）。
- 小於 12 bytes 當作損壞；解不開 = 鑰匙不符。

#### 明文（UTF-8 JSON，緊湊格式）

最外層：

| 鍵 | 型別 | 意思 |
|---|---|---|
| `items` | 陣列 | 所有項目（登入、卡片、聯絡人混在一起） |
| `imported_keychain` | 布林 | `true` = 更早版本存在鑰匙圈的登入已經匯入過（§3.5） |

每個項目**都有全部欄位**（用不到的是空值），差別在 `kind`：

| 鍵 | 型別 | 登入 | 卡片 | 聯絡人 |
|---|---|---|---|---|
| `id` | 字串，16 個十六進位字 | ✔ | ✔ | ✔ |
| `kind` | `Login`／`Card`／`Contact`（大小寫照寫） | ✔ | ✔ | ✔ |
| `title` | 字串，≤200 字 | 網站名（第一個網站的可註冊網域） | 卡別（建立時） | 標籤 |
| `notes` | 字串，≤2000 字 | 提示：告訴 Bot 什麼時候用 | 同左 | 同左 |
| `created_ms`、`updated_ms` | 整數（毫秒） | ✔ | ✔ | ✔ |
| `sites` | 字串陣列，≤10 個 | 主機名（小寫、去掉 `www.`）或可註冊網域 | — | — |
| `username` | 字串，≤512 字 | 帳號 | — | — |
| `password` | 字串，≤4096 bytes；**空的就省略整個鍵** | **秘密** | — | — |
| `number` | 字串，只有數字（12–19 位，通過 Luhn 檢查）；空的就省略 | — | **秘密**，卡號 | — |
| `csc` | 字串，3–4 位數字；空的就省略 | — | **秘密**，安全碼 | — |
| `brand` | 字串 | — | `Visa`、`Mastercard`、`American Express`、`Discover`、`JCB`、`UnionPay`、`Diners Club`、`卡片`（認不出） | — |
| `last4` | 字串，4 位數字 | — | 卡號末 4 碼 | — |
| `exp_month` | 整數 1–12（未用為 0） | — | 有效月 | — |
| `exp_year` | 整數，四位數（未用為 0） | — | 有效年 | — |
| `holder` | 字串，≤200 字 | — | 持卡人 | — |
| `nickname` | 字串，≤200 字 | — | 暱稱 | — |
| `billing` | 地址物件 | — | 帳單地址 | — |
| `name` | 字串 | — | — | 姓名 |
| `address` | 地址物件 | — | — | 地址 |
| `phones` | 字串陣列，≤10 個 | — | — | 電話（6–16 位數字，可含 `+-().# ` 和空白） |
| `emails` | 字串陣列，≤10 個 | — | — | 電子郵件 |

地址物件：`line1`、`line2`、`city`、`region`、`postal`、`country`，全是字串。

- 卡號一旦存了不能改；同一張卡號不能存兩次。
- 匯入密碼管理器的 CSV 時，一個檔最多 5 MiB。

### 3.4 加封值

見 §0.4。

### 3.5 `vault-index.json` 與匯入時機 — 丟

**格式**：JSON 陣列（縮排排版），每個元素是 `[網站主機名, 帳號]` 兩個字串。只有 macOS 會有；它是「舊版存進鑰匙圈的網站登入」的清單（密碼在鑰匙圈，不在這裡）。

旁邊可能有 **`vault-index-imported`**：內容是 `1` 的標記檔，表示「沒有清單之前存的鑰匙圈登入（標籤 `AnyBot 登入`）已經掃進清單一次」。— 丟。

**什麼時候匯入 `vault.bin`（解 `01` 的待確認）**：第一次「列出保險庫」而且 `vault.bin` 的 `imported_keychain` 還是 `false` 的時候。會列出保險庫的時機：

- 打開保險庫畫面；
- **每一輪開始時**，只要這個 Bot 有 Bot 瀏覽器可用、而且這則訊息不是外部（不受信任的）訊息——要決定能給 Bot 哪些「填入」工具；
- Bot 要填入前的編號檢查、Bot 用「列出保險庫」工具。

匯入做的事：

1. 列出舊的鑰匙圈登入：macOS 讀 `vault-index.json`（必要時先掃一次標籤 `AnyBot 登入` 的項目補進清單），並剔除鑰匙圈裡已經不存在的；Linux 直接搜 Secret Service 裡 `application=anybot` 而且有 `server` 屬性的項目。
2. 逐一向鑰匙圈要密碼（macOS 可能跳出允許視窗）。
3. 每筆變成一個 `Login` 項目：`title` = 網站的可註冊網域、`sites` = `[主機名]`、帳號、密碼；已經有同網站同帳號（不分大小寫）的跳過。
4. `imported_keychain` 設成 `true` 並存檔。

`vault.bin` 打不開（鑰匙不在）時不會匯入，下次再試。

> **遷移**：只要使用者在 OLD 最後一版用過任何一輪有瀏覽器的對話，就已經匯入。遷移工具讀到 `imported_keychain = false` 時，要在使用者電腦上照上面同樣的步驟補做一次（需要鑰匙圈），不能直接丟掉這些登入。

### 3.6 `providers/chatgpt.json`（ChatGPT 方案登入）— 搬

縮排排版的 JSON 物件，`0600`，先寫暫存檔再換名。

| 鍵 | 型別 | 意思 |
|---|---|---|
| `client_id` | 字串 | OpenAI 發給這台 AnyBot 的用戶端編號（`oaiapp_…`）；登出後保留，重新登入不再註冊 |
| `host_id` | 字串 | 這台電腦給 OpenAI 的固定匿名編號（`urn:uuid:…`），做一次永不換 |
| `access` | 字串，**加封** | 存取權杖（約 1 小時有效，到期前 5 分鐘換新） |
| `refresh` | 字串，**加封** | 更新權杖（每次換新都會換） |
| `id_token` | 字串，**加封** | 身分憑證 |
| `expires_at` | 整數（毫秒） | `access` 到期時間；0 = 沒說 |
| `scopes` | 字串陣列 | 授權範圍；要有 `chatgpt.tokens.use.direct` 才能用方案 |
| `sub` | 字串 | 登入者身分 |
| `email` | 字串 | 登入者信箱（顯示用） |
| `welcomed` | 布林 | 第一次用方案的說明已經顯示過 |

- 登出：三個權杖清空，其他保留。
- 加封見 §0.4；拿不到鑰匙時是明文。

### 3.7 `connectors/oauth.json`（連接器的 OAuth 登入）— 搬

縮排排版的 JSON 物件，`0600`。**鍵 = 連接器（MCP 伺服器）的名稱**（跟 `state.json` 裡連接器清單的名稱對得上；連接器改名時這裡跟著改）。值：

| 鍵 | 型別 | 意思 |
|---|---|---|
| `access` | 字串，**加封** | 存取權杖 |
| `refresh` | 字串，**加封**（可空） | 更新權杖 |
| `expires_at` | 整數（毫秒） | 到期時間；0 = 沒說 |
| `token_endpoint` | 字串 | 換新權杖的網址 |
| `client_id` | 字串 | 動態註冊拿到的用戶端編號 |
| `client_secret` | 字串，**加封**（可空） | 用戶端密鑰 |
| `resource` | 字串 | 資源指示（RFC 8707） |
| `url` | 字串 | 登入當時連接器的網址；權杖只送往這裡 |

### 3.8 區網 HTTPS 憑證 — 搬

| 檔 | 格式 |
|---|---|
| `lan-cert.der` | 自簽憑證，DER。CN = `AnyBot`；SAN = `lan-cert.json` 的 `hosts`；用途 serverAuth；有效 **800 天**（從前一天起算） |
| `lan-key.der` | 私鑰，PKCS#8 DER（ECDSA P-256；待確認） |
| `lan-cert.json` | `{"hosts": [...], "until": Unix 秒}`。`hosts` 最多 40 個，一定含 `localhost`、`127.0.0.1`，加上這台出現過的區網位址；`until` = 憑證到期前一天 |

- 舊版遇到「1 天內到期」或「這台有新的位址不在 `hosts` 裡」就重做一張（舊位址保留在新憑證）。
- 設定畫面給使用者比對的指紋 = 憑證 DER 的 SHA-256，大寫十六進位、兩兩用 `:` 隔開（95 字）。
- 換掉的話信任過舊憑證的手機要重新信任（`01`）。

### 3.9 `wayland-remote-token.txt` — 搬

只有 Linux Wayland（GNOME、KDE）。內容是桌面「遠端桌面」入口回傳的恢復權杖（一段文字，沒有固定格式），讓之後的連線不用再問使用者。入口版本 2 以上才有。

### 3.10 Bot 的電腦的秘密

見 §4.10（`box/secrets.json`、`box/secrets.env`）。**注意那兩個檔是明文**。

---

## 4. 其他資料夾內的檔

### 4.1 hooks（`hooks/hooks.json` 與管理員的 `hooks.json`）

| 誰的 | 位置 | 分類 |
|---|---|---|
| 使用者 | `<資料夾>/hooks/hooks.json` | 搬 |
| 管理員（macOS） | `/Library/Application Support/AnyBot/hooks.json` | 原地（決定 6） |
| 管理員（Windows） | `%ProgramData%\AnyBot\hooks.json` | 原地 |
| 管理員（Linux） | `/etc/anybot/hooks.json` | 原地 |

格式跟 Claude Code 的 hooks 一樣：

```
{"hooks": {"<事件>": [ {"matcher": "<規則>", "hooks": [ <hook>, … ]}, … ]}}
```

| 層 | 鍵 | 值 |
|---|---|---|
| 最外層 | `hooks` | 物件：事件 → 群組陣列。只認這一個鍵（其他鍵讀的時候忽略，Bot 改寫使用者的檔時會消失） |
| 事件 | `SessionStart`（一段對話的第一輪）、`UserPromptSubmit`（每一輪）、`PreToolUse`、`PostToolUse`、`Stop`（Bot 要結束時）、`Notification`（AnyBot 通知使用者時） | |
| 群組 | `matcher` | 工具名稱的正規表示式（整個名稱要符合）；空或 `*` = 全部；看不懂的正規表示式就當一般字串比對 |
| | `hooks` | hook 陣列 |
| hook | `type` | `command`（預設）或 `http` |
| | `command` | 要跑的指令（事件以 JSON 從標準輸入給它） |
| | `url` | `http` 型：POST 事件 JSON 到這裡 |
| | `headers` | `http` 型：額外的標頭（物件，字串對字串；**明文**） |
| | `timeout` | 秒；沒寫 = 60；限制在 1–600 |

- `matcher` 也認 Claude Code 的工具名：`Bash` = `shell`、`Read` = `read_file`、`Edit`／`Write`／`MultiEdit` = `edit_file`、`Grep` = `search`、`LS` = `list_dir`、`WebFetch` = `web_fetch`、`WebSearch` = `web_search`。
- 使用者的先跑、管理員的後跑；最嚴格的決定成立（拒絕 > 詢問 > 允許）。
- 檔案不存在或是空的 = 沒有 hooks；看不懂 = 當作沒有並寫進紀錄檔。

### 4.2 管理員政策（MDM）— 原地（決定 6）

| 鍵 | 管的開關 |
|---|---|
| `AllowDirectMode` | 直接操作模式 |
| `AllowLocalExecution` | 本機執行 |
| `AllowBoxTrafficViaThisMac` | 網路流量改走你的電腦（所有平台都用這個名字） |

| 平台 | 從哪裡讀 |
|---|---|
| macOS | `defaults` 網域 `com.anybot.AnyBot`（MDM 設定描述檔可強制） |
| Linux | `/etc/anybot/policy.json`；**這個檔存在時只讀它**（即使缺某個鍵也不看使用者的檔），不存在才讀 `~/.anybot/policy.json` |
| Windows | 舊版跟 Linux 同一套：先找目前磁碟機根目錄的 `\etc\anybot\policy.json`，再找 `%USERPROFILE%\.anybot\policy.json`（待確認：沒有在 Windows 上驗證過；不讀登錄檔） |

`policy.json` 是一個 JSON 物件，鍵就是上面三個；值可以是字串、布林或數字（非字串的值照 JSON 文字看待，例如 `true` 當 `"true"`，`null` 當 `"null"`）。

值的判讀（去頭尾空白、不分大小寫）：

| 值 | 結果 |
|---|---|
| 沒有這個鍵、或空字串 | 不受管：使用者自己決定 |
| `true`、`1`、`yes` | 允許使用者打開（**不會強制打開**） |
| `false`、`0`、`no` | 鎖住在關閉 |
| 其他任何值（含 `null`） | 視為無效，**一樣鎖在關閉** |

### 4.3 `drops/`（裝置之間傳檔）— 丟（`01`：遷移前要送完）

| 路徑 | 內容 |
|---|---|
| `drops/index.json` | 包裹陣列（在大腦上：待送與已送） |
| `drops/<包裹編號>.bin` | 包裹的檔案內容（大腦保管一份完整的，直到送達） |
| `drops/in-<包裹編號>.part` | 接收端收到一半的檔案 |
| `drops/received.json` | 這台收到的東西，最新在前，最多 **100 筆** |
| `~/Downloads/AnyBot/`（或系統的下載資料夾下 `AnyBot/`） | 收到的檔案最後放的地方；同名加 ` (2)`、` (3)`… — 原地 |

包裹（`index.json` 的元素）：

| 鍵 | 意思 |
|---|---|
| `id` | 16 個十六進位字 |
| `name` | 檔名（去掉路徑、控制字元、開頭的點，最多 120 字） |
| `bytes` | 大小 |
| `text` | 傳文字時的內容（最多 100,000 字）；沒有就省略 |
| `from`、`from_id` | 寄件者的名稱（電腦名、`手機`、Bot 名）與編號 |
| `to`、`to_name` | 收件的主機編號（或 `phone` = 所有登入這個帳號的手機）與名稱 |
| `at` | 建立時間（毫秒） |
| `state` | `uploading`（寄件者還在上傳）、`waiting`（等對方上線）、`sending`、`ready`（手機可以下載了）、`done`、`failed` |
| `note` | 說明（失敗原因等）；沒有就省略 |
| `sent` | 已傳的 bytes |

收到的東西（`received.json` 的元素）：`id`、`name`、`path`（檔案現在在哪；文字是空的）、`bytes`、`text`（沒有就省略）、`from`、`at`。

- 包裹（不管送到沒）保留 **7 天**；沒送到的每 **15 秒**重試一次。
- 一個檔最大 **2 GiB**；分塊 2 MiB。
- 重開時 `uploading` 的包裹丟掉、`sending` 改回 `waiting`。

### 4.4 `attachments/bot-<id>/`（聊天附件）— 搬

| 項目 | 值 |
|---|---|
| 檔名 | `<YYYYMMDDHHMMSSmmm>-<原檔名>`（本機時間到毫秒；原檔名去掉路徑、控制字元、`:`、開頭的點，最多 120 字） |
| 貼上的圖片 | 原檔名 `貼上的圖片 HHMMSS.png` |
| 單檔上限 | **50 MiB**（52,428,800 bytes） |
| 聊天怎麼引用 | 附件記錄裡的 `path` 是這個檔的**絕對路徑**；另有 `id`（12 個十六進位字）、`sha`（檔案 SHA-256 前 12 個十六進位字）等，見 `01a` |
| 清理 | 每晚：超過 1 天、而且沒有被任何聊天（目前的、過去的對話、重新產生的還原、還沒送出的附件）引用、也沒在任何頁面內文出現檔名的，刪掉；空的 `bot-<id>` 資料夾也刪。刪掉一個 Bot 時整個資料夾刪掉 |

> **遷移注意**：聊天用絕對路徑指到附件。搬到新位置後要改寫路徑（或新版改用編號）。

### 4.5 `avatars/`（頭像圖檔）— 重建（`01` 寫搬，見 §8）

檔名 `<形狀>-<rrggbb>-<像素>.png`，是**依 Bot 的形狀與顏色畫出來的**通知用圖片（形狀、顏色存在 `state.json`）。沒有使用者上傳的頭像。

### 4.6 `demos/`（教它一項任務的錄製）— 搬

| 路徑 | 內容 |
|---|---|
| `demos/<YYYYMMDD-HHMMSS>/` | 一次錄製（錄製開始的本機時間） |
| `…/step_NN.jpg` | 第 NN 步（01 起，兩位數）當下的畫面 |
| `…/recording.mp4` | 整段錄影（H.264；有 ffmpeg 才有） |
| `…/frame_NNNNN.jpg` | 錄製中的畫面（每秒 2 張、寬度最多 1280）；做成錄影後刪掉，所以通常不在 |

`state.json` 裡 Bot 的示範紀錄用**絕對路徑**指到 `recording.mp4` 和每張 `step_NN.jpg`（互查 `01a`），遷移要改寫路徑。

### 4.7 Bot 的瀏覽器設定檔

| 路徑 | 內容 | 分類 |
|---|---|---|
| `browser/` | 「預設」設定檔（Chrome 系的使用者資料夾） | 原地 |
| `browser-profiles/<名字>/` | 另外加的設定檔 | 原地 |
| `firefox-profiles/<名字>/` | **`01` 沒列**：Linux 用 Firefox 當 Bot 瀏覽器時，AnyBot 自己的 Firefox 設定檔。Snap 版 Firefox 改放 `~/snap/firefox/common/AnyBot/<名字>/`，Flatpak 版放 `~/.var/app/org.mozilla.firefox/AnyBot/<名字>/`（在資料夾外） | 原地 |
| `browser-profile.txt` | 目前用哪個設定檔：純文字一行，值是 `預設`、另外加的設定檔名稱、`<使用者的 Firefox 設定檔名稱>（Firefox）`、或 `你的瀏覽器`（只有 macOS：用使用者自己的瀏覽器）。認不得就當 `預設` | 搬 |

### 4.8 `person-computer.txt` — 搬

純文字，一個主機編號（英數字，最多 64 字）：使用者上次用的是哪台電腦（帳號伺服器上的主機編號，見 §5.2 `hosts[].id`）。

### 4.9 `image-notes.json`（圖片描述快取）— 重建

JSON 物件：鍵 = 圖片識別碼（24 個十六進位字，由「這張圖是螢幕截圖還是附件」加上圖片內容算出的雜湊），值 = `[描述文字, 描述它的模型]`。超過 **500 筆**時整個清空重來。

### 4.10 `box/`（Bot 的電腦的設定）

| 路徑 | 分類 | 格式 |
|---|---|---|
| `box/config.json` | 搬 | 見下表 |
| `box/secrets.json` | 搬 | JSON 物件：名稱 → 值，**明文**（沒有加封），`0600` |
| `box/secrets.env` | 搬（可由 `secrets.json` 重建） | 每行 `名稱=值`；給容器的環境變數檔 |
| `box/snapshots/snapshot-YYYY-MM-DD-HHMMSS.tar.gz` | 待決定（`01`） | 見下 |
| `box/snapshots/snapshot-….tmp` | 丟 | 存到一半的快照 |
| `box/snapshot.tar.gz` | 待決定 | 更早版本的單一快照；OLD 最後一版第一次列快照時會改名搬進 `snapshots/` 成 `snapshot-<檔案時間>-legacy.tar.gz` |
| `box/recreate-resume.json` | 丟 | 重建做到一半的標記：`{"kind": 重建種類, "written_at_ms": 毫秒}`；超過 10 分鐘就不接續 |
| `box/update-request`、`box/update-answer` | 丟 | 更新進行中的交接檔 |
| `box/last-screen.png`、`box/last-screen-<畫面代號>.png` | 丟 | 最後一張畫面（每個 Bot 的桌面各一張） |

`box/config.json`（縮排排版 JSON，`0600`）：

| 鍵 | 型別 | 意思 |
|---|---|---|
| `docker_host` | 字串 | 空 = 這台的 Docker；否則例如 `ssh://me@server` |
| `auto_update_at` | 字串或 null | 排程更新時間 `HH:MM`（使用者時區）；null = 關閉。錯過後 60 分鐘內仍會跑 |
| `auto_update_done` | 字串 | 最後一次排程更新的日期 `YYYY-MM-DD` |
| `egress_via_mac` | 布林 | 網路流量改走你的電腦（也受管理員政策 `AllowBoxTrafficViaThisMac` 管） |
| `vnc_password` | 字串，**明文** | 看畫面用的 VNC 密碼（第一次用時產生） |
| `runtime` | 字串 | 容器隔離：空 = 標準；`runsc` = gVisor |

`box/secrets.json` 的規則：名稱以大寫字母開頭、只能有 `A–Z 0–9 _`；最多 **100** 個；值最多 **32 KiB**、不能有換行。`secrets.env` 由它產生，兩者內容一致。

快照：

| 項目 | 值 |
|---|---|
| 內容 | Bot 的電腦的家目錄 `/home/bot` 打包成 tar.gz（排除 `.cache`、`Cache`、`Code Cache`、`GPUCache`、`CacheStorage`、`ScriptCache`、`Crashpad`） |
| 什麼時候做 | 大約每 24 小時一份（沒人用超過 15 分鐘時）；更新、重設、復原之前也各做一份 |
| 留幾份 | 總數 ≤7 全留；否則：7 天內的全留，8–28 天的每個 ISO 週留最新一份，更舊的刪掉（通常 7＋3 ≈ 10 份） |
| 時間 | 看檔名（`-legacy` 後綴的也是）；檔名讀不出來才看檔案時間 |

### 4.11 技能（`../skills/`）— 搬

| 位置 | 說明 |
|---|---|
| `<資料夾>/../skills/` | OLD 最後一版的位置（資料夾的**上一層**；例如 macOS 是 `~/Library/Application Support/skills/`） |
| `<資料夾>/skills/` | 更早的位置，照讀、不搬；遷移兩處都要掃（`01`） |

每個技能一個資料夾：

```
<slug>/
  SKILL.md          （或 skill.md）
  scripts/          可執行的輔助腳本（選用）
  references/       參考文件（選用）
  assets/、examples/…（選用，任何相對路徑都算附帶檔）
```

也接受舊式單檔 `<名字>.md`。安裝時一個技能最多 **48 個檔、6 MiB**。

`SKILL.md` 的開頭（YAML front matter，`---` 包起來）認得的鍵：

| 鍵 | 意思 |
|---|---|
| `name` | 名稱（**必填**，沒有就不是技能） |
| `description` | 說明 |
| `enabled` | `false`／`0`／`no`／`off` = 停用；其他或沒寫 = 啟用 |
| `argument-hint`（或 `argument_hint`） | 參數提示 |
| `arguments` | 參數清單（YAML 清單） |
| `allowed-tools`（或 `allowed_tools`） | 允許的工具清單 |
| `context` | `fork` = 在支線裡跑 |

內文 = 給 Bot 的指示。可以有一個 ` ```steps ` 區塊：內容是 JSON 陣列，每個元素 `{"tool": 工具名, "args": 參數 JSON 文字, "targets": [字串…]（可無）}`，是重播用的步驟。

內建技能（`pdf`、`xlsx`、`docx`、`pptx`、`web-design`、`deep-research`、`browser-automation`、`skill-creator`、`看報快摘`、`每日打卡`、`螢幕報表存檔`）也會寫成資料夾放在這裡。`state.json` 的技能清單記著每個技能資料夾的絕對路徑（互查 `01a`）。

### 4.12 螢幕電腦（顯示另一台大腦的電腦）才有的檔 — `01` 沒列

| 路徑 | 內容 | 分類 |
|---|---|---|
| `brain-copy.json` | 大腦狀態的整份副本（跟 `state.json` 同形狀，含聊天），連不到大腦時先顯示它 | 重建 |
| `brain-copy.at.json` | 副本的標記：`brain`（大腦主機編號）、`client`（這台在大腦那邊的名字）、`version`、`at`（毫秒）。沒有標記 = 副本寫到一半，不採用 | 重建 |
| `brain-queue-<12 個十六進位>.json` | **還沒送到大腦的訊息與設定修改**（每個大腦一個檔，名字是大腦編號的雜湊）。內容：`brain`、`client`、`items`；秘密加封（§0.4）。佇列空了檔案就刪掉 | 丟（前提：遷移前佇列清空；遷移工具檢查到有這個檔就停下來） |
| `brain-queue-….unread-<毫秒>.json` | 讀不懂（別的版本寫的、或鑰匙打不開）的佇列，被移到旁邊 | 丟（列在遷移報告） |
| `brain-files/<16 個十六進位>-<檔名>` | 從大腦拿來的檔案快取（`01` 已列） | 重建 |

### 4.13 其他小檔

| 路徑 | 內容 | 分類 |
|---|---|---|
| `restored-from-account` | 「從帳號還原過」的標記；內容是還原的時間（RFC 3339），**不是空檔** | 丟 |
| `anybot.lock` | 同一個資料夾只能一份在跑：作業系統的檔案鎖，內容是持有者的程序編號 | 丟 |
| `anybot-icon.png` | 通知用圖示（128×128，Linux 用） | 重建 |
| `models/potion-multilingual-128M/` | 本機嵌入模型：`vocab.bin`、`table.i8`、`dims.json`（`{"model","rows","dim","median","unk"}`；`dims.json` 最後寫，有它才代表下載完整）；下載中的是 `*.part` | 重建 |

---

## 5. 自架帳號伺服器（relay）

### 5.1 資料夾在哪

| 部署方式 | 位置 |
|---|---|
| Docker（官方的伺服器安裝） | 資料卷 `relay-data`，掛在 `/data` |
| 直接執行 | `--data <資料夾>`；沒給就是目前目錄下的 `relay-data/` |
| macOS 服務 | `~/Library/Application Support/AnyBot/relay/data`（待確認：依安裝方式） |
| Linux systemd（舊的搬遷腳本） | `~/.anybot-relay/data` |

裡面只有：`data.json`、`backups/`、`sites/`、`vapid.pk8`。

### 5.2 `data.json` — 搬

緊湊 JSON，`0600`，整檔重寫。**讀到壞掉的檔案時伺服器拒絕啟動**（不會當成空的覆蓋）。時間都是 **Unix 秒**。最外層：

| 鍵 | 型別 | 意思 |
|---|---|---|
| `users` | 陣列 | 帳號 |
| `sessions` | 陣列 | 登入工作階段（手機、瀏覽器） |
| `hosts` | 陣列 | 這個伺服器上所有帳號的 AnyBot（電腦、雲端大腦、配對加入的手腳） |
| `subs` | 陣列 | 手機的推播訂閱 |
| `sites` | 物件 | 發布的網站：資料夾名稱 → 擁有者帳號 |

沒有 `invites` 鍵：註冊用的邀請碼來自設定（§5.6）；團隊邀請放在 `hosts[].members[]` 的 `pending`。

#### `users[]`

| 鍵 | 型別 | 意思 |
|---|---|---|
| `name` | 字串 | 帳號名稱：小寫、2–64 字、只能有英數字和 `. _ @ -` |
| `pass` | 字串 | 密碼雜湊，兩種格式（見下） |
| `created` | 整數（秒） | 建立時間 |
| `totp` | 字串 | 兩步驟驗證的共用密鑰（Base32，20 bytes → 32 字，**明文**）；空 = 沒開 |
| `totp_pending` | 字串 | 正在開啟、還沒驗證第一個碼的新密鑰 |
| `recover` | 字串陣列 | 備用碼的 SHA-256（十六進位）；用過的那格變空字串。開啟時發 8 組，每組 10 個十六進位字 |
| `recover_pending` | 字串陣列 | 跟 `totp_pending` 一起發的備用碼雜湊，驗證成功才取代 `recover` |

密碼雜湊：

| 格式 | 內容 |
|---|---|
| `argon2id$<鹽 32 個十六進位>$<雜湊 64 個十六進位>` | 現行：Argon2id v1.3，記憶體 19,456 KiB、2 回合、平行 1、輸出 32 bytes；鹽 16 bytes |
| `<鹽>$<雜湊 64 個十六進位>` | 更早的帳號：PBKDF2-HMAC-SHA256，**120,000** 回合，鹽是 `$` 前面那段**字串的 UTF-8 bytes**，輸出 32 bytes。舊版照讀、不主動改寫 |

兩步驟驗證碼：TOTP、HMAC-SHA1、30 秒、6 位數、前後各容許一個時間片；同一個碼 2 分鐘內不能用第二次。

#### `sessions[]`

| 鍵 | 意思 |
|---|---|
| `hash` | 權杖的 SHA-256（十六進位）；權杖本身是 64 個十六進位字，只在登入時給出 |
| `user` | 帳號 |
| `device` | 裝置名稱（最多 40 字） |
| `created` | 登入時間（秒）；15 分鐘內算「剛登入」（某些敏感操作要） |
| `last` | 最後使用（秒；最多每分鐘更新一次） |

保留：**90 天**沒用就失效並移除；每個帳號最多 **50** 個，超過刪最久沒用的。

#### `hosts[]`

| 鍵 | 意思 |
|---|---|
| `id` | 主機編號，16 個十六進位字（`state.json`、`person-computer.txt` 會引用） |
| `name` | 電腦名稱（同帳號同名的第二台變成「名字 (2)」） |
| `user` | 擁有者帳號 |
| `token_hash` | 主機權杖的 SHA-256；權杖本身 64 個十六進位字，存在那台 AnyBot 的 `state.json`（加封） |
| `created` | 秒 |
| `last_seen` | 最後上線（秒；最多每分鐘更新） |
| `paired` | `true` = 用 6 位數配對碼加入的手腳：只能被叫去做事，不能指揮別台、不能改大腦、不能覆蓋備份、不能發布網站 |
| `members` | 團隊：分享給哪些帳號，見下 |
| `managed` | `true` = 伺服器的大腦管理員建的雲端大腦 |

`members[]`：`user`（帳號）、`role`（`admin` = 跟擁有者一樣能在手機核准、看畫面；`member` = 只能聊天和附檔）、`pending`（`true` = 邀請了還沒接受，沒有任何權限）。

每個帳號最多 **100** 台主機。

#### `subs[]`

| 鍵 | 意思 |
|---|---|
| `user` | 帳號 |
| `sub` | 瀏覽器的推播訂閱：`{"endpoint": https 網址（≤2048 字）, "keys": {"p256dh": …（≤256 字）, "auth": …（≤128 字）}}` |
| `created` | 秒 |
| `kinds` | 要收哪幾種：`need`（需要你）、`done`（做完了）、`routine`（例行任務）、`file`（收到檔案）的子集合；**空陣列 = 全部**；`["-"]` = 全部不要 |

每個帳號最多 **16** 個訂閱（多的刪最舊）。推播服務回報訂閱失效時刪掉。訂閱綁著 `vapid.pk8` 的公鑰。

#### `sites`

`{"<帳號把非英數字換成 -> - <網站名>": "<帳號>"}`。網站名：小寫英數字和 `-`、最多 40 字。每個帳號最多 **50** 個。兩個帳號算出同一個資料夾名時，先佔的贏。

### 5.3 `backups/<帳號雜湊>.bin` — 丟（決定 5）

| 項目 | 值 |
|---|---|
| 檔名 | 帳號名稱（小寫）的 SHA-256 十六進位 ＋ `.bin`；一個帳號一個檔，每次上傳覆蓋 |
| 裡面是什麼 | 使用者電腦上整份 `state.json` 的內容（**含每個 Bot 的聊天與過去的對話**；不含帳號連線設定那一塊；金鑰欄位在裡面是明文，靠整包加密保護） |
| 加密 | AES-256-GCM；前 12 bytes 是 nonce，後面是密文＋16 bytes 標籤；**沒有 AAD** |
| 鑰匙 | PBKDF2-HMAC-SHA256（帳號密碼，鹽 = `anybot-backup:<帳號小寫>`，**600,000** 回合，32 bytes）。更早的備份用 120,000 回合 |
| 誰拿著鑰匙 | 只有使用者的電腦：登入時算好，存在那台的 `state.json`（加封）。伺服器沒有，打不開。雲端大腦沒有這把鑰匙，不會上傳 |
| 誰能上傳 | 非配對加入的主機；最大 **200 MB** |
| 多久一次 | 自動備份打開時，距上次超過 24 小時就再傳一次 |

### 5.4 `sites/<資料夾名>/` — 搬

發布的網頁原樣放在這裡（任意相對路徑的檔），對外網址 `/s/<資料夾名>/`；網址指到資料夾時給 `index.html`。重新發布 = 整個資料夾刪掉重寫。擁有者記在 `data.json` 的 `sites`。

### 5.5 `vapid.pk8` — 搬

網頁推播的伺服器簽章金鑰：PKCS#8 DER，ECDSA P-256，`0600`。第一次啟動時產生。手機訂閱用的是它的公鑰（未壓縮點、base64url）；換掉 = 所有手機的推播訂閱失效。

### 5.6 伺服器的設定（環境變數、`.env`、命令列）— 搬（轉進新版設定，決定 7）

| 名稱 | 意思 |
|---|---|
| `--port`（或 `PORT`） | 埠，預設 8080 |
| `--bind` | 聽哪個位址，預設 `0.0.0.0` |
| `--data` | 資料夾 |
| `ANYBOT_RELAY_INVITE` | 註冊邀請碼。有設時連第一個帳號都要；沒設時只有第一個帳號能免費註冊，之後不開放 |
| `ANYBOT_RELAY_PUBLIC_URL` | 對外網址（發布網站的連結用） |
| `ANYBOT_RELAY_MANAGER_TOKEN` | 大腦管理員的鑰匙（32 字以上；空 = 沒有管理員） |
| `ANYBOT_RELAY_PROXY_SECRET`（`.env` 裡叫 `ANYBOT_PROXY_SECRET`） | 只相信帶這個密語的反向代理報的來源位址 |
| `RELAY_TRUST_PROXY` | `1` = 相信反向代理報的來源位址 |
| `ANYBOT_RELAY_CONTACT` | 推播服務的聯絡方式（`mailto:` 或 `https://`） |

`.env` 其他鍵（`ANYBOT_USER`、模型金鑰、語音、Tailscale…）屬於整套伺服器的安裝設定，見 `01` §E 與安裝規格。

---

## 6. Docker 與雲端大腦

### 6.1 雲端大腦的資料卷 — 搬

| 項目 | 值 |
|---|---|
| 容器 | `anybot-brain-<代稱>`，代稱 = 帳號轉小寫、非英數字換成 `-`、最多 30 字，再加 `-` 和帳號 SHA-256 的前 6 個十六進位字 |
| 資料卷 | 同名 `anybot-brain-<代稱>`，掛在 `/data` |
| 單一帳號舊式部署的大腦 | 服務 `anybot`，資料卷 `anybot-data` 掛在 `/data`（同一套格式） |
| 資料夾 | 容器設 `ANYBOT_DATA_DIR=/data`，所以 **`/data` 就是一份完整的「資料夾」，格式跟 §1–§4 完全一樣** |

跟本機資料夾不一樣的地方：

1. **沒有系統鑰匙圈**：映像裡沒有 `secret-tool`，保險庫鑰匙在 `/data/vault.key`（§6.2）。
2. **技能的位置在資料卷外面（待確認實際影響）**：技能放在「資料夾的上一層」，在容器裡會落在 `/skills`，**不在 `/data` 資料卷裡**；容器被管理員重建時可能不見。資料卷裡的 `/data/skills/`（更早的位置）才會留下。遷移時兩處都要找；容器已經重建過的，`/skills` 可能是空的。
3. 多了快取（重建）：`/data/cache/npm`、`/data/cache/uv`、`/data/cache/uv-python`（連接器用的套件）、`/data/models/`。
4. 大腦的帳號登入（主機編號＋主機權杖）由管理員在建容器時用環境變數 `ANYBOT_HOST_ID`、`ANYBOT_HOST_TOKEN` 給，大腦再存進自己的 `state.json`；**沒有備份鑰匙**，不做帳號備份。
5. 給 Docker 的大腦（擁有者，和 `ANYBOT_BRAINS_DOCKER` 列的帳號）有自己的 Bot 電腦：容器 `anybot-box-<代稱>`、家目錄資料卷 `anybot-box-home-<代稱>`。單一帳號舊式部署的大腦用 `anybot-brain-box`。

### 6.2 雲端大腦的保險庫鑰匙在哪（解 `01` 的待確認）

**在大腦自己的資料卷裡：`/data/vault.key`**（64 個十六進位字，`0600`）。容器裡沒有系統鑰匙圈，舊版照 §3.1 的順序落到這個檔；建容器時沒有設 `ANYBOT_VAULT_DIR`、`ANYBOT_VAULT_KEY_FILE`（自己改過容器設定的要用 `docker inspect` 確認）。

所以：

- 大腦的 `vault.bin`、`state.json` 的加封欄位、ChatGPT 與連接器權杖，**在伺服器上就解得開**，遷移可以在伺服器上直接跑，不需要使用者的電腦。
- 反過來說：拿得到資料卷的人（伺服器管理員）就解得開。這跟 `02` D1 的取捨是同一件事。

### 6.3 大腦管理員自己的資料卷 `brains-data` — 丟

掛在 `/data`。除了一般資料夾的檔（紀錄檔等），有：

| 檔 | 內容 |
|---|---|
| `brains-since` | 管理員第一次執行的時間（Unix 秒）；在這之前建的帳號不自動給雲端大腦 |
| `brains-owner` | 伺服器擁有者的帳號名稱（第一次看到的第一個帳號） |

新版沒有「每個帳號一個大腦容器」（`02` §10），這兩個檔沒有對應。擁有者是誰，遷移時可以參考 `brains-owner`（待決定）。

### 6.4 Bot 的電腦 — 原地（`01`）

容器 `anybot-box`、家目錄資料卷 `anybot-box-home`（掛在 `/home/bot`，每個 Bot 一個 `bot-<id>/`）。名字可用 `ANYBOT_BOX_NAME`、`ANYBOT_BOX_VOLUME` 改；映像 `anybot-box:1`（或 `ANYBOT_BOX_IMAGE`）。

---

## 7. `01` 三個待確認的答案

| 問題 | 答案 |
|---|---|
| `sent.jsonl` 是做什麼的 | 「已送帳本」：Bot 透過瀏覽器真的送出去的每一則（回覆、貼文、訊息），送出後確認過才記。用來擋「同一個帳號 60 天內回覆同一則貼文兩次」與「24 小時內在同一頁送同樣的字」，也讓 Bot 查最近送過什麼。保留 60 天。建議**搬**（§2.4）。 |
| `vault-index.json` 什麼時候匯入 `vault.bin` | 第一次列出保險庫、而且 `vault.bin` 的 `imported_keychain` 是 `false` 時：打開保險庫畫面、**任何一輪有 Bot 瀏覽器的對話開始時**、填入前的檢查。匯入清單裡仍在鑰匙圈的登入（Linux 直接從 Secret Service），完成後設 `imported_keychain = true`。遷移時若還是 `false`，要在使用者電腦上補做（§3.5）。 |
| 雲端大腦的保險庫鑰匙在哪 | 資料卷裡的 `/data/vault.key`，伺服器上解得開（§6.2）。 |

## 8. 跟 `01` 不一致或 `01` 沒列的（請回頭改 `01`）

| # | 項目 | `01` 寫的 | 實際 | 建議 |
|---|---|---|---|---|
| 1 | `avatars/` | Bot 頭像，搬 | 依形狀與顏色畫出來的通知圖片，沒有上傳的頭像 | 改**重建** |
| 2 | `restored-from-account` | 空檔 | 內容是還原時間（RFC 3339） | 分類不變（丟） |
| 3 | `sent.jsonl` | 待確認 | 已送帳本（§2.4） | 改**搬** |
| 4 | `vault-index.json` | 只有網站和帳號、匯入時機待確認 | 只有 macOS 有；Linux 直接讀 Secret Service；匯入時機見 §3.5 | 加註：`imported_keychain = false` 時遷移要補匯入 |
| 5 | 管理員政策 | macOS、Linux | Windows 也有（讀檔，§4.2）；Linux 有 `/etc` 的檔時完全不看使用者的檔；`true` 只是「允許」，不是強制打開 | 補進 C 節 |
| 6 | 保險庫鑰匙 | 系統鑰匙圈或 `vault.key` | 另有 `ANYBOT_VAULT_DIR`、`ANYBOT_VAULT_KEY_FILE` 兩個環境變數改位置；同一把也封 ChatGPT、連接器權杖 | 補進 B14 環境變數清單 |
| 7 | 沒列的檔 | — | `vault-index-imported`、`firefox-profiles/`（及 Snap／Flatpak 的位置）、`brain-copy.json`、`brain-copy.at.json`、`brain-queue-*.json`、`*.unread-*.json`、`checkpoints/blobs/`、`drops/<id>.bin`、`drops/in-<id>.part`、`models/<模型>/`、`box/last-screen-<代號>.png`、`box/snapshots/*.tmp`、`logs/crash-*.seen.txt` | 補進 B 節（分類見本檔） |
| 8 | 伺服器資料卷 | 只列 `relay-data` | 還有 `anybot-data`（單一帳號大腦）、`brains-data`（管理員）、`caddy-data`（HTTPS 憑證，Caddy 的格式） | 補進 D／E 節；`caddy-data` 新版不附 Caddy（`02` A11），建議丟 |
| 9 | 遷移前提 | 所有 Bot 閒置、傳檔送完、Bot 的電腦沒在重建 | 另外要：螢幕電腦的 `brain-queue-*.json` 不存在（佇列清空）；`oplog/bot-*.jsonl` 全是空的 | 補進 F.2 |
| 10 | 當機報告 | 丟 | 舊版從不自動刪 `crash-*.txt` | 分類不變 |

## 9. 遷移風險（由大到小）

1. **保險庫鑰匙找錯或找不到 = 秘密全滅**：鑰匙可能在鑰匙圈、`vault.key`、或環境變數指的地方，Linux 甚至可能兩處都有。同一把鑰匙封著 `vault.bin`、`state.json` 的金鑰、ChatGPT 與連接器權杖。每個候選都要試，用能解開的那把；全部失敗要明講，不能默默產生新鑰匙（會讓舊資料永遠打不開）。
2. **SQLite 的 WAL**：舊資料夾唯讀，直接開會漏掉 `-wal` 裡的最新記憶。三個檔一起複製再開（§0.3）。
3. **絕對路徑**：聊天的附件、示範錄製、技能資料夾都用絕對路徑記。換位置（尤其換電腦、換作業系統、雲端大腦搬進伺服器）要改寫。
4. **同一欄位明文、加封兩種都有**：沒有鑰匙時舊版存明文；遷移要兩種都接受，且不要把加封字串當成真的金鑰寫進新版。
5. **雲端大腦的技能可能不在資料卷**（§6.1 第 2 點）。
6. **明文秘密**：`box/secrets.json`、`box/config.json` 的 `vnc_password`、hooks 的 `headers`、帳號伺服器的 `totp` 都是明文；新版要決定是否加密存放。
7. **用量的供應商是顯示名稱**：要對照表；舊名稱可能不只九種。
8. **帳號伺服器的兩種密碼雜湊**：新版必須能驗證 Argon2id 與 PBKDF2 兩種（或登入成功時改寫成新格式）；`vapid.pk8` 換掉推播全斷。
9. **記憶的兩份來源**：`memory.db`（完整）與 `state.json` 的副本（≤200 條）；Bot 0 是「關於你」，不能當成一個 Bot 建出來。
10. **`audit.db` 的 `prompt_at` 可能缺欄**：讀的時候要用欄位清單判斷。

## 10. 保留數字總表

| 東西 | 數字 |
|---|---|
| `audit.db` | 30 天 |
| 記憶「最近刪除」 | 30 天 |
| 做過的事 → 每週一行 | 30 天後 |
| 每週一行 → 每月一行 | 180 天後 |
| 很久沒用的記憶（只提示） | 180 天 |
| `fact` 長度 | 500 字 |
| `state.json` 記憶副本 | 每個 Bot 200 條 |
| 記憶備份 | 7 份（每天一份） |
| `docs.db` 壓實 | 每天；空頁 > 16 MiB 才 VACUUM |
| 文件索引的檔案大小 | 32 bytes–2,000,000 bytes |
| 檢查點 | 7 天；blob 8 天；單檔 2 MiB |
| `outputs/` | 7 天；單檔 20 MiB |
| `sent.jsonl` | 60 天；每則存 300 字 |
| 紀錄檔 | 7 天；每天 20 MiB |
| 當機報告 | 不刪 |
| 用量 | 不刪 |
| `image-notes.json` | 超過 500 筆清空 |
| 傳檔 | 7 天；重試 15 秒；單檔 2 GiB；文字 100,000 字；收件紀錄 100 筆 |
| 附件 | 單檔 50 MiB；沒被引用且超過 1 天的每晚刪 |
| 快照 | 約每 24 小時；7 天內全留＋8–28 天每週一份 |
| Bot 電腦的秘密 | 100 個；每個 32 KiB |
| 區網憑證 | 800 天 |
| ChatGPT 存取權杖 | 約 1 小時（到期前 5 分鐘換新） |
| 帳號伺服器工作階段 | 90 天沒用；每帳號 50 個 |
| 主機／推播訂閱／網站 | 每帳號 100／16／50 |
| 帳號備份 | 每帳號 1 份；200 MB；24 小時一次 |
| 配對碼 | 10 分鐘、用一次 |

---

## 盤點統計

| 項目 | 數量 |
|---|---|
| 涵蓋的檔案／資料夾／鑰匙圈項目（本機 §1–§4） | 71 |
| 涵蓋的檔案／資料卷／設定項目（帳號伺服器、Docker §5–§6） | 14 |
| SQLite 資料庫 | 3 個（含 1 份備份格式） |
| SQLite 表 | 8 個（一般表 5：`memories`、`meta`×2、`docs`、`audit`；FTS5 虛擬表 3：`memories_fts`、`memories_words`、`docs_fts`） |
| SQLite 欄位 | 35 個（`memories` 12、`audit` 11、`docs` 5、`meta` 2×2、FTS 表 1×3） |
| 索引 | 4 個具名（`memories_bot`、`memories_alive`、`audit_bot_at`、`audit_call`）＋2 個主鍵／唯一鍵 |
| 觸發器 | 7 個 |
| `meta` 鍵 | 2 個（`embed_engine`、`last_tidy`） |
| 紀錄種類 | 工作紀錄 9 種；操作紀錄列 5 種；記憶 3 種；檢查點位置 5 種；包裹狀態 6 種 |
| JSON 檔的鍵（不含 `state.json`） | 保險庫 30（項目 22、最外層 2、地址 6）、ChatGPT 10、連接器 8、用量 9、已送帳本 8、包裹 12、收件 7、檢查點 7、Bot 電腦設定 6、hooks 8、帳號伺服器 `data.json` 33（最外層 5、帳號 7、工作階段 5、主機 9、成員 3、訂閱 4；另加推播訂閱內 3 個） |
| 鑰匙圈項目 | 1 個現行（`AnyBot vault key`）＋舊版網站登入（macOS、Linux 各一種存法） |
| 加密格式 | 3 種（`vault.bin`、`enc:v1:` 加封值、帳號備份） |

## 待確認

1. `usage/*.jsonl` 較早版本的 `provider` 有沒有用過其他名稱（需要拿實際的舊檔看）。
2. `anybot-btw-<編號>` 的編號是支線 Bot 的編號還是主 Bot 的。
3. 舊版 Linux 存在 Secret Service 的網站登入，當初的標籤是什麼（只影響找得到找不到，匯入是用屬性搜）。
4. `lan-key.der` 的演算法是否一律 ECDSA P-256。
5. Windows 上管理員政策實際讀到的位置（§4.2；沒有在 Windows 上驗證過）。
6. 雲端大腦容器重建後 `/skills` 裡的技能是否真的不見（§6.1 第 2 點）；實際伺服器上有多少技能只在 `/skills`。
7. 帳號伺服器在 macOS 服務部署時的資料夾位置是否固定（§5.1）。
8. 雲端大腦的管理員建容器時，有沒有使用者自行加上 `ANYBOT_VAULT_DIR`／`ANYBOT_VAULT_KEY_FILE`（要在實際伺服器上 `docker inspect` 確認）。
9. 當機報告與 `.seen.txt` 是否要在遷移報告裡列出（目前分類為丟）。
