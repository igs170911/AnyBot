# 01a · 舊版資料格式（欄位級）：`state.json`、聊天檔、大腦副本

> 來源：看過舊版的一方盤點 OLD 的資料格式（日期 2026-11-24），不含程式結構。
> 用途：遷移工具的輸入規格。新版要讀 OLD 最後一版寫出的這幾個檔，逐欄位轉進新版；每一欄都標了去處（搬／重建／丟）或等你決定。
> 互查：`01-data-inventory.md`（檔案清單與分類、遷移規則草案）、`00-boundaries.md`（B11、B12，決定 4、5、8）。

## 0. 通則

### 0.1 檔案

- 三種檔都是 UTF-8 的 JSON，**不縮排**（一整行），權限只有使用者本人可讀寫（0600），資料夾 0700。
- `state.json` 每次存檔都整份重寫；聊天檔只有那個 Bot 的聊天變了才重寫。寫的順序：先各個聊天檔，最後 `state.json`。
- 鍵名大小寫照抄。大部分是小寫加底線；例外：`daysOfWeek`、`monthDay`、`weekdayOfMonth`、`minutesBefore`、`apiKey`、`match`，還有幾個大寫開頭的列舉值（見各表）。
- **多出來的鍵**：舊版讀的時候直接忽略。
- **`null`**：表裡寫「可為 null」或「缺少時」寫「沒有」的欄位，`null` 等於沒有這個鍵；其他欄位（包括有預設值的）寫 `null` 都算型別錯。
- **必填**（表裡「缺少時」寫「必填」）：缺了這個鍵；或任何欄位型別錯（例如上一條的 `null`）；或列舉值不在清單裡——舊版就把**整個檔**當成壞檔：
  - `state.json` 讀不了：原檔複製一份成 `state.json.damaged-<Unix 秒>`，然後當成第一次開（沒有 Bot）。檔案在、但讀不出文字（權限、磁碟錯誤）：改名成 `state.json.unreadable-<Unix 秒>`，改名也失敗的話，這次執行不會存檔蓋過它。
  - 聊天檔讀不了：改名成 `bot-<id>.json.damaged-<Unix 秒>`，那個 Bot 的聊天當成空的。
  - 遷移工具碰到這種檔，要寫進遷移報告（是否比舊版寬鬆、救回部分內容：待決定，見文末）。
- 整數欄位存成不帶小數的數字；「小數」欄位存成 JSON 數字（可能帶小數）。

### 0.2 表格欄位怎麼讀

| 欄 | 意思 |
|---|---|
| 鍵 | JSON 的鍵名。巢狀用路徑寫：`bots[].routines[].schedule.time` |
| 型別 | 字串、整數、小數、布林、陣列〈…〉、物件、列舉（字串，值列在意思欄） |
| 缺少時 | 舊版讀到沒有這個鍵時用的值；「必填」見上 |
| 寫入 | **寫**＝一律寫；**空不寫**＝值是空字串、0、false、空陣列或「沒有」時這個鍵不寫；**只讀**＝舊檔有就讀，存檔不再寫；**另檔**＝平常寫在別的檔；**不寫**＝只在記憶體，磁碟上沒有 |
| 秘密 | **封存**＝用保險庫的鑰匙加密後才寫（格式見 §1）；**明文秘密**＝是秘密但沒有加密；**雜湊**＝只存雜湊；「—」＝不是秘密 |
| 遷移 | 搬／重建／丟／待決定（定義同 `01`） |

### 0.3 時間的單位（混用，遷移時要逐欄換算）

| 單位 | 用在 |
|---|---|
| 毫秒（Unix） | 訊息 `at`、交付確認 `at`、追進度 `*_ms`、任務事件 `noted`／`at`／`heartbeat`／`finished`、目標與計畫的 `since`、支線 `at`、上一輪結算 `settled_at_ms`、頁面 `created`／`updated`／`history[].at`、工作流執行步驟的 `asked_at` |
| 秒（Unix） | 例行任務 `created_at`／`last_run`／`next_run`／`runs[].started`／`runs[].ended`、工作流 `created_at`／`updated_at`／`next_run`／`versions[].at`／`runs[].started`／`runs[].ended`／`queued[].at`、待改進 `first`／`last`、帳號 `last_backup`、登入裝置 `created`／`last_seen` |
| 日期整數 `yyyymmdd` | `garden_daily`、`garden_weekly` |
| 本地時間文字 `YYYY-MM-DD HH:MM` | 教學示範 `created` |
| 時刻文字 `HH:MM`（24 小時） | 排程 `time` |

### 0.4 寫死在資料裡的絕對路徑

下列欄位存的是**這台電腦上的絕對路徑**，資料夾換位置後要改寫：`bots[].messages[].attachments[].path`（在 `attachments/bot-<id>/` 下）、`bots[].demos[].video`／`shots[]`（在 `demos/<時間>/` 下）、`skills[].dir`、工作流「素材」節點的 `files[]` 與執行步驟的 `files[]`、`doc_folders[]`、`bots[].workdir`。`remote_computers[].dir` 是遠端電腦上的路徑，不用改。

### 0.5 編號之間的引用

遷移時如果新版換了編號，下列引用要跟著換：

| 編號 | 來源 | 被誰引用 |
|---|---|---|
| Bot 編號（正整數） | `bots[].id`，新的從 `next_id` 發 | `primary_bot`、`selected`、`pin_order[]`、`bots[].members[]`、`bots[].branch_of`、`channels[].bot_id`、`pages[].bot`、工作流節點與執行步驟的 `bot`、任務事件 `bot` 與 `id`（`<bot>-<時間>`）、訊息 `author`／`from_bot`；檔名 `chats/bot-<id>.json`、`chats/threads/bot-<id>.json`、`attachments/bot-<id>/`、`oplog/bot-<id>.jsonl`、`checkpoints/bot-<id>.jsonl`、`~/AnyBot 工作區/bot-<id>/`；`memory.db` 的 Bot 欄（0 ＝「關於你」） |
| 例行任務編號 | `bots[].routines[].id`（全部 Bot 共用一個序號），新的從 `next_routine_id` 發 | 訊息 `routine`；外部服務呼叫的 webhook 位址 `http://<這台>:8792/hooks/<例行任務編號>` |
| 工作流編號 | `workflows[].id`（現有最大值＋1） | webhook 位址 `/hooks/<1000000＋工作流編號>`；訊息 `seen` 的文字（「工作流「早報」（#1）的畫布」） |
| 頁面編號 | `pages[].id`，新的從 `next_page_id` 發 | 訊息文字裡的連結 `[標題](page:<編號>)` |
| 訊息管道編號 | `channels[].id`，新的從 `next_channel_id` 發 | — |
| 支線編號 | `bots[].branches[].id`（就是那個暫時 Bot 的編號，支線結束就沒了） | 訊息 `branch` |
| 附件編號 | 附件 `id`（12 個十六進位字元） | 任務事件 `artifacts[].id` |
| 長輸出編號 | `outputs/r<12 個十六進位>.txt` | 步驟結果文字裡的「…（中間省略 N 個字；完整內容用 read_result 讀：id=r…）…」 |
| 名字 | 技能名、電腦名、MCP 伺服器名 | `bots[].skills[]`、`bots[].computers[]`、`default_computer`、例行任務與追進度的 `computer`、允許規則 `mcp:<名字>` |

## 1. 封存（加密）的欄位

### 1.1 封存的格式

- 封存後的值是字串：`enc:v1:` 後面接一串**小寫十六進位**。
- 十六進位還原成位元組後：**前 12 bytes 是 nonce，其餘是 AES-256-GCM 的密文，最後 16 bytes 是驗證碼**。附加資料（AAD）是 ASCII 字串 `anybot-state-v1`。解開後是 UTF-8 的原文。
- 鑰匙就是保險庫（`vault.bin`）的鑰匙，32 bytes：
  - macOS：鑰匙圈的一般密碼項目，服務名 `AnyBot vault key`、帳號 `anybot`，內容是 64 個十六進位字元。
  - Linux：Secret Service，屬性 `application=anybot`、`kind=vault-key`，標籤 `AnyBot vault key`，內容同上。
  - 都沒有（Windows、沒有 Secret Service 的 Linux）：資料夾裡的 `vault.key`，內容同上。
  - 環境變數 `ANYBOT_VAULT_KEY_FILE` 設了就用那個檔；`ANYBOT_VAULT_DIR` 設了，`vault.bin` 和 `vault.key` 都改到那個資料夾。
- 空字串不封存；已經是 `enc:v1:` 開頭的不再封一次。
- **拿不到鑰匙時照明文存。** 所以同一個欄位在磁碟上可能是：空字串、明文、`enc:v1:…` 三種之一，遷移工具三種都要接受。
- **打不開的保持原樣**（鑰匙換了、鑰匙圈鎖住、檔案從別台搬來）：舊版讀進來還是 `enc:v1:…`，下次存檔照原樣寫回，不會變成空的。遷移時打不開的值不能當成金鑰用，要列進遷移報告。

### 1.2 哪些鍵會被封存

規則看**鍵名**，不看位置，`state.json` 全檔任何一層都適用：

- 鍵名是 `api_key`、`token`、`host_token`、`backup_key`、`link_key` 的字串值；
- 鍵名是 `link_keys` 的陣列裡每個字串；
- 鍵名是 `env` 或 `headers` 的物件裡每個字串值；
- 鍵名是 `password`，而且**同一個物件裡也有 `url` 鍵**；
- 鍵名是 `key`，而且同一個物件裡也有 `base_url` 或 `url`（OLD 最後一版的 `state.json` 沒有這種欄位）。

注意：
- 聊天檔（`chats/…`）**完全不封存**，全部明文。
- 但 `state.json` 裡出現的訊息（`bots[].regen_undo[]`，或聊天檔寫失敗時整份聊天暫存在 `state.json`，見 §3）也照同一規則處理：訊息裡任何叫 `token`、`api_key`… 的鍵（例如模型內容區塊裡步驟參數）也會被封存。遷移時要對 `state.json` **全檔**做同樣的開封，不能只開下表那幾欄。

### 1.3 實際會被封存的欄位（OLD 最後一版）

| # | 欄位 | 內容 |
|---|---|---|
| 1 | `providers.<代號>.api_key` | 模型供應商的 API 金鑰 |
| 2 | `voice.listen.api_key` | 語音「聽」服務的金鑰 |
| 3 | `voice.speak.api_key` | 語音「說」服務的金鑰 |
| 4 | `channels[].token` | 訊息管道的權杖（Teams 是 refresh token；Twilio 是「Account SID:Auth Token」） |
| 5 | `mcp_servers[].env.<名稱>` | MCP 伺服器的環境變數值（每一個） |
| 6 | `mcp_servers[].headers.<名稱>` | MCP 伺服器的 HTTP 標頭值（每一個） |
| 7 | `mcp_servers[].password` | MCP 伺服器的登入密碼 |
| 8 | `remote.host_token` | 這台在帳號伺服器上的金鑰 |
| 9 | `remote.backup_key` | 加密備份的鑰匙（由帳號密碼導出，64 個十六進位字元） |
| 10 | `remote.link_key` | 手腳電腦：大腦配對時給的簽章鑰匙 |
| 11 | `remote.link_keys[]` | 大腦：配對時發出去的簽章鑰匙（每一個） |
| 12 | 觸發條件 `github` 的 `token` | GitHub 權杖（例行任務和工作流觸發器都有） |
| 13 | 觸發條件 `slack` 的 `token` | Slack 權杖 |
| 14 | 觸發條件 `sentry` 的 `token` | Sentry 權杖 |
| 15 | 觸發條件 `pagerduty` 的 `token` | PagerDuty 權杖 |

### 1.4 是秘密但**沒有**封存的（風險）

