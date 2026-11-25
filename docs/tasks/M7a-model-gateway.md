# 里程碑 M7a：模型閘道

這次只做 `crates/models`：把「問模型」這件事做好。執行端的回合、上下文編譯器、伺服器、畫面都不做（M7 其他段做）。

做的東西：
- 中立的請求和結果。
- 兩個客戶端：OpenAI 相容（官方格式和本機模型伺服器的格式）、Claude（Anthropic Messages API）。
- 串流、工具呼叫、思考。
- 錯誤分類、逾時、重試、備用模型。
- 能力表、token 估算和校正。
- 資料政策的檢查。
- 換模型在閘道這一層的部分。

做完後，規格覆蓋增加：37 個新的編號，加上下面「原本的編號」裡，每一列的預期都在這一層驗到的那些。名稱照 `AGENTS.md` 第二節第 4 條。

先讀：
- `AGENTS.md`：五條鐵律、第三節的開發原則、**第七節「並行工作」**。
- `docs/testing.md`：假模型的兩種格式、`make live-model-check`、網路控制、假時鐘。
- `spec/09`：
  - K95、K102、K104、K107，最重要。
  - K13、K91、R44、R56、R58。
- `spec/07` §3：`models` 只准依賴 `foundation`、`protocol`、`domain`；可以發 HTTP；**不准讀寫檔案**；不准依賴 `tools`、`computers`、`mcp`。
- `spec/06`：「HTTP 客戶端」「token 估算」「模型供應商」三列。
- `spec/03c` §13：供應商表。
- `spec/10-behavior/6-integrations-models-settings.md` 的「5. 模型」，**每一段整段讀**，包括新的五段：`models-normalize`、`models-errors`、`models-capabilities`、`models-handoff`、`models-egress`。段落開頭有「以 `09` 為準」的，照 `09` 和新的五段做。
- `spec/10-behavior/3-chat-bots.md`：`chat-stall`、`chat-n15`。
- `spec/10-behavior/1-followup.md`：`task-events` S2、`task-budget` S2、`task-effects`。只要看懂，這次不改。

## 工作方式

- 在你自己的 worktree 和分支（例如 `ws/m7a-models`）做，照 `AGENTS.md` 第七節。
- 分成 **A、B 兩段**。A 做完、`make check` 全綠，commit 一次；B 做完再 commit 一次。
- 回報前先 `git merge main`，解決衝突、`make check` 全綠再回報。不准 rebase。
- 這次不碰 `protocol` 的任務事件。M5 可能還在改那裡。閘道交出自己的「換模型紀錄」，M7 的執行端再把它寫成任務事件。

## 這次的整合測試怎麼算「從外面打」

- 閘道的整合測試：用 `models` 的公開介面，經網路代理打假模型。真的 HTTP，算從外面打（`spec/09` K86）。
- 規格步驟寫「送「hi」」的，＝問閘道一次，你的訊息是「hi」。
- **原本的編號只在整列都驗到時才用**（`09` K86、K109 ⑪）：那一列的每個預期都在這一層驗到，才用那個編號命名測試。預期裡有「畫面出現」「任務狀態＝」這類要執行端或畫面才驗得到的，就用描述性的名字測閘道這一層的部分，編號留在基準檔，回報列出來。
- 「有人在看／沒人在看」（例行任務到點）由呼叫的人告訴閘道。
- 只有最後清單列的編號照這樣算。其他編號要執行端或畫面，不准用編號命名。

## A 段：中立格式、兩個客戶端、串流