| 欄位 | 磁碟上是什麼 |
|---|---|
| 觸發條件 `linear` 的 `apiKey` | **明文** Linear API 金鑰（鍵名不在封存清單裡） |
| 觸發條件 `webhook` 的 `secret` | **明文** webhook 密語（24 個十六進位字元；呼叫方帶 `Authorization: Bearer <secret>`） |
| `remote.password` | 手機登入密碼的**雜湊**（不是封存，見 §7.4） |
| `remote.devices[].token_hash` | 手機登入權杖的 SHA-256（64 個小寫十六進位字元） |
| 聊天檔全部、`brain-copy.json` 全部 | 明文（見 §9、§11） |

## 2. `state.json` 頂層

一個 JSON 物件。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `bots` | 陣列〈Bot 設定〉 | 必填 | 側邊欄裡所有 Bot 和群組（§3）。群組也是一個 Bot，`members` 不是空的 | 寫 | 內含 | 搬 |
| `primary_bot` | 整數 | 必填 | 主要 Bot 的編號（0 ＝ 沒有）；找不到就改成第一個 Bot | 寫 | — | 搬 |
| `selected` | 整數 | 必填 | 上次選著的 Bot；找不到就改成主要 Bot | 寫 | — | 搬 |
| `next_id` | 整數 | 必填 | 下一個新 Bot 的編號 | 寫 | — | 重建 |
| `theme` | 列舉 | 必填 | 主題：`"Light"`、`"Dark"`、`"System"` | 寫 | — | 搬 |
| `accent` | 列舉 | 必填 | 你的訊息泡泡：`"BotColor"`（跟著 Bot 的顏色）、`"Black"`（黑底白字）、`"Quiet"`（淡灰，預設） | 寫 | — | 搬 |
| `bubble_quiet_offered` | 布林 | false | 「淡灰泡泡變成預設」的一次性轉換做過了沒（§10） | 寫 | — | 丟 |
| `shell_tone` | 小數 | 1.0 | 外觀色調，0 冷 … 1 暖 | 寫 | — | 搬 |
| `shell_accent` | 字串 | `"clay"` | 強調色：`clay` 黏土、`indigo` 靛藍、`moss` 苔綠、`teal` 藍綠、`rose` 玫紅、`amber` 琥珀、`slate` 石板、`ink` 墨黑 | 寫 | — | 搬 |
| `shell_contrast` | 小數 | 1.0 | 對比，0.75 柔和 … 1.35 鮮明 | 寫 | — | 搬 |
| `shell_template` | 字串 | `""` | 風格；`""` ＝ 自訂（用上面三個），或 `paper` 紙、`night` 夜、`ink` 墨、`moss` 苔、`ocean` 海、`candy` 糖、`sunset` 夕陽、`terminal` 終端機 | 空不寫 | — | 搬 |
| `dense` | 布林 | false | 緊湊間距 | 寫 | — | 搬 |
| `celebrate` | 布林 | true | 工作完成時的慶祝動畫 | 寫 | — | 搬 |
| `motion` | 字串 | `""` | 頭像動態：`""` 自動、`"full"` 完整、`"calm"` 減少 | 寫 | — | 搬 |
| `providers` | 物件〈供應商代號 → 供應商設定〉 | `{}` | 各模型供應商的金鑰、位址、模型（§5） | 寫 | 內含 | 搬 |
| `sidebar_collapsed` | 布林 | false | 側邊欄收起 | 寫 | — | 搬 |
| `embed_model` | 字串 | `""` | 用意思找記憶：`""` 或 `"local"` ＝ 本機模型；`"off"` ＝ 關；`"ollama"` ＝ 用 Ollama；其他字串也當成用 Ollama（待確認：是不是 Ollama 的模型名） | 寫 | — | 搬 |
| `doc_folders` | 字串陣列 | `[]` | 文件索引的資料夾（可以是 `~/…`）；空 ＝ 這台的「文件」資料夾 | 寫 | — | 搬 |
| `sidebar_width` | 小數 | 280 | 側邊欄寬度（點） | 寫 | — | 搬 |
| `local_execution` | 布林 | false | 允許在這台電腦上執行（不只在 Bot 的電腦上） | 寫 | — | 搬 |
| `remote_computers` | 陣列〈遠端電腦〉 | `[]` | 用 SSH 連的其他電腦（§7.1） | 寫 | — | 搬 |
| `auto_review` | 布林 | false | 自動審核 | 寫 | — | 搬 |
| `review_allow_rules` | 字串陣列 | `[]` | 自動審核的「允許」規則（使用者寫的句子） | 寫 | — | 搬 |
| `review_ask_rules` | 字串陣列 | `[]` | 「先詢問」規則（和允許衝突時先詢問贏）。只存使用者自己的；直接操作打開時另外一定加上「刪除檔案或資料夾」「付款、購買或轉帳」「用我的帳號發文、留言，或傳訊息給別人」三條，這三條不存 | 寫 | — | 搬 |
| `timezone` | 字串 | 這台的時區，查不到用 `"Asia/Taipei"` | 排程和提醒用的時區（IANA 名稱） | 寫 | — | 搬 |
| `next_routine_id` | 整數 | 1 | 下一個例行任務的編號 | 寫 | — | 重建 |
| `notify` | 布林 | true | Bot 完成任務或等你處理時通知你 | 寫 | — | 搬 |
| `alert_sound` | 布林 | true | 需要你時發出提示音 | 寫 | — | 搬 |
| `direct_mode` | 布林 | false | 直接操作 | 寫 | — | 搬 |
| `smart_mode` | 布林 | false | 智慧模式（/smart） | 寫 | — | 搬 |
| `smart_timeout_secs` | 整數 | 300 | 智慧模式的卡片多久沒人回就自己回（秒；0 ＝ 永不） | 寫 | — | 搬 |
| `always_approve` | 布林 | false | 一律放行（/always-approve） | 寫 | — | 搬 |
| `direct_before_always` | 布林 | false | 打開一律放行之前直接操作是不是已經開著（關掉一律放行時照它還原） | 寫 | — | 搬 |
| `auto_update_when_away` | 布林 | true | 在你離開時自動更新 | 寫 | — | 搬 |
| `close_browsers_when_done` | 布林 | true | 做完就關掉 Bot 用過的瀏覽器 | 寫 | — | 搬 |
| `send_check` | 布林 | true | 發布前檢查（瀏覽器送出文字前先讓另一個模型照規則看過） | 寫 | — | 搬 |
| `send_rules` | 字串 | `""` | 使用者自己加的發布前檢查規則 | 空不寫 | — | 搬 |
| `channels` | 陣列〈訊息管道〉 | `[]` | Slack、Discord、Telegram… 管道（§7.2） | 寫 | 內含 | 搬 |
| `next_channel_id` | 整數 | 1 | 下一個訊息管道的編號 | 寫 | — | 重建 |
| `mcp_servers` | 陣列〈MCP 伺服器〉 | `[]` | 連接器（§7.3） | 寫 | 內含 | 搬 |
| `skills` | 陣列〈技能〉 | `[]` | 技能清單（§8.1）。每次啟動會和技能資料夾合併 | 寫 | — | 搬 |
| `about_you` | 字串陣列 | `[]` | 關於你：每個 Bot 都知道的、關於使用者的事（每條一句）。`memory.db` 裡也有一份（Bot 欄 0） | 空不寫 | — | 搬 |
| （無鍵）進行中的支線 Bot | — | — | 只在記憶體；重開就結束，紀錄留在 `bots[].branches[]` | 不寫 | — | — |
| `skills_seeded` | 布林 | false | 內建技能放進技能資料夾過了沒（使用者刪掉的內建技能不會再被放回來） | 寫 | — | 搬 |
| `pages` | 陣列〈頁面〉 | `[]` | 頁面（§8.2） | 寫 | — | 搬 |
| `workflows` | 陣列〈工作流〉 | `[]` | 工作流（§8.3） | 空不寫 | 內含 | 搬 |
| `improvements` | 陣列〈待改進〉 | `[]` | AnyBot 自己的問題清單（§8.4），最多 50 條 | 空不寫 | — | 搬 |
| `garden_reports` | 布林 | true | 園丁報告：平日早上有問題時、週一例行任務太貴時，在主要 Bot 的聊天寫一行 | 寫 | — | 搬 |
| `garden_daily` | 整數 | 0 | 上次做日報的日期（`yyyymmdd`） | 寫 | — | 搬 |
| `garden_weekly` | 整數 | 0 | 上次做週報的日期（`yyyymmdd`） | 寫 | — | 搬 |
| `next_page_id` | 整數 | 1 | 下一個頁面的編號 | 寫 | — | 重建 |
| `remote` | 物件 | 見 §7.4 | 手機與帳號 | 寫 | 內含 | 見 §7.4 |
| `folders` | 字串陣列 | `[]` | 側邊欄的分區，照順序，包含空的 | 寫 | — | 搬 |
| `machine_label` | 字串 | `""` | 這台電腦在「設定 › 電腦」的名字；空 ＝ 用帳號裡的名字，再沒有用系統的電腦名 | 空不寫 | — | 搬 |
| `local_never` | 布林 | false | 在這台電腦上執行 ＝ 永不允許（不問，直接拒絕） | 寫 | — | 搬 |
| `messages_send` | 布林 | true | Bot 可以從這台 Mac 傳 iMessage／簡訊 | 寫 | — | 搬 |
| `messages_no_ask` | 布林 | false | 傳訊息不用每次出核准卡 | 寫 | — | 搬 |
| `pin_order` | 整數陣列 | `[]` | 已釘選的 Bot，照側邊欄上方格子的順序 | 空不寫 | — | 搬 |
| `loose_index` | 整數，可為 null | 沒有 ＝ 最後 | 「未分區」排在分區之間的第幾個 | 空不寫 | — | 搬 |
| `folded` | 字串陣列 | `[]` | 側邊欄收起來的分區（`""` ＝ 未分區） | 空不寫 | — | 搬 |
| `language` | 字串 | `"auto"` | 介面語言：`auto`（跟系統）或 `af` `ar` `bn` `cs` `da` `de` `el` `en` `es` `fi` `fr` `ha` `he` `hi` `hu` `id` `it` `ja` `ko` `mr` `nb` `nl` `pl` `pt-PT` `pt` `ru` `sv` `sw` `ta` `te` `th` `tr` `uk` `ur` `vi` `zh-CN` `zh-TW` | 寫 | — | 搬 |
| `voice` | 物件 | 全部關、`saved` false | 設定 › 語音（§6） | 寫 | 內含 | 搬 |

**螢幕電腦自己的設定**：一台電腦當「螢幕」顯示別台大腦時，下列欄位仍是這台自己的（不跟大腦同步）：`remote`、`theme`、`accent`、`sidebar_collapsed`、`sidebar_width`、`machine_label`、`local_execution`、`local_never`、`messages_send`、`messages_no_ask`、`shell_tone`、`shell_accent`、`shell_contrast`、`dense`、`celebrate`、`motion`、`selected`。

## 3. 每個 Bot：`bots[]`

一個 JSON 物件。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | Bot 編號 | 寫 | — | 搬 |
| `name` | 字串 | 必填 | 名字 | 寫 | — | 搬 |
| `description` | 字串 | `""` | 一句描述 | 寫 | — | 搬 |
| `instructions` | 字串 | `""` | 指示（個性 Soul 與指示合在一起；送給模型當系統提示） | 寫 | — | 搬 |
| `shape` | 字串 | 必填 | 頭像形狀：`squircle` `blob` `capsule` `egg` `hex` `gem` `cloud` `arch` `star` `heart` `shield` `pebble` `moon` `pumpkin`；不認得的畫成圓球 | 寫 | — | 搬 |
| `color` | 字串 | 必填 | 頭像顏色：`black` `red` `orange` `yellow` `green` `cyan` `blue` `purple` `magenta` `brown`，或自訂 `#rrggbb` | 寫 | — | 搬 |
| `provider` | 列舉 | 必填 | 供應商：`"Xai"` `"OpenAi"` `"ChatGpt"` `"Anthropic"` `"Gemini"` `"OpenRouter"` `"OpenCodeGo"` `"Ollama"` `"Custom"`（注意：和 `providers` 的鍵寫法不同，見 §5） | 寫 | — | 搬 |
| `model` | 字串 | 必填 | 模型名 | 寫 | — | 搬 |
| `step_model` | 字串 | `""` | 執行步驟用的模型：`""` ＝ 供應商的快速模型；`"off"` ＝ 每一步都用 `model`；其他 ＝ 那個模型 | 空不寫 | — | 搬 |
| `fallbacks` | 陣列〈備用模型〉 | `[]` | 備用模型，照順序試，最多 8 個（§3.1） | 空不寫 | — | 搬 |
| `thinking` | 字串 | `""` | 思考強度：`""` 自動、`"low"` 快速、`"medium"` 平衡、`"high"` 深入 | 空不寫 | — | 搬 |
| `pinned` | 布林 | false | 釘選 | 寫 | — | 搬 |
| `folder` | 字串 | `""` | 在側邊欄哪個分區（`""` ＝ 未分區） | 空不寫 | — | 搬 |
| `messages` | 陣列〈一則訊息〉 | `[]` | 目前的聊天。平常寫在 `chats/bot-<id>.json`；**這次存檔有任何一個聊天檔寫不進去時，所有 Bot 的聊天都整份寫進 `state.json`** | 另檔 | — | 搬（§9） |
| `unread` | 布林 | false | 有未讀 | 寫 | — | 搬 |
| `tools_enabled` | 布林 | false | 讓這個 Bot 在工作資料夾裡讀寫檔、跑指令 | 寫 | — | 搬 |
| `workdir` | 字串 | `""` | 在這台電腦上的工作資料夾（絕對路徑或 `~`；空 ＝ 家目錄） | 寫 | — | 搬（改寫路徑） |
| `allow_rules` | 字串陣列 | `[]` | 「永遠允許」記下的類別。已知寫法：`edit_file`、`browser:read`、`read:anywhere`、`computer:see`、`computer:control`、`shell:<指令第一個字，git/npm/pnpm/yarn/cargo/brew/pip/pip3/python3/docker/go 取前兩個字>`、`app:<App 名>`、`mcp:<連接器名>`、`login:<網站>`、`macapp:<App>`（讀）、`macapp-write:<App>`（改）；其他字串是工具名（待確認：完整清單） | 寫 | — | 搬 |
| `members` | 整數陣列 | `[]` | 群組聊天：回答的 Bot 編號，照順序（空 ＝ 一般 Bot） | 空不寫 | — | 搬 |
| `threads` | 陣列〈[標題, 訊息陣列]〉 | `[]` | 過去的對話。平常寫在 `chats/threads/bot-<id>.json`；寫入條件同 `messages`（另外空的時候一定不寫） | 另檔 | — | 搬（§9） |
| `memories` | 字串陣列 | `[]` | 這個 Bot 記得的事（每條最多 500 字，最多 200 條）。`memory.db` 裡也有一份（種類 fact） | 寫 | — | 搬（待確認：與 `memory.db` 以誰為準） |
| `speak_replies` | 布林 | false | 把回答唸出來 | 寫 | — | 搬 |
| `voice` | 字串 | `""` | 「說」用的聲音名（空 ＝ 服務預設） | 空不寫 | — | 搬 |
| `voice_rate` | 小數 | 1.0 | 語速（≤0 當 1.0） | 寫 | — | 搬 |
| `voice_lang` | 字串 | `""` | 聽寫用的語言代碼（空 ＝ 自動偵測） | 空不寫 | — | 搬 |
| `demos` | 陣列〈教學示範〉 | `[]` | 「教它一項任務」的錄製（§3.2） | 寫 | — | 搬 |
| `context_tokens` | 整數 | 128000 | 這個 Bot 的模型一次能讀多少 token（實際用時最少當 4000） | 寫 | — | 搬 |
| `model_limits` | 物件〈`<供應商代號>/<模型>` → 整數〉 | `{}` | 各模型自己回報的上限（學到的；例：`"anthropic/claude-opus-5-5": 200000`） | 空不寫 | — | 搬 |
| `follow_ups` | 陣列〈追進度〉 | `[]` | 正在追的長工作，每個 Bot 最多 8 個（§3.3） | 空不寫 | — | 搬 |
| `routines` | 陣列〈例行任務〉 | `[]` | 例行任務（§3.4） | 寫 | 內含 | 搬 |
| `last_turn` | 物件，可為 null | null | 上一輪怎麼結束（§3.5） | 寫（沒有時寫 `null`） | — | 丟 |
| `tasks` | 陣列〈任務事件〉 | `[]` | 任務狀態變化的紀錄，只往後加，保留最新 40 筆（§3.6） | 空不寫 | — | 搬 |
| `goal` | 物件 | 沒有 | 目標（/goal）（§3.7） | 空不寫 | — | 搬 |
| `plan` | 物件 | 沒有 | 計畫（/plan）（§3.8） | 空不寫 | — | 搬 |
| `default_computer` | 字串 | `"Bot 的電腦"` | 預設在哪台做事：`"Bot 的電腦"`（Bot 自己的電腦）、這台電腦的名字、SSH 電腦的名字、帳號裡其他電腦的名字；舊值 `"你的電腦"` ＝ 使用者現在用的這台 | 寫 | — | 搬 |
| `draft` | 字串 | `""` | 輸入框裡還沒送出的字 | 空不寫 | — | 搬 |
| `regen_undo` | 陣列〈一則訊息〉 | `[]` | 「重新產生」換掉的答案，給「還原上一個答案」用；下一個問題就清掉 | 空不寫 | 照 §1.2 規則 | 搬 |
| `team` | 布林 | false | 團隊成員可以看、可以跟它說話 | 空不寫 | — | 搬 |
| `branches` | 陣列〈支線〉 | `[]` | 開過的支線（/btw）紀錄（§3.9） | 空不寫 | — | 搬 |
| `branch_of` | 整數 | 0 | 這個 Bot 本身是支線時，主 Bot 的編號（存檔的 Bot 都是 0） | 空不寫 | — | 丟 |
| `soul` | 字串 | `""` | 更舊版本分開存的個性；讀進來就併到 `instructions` 最前面 | 只讀 | — | 丟（OLD 已併入） |
| `strengths` | 字串 | `""` | 擅長（公開在同事名冊） | 空不寫 | — | 搬 |
| `not_for` | 字串 | `""` | 不找它做的事（公開在同事名冊） | 空不寫 | — | 搬 |
| `skills` | 字串陣列，可為 null | 沒有 ＝ 全部技能 | 會的技能（技能名）。**沒有這個鍵 ＝ 全部；`[]` ＝ 一個都沒有** | 空不寫（沒有時） | — | 搬 |
| `abilities` | 字串陣列，可為 null | 沒有 ＝ 全部 | 能力（工具類別）：`web` 上網、`browser` 瀏覽器、`screen` 看螢幕與操作、`files` 檔案與指令、`mac_apps` Mac App、`connectors` 連接器、`phone` 電話簡訊 X、`routines` 例行任務與工作流、`pages` 頁面、`team` 和其他 Bot 合作、`admin` 管理 App。沒有這個鍵 ＝ 全部；勾回全部時存成沒有 | 空不寫（沒有時） | — | 搬 |
| `computers` | 字串陣列，可為 null | 沒有 ＝ 全部電腦 | 可以用的電腦（名字）。自己的電腦一定算 | 空不寫（沒有時） | — | 搬 |

### 3.1 備用模型：`bots[].fallbacks[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `provider` | 列舉 | 必填 | 同 `bots[].provider` 的九個值 | 寫 | — | 搬 |
| `model` | 字串 | 必填 | 模型名 | 寫 | — | 搬 |

### 3.2 教學示範：`bots[].demos[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `name` | 字串 | 必填 | 任務名 | 寫 | — | 搬 |
| `created` | 字串 | 必填 | 錄的時間，本地時間 `YYYY-MM-DD HH:MM` | 寫 | — | 搬 |
| `steps` | 字串陣列 | 必填 | 每一步的文字說明（「點 (x, y)」之類） | 寫 | — | 搬 |
| `video` | 字串 | `""` | 錄影檔的絕對路徑（`demos/<YYYYMMDD-HHMMSS>/recording.mp4`）；空 ＝ 沒有 | 寫 | — | 搬（改寫路徑） |
| `shots` | 字串陣列 | `[]` | 每一步的截圖絕對路徑（同資料夾 `step_NN.jpg`）；空字串 ＝ 那一步沒截到 | 寫 | — | 搬（改寫路徑） |

### 3.3 追進度：`bots[].follow_ups[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 編號 | 寫 | — | 搬 |
| `what` | 字串 | 必填 | 追的是什麼（使用者的話） | 寫 | — | 搬 |
| `check` | 字串 | 必填 | 看進度的指令 | 寫 | — | 搬 |
| `computer` | 字串 | `""` | 在哪台跑（空 ＝ Bot 選的那台） | 寫 | — | 搬 |
| `done_when` | 字串 | 必填 | 輸出出現這個就算完成（`/…/` 是正規表示式） | 寫 | — | 搬 |
| `fail_when` | 字串 | `""` | 輸出出現這個就算失敗（空 ＝ 不判失敗） | 寫 | — | 搬 |
| `then` | 字串 | `""` | 完成後 Bot 說它要做什麼（叫醒時還給它） | 寫 | — | 搬 |
| `every_secs` | 整數 | 必填 | 幾秒看一次（15–3600，預設 60） | 寫 | — | 搬 |
| `started_ms` | 整數 | 必填 | 開始時間（毫秒） | 寫 | — | 搬 |
| `until_ms` | 整數 | 必填 | 放棄的時間（毫秒；預設開始後 180 分鐘，最多 1440 分鐘） | 寫 | — | 搬 |
| `next_ms` | 整數 | 0 | 下次看的時間（毫秒） | 寫 | — | 重建 |
| `last` | 字串 | `""` | 最近一次輸出的結尾（最多 1500 字） | 寫 | — | 搬 |
| `last_change_ms` | 整數 | 0 | 輸出上次變化的時間（毫秒） | 寫 | — | 搬 |
| `reported_ms` | 整數 | 0 | 上次在聊天寫進度的時間（毫秒） | 寫 | — | 搬 |
| `errors` | 整數 | 0 | 看進度的指令連續失敗幾次（3 次就結束） | 寫 | — | 搬 |

### 3.4 例行任務：`bots[].routines[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 編號（全部 Bot 共用一個序號） | 寫 | — | 搬 |
| `title` | 字串 | 必填 | 標題 | 寫 | — | 搬 |
| `prompt` | 字串 | 必填 | 每次要 Bot 做什麼 | 寫 | — | 搬 |
| `schedule` | 物件 | 必填 | 排程（§3.4a）；有 `trigger` 時不照排程跑 | 寫 | — | 搬 |
| `paused` | 布林 | false | 暫停 | 寫 | — | 搬 |
| `created_at` | 整數 | 0 | 建立時間（**秒**） | 寫 | — | 搬 |
| `last_run` | 整數，可為 null | null | 上次開始跑的時間（秒） | 寫 | — | 搬 |
| `next_run` | 整數，可為 null | null | 下次要跑的時間（秒） | 寫 | — | 重建 |
| `computer` | 字串 | `""` | 在哪台做（空 ＝ Bot 的預設） | 寫 | — | 搬 |
| `trigger` | 物件 | 沒有 | 事件觸發（§4）；有它就是「發生某事時跑」 | 空不寫 | 內含 | 搬 |
| `runs` | 陣列〈執行紀錄〉 | `[]` | 最近 20 次怎麼跑的，舊到新（§3.4c） | 空不寫 | — | 搬 |
| `paused_why` | 字串 | `""` | AnyBot 自動暫停的原因（連續失敗 2 次）；空 ＝ 使用者自己暫停 | 空不寫 | — | 搬 |
| `target_sends` | 整數 | 0 | 完成條件：每次要送出幾則（0 ＝ 沒有） | 空不寫 | — | 搬 |