1. **假模型要補的**（`tests/harness`，先做）。每一項都要有自我測試，openai、vllm、Claude 三種格式能做的都做：
   - **金鑰比對**：建構時給一個期望的金鑰，只記「有帶、對」「有帶、不對」「沒帶」三種。不准記金鑰的值。
   - **錯誤**：回錯誤時可以帶 Retry-After。錯誤內容照各家的樣子，包括：
     - 「太長」，說明裡帶上限。
     - 「不支援工具」「不支援這個參數」、402。
     - 各家的格式照官方文件；本機模型伺服器的照 `make live-model-check` 實際看到的。
   - **串流裡的錯誤**：「忙」和其他。
   - **用量**：照腳本給輸入、快取讀到、快取寫入、輸出、思考。
   - **Claude**：
     - 思考區塊帶簽章。
     - 被遮住的思考區塊。
     - 拒答、到長度上限的結束原因。
   - **工具呼叫和結束原因**：
     - 自訂的 `finish_reason`（例如 `tool_calls` 卻沒有工具呼叫）。
     - 工具呼叫沒有編號；參數分很多段；參數不是合法的 JSON；一個回答好幾個工具呼叫。
   - **其他回應**：回空白；回 302 轉址；模型清單（有、空的、不回應）。
   - **網路控制**：加一種「連線建立不起來」：連線一直卡住，不是被拒絕。自我測試要證明客戶端真的卡在建立連線。
   - **雲端的名字**：測試要能用雲端的名字（例如 `http://api.cloud.test/v1`）打到假模型。
     - 不准為了測試在產品加旗標或環境變數。
     - 用建構函式傳進去的名稱對照，或讓代理對過去，選一種寫進 `docs/testing.md`。
2. **中立的請求和結果**（`models-normalize` S1、S2、S6–S9）：
   - 中立型別裡不准有任何一家的欄位。原生內容是不透明的附件，標著供應商和模型。
   - 型別放在 `models` 還是 `protocol`，照 `spec/07` 的依賴方向決定，寫進回報。
3. **供應商表**（`models-providers` S1、S3、S4、S5；`spec/03c` §13）：
   - 八家：xAI、OpenAI、Claude、Gemini、OpenRouter、OpenCode Go、Ollama、自訂。ChatGPT 方案這次不做。
   - 網址空的用預設、結尾的「/」拿掉。金鑰去掉前後空白，用 `Secret` 裝，只送到那家設定的網址（`models-n6` S6）。
   - Claude 最多輸出 64,000；OpenRouter 帶 App 名稱「AnyBot」（照 OpenRouter 官方文件用哪個標頭）。
   - OpenCode Go 每個模型的格式寫在能力表。
4. **OpenAI 相容的客戶端**：
   - 串流和不串流。
   - 工具呼叫的參數照編號接起來。
   - 非標準的 `reasoning` 欄位算思考。
   - 有完整工具呼叫就算「要用工具」，不管 `finish_reason`。
   - 串流時要用量（`stream_options.include_usage`）。
5. **Claude 的客戶端**：
   - 串流事件。
   - 思考區塊和簽章存成附件，只原樣送回同一家、同一個模型（`models-normalize` S7）。
   - 拒答、到長度上限。
   - 可快取的標記（`models-n8` S1）。`T-models-n8-1` 要等執行端，這次用描述性的名字測請求裡有標記。
6. **思考模式**（`models-thinking` S1）：自動、快速、平衡、深入，在兩家各帶什麼參數、什麼值，照官方文件定。對照表寫在能力表裡，也貼進回報。
7. **時間一律走 `Clock`**：
   - 連線 15 秒；等第一個字照 `chat-stall` S1 的公式；出字後 35 秒沒動靜算卡住。
   - 不准用 reqwest 自己的 timeout、connect_timeout：它們用 tokio 的時間，假時鐘轉不動，clippy 也禁了。
   - 連線階段要能分開計時。
   - 測試用假時鐘快轉。
8. A 段的測試：
   - `T-models-normalize-1`–`9`、`11`–`14`。
   - `T-models-providers-2`、`3`；`T-models-n6-5`。
   - `T-models-thinking-1`。
   - `T-models-where-1`、`T-chat-stall-1`、`T-models-n1-7`、`T-chat-stall-3`。
   - vllm 格式現在也用在產品測試（整理怪格式要測它），`docs/testing.md` 跟著改。

## B 段：錯誤、重試、備用、能力、資料政策、換模型

9. **錯誤分類**（`models-errors` S1–S3、`models-n2` S1、S2）：
   - 閘道只交出類別、訊息代碼和參數。
   - 新的代碼放 `locales/` 的 en 和 zh-TW。zh-TW 的字照規格裡的字，en 你來寫。
   - 測試：`T-models-errors-1`–`4`、`T-models-n2-1`–`4`、`T-models-providers-1`。
   - 取得模型清單（`models-n3` S1、S2）也在這裡做：最多等 45 秒；空的回「供應商沒有回傳任何模型」；排序、去重、去掉「models/」用描述性的名字測。測試：`T-models-n3-3`、`4`。