#### 3.4a 排程：`…schedule`（工作流觸發器的 `schedule` 也是這個）

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `frequency` | 列舉 | 必填 | `"minutely"`（每 N 分鐘，從建立時算，最少 5 分鐘）、`"hourly"`（每 N 小時，在 `time` 的分鐘）、`"daily"`、`"weekly"`、`"monthly"`、`"yearly"` | 寫 | — | 搬 |
| `interval` | 整數 | 1 | 每幾個單位一次 | 寫 | — | 搬 |
| `daysOfWeek` | 字串陣列 | `[]` | 星期幾：`MO` `TU` `WE` `TH` `FR` `SA` `SU` | 空不寫 | — | 搬 |
| `monthDay` | 整數 | 沒有 | 每月第幾天 | 空不寫 | — | 搬 |
| `weekdayOfMonth` | 物件 | 沒有 | 每月第幾個星期幾：`{"week": 1–4 或 -1（最後一個）, "day": "MO"…"SU"}`，兩個鍵都必填 | 空不寫 | — | 搬 |
| `month` | 整數 | 沒有 | 第幾月（1–12，每年用） | 空不寫 | — | 搬 |
| `time` | 字串 | 必填 | 時刻 `HH:MM`（24 小時） | 寫 | — | 搬 |
| `timezone` | 字串 | 必填 | 時區（IANA） | 寫 | — | 搬 |
| `once` | 布林 | false | 僅一次 | 空不寫 | — | 搬 |
| `flexible` | 布林 | false | 「左右」：前後 15 分鐘內都可以 | 空不寫 | — | 搬 |

`weekdayOfMonth` 的兩個子鍵 `week`、`day` 各算一欄。

#### 3.4c 執行紀錄：`bots[].routines[].runs[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `started` | 整數 | 必填 | 開始（秒） | 寫 | — | 搬 |
| `ended` | 整數 | 0 | 結束（秒） | 寫 | — | 搬 |
| `outcome` | 列舉 | 必填 | `"running"`、`"succeeded"`、`"failed"`、`"stopped"`（使用者或 15 分鐘無人核准停掉）、`"interrupted"`（AnyBot 在執行中被關掉；開機時 `running` 會改成這個） | 寫 | — | 搬 |
| `note` | 字串 | `""` | 失敗或停止的原因；成功時是報告開頭 | 空不寫 | — | 搬 |
| `steps` | 整數 | 0 | 做了幾步 | 寫 | — | 搬 |
| `failed_steps` | 整數 | 0 | 其中失敗或被拒的步數 | 寫 | — | 搬 |
| `sent` | 整數 | 0 | 系統看到真的送出去的步數 | 寫 | — | 搬 |
| `sends` | 字串陣列 | `[]` | 送出了什麼、送到哪（「回覆 <貼文> → <回覆的網址>」），最多 30 條 | 空不寫 | — | 搬 |
| `by_hand` | 布林 | false | 「立即執行」跑的（不算在日程上） | 空不寫 | — | 搬 |

### 3.5 上一輪結算：`bots[].last_turn`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `turn_id` | 字串 | 必填 | 那一輪的編號；`"interrupted"` ＝ 開機時發現上次被中斷 | 寫 | — | 丟 |
| `outcome` | 列舉 | 必填 | `"success"`、`"cancelled"`、`"error"` | 寫 | — | 丟 |
| `settled_at_ms` | 整數 | 必填 | 結束時間（毫秒） | 寫 | — | 丟 |

### 3.6 任務事件：`bots[].tasks[]`

一筆是「某個任務在某時的狀態」，只往後加、不改。除了 `noted`，其他鍵和「一個任務」同一層（攤平）。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `noted` | 整數 | 必填 | 記下的時間（毫秒） | 寫 | — | 搬 |
| `id` | 字串 | 必填 | 任務編號：`<Bot 編號>-<開頭訊息的毫秒>`；開頭訊息沒有時間時 `<Bot 編號>-h<十六進位>` | 寫 | — | 搬 |
| `bot` | 整數 | 必填 | 哪個 Bot | 寫 | — | 搬 |
| `at` | 整數 | 必填 | 開頭訊息的時間（毫秒） | 寫 | — | 搬 |
| `text` | 字串 | 必填 | 開頭的字（一行，最多 60 字） | 寫 | — | 搬 |
| `source` | 列舉 | `"owner"` | 誰交辦的：`owner` 你、`member` 團隊成員、`routine` 例行任務、`trigger` 觸發、`goal` 目標、`plan` 計畫、`follow_up` 追進度、`monitor` 監看、`channel` 訊息管道、`bot` 其他 Bot、`delivery` 交付 | 寫 | — | 搬 |
| `status` | 列舉 | `"unverified"` | `running` 進行中、`waiting` 等你、`following` 追進度中、`verifying` 驗收中、`produced` 已產出、`delivered` 已交付、`confirmed` 你已確認、`answered` 已回覆、`verified` 已驗收、`failed` 失敗、`stopped` 已停止、`unverified` 結束但未驗證 | 寫 | — | 搬 |
| `doing` | 字串 | `""` | 正在做什麼／最後一步 | 空不寫 | — | 搬 |
| `computer` | 字串 | `""` | 在哪台 | 空不寫 | — | 搬 |
| `steps` | 整數 | 0 | 步數 | 空不寫 | — | 搬 |
| `failed` | 整數 | 0 | 失敗步數 | 空不寫 | — | 搬 |
| `heartbeat` | 整數 | 0 | 最後一次有動靜（毫秒） | 空不寫 | — | 搬 |
| `finished` | 整數 | 0 | 結束時間（毫秒；0 ＝ 還在進行） | 空不寫 | — | 搬 |
| `artifacts` | 陣列〈產物〉 | `[]` | 交出的檔案和送到哪了（§3.6a） | 空不寫 | — | 搬 |
| `evidence` | 列舉 | `"none"` | 證據等級：`"b"` 你親眼看到、`"a"` 系統看到、`"c"` 別的 Bot 轉述、`"none"` 未驗 | 寫 | — | 搬 |
| `proof` | 字串陣列 | `[]` | 系統看到的事，一行一件 | 空不寫 | — | 搬 |
| `why` | 字串 | `""` | 卡住、等待或失敗的原因 | 空不寫 | — | 搬 |
| `ended` | 列舉 | 沒有 | 最後一輪怎麼結束：`success`、`cancelled`、`error`、`interrupted`、`no_reply` | 空不寫 | — | 搬 |
| `kept` | 布林 | false | 聊天裡已經找不到，只剩這筆紀錄 | 空不寫 | — | 搬 |

#### 3.6a 產物：`bots[].tasks[].artifacts[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `name` | 字串 | 必填 | 檔名 | 寫 | — | 搬 |
| `id` | 字串 | `""` | 附件編號 | 空不寫 | — | 搬 |
| `delivery` | 列舉 | `"in_chat"` | `in_chat` 在聊天裡還沒裝置回報、`shown` 已顯示、`opened` 已點開、`failed` 顯示失敗 | 寫 | — | 搬 |
| `device` | 字串 | `""` | 走最遠（或失敗）的那台裝置 | 空不寫 | — | 搬 |
| `why` | 字串 | `""` | 失敗原因 | 空不寫 | — | 搬 |

### 3.7 目標：`bots[].goal`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `condition` | 字串 | 必填 | 達成條件（最多 4000 字） | 寫 | — | 搬 |
| `since` | 整數 | 必填 | 設定的時間（毫秒；從這則訊息開始檢查） | 寫 | — | 搬 |
| `turns` | 整數 | 0 | 已檢查幾輪 | 寫 | — | 搬 |
| `until` | 整數 | 0 | 到幾輪停下來問（每次給 20 輪） | 寫 | — | 搬 |
| `idle` | 整數 | 0 | 連續幾輪沒用工具（3 輪停） | 寫 | — | 搬 |
| `reason` | 字串 | `""` | 檢查者最新的理由 | 空不寫 | — | 搬 |
| `paused` | 字串 | `""` | 暫停的原因；空 ＝ 進行中。開機時進行中的目標都會被暫停並寫原因 | 空不寫 | — | 搬 |

### 3.8 計畫：`bots[].plan`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `task` | 字串 | 必填 | /plan 後面要做的事（最多 4000 字） | 寫 | — | 搬 |
| `since` | 整數 | 必填 | 提出的時間（毫秒） | 寫 | — | 搬 |
| `stage` | 列舉 | `"drafting"` | `"drafting"` 還在看、寫計畫；`"doing"` 已核准，照計畫做 | 寫 | — | 搬 |
| `steps` | 陣列 | `[]` | 核准後的步驟，最多 20 步；每步 `{"title": 字串（必填，最多 160 字）, "status": "pending" \| "in_progress" \| "done" \| "skipped"（預設 pending，一律寫）}` | 空不寫 | — | 搬 |
| `revisions` | 整數 | 0 | 被退回修改幾次 | 寫 | — | 搬 |

`steps[]` 的 `title`、`status` 各算一欄。

### 3.9 支線：`bots[].branches[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 支線 Bot 的編號（結束後就不存在） | 寫 | — | 搬 |
| `task` | 字串 | 必填 | /btw 後面問的事 | 寫 | — | 搬 |
| `at` | 整數 | 必填 | 問的時間（毫秒） | 寫 | — | 搬 |
| `state` | 列舉 | 必填 | `"checking"`、`"waiting"`、`"running"`、`"done"`、`"failed"`、`"stopped"`。開機時前三種都改成 `stopped`，`note` 寫「AnyBot 重新啟動，這條支線停了」 | 寫 | — | 搬 |
| `note` | 字串 | `""` | 為什麼在等、為什麼這樣結束 | 空不寫 | — | 搬 |
| `doing` | 字串 | `""` | 正在做什麼 | 空不寫 | — | 搬 |
| `steps` | 整數 | 0 | 做了幾步 | 寫 | — | 搬 |
| `result` | 字串 | `""` | 找到什麼（最後的話） | 空不寫 | — | 搬 |
| `log` | 字串陣列 | `[]` | 過程，一步一行（「看過程」） | 空不寫 | — | 搬 |

## 4. 觸發條件（例行任務的 `trigger`、工作流觸發器的 `event` 共用）

一個 JSON 物件，用 `type` 分種類，其他鍵跟 `type` 在同一層。例：`{"type":"github","repo":"me/app","token":"enc:v1:…","events":["pr_merged"],"users":[],"branch":"","pr":0}`。所有鍵一律寫（沒有「空不寫」）。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `type` | 列舉 | 必填 | `github`、`slack`、`linear`、`sentry`、`pagerduty`、`email`、`webhook`、`calendar`、`group` | 寫 | — | 搬 |
| `github` › `repo` | 字串 | 必填 | `owner/repo` | 寫 | — | 搬 |
| `github` › `token` | 字串 | `""` | GitHub 權杖（空 ＝ 不帶，只能看公開 repo） | 寫 | 封存 | 搬 |
| `github` › `events` | 字串陣列 | `[]` | `pr_opened` `pr_updated` `pr_merged` `pr_closed` `pr_comment` `ci_passed` `ci_failed` `review_approved` `review_changes_requested` `review_comment` `thread_resolved` `thread_reopened` `issue_assigned` `issue_opened` `push`；空 ＝ 任何事件 | 寫 | — | 搬 |
| `github` › `users` | 字串陣列 | `[]` | 只看這些人的動作（空 ＝ 任何人） | 寫 | — | 搬 |
| `github` › `branch` | 字串 | `""` | 只看這個分支（空 ＝ 任何） | 寫 | — | 搬 |
| `github` › `pr` | 整數 | 0 | 只看這個 PR（0 ＝ 任何） | 寫 | — | 搬 |
| `slack` › `channel` | 字串 | 必填 | 頻道 ID | 寫 | — | 搬 |
| `slack` › `token` | 字串 | `""` | Slack 權杖（空 ＝ 用 Slack 訊息管道的） | 寫 | 封存 | 搬 |
| `slack` › `match` | 字串 | `"any"` | `any` 任何訊息、`mention` 被 @、`keyword` 提到關鍵字、`reaction` 有人加表情 | 寫 | — | 搬 |
| `slack` › `value` | 字串 | `""` | 關鍵字或表情名 | 寫 | — | 搬 |
| `linear` › `apiKey` | 字串 | 必填 | Linear API 金鑰 | 寫 | **明文秘密** | 搬（新版要加密） |
| `linear` › `events` | 字串陣列 | `[]` | `issue_created`、`status_changed`（空 ＝ 兩種都要） | 寫 | — | 搬 |
| `linear` › `team` | 字串 | `""` | 只看這個團隊 | 寫 | — | 搬 |
| `sentry` › `token` | 字串 | 必填 | Sentry 權杖 | 寫 | 封存 | 搬 |
| `sentry` › `org` | 字串 | 必填 | 組織 | 寫 | — | 搬 |
| `sentry` › `project` | 字串 | `""` | 專案 | 寫 | — | 搬 |
| `sentry` › `events` | 字串陣列 | `[]` | `issue_created` `issue_resolved` `issue_unresolved` `issue_ignored` `issue_assigned` `issue_changed` | 寫 | — | 搬 |
| `pagerduty` › `token` | 字串 | 必填 | PagerDuty 權杖 | 寫 | 封存 | 搬 |
| `pagerduty` › `service` | 字串 | `""` | 服務 | 寫 | — | 搬 |
| `pagerduty` › `events` | 字串陣列 | `[]` | `triggered` `acknowledged` `resolved` `changed` | 寫 | — | 搬 |
| `email` › `from` | 字串 | `""` | 寄件人包含（空 ＝ 任何人） | 寫 | — | 搬 |
| `email` › `subject` | 字串 | `""` | 主旨包含 | 寫 | — | 搬 |
| `webhook` › `secret` | 字串 | 必填 | 呼叫方要帶的密語（24 個十六進位） | 寫 | **明文秘密** | 搬（新版要加密；webhook 位址見 §0.5） |
| `calendar` › `calendar` | 字串 | `""` | 只看這個行事曆（空 ＝ 全部） | 寫 | — | 搬 |
| `calendar` › `title` | 字串 | `""` | 只看標題包含這個的活動 | 寫 | — | 搬 |
| `calendar` › `minutesBefore` | 整數 | 0 | 活動開始前幾分鐘觸發（0 ＝ 開始時） | 寫 | — | 搬 |
| `group` › `triggers` | 陣列〈觸發條件〉 | 必填 | 任一個發生就觸發（可以巢狀） | 寫 | 內含 | 搬 |

## 5. 供應商設定：`providers.<代號>`

`providers` 是物件，鍵是供應商代號（**小寫**，和 `bots[].provider` 的寫法不同）：

| 代號 | 對應 `bots[].provider` | 顯示名 | `default_model` 空時用 |
|---|---|---|---|
| `xai` | `Xai` | xAI Grok | `grok-4` |
| `openai` | `OpenAi` | OpenAI | `gpt-5` |
| `chatgpt` | `ChatGpt` | ChatGPT 方案（登入在 `providers/chatgpt.json`，不在這裡） | `gpt-6.1-sol` |
| `anthropic` | `Anthropic` | Claude | `claude-opus-5-5` |
| `gemini` | `Gemini` | Google Gemini | `gemini-2.5-flash` |
| `openrouter` | `OpenRouter` | OpenRouter | `openrouter/auto` |
| `opencode-go` | `OpenCodeGo` | OpenCode Go | `kimi-k3` |
| `ollama` | `Ollama` | Ollama | `llama3.2` |
| `custom` | `Custom` | 自訂（OpenAI 相容） | `local-model` |

只有設過的供應商才有鍵。每個值：

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `api_key` | 字串 | `""` | API 金鑰 | 寫 | 封存 | 搬（`02` A9：同步到鑰匙圈） |
| `base_url` | 字串 | `""` | 位址（空 ＝ 供應商預設） | 寫 | — | 搬 |
| `default_model` | 字串 | `""` | 預設模型（空 ＝ 上表） | 寫 | — | 搬 |
| `known_models` | 字串陣列 | `[]` | 從供應商抓來的模型清單 | 寫 | — | 重建 |
| `cloud_eyes` | 布林 | false | 看圖小幫手：本機／區網模型看不到的圖，可以交給使用者的雲端模型描述 | 寫 | — | 搬 |

## 6. 語音：`voice`

整個物件和每個子物件都是「缺哪個鍵就用預設」。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `voice.listen` | 物件 | 關 | 「聽」用的服務（下表） | 寫 | 內含 | 搬 |
| `voice.speak` | 物件 | 關 | 「說」用的服務（下表） | 寫 | 內含 | 搬 |
| `voice.saved` | 布林 | false | 這份設定是有「設定 › 語音」的版本寫的；false 時開機會做一次轉換（§10） | 寫 | — | 丟 |
| `voice.from_server` | 字串 | `""` | 自動填過一次的「大腦旁邊的語音服務」位址；之後使用者關掉就不再自動填 | 空不寫 | — | 搬 |

`voice.listen` 和 `voice.speak` 各自：

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `kind` | 列舉 | `"off"` | `"off"` 關、`"open_ai"`（任何 OpenAI 相容語音 API：OpenAI、Groq、自架 Whisper）、`"xai"`、`"eleven_labs"` | 寫 | — | 搬 |
| `base_url` | 字串 | `""` | OpenAI 相容服務的位址（結尾 `/v1`；空 ＝ OpenAI 本身） | 寫 | — | 搬 |
| `api_key` | 字串 | `""` | 金鑰（空 ＝ 用「設定 › 模型」裡同一家的金鑰） | 寫 | 封存 | 搬 |
| `model` | 字串 | `""` | 模型（空 ＝ 服務預設） | 寫 | — | 搬 |
| `voice` | 字串 | `""` | 「說」的聲音（空 ＝ 服務預設） | 寫 | — | 搬 |

## 7. 連接

### 7.1 遠端電腦：`remote_computers[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `name` | 字串 | 必填 | 顯示名 | 寫 | — | 搬 |
| `ssh_target` | 字串 | 必填 | SSH 目標，例 `me@studio.local` | 寫 | — | 搬 |
| `dir` | 字串 | `""` | 在那台上的工作資料夾 | 寫 | — | 搬 |

### 7.2 訊息管道：`channels[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 編號 | 寫 | — | 搬 |
| `kind` | 列舉 | 必填 | `"Slack"`、`"Discord"`、`"Telegram"`、`"Teams"`、`"IMessage"`（Mac 訊息）、`"Email"`（Mac 郵件）、`"Sms"`（Twilio 簡訊）、`"X"` | 寫 | — | 搬 |
| `name` | 字串 | 必填 | 顯示名 | 寫 | — | 搬 |
| `bot_id` | 整數 | 必填 | 由哪個 Bot 回答 | 寫 | — | 搬 |
| `token` | 字串 | 必填 | Slack `xoxb-…`、Discord Bot Token、Telegram `123456:ABC…`、Twilio `Account SID:Auth Token`、X 使用者存取權杖、Teams 的 refresh token；iMessage 和郵件是空的 | 寫 | 封存 | 搬 |
| `target` | 字串 | `""` | Slack／Discord 頻道 ID、Telegram 聊天 ID、iMessage 電話或 Apple ID、郵件信箱名稱、Twilio 號碼、Teams `chat:<聊天 ID>` 或 `<團隊 ID>/<頻道 ID>`；空 ＝ 任何 | 寫 | — | 搬 |
| `enabled` | 布林 | true | 啟用 | 寫 | — | 搬 |
| `client_id` | 字串 | `""` | Teams：Entra 應用程式（用戶端）ID | 寫 | — | 搬 |
| `tenant` | 字串 | `""` | Teams：租用戶（空 ＝ organizations） | 寫 | — | 搬 |

### 7.3 MCP 伺服器：`mcp_servers[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `name` | 字串 | 必填 | 名字（允許規則 `mcp:<名字>` 用它） | 寫 | — | 搬 |
| `command` | 字串 | `""` | stdio 伺服器的指令；用 `url` 時是空的 | 寫 | — | 搬 |
| `args` | 字串陣列 | `[]` | 指令參數（可以含 `{username}`、`{password}` 佔位） | 寫 | — | 搬 |
| `env` | 物件〈名稱 → 字串〉 | `{}` | 環境變數（值可以含佔位） | 寫 | 每個值封存 | 搬 |
| `url` | 字串 | `""` | Streamable HTTP 位址 | 寫 | — | 搬 |
| `headers` | 物件〈名稱 → 字串〉 | `{}` | HTTP 標頭，例 `Authorization: Bearer …` | 寫 | 每個值封存 | 搬 |
| `enabled` | 布林 | true | 啟用 | 寫 | — | 搬 |
| `instructions` | 字串 | `""` | 給 Bot 的使用說明 | 寫 | — | 搬 |
| `username` | 字串 | `""` | 帳號（沒寫佔位時：指令伺服器拿到 `MCP_USERNAME`，HTTP 伺服器變成 Basic 認證） | 空不寫 | — | 搬 |
| `password` | 字串 | `""` | 密碼（同上，`MCP_PASSWORD`） | 空不寫 | 封存 | 搬 |

MCP 的 OAuth 登入不在這裡，在 `connectors/oauth.json`（`01`）。

### 7.4 手機與帳號：`remote`

整個 `remote` 不存在時：`auto_backup` 是 true、`host_name` 是這台的系統電腦名，其餘空。**`remote` 在、但缺某個鍵時，`auto_backup` 是 false**（兩種預設不一樣）。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `lan` | 布林 | false | 在區網／Tailscale 上用 8793 埠直接回應手機 | 寫 | — | 待決定 |
| `password` | 字串 | `""` | 手機登入密碼的雜湊：新格式 `argon2id$<鹽，32 個十六進位>$<雜湊，64 個十六進位>`（argon2id v19，記憶體 19456 KiB、2 輪、1 平行、輸出 32 bytes，鹽 16 bytes）；舊格式 `<鹽>$<雜湊>`（PBKDF2-HMAC-SHA256，120000 輪，鹽是 `$` 前面那段文字本身，輸出 32 bytes 十六進位） | 寫 | 雜湊 | 待決定 |
| `devices` | 陣列〈登入的裝置〉 | `[]` | 直接登入的手機和瀏覽器（§7.4a） | 寫 | 雜湊 | 丟（建議：新版 API 重做，手機重新登入） |
| `mac_from_phone` | 布林 | false | 手機可以看、可以操作這台的螢幕 | 寫 | — | 搬 |
| `device_link` | 布林 | false | 同帳號的其他 AnyBot 可以操作這台（裝置連結） | 寫 | — | 搬 |
| `relay_url` | 字串 | `""` | 帳號伺服器位址 | 寫 | — | 搬 |
| `relay_user` | 字串 | `""` | 帳號名 | 寫 | — | 搬 |
| `host_id` | 字串 | `""` | 這台在帳號伺服器上的編號 | 寫 | — | 待決定 |
| `host_token` | 字串 | `""` | 這台在帳號伺服器上的金鑰 | 寫 | 封存 | 待決定 |
| `host_name` | 字串 | 系統電腦名 | 這台在手機上顯示的名字 | 寫 | — | 搬 |
| `backup_key` | 字串 | `""` | 加密備份的鑰匙（64 個十六進位，由帳號密碼導出，從不送出） | 寫 | 封存 | 丟（決定 5） |
| `auto_backup` | 布林 | false（見上） | 自動備份 | 寫 | — | 搬 |
| `last_backup` | 整數 | 0 | 上次備份（秒） | 寫 | — | 丟 |
| `brain_id` | 字串 | `""` | 這台是「螢幕」時，大腦在帳號伺服器上的編號；空 ＝ 這台自己是大腦 | 空不寫 | — | 待決定 |
| `brain_name` | 字串 | `""` | 大腦的名字 | 空不寫 | — | 待決定 |
| `hands_for` | 字串 | `""` | 這台當「手腳」時只給這個大腦用（空 ＝ 帳號裡任何 AnyBot） | 空不寫 | — | 待決定 |
| `hands_for_name` | 字串 | `""` | 那個大腦的名字 | 空不寫 | — | 待決定 |
| `link_key` | 字串 | `""` | 手腳：配對時大腦給的簽章鑰匙（24 個小寫英文字母） | 空不寫 | 封存 | 待決定 |
| `link_keys` | 字串陣列 | `[]` | 大腦：配對時發出去的簽章鑰匙，保留最新 24 把 | 空不寫 | 每個封存 | 待決定 |