10. **重試**（`models-n1` S1–S5、`chat-stall` S1、S3）：
    - 有人在看：共 3 次。沒人在看：共 8 次，加減兩成。
    - Retry-After 最多 30 秒；串流裡說「忙」等 8 秒；回了空白再問一次。
    - 等待中交出通知（代碼和參數），例如「{錯誤}\n{N} 秒後重試（第 k 次，共 m 次）…」。
    - 加減兩成的亂數由建構函式傳進來，閘道自己不產生。
    - 停止的訊號：等待重試時收到就安靜結束、不報錯。`T-models-n1-8` 是端對端，這次用描述性的名字測。
    - 測試：`T-models-n1-1`–`6`、`9`、`10`、`11`。
11. **備用模型**（`models-fallback` S1–S4）：
    - 最多 8 個，照順序；沒金鑰的跳過；資料政策外的跳過。
    - 不能用的那個休息 10 分鐘，用 `Clock` 算。休息的狀態放在哪、多大範圍（每個執行端？每個使用者？），寫進回報。
    - 換過去時不用思考模式。
    - 「從同一步接著做」：閘道向呼叫的人要「給這個模型的請求」，讓 M7 的上下文編譯器可以照新模型重組。
    - 交出換模型紀錄（前、後、原因）。
    - 測試：`T-models-fallback-1`、`2`、`3`、`5`。`T-models-fallback-4` 的「照 `models-n5` 整理」要編譯器，這次只用描述性的名字測「太長不換」。
12. **能力表和估算**（`models-capabilities` S1–S8）：
    - 內建表照當時的官方文件填，每個值附來源。
    - 記下來的能力和校正狀態：做成可以存、可以讀回的快照。真的寫進本機資料庫是 M7 的事。`T-models-capabilities-3` 的「重開」＝用存下的快照建一個新的閘道。
    - 分詞器：
      - o200k 用 `tiktoken-rs`。
      - tokenizer.json 用 `tokenizers`。`models` 不准讀檔，內容由呼叫的人傳進來。
      - 先查這兩個套件的維護者、RustSec 紀錄、編譯時間和大小。太重就先停下回報，不要硬加。
    - 估算加安全邊際超過可用的上下文，就不送出。
    - 測試：`T-models-capabilities-1`–`10`、`T-models-thinking-2`。
13. **資料政策**（`models-egress` S1、S2、S4–S8）：
    - 每次連出去之前檢查，在建立連線之前就擋下。
    - 不跟轉址。
    - 驗收的挑選是一個純函式，給 M7 用。
    - 測試：`T-models-egress-1`–`6`。
    - S3（只有你能改、畫面先問）要工具和畫面，這次不做。
14. **換模型：閘道這一層**（`models-handoff` S2 第一點、S3 的原生內容和思考、S5）：
    - 換模型的要求只影響之後開始的請求，正在收的回答照舊收完。
    - 交出換模型紀錄。
    - 新模型不支援工具、這一步要帶工具 → 不送出。
    - 測試：`T-models-handoff-1`、`6`、`9`、`10`。
15. **跟真的本機模型比對**（`make live-model-check`，不進 `make check`）：
    - 加一組「產品的客戶端」比對：同樣的請求打 vllm 格式的假模型和真的模型。
    - 請求有四種：一般、串流、工具呼叫、思考。
    - 兩邊閘道交出的中立結果，**結構要一樣**（比欄位，不比文字）。
    - 那台的模型清單如果有回可讀長度，加一個超過長度的請求，比對「太長」錯誤的格式。
    - 報告照舊放 `target/live-model/`，不准有金鑰。這個由我在審核時跑，你寫好、說明怎麼跑就好。
16. **`docs/testing.md`**：寫進假模型新的腳本動作、雲端名字怎麼對到假模型、「連線建立不起來」怎麼用。