#### 7.4a 登入的裝置：`remote.devices[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `name` | 字串 | 必填 | 裝置名 | 寫 | — | 丟 |
| `token_hash` | 字串 | 必填 | 登入權杖的 SHA-256（64 個小寫十六進位） | 寫 | 雜湊 | 丟 |
| `created` | 整數 | 必填 | 登入時間（秒） | 寫 | — | 丟 |
| `last_seen` | 整數 | 0 | 最後一次使用（秒） | 寫 | — | 丟 |

## 8. 內容

### 8.1 技能：`skills[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `name` | 字串 | 必填 | 名字（`/名字` 叫它；`bots[].skills[]` 用它） | 寫 | — | 搬 |
| `description` | 字串 | 必填 | 描述 | 寫 | — | 搬 |
| `instructions` | 字串 | 必填 | 內容（`SKILL.md` 本文） | 寫 | — | 搬 |
| `steps` | 陣列 | `[]` | 重播用的步驟：每步 `{"tool": 字串（必填）, "args": JSON 文字（必填）, "targets": 字串陣列（空不寫；批次操作每個動作一個，按的是什麼控制項，沒有的寫 ""）}` | 空不寫 | — | 搬 |
| `enabled` | 布林 | true | Bot 看得到、可以用 | 寫 | — | 搬 |
| `dir` | 字串 | `""` | 技能資料夾的絕對路徑（在資料夾上一層的 `skills/<代稱>`） | 空不寫 | — | 搬（改寫路徑） |
| `resources` | 字串陣列 | `[]` | 資料夾裡附帶的檔（`scripts/…`、`references/…`、`assets/…`） | 空不寫 | — | 搬 |
| `source` | 字串 | `""` | 來源：`builtin:<代稱>`、`owner/repo/…`、`learned`… | 空不寫 | — | 搬 |
| `argument_hint` | 字串 | `""` | `/名字` 後面要打什麼的提示 | 空不寫 | — | 搬 |
| `arguments` | 字串陣列 | `[]` | 具名參數（`$name`） | 空不寫 | — | 搬 |
| `allowed_tools` | 字串陣列 | `[]` | 載入後只能用這些工具（空 ＝ 不限） | 空不寫 | — | 搬 |
| `fork` | 布林 | false | `/名字` 在支線裡跑 | 空不寫 | — | 搬 |

`steps[]` 的 `tool`、`args`、`targets` 各算一欄。

### 8.2 頁面：`pages[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 編號 | 寫 | — | 搬 |
| `bot` | 整數 | 必填 | 誰做的（誰能改） | 寫 | — | 搬 |
| `title` | 字串 | 必填 | 標題（最多 120 字） | 寫 | — | 搬 |
| `summary` | 字串 | `""` | 卡片上的一行現況 | 寫 | — | 搬 |
| `body` | 字串 | 必填 | 內文 Markdown（最多 200000 字），可含 `stats`／`steps`／`chart`／`details`／`mermaid` 區塊 | 寫 | — | 搬 |
| `created` | 整數 | 必填 | 建立（毫秒） | 寫 | — | 搬 |
| `updated` | 整數 | 必填 | 更新（毫秒） | 寫 | — | 搬 |
| `revision` | 整數 | 1 | 第幾版 | 寫 | — | 搬 |
| `kind` | 字串 | `""` | 文件種類：`data` `compare` `decision` `howto` `timeline` `prose` `notes`；空 ＝ 一般 | 空不寫 | — | 搬 |
| `kpis` | 陣列 | `[]` | 數字卡：`{"label": 必填, "value": 必填, "delta": 空不寫（例 "+12%"）}` | 空不寫 | — | 搬 |
| `verdict` | 字串 | `""` | 一句結論 | 空不寫 | — | 搬 |
| `note` | 字串 | `""` | 這次改了什麼（「更新：…」） | 空不寫 | — | 搬 |
| `history` | 陣列 | `[]` | 舊版，舊到新，最多 10 份：`{"revision": 必填, "at": 必填（毫秒）, "title": 必填, "summary": 空不寫, "body": 必填, "note": 空不寫}` | 空不寫 | — | 搬 |

`kpis[]` 三欄、`history[]` 六欄各算。

### 8.3 工作流：`workflows[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 編號 | 寫 | — | 搬 |
| `name` | 字串 | 必填 | 名字 | 寫 | — | 搬 |
| `nodes` | 陣列〈節點〉 | `[]` | 畫布上的步驟 | 寫 | 內含 | 搬 |
| `edges` | 陣列〈連線〉 | `[]` | 步驟之間的線 | 寫 | — | 搬 |
| `notes` | 陣列〈畫布筆記〉 | `[]` | 畫的、寫的（不執行） | 寫 | — | 搬 |
| `versions` | 陣列〈版本〉 | `[]` | 舊版，舊到新，最多 30 份 | 空不寫 | 內含 | 搬 |
| `runs` | 陣列〈執行〉 | `[]` | 最近 20 次執行，舊到新 | 空不寫 | — | 搬 |
| `created_at` | 整數 | 0 | 建立（**秒**） | 寫 | — | 搬 |
| `updated_at` | 整數 | 0 | 更新（秒） | 寫 | — | 搬 |
| `next_run` | 整數 | 沒有 | 排程觸發的下次時間（秒）；沒有 ＝ 只能手動 | 空不寫 | — | 重建 |
| `queued` | 陣列 | `[]` | 執行中又來的、排隊等的（最多 10）：`{"at": 秒（必填）, "by": 字串（必填）, "input": 空不寫}` | 空不寫 | — | 丟（前提：沒有執行中的工作流） |
| `uid` | 字串 | `""` | 永久識別碼（24 個十六進位），編號被換掉時用它找 | 空不寫 | — | 搬 |

`queued[]` 三欄各算。

#### 8.3a 節點：`workflows[].nodes[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 節點編號（工作流內） | 寫 | — | 搬 |
| `kind` | 物件 | 必填 | 節點內容，用 `kind.type` 分種類（下表） | 寫 | 內含 | 搬 |
| `x` | 小數 | 必填 | 畫布座標（左上角） | 寫 | — | 搬 |
| `y` | 小數 | 必填 | 同上 | 寫 | — | 搬 |

`kind` 的種類（例：`{"id":2,"kind":{"type":"step","bot":3,"prompt":"整理新聞"},"x":0,"y":190}`）：

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `kind.type` | 列舉 | 必填 | `trigger` 觸發器、`prompt` 提示詞、`material` 素材、`step` 助手、`llm` LLM、`media` 圖片／影片、`approve` 審批、`check` 條件、`output` 輸出；舊的 `deliver`、`notify`（現在都做成 `output`） | 寫 | — | 搬 |
| `trigger` › `schedule` | 物件 | 沒有 | 排程（同 §3.4a） | 空不寫 | — | 搬 |
| `trigger` › `event` | 物件 | 沒有 | 事件（同 §4）。兩個都沒有 ＝ 手動 | 空不寫 | 內含 | 搬 |
| `prompt` › `text` | 字串 | 必填 | 交給下一步的文字 | 寫 | — | 搬 |
| `material` › `files` | 字串陣列 | `[]` | 交給下一步的檔案（絕對路徑） | 寫 | — | 搬（改寫路徑） |
| `material` › `note` | 字串 | `""` | 說明 | 空不寫 | — | 搬 |
| `step` › `bot` | 整數 | 必填 | 哪個 Bot 做 | 寫 | — | 搬 |
| `step` › `prompt` | 字串 | 必填 | 要它做什麼 | 寫 | — | 搬 |
| `llm` › `model` | 字串 | `""` | `<供應商代號>:<模型>`（例 `custom:m`）；空 ＝ 主要 Bot 的模型 | 空不寫 | — | 搬 |
| `llm` › `prompt` | 字串 | 必填 | 提示詞 | 寫 | — | 搬 |
| `llm` › `system` | 字串 | `""` | 系統提示 | 空不寫 | — | 搬 |
| `media` › `bot` | 整數 | 必填 | 哪個 Bot 做 | 寫 | — | 搬 |
| `media` › `prompt` | 字串 | 必填 | 要做什麼圖／影片 | 寫 | — | 搬 |
| `media` › `video` | 布林 | false | true ＝ 影片，false ＝ 圖片 | 寫 | — | 搬 |
| `approve` › `prompt` | 字串 | 必填 | 要使用者確認什麼 | 寫 | — | 搬 |
| `check` › `bot` | 整數 | 必填 | 哪個 Bot 判斷 | 寫 | — | 搬 |
| `check` › `condition` | 字串 | 必填 | 什麼情況走「是」 | 寫 | — | 搬 |
| `deliver` › `bot` | 整數 | 必填 | （舊）結果送進這個 Bot 的聊天 | 寫 | — | 搬（轉成 output） |
| `notify` › `text` | 字串 | 必填 | （舊）通知文字 | 寫 | — | 搬（轉成 output） |
| `output` › `bot` | 整數 | 0 | 結果送進這個 Bot 的聊天（0 ＝ 不送） | 寫 | — | 搬 |
| `output` › `notify` | 布林 | false | 發通知 | 寫 | — | 搬 |
| `output` › `text` | 字串 | `""` | 通知文字；`{結果}` ＝ 上一步結果的第一行 | 空不寫 | — | 搬 |

#### 8.3b 連線、畫布筆記、版本

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `edges[].from` | 整數 | 必填 | 從哪個節點 | 寫 | — | 搬 |
| `edges[].to` | 整數 | 必填 | 到哪個節點 | 寫 | — | 搬 |
| `edges[].port` | 字串 | `""` | 從「條件」出去走哪邊：`yes` 是、`no` 否；空 ＝ 唯一的路 | 空不寫 | — | 搬 |
| `notes[].id` | 整數 | 必填 | 編號（和節點共用編號空間） | 寫 | — | 搬 |
| `notes[].kind` | 列舉 | 必填 | `pen` 筆畫、`sticky` 便利貼、`shape` 形狀、`text` 文字 | 寫 | — | 搬 |
| `notes[].x` | 小數 | 必填 | 位置 | 寫 | — | 搬 |
| `notes[].y` | 小數 | 必填 | 位置 | 寫 | — | 搬 |
| `notes[].w` | 小數 | 0 | 寬（便利貼預設 200） | 寫 | — | 搬 |
| `notes[].h` | 小數 | 0 | 高（便利貼預設 160） | 寫 | — | 搬 |
| `notes[].text` | 字串 | `""` | 文字 | 空不寫 | — | 搬 |
| `notes[].color` | 字串 | `""` | 顏色名（同頭像顏色）或 `#rrggbb`；便利貼用 `#f7c9b0` `#f6e3a1` `#bfe3c0` `#bcd7f2` `#f3c1d9` `#e2dfd8`；空 ＝ 文字色 | 空不寫 | — | 搬 |
| `notes[].author` | 字串 | `""` | 誰貼的 | 空不寫 | — | 搬 |
| `notes[].points` | 陣列〈[x, y]〉 | `[]` | 筆畫的點（相對 x, y） | 空不寫 | — | 搬 |
| `versions[].at` | 整數 | 必填 | 版本時間（秒） | 寫 | — | 搬 |
| `versions[].name` | 字串 | 必填 | 當時的名字 | 寫 | — | 搬 |
| `versions[].nodes` | 陣列〈節點〉 | 必填 | 當時的節點 | 寫 | 內含 | 搬 |
| `versions[].edges` | 陣列〈連線〉 | 必填 | 當時的連線 | 寫 | — | 搬 |
| `versions[].notes` | 陣列〈畫布筆記〉 | 必填 | 當時的筆記 | 寫 | — | 搬 |

#### 8.3c 執行：`workflows[].runs[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 整數 | 必填 | 執行編號（工作流內） | 寫 | — | 搬 |
| `started` | 整數 | 必填 | 開始（秒） | 寫 | — | 搬 |
| `ended` | 整數 | 沒有 | 結束（秒） | 空不寫 | — | 搬 |
| `state` | 列舉 | 必填 | `running`、`waiting`（等你確認）、`done`、`failed`、`stopped`；開機時執行中的會被結束成中斷，原因「AnyBot 在執行中被關掉，這一步可能做到一半」 | 寫 | — | 搬 |
| `by` | 字串 | `""` | 誰叫它跑：`手動`、`定時`、`觸發`、`試跑這一步`，或叫它跑的 Bot 的名字 | 寫 | — | 搬 |
| `order` | 整數陣列 | 必填 | 節點執行順序（開始時定好） | 寫 | — | 搬 |
| `at` | 整數 | 必填 | 跑到 `order` 的第幾個 | 寫 | — | 搬 |
| `steps` | 陣列〈執行步驟〉 | 必填 | 每個節點的狀態（下表） | 寫 | — | 搬 |
| `why` | 字串 | `""` | 停止或失敗的原因 | 空不寫 | — | 搬 |
| `input` | 字串 | `""` | 開始時帶的東西（事件內容，或試跑的輸入），最多 6000 字 | 空不寫 | — | 搬 |
| `steps[].node` | 整數 | 必填 | 哪個節點 | 寫 | — | 搬 |
| `steps[].state` | 列舉 | 必填 | `waiting`、`running`、`done`、`failed`、`skipped`（條件沒走的路） | 寫 | — | 搬 |
| `steps[].output` | 字串 | `""` | 這一步交給下一步的結果（簡短） | 空不寫 | — | 搬 |
| `steps[].bot` | 整數 | 0 | 做這一步的 Bot | 寫 | — | 搬 |
| `steps[].asked_at` | 整數 | 0 | 問題送進那個 Bot 聊天的時間（**毫秒**） | 寫 | — | 搬 |
| `steps[].files` | 字串陣列 | `[]` | 交給下一步的檔案（絕對路徑） | 空不寫 | — | 搬（改寫路徑） |

### 8.4 待改進：`improvements[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `text` | 字串 | 必填 | 問題描述 | 寫 | — | 搬 |
| `count` | 整數 | 必填 | 被回報幾次（2 次以上算重複） | 寫 | — | 搬 |
| `first` | 整數 | 必填 | 第一次回報（秒） | 寫 | — | 搬 |
| `last` | 整數 | 必填 | 最近一次（秒） | 寫 | — | 搬 |
| `bots` | 字串陣列 | `[]` | 回報的 Bot（名字） | 空不寫 | — | 搬 |

## 9. 聊天檔

### 9.1 `chats/bot-<id>.json`（目前的聊天）

一個 JSON 物件。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `messages` | 陣列〈一則訊息〉 | `[]` | 目前的聊天，舊到新。包含摘要訊息（`summary`），人看到的訊息一則都不刪 | 寫 | — | 搬 |
| `threads` | 陣列〈[標題, 訊息陣列]〉 | `[]` | 過去的對話：**只有 `chats/threads/` 那個檔寫不進去時才放這裡**；也可能是更舊版本放的（§10） | 平常不寫 | — | 搬 |

- 讀的順序：`state.json` 裡那個 Bot 的 `messages` 和 `threads` 都是空的，才讀聊天檔。`state.json` 裡有（聊天檔寫失敗那次），就用 `state.json` 的，聊天檔下次存檔會被蓋掉。
- 開機時，`bots[]` 裡沒有的 Bot 的聊天檔會被刪掉。遷移時看到孤兒聊天檔：列進報告，不搬（待確認）。

### 9.2 `chats/threads/bot-<id>.json`（過去的對話）

一個 JSON 陣列，**新的在前**。每個元素是兩個元素的陣列：`[標題字串, 訊息陣列]`。標題是第一則使用者訊息的第一行（最多 24 字），找不到時是「對話」。沒有過去的對話時這個檔不存在。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `[0]` | 字串 | 必填 | 標題 | 寫 | — | 搬 |
| `[1]` | 陣列〈一則訊息〉 | 必填 | 那段對話的訊息，舊到新 | 寫 | — | 搬 |

### 9.3 一則訊息

出現在：聊天檔的 `messages`、過去的對話、`state.json` 的 `bots[].regen_undo`（和聊天檔寫失敗時的 `bots[].messages`／`threads`）、`brain-copy.json`。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `role` | 列舉 | 必填 | `"User"` 人（或叫醒 Bot 的例行任務、別的 Bot、管道）、`"Assistant"` Bot 的回答、`"Tool"` 一個步驟的結果 | 寫 | — | 搬 |
| `text` | 字串 | 必填 | 內容（Markdown）。引用別則訊息時，被引用的字以 `> ` 開頭的行放在最前面（最多 8 行）；叫醒的訊息開頭有標籤：「［目標］」「［例行任務］」「［觸發］」「［訊息管道］」「［來自 Bot］」「［交付］」「［核對］」，舊的是「⏰ 」「⚡ 」「💬 」「📨 」 | 寫 | — | 搬 |
| `thinking` | 字串 | `""` | 模型的思考過程（顯示用） | 寫 | — | 搬 |
| `via` | 字串 | `""` | 誰寫的：`<供應商顯示名> · <模型>`；群組裡 `<Bot 名> · <供應商> · <模型>`；重播技能時後面加「（重播）」 | 寫 | — | 搬 |
| `error` | 布林 | false | 這則是錯誤訊息 | 寫 | — | 搬 |
| `tool_calls` | 陣列〈步驟呼叫〉 | `[]` | 這則回答要跑的步驟（§9.4） | 空不寫 | — | 搬 |
| `tool_call_id` | 字串 | `""` | `"Tool"` 訊息：回答的是哪個步驟 | 空不寫 | — | 搬 |
| `tool_name` | 字串 | `""` | `"Tool"` 訊息：步驟（工具）名 | 空不寫 | — | 搬 |
| （無鍵）給模型看的截圖 | — | — | 只在記憶體，不存檔 | 不寫 | — | — |
| `summary` | 布林 | false | 這則是「縮短對話」的摘要；模型從最新的摘要開始讀 | 寫 | — | 搬 |
| `blocks` | 陣列〈JSON 物件〉 | `[]` | Claude 原生的內容區塊（含思考簽章、工具呼叫），下一輪原樣送回 Claude | 空不寫 | — | 待決定 |
| `wake` | 布林 | false | 例行任務、觸發、別的 Bot 叫醒 Bot 的訊息（像人的訊息一樣開始一輪） | 寫 | — | 搬 |
| `fresh` | 布林 | false | 從這則重新開始：這一輪模型不讀之前的聊天（例行任務的一次執行） | 空不寫 | — | 搬 |
| `steer` | 布林 | false | Bot 工作中送的，插話改方向，不另開一輪 | 寫 | — | 搬 |
| `author` | 整數 | 0 | 群組裡是哪個 Bot 回答的（0 ＝ 這個聊天自己的 Bot） | 空不寫 | — | 搬 |
| `reactions` | 字串陣列 | `[]` | 使用者加的表情回應（`👍` ＝ 你已確認） | 空不寫 | — | 搬 |
| `attachments` | 陣列〈附件〉 | `[]` | 使用者附的檔、Bot 交出的檔（§9.5） | 空不寫 | — | 搬 |
| `at` | 整數 | 0 | 寫的時間（毫秒；0 ＝ 更舊版本、沒記時間） | 空不寫 | — | 搬 |
| `interrupted` | 布林 | false | 「AnyBot 關掉時這輪被切斷」的卡片，有「繼續」；不送給模型 | 空不寫 | — | 搬 |
| `memories` | 字串陣列 | `[]` | 這則訊息帶入的記憶（使用者在輸入框下選的，或 AnyBot 這輪開始時挑的） | 空不寫 | — | 搬 |
| `memories_set` | 布林 | false | 帶入的記憶已經定了，不再挑 | 空不寫 | — | 搬 |
| `seen` | 字串 | `""` | 送出時使用者旁邊開著什麼（例「工作流「早報」（#1）的畫布」） | 空不寫 | — | 搬 |
| `goal` | 字串 | `""` | 這則訊息朝向的目標（「/goal …」或 AnyBot 的「還沒達成」） | 空不寫 | — | 搬 |
| `folded` | 布林 | false | 這個步驟結果已收起：人看得到全文，模型只讀開頭 | 空不寫 | — | 搬 |
| `note` | 布林 | false | AnyBot 自己寫的一行（目標進度），小字置中顯示，不送給模型 | 空不寫 | — | 搬 |
| `routine` | 整數 | 0 | 這則叫醒訊息跑的是哪個例行任務 | 空不寫 | — | 搬 |
| `sent` | 字串 | `""` | `"Tool"` 訊息：系統看到它送出了什麼、送到哪（「回覆 <貼文> → <回覆的網址>」） | 空不寫 | — | 搬 |
| `untrusted_sender` | 布林 | false | 不是擁有者送的（團隊成員、外部訊息管道）：由它開始的回合不帶擁有者的記憶與權限 | 空不寫 | — | 搬 |
| `from_bot` | 整數 | 0 | 同事之間：哪個 Bot 送來的（0 ＝ 人、例行任務、管道） | 空不寫 | — | 搬 |
| `asked` | 布林 | false | 那個 Bot 在等回答（這輪最後的話會送回給它） | 空不寫 | — | 搬 |
| `hops` | 整數 | 0 | 從人那裡轉了幾個 Bot 才到（超過 3 不能再轉） | 空不寫 | — | 搬 |
| `branch` | 整數 | 0 | 這則是關於哪條支線（開始的卡片或它的報告） | 空不寫 | — | 搬 |
| `check` | 陣列〈核對標記〉 | `[]` | 完成宣稱核對：回答說的和這輪實際做的對不上（§9.7），顯示「未驗證」 | 空不寫 | — | 搬 |
| （無鍵）給模型讀的附加說明 | — | — | 每輪臨時組出來，不存檔 | 不寫 | — | — |

**卡片**沒有獨立欄位，都是步驟：生活卡片是名字 `show_card` 的步驟，參數 `{"kind": "flight"|"place"|"weather"|"product"|"event"|"other", "title", "subtitle", "rows": [{"label","value"}], "link"}`；問題卡是名字 `ask_person` 的步驟。**核准卡**不存在聊天裡（只在執行時）。

### 9.4 步驟呼叫：`tool_calls[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `id` | 字串 | 必填 | 步驟編號（模型給的；對應 `"Tool"` 訊息的 `tool_call_id`） | 寫 | — | 搬 |
| `name` | 字串 | 必填 | 工具名 | 寫 | — | 搬 |
| `args` | 字串 | 必填 | 參數，模型寫的 JSON **文字**（不是物件） | 寫 | — | 搬 |
| `status` | 列舉 | `"Done"` | `"Waiting"` 等核准、`"Running"`、`"Done"`、`"Denied"` 被拒、`"Failed"`。開機時 `Waiting`／`Running` 都改成 `Denied` | 寫 | — | 搬 |

### 9.5 附件：`attachments[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `name` | 字串 | 必填 | 顯示的檔名（最多 120 字） | 寫 | — | 搬 |
| `path` | 字串 | 必填 | 資料夾裡那份副本的絕對路徑：`attachments/bot-<id>/<YYYYMMDDHHMMSS毫秒>-<檔名>`（單檔最多 50 MB）。工作流素材的附件可能直接指向原檔 | 寫 | — | 搬（改寫路徑） |
| `image` | 布林 | false | 是圖片 | 寫 | — | 搬 |
| `bytes` | 整數 | 0 | 大小 | 寫 | — | 搬 |
| `on_box` | 字串 | `""` | 在 Bot 的電腦上也放了一份的位置（空 ＝ 沒有） | 寫 | — | 搬 |
| `id` | 字串 | `""` | 附件編號（12 個十六進位；更舊的沒有） | 空不寫 | — | 搬 |
| `sha` | 字串 | `""` | 檔案 SHA-256 的前 12 個十六進位（更舊的沒有） | 空不寫 | — | 搬 |
| `seen` | 陣列〈交付確認〉 | `[]` | 各裝置回報看到、點開或顯示失敗（§9.6） | 空不寫 | — | 搬 |