## 寫法

- 純邏輯（分類、能力合併、估算、校正、政策、挑驗收的模型）不准讀時鐘、不准自己產生亂數。
- 不准為了測試加旗標、環境變數、輸出模式。假的東西從建構函式傳進去。
- 金鑰不准出現在錯誤、紀錄、快照、報告裡。
- 規格沒寫清楚的，不要猜，寫進回報的「要問的問題」。不得不先做時，選最小、之後好改的做法，也寫進回報。目前已知沒寫清楚的：
  - `models-errors` 表裡「等不到第一個字」要不要換備用。
  - 沒人在看的重試，間隔從幾秒開始、怎麼加倍。
  - OpenAI 相容的 `max_tokens` 要不要帶、帶多少。
  - 「串流裡說忙」在 OpenAI 相容格式怎麼認。

## 這次不做（之後的里程碑做）

- 要執行端的回合或上下文編譯器（M7）：
  - `T-models-switch-1`–`3`。
  - `T-models-handoff-2`–`5`、`7`、`8`、`11`。
  - `T-models-normalize-10`。
  - `T-models-fallback-4`、`T-models-n5-2`、`T-models-n5-3`、`T-models-n8-1`。
  - `T-chat-stall-2`、`4`、`5`；`T-chat-n15-1`–`5`。
  - `T-models-egress-7`。
- 要伺服器、帳號、金鑰同步（M6、M7）：`T-models-providers-5`、`T-models-n6-1`–`4`。
- 要電腦和排程（M7、M9）：`T-models-n9-1`–`6`。
- 要畫面（M8）：
  - `T-models-providers-4`、`6`；`T-models-where-2`、`3`；`T-models-bot-1`、`2`。
  - `T-models-thinking-3`；`T-models-n1-8`；`T-models-n2-5`；`T-models-n3-1`、`2`、`5`；`T-models-n5-1`。
  - `T-models-fallback-6`、`8`；`T-models-egress-8`。
- 要工具和核准卡（M7）：`T-models-fallback-7`、`T-models-bot-3`。
- 用量頁（M8）：`T-models-n8-2`。
- ChatGPT 方案（M10 之後）：`T-models-chatgpt-1`–`14`。
- 還沒照 `09` 重寫的：`T-goal-judge-6`。`models-egress` S7 的挑選函式這次先做好。

## 這次要有測試的規格編號

- 原本的（29 個候選，照上面「只在整列都驗到時才用」逐列判斷）：
  - `T-models-where-1`、`T-chat-stall-1`、`T-chat-stall-3`。
  - `T-models-providers-1`、`2`、`3`。
  - `T-models-thinking-1`、`2`。
  - `T-models-n1-1`–`7`、`9`、`10`、`11`。
  - `T-models-n2-1`–`4`；`T-models-n3-3`、`4`；`T-models-n6-5`。
  - `T-models-fallback-1`、`2`、`3`、`5`。
- 新的（37 個）：
  - `T-models-normalize-1`–`9`、`11`–`14`。
  - `T-models-errors-1`–`4`。
  - `T-models-capabilities-1`–`10`。
  - `T-models-handoff-1`、`6`、`9`、`10`。
  - `T-models-egress-1`–`6`。

## 回報

照 `AGENTS.md` 第六節，逐項對照實際檔案。另外附上：
- 每個用了編號的測試：名稱和所在檔案。原本的候選裡沒用編號的，列出來並說哪個預期在這一層驗不到。
- 覆蓋數字前後的變化。
- 照的官方文件和版本：OpenAI Chat Completions、Anthropic Messages（串流、思考、提示快取）、OpenRouter 的標頭。
- 思考模式的對照表、內建能力表的每個值和來源、估算公式和校正的數字。
- 新加的第三方套件：維護者、最近更新、RustSec 紀錄、編譯時間和大小。
- 假模型、網路控制新加了什麼，自我測試的輸出。
- 中立型別放在哪個套件、為什麼；閘道給 M7 用的介面長什麼樣。
- 備用模型的休息狀態放在哪。
- `make live-model-check` 新的那組怎麼跑。
- 規格裡讀不懂、或彼此矛盾的地方。