### 9.6 交付確認：`attachments[].seen[]`

每台裝置每種只記第一次；最多 12 筆，其中團隊成員的裝置最多 4 筆。

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `device` | 字串 | 必填 | 哪台：「這台電腦」、螢幕電腦的名字、「iPhone」… | 寫 | — | 搬 |
| `kind` | 列舉 | 必填 | `"shown"` 顯示了、`"opened"` 點開了、`"failed"` 顯示不了 | 寫 | — | 搬 |
| `at` | 整數 | 必填 | 時間（毫秒） | 寫 | — | 搬 |
| `why` | 字串 | `""` | 顯示不了的原因代碼（例 `missing`、`unreadable`） | 空不寫 | — | 搬 |
| `guest` | 布林 | false | 團隊成員的裝置（記下來但不叫醒 Bot） | 空不寫 | — | 搬 |

### 9.7 核對標記：`check[]`

| 鍵 | 型別 | 缺少時 | 意思 | 寫入 | 秘密 | 遷移 |
|---|---|---|---|---|---|---|
| `kind` | 列舉 | 必填 | `undone` 說做完了但那一步失敗或被拒、`no_file` 說放了檔案卻沒有、`dead_link` 只有 Bot 的電腦打得開的連結、`file_link` 指向聊天裡檔案的連結（不顯示）、`count` 數字對不上 | 寫 | — | 搬 |
| `why` | 字串 | 必填 | 白話原因 | 寫 | — | 搬 |

## 10. OLD 最後一版還接受的舊格式

遷移只吃 OLD 最後一版開過一次之後的資料（決定 4）。下面標「開一次就轉掉」的，遷移工具不會碰到；其他的**還可能在磁碟上**，遷移工具要接受。

| 舊格式 | OLD 怎麼處理 | 遷移工具會不會碰到 |
|---|---|---|
| 聊天放在 `state.json` 的 `bots[].messages`／`threads` 裡 | 照讀，第一次存檔搬到聊天檔 | **會**：只要有一次聊天檔寫失敗，那次的 `state.json` 就整份帶聊天 |
| 過去的對話放在 `chats/bot-<id>.json` 的 `threads` 裡 | 照讀，和 `chats/threads/` 的合併（聊天檔裡的排前面；標題、則數、第一則時間都一樣的算同一段，不重複），下次存檔搬出去 | **會**：`chats/threads/` 寫失敗時就留在這裡 |
| 秘密欄位是明文 | 照讀；有鑰匙時下次存檔封存 | **會**：沒有鑰匙的電腦一直是明文 |
| 工作流節點 `deliver`、`notify` | 照讀、照跑 | **會** |
| 手機密碼舊格式 `<鹽>$<PBKDF2>` | 照驗證 | **會** |
| `default_computer` 是 `"你的電腦"` | 當成使用者現在用的這台 | **會** |
| 訊息沒有 `at`（0）、附件沒有 `id`／`sha` | 照讀 | **會** |
| 叫醒訊息的舊標籤「⏰ 」「⚡ 」「💬 」「📨 」 | 照顯示成對應的標籤 | **會** |
| 計畫沒有 `stage`／`steps`／`revisions` | 用預設 | 會（極少） |
| Bot 有 `soul`（個性和指示分開存） | 併到 `instructions` 最前面（已經包含就不併），以後不再寫 | 開一次就轉掉 |
| `accent` 是 `"BotColor"` 而且沒有 `bubble_quiet_offered` | 改成 `"Quiet"`，`bubble_quiet_offered` 設 true | 開一次就轉掉 |
| 沒有 `voice` 或 `voice.saved` 是 false | 每個 Bot 的 `voice` 清空（舊的是 macOS 聲音名）；有 OpenAI 金鑰的話 `voice.listen` 設成 `open_ai`；`saved` 設 true | 開一次就轉掉 |
| 更舊版本預設的五個範例 Bot（編號 1–5，名字和描述沒改、沒聊過、沒例行任務、沒記憶、沒過去的對話） | 全部移除，當成全新安裝 | 開一次就轉掉 |
| 進行中的回合（步驟 `Waiting`／`Running`、空的回答） | 步驟改 `Denied`；空回答改成「（這次的回應被中斷了，可能是 App 關閉或重新啟動。按「重新產生」再試一次。）」並標 `error`；`last_turn` 設成 `interrupted`／`error` | 前提是 Bot 都閒置，所以不會 |
| 例行任務執行紀錄 `running`、工作流執行中、目標進行中、支線進行中 | 開機時改成中斷／暫停／停止（見各表） | 前提是都閒置，所以不會 |

## 11. `brain-copy.json`（螢幕電腦的大腦副本）

- **在哪**：一台電腦設成「螢幕」（`remote.brain_id` 有值）時，在它的資料夾（`state.json` 旁邊）。
- **是什麼**：大腦的資料在這台的離線副本，讓大腦連不上時也能先打開。最多每 5 秒寫一次。
- **格式**：跟 `state.json` 同一個頂層物件，差別：
  1. **聊天直接在裡面**：每個 Bot 都有 `messages`（空的也寫 `[]`），有過去的對話時也有 `threads`。訊息格式同 §9.3。
  2. **完全不封存**，沒有 `enc:v1:`。
  3. 大腦的秘密換成佔位字串 `"••••（已設定，在大腦上）"`：`providers.*.api_key`、`channels[].token`、`voice.listen/speak.api_key`、`mcp_servers[].env.*`、`mcp_servers[].headers.*`、`mcp_servers[].password`、觸發條件的 `token`／`apiKey`／`secret`／`api_key`（空的不換）。
  4. **`remote` 是這台自己的帳號設定，明文**（含 `host_token`、`backup_key`、`link_key`）；§2 列的「螢幕電腦自己的設定」也是這台自己的值。
- **旁邊的檔**：
  - `brain-copy.at.json`：`{"brain": 大腦編號, "client": 這台在大腦那邊的名字, "version": 副本對應的大腦版本號, "at": 副本收到的時間（毫秒；0 ＝ 更舊版本）}`。副本寫完才寫它；沒有它或 `brain` 對不上，副本就不用。
  - `brain-queue-<大腦編號 SHA-256 的前 12 個十六進位>.json`：還沒送到大腦的訊息和設定修改，`{"brain", "client", "items": [{"job": {"kind": "send", "bot", "id", "text", "atts": [附件], "memories": 字串陣列或 null, "typed", "seen", "uploaded": [...]} 或 {"kind": "edit", "seq", "ops": [...]}, "at": 毫秒, "tried": 毫秒, "first": 毫秒}]}`；秘密照 §1 封存；讀不了就改名 `….unread-<毫秒>.json`。
- **遷移**：`brain-copy.json`、`brain-copy.at.json` → **丟**（大腦自己的資料夾才是正本，在大腦那邊遷移）。`brain-queue-*.json` → 丟，前提：遷移前螢幕的待送都送到了（遷移工具要檢查，有就停下來說原因）。
- **風險**：`brain-copy.json` 裡這台的帳號金鑰是明文；遷移不讀它的 `remote`，以這台 `state.json` 的 `remote` 為準。

## 12. 盤點時順便發現的（請和 `01` 互查）

1. **`avatars/` 不是使用者內容**：裡面是依「形狀＋顏色＋尺寸」產生的頭像 PNG（`<形狀>-<rrggbb>-<像素>.png`），給通知用，隨時可以重畫。`01` 標「搬」，建議改成**重建**。Bot 的頭像只由 `shape` 和 `color` 決定，沒有自訂圖片。
2. **兩個秘密沒加密**：Linear 觸發的 `apiKey`、webhook 觸發的 `secret`（§1.4）。新版要加密存。
3. **Bot 的記憶存兩份**：`bots[].memories`（和 `about_you`）在 `state.json`，`memory.db` 也有。以哪邊為準：待確認。
4. **雲端大腦的鑰匙**（`01` F1 的待確認）：伺服器容器裡通常沒有 Secret Service，鑰匙最可能就是資料卷裡的 `vault.key`，也就是在伺服器上解得開。待確認（看實際的大腦容器）。
5. **webhook 位址綁在例行任務／工作流編號上**（§0.5）：外部服務設定的是 `http://<電腦>:8792/hooks/<編號>`。新版換編號或換位址，外部設定要重設。
6. **時間單位混用**（§0.3）：同一個檔裡毫秒和秒並存，例行任務和工作流是秒。
7. **兩個鍵名寫法**：同一個供應商在 `bots[].provider` 是 `"OpenAi"`，在 `providers` 是 `"openai"`，在 `model_limits` 是 `openai/<模型>`，在工作流 LLM 節點是 `openai:<模型>`。
8. 聊天檔、過去的對話完全不加密（使用者貼進聊天的密碼也是明文）。

## 盤點統計

| 項目 | 數量 |
|---|---|
| 物件種類 | 49：`state.json` 40、聊天檔 7、大腦副本旁的檔 2（`brain-copy.json` 本身沿用 `state.json` 的形狀，不另計）。觸發條件算 1 種（9 個變體），工作流節點內容算 1 種（11 個變體） |
| 欄位（表格逐列 456，加表下註明的子欄位 19） | 475；§11 兩個旁邊檔的鍵寫在內文，未計 |
| 其中只在記憶體、不存檔 | 3（進行中的支線 Bot、訊息的截圖、訊息給模型讀的附加說明） |
| 其中只讀不寫 | 1（`soul`） |
| 封存的秘密欄位 | 15（§1.3；表格裡 14 列，語音的 `api_key` 一列代表聽和說兩欄） |
| 是秘密但明文 | 2（`apiKey`、`secret`） |
| 只存雜湊 | 2（`remote.password`、`devices[].token_hash`） |
| 遷移分類「待決定」的欄位 | 11（`remote` 的 10 欄、訊息的 `blocks`）；另有 1 個做法問題（遷移寬鬆度） |

## 待確認

1. `embed_model` 除了 `""`、`local`、`off`、`ollama` 以外的字串，是不是就是 Ollama 的模型名。
2. `allow_rules` 的完整寫法清單（除了 §3 列的，還有哪些工具名會直接被記成規則）。
3. `bots[].memories`／`about_you` 和 `memory.db` 的 fact 記憶以哪邊為準；兩邊不一致時怎麼合。
4. 孤兒聊天檔（`bots[]` 裡沒有的 Bot）：OLD 開機會刪，遷移時理論上不會有；有的話是不是直接不搬。
5. 雲端大腦容器裡 `state.json` 的封存鑰匙實際在哪（最可能是資料卷裡的 `vault.key`）。

## 待決定

1. **〔已決定 2026-11-24：丟，升級後全部重新登入、重新配對，見 `09` K10〕`remote` 的帳號與配對欄位**（`lan`、`password`、`host_id`、`host_token`、`brain_id`、`brain_name`、`hands_for`、`hands_for_name`、`link_key`、`link_keys`，以及 `devices` 建議丟）：新版的伺服器、配對、手機登入都重新設計（`02`），這些舊身分要不要帶過去，取決於帳號伺服器 `data.json` 怎麼遷移。
2. **〔已決定 2026-11-24：搬，新版要能原樣送回 Claude，見 `09` K13〕訊息的 `blocks`**（Claude 原生內容區塊）：搬的話新版要能原樣送回 Claude；丟的話升級後第一輪少了上一輪的思考簽章，聊天內容不受影響。
3. **〔已決定 2026-11-24：逐欄救回，讀不懂的列進報告，見 `09` K12〕遷移工具要不要比 OLD 寬鬆**：OLD 碰到缺必填欄位、未知列舉值會把整個檔當壞檔；遷移工具要照做（整檔列進報告），還是盡量救回其他欄位。
