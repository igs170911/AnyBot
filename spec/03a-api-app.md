# 03a · App 的介面（舊版盤點）

> 來源：看過舊版的一方盤點 OLD 的 App 本身對外提供、或對外說的介面（手機網頁、其他電腦、帳號伺服器的長連線、這台電腦上的小服務），並和手機頁面、`docs/protocol.md`、`docs/architecture.md` 對照（2026-11-24 初稿）；**不含舊程式結構**。
> 用途：功能清單的來源之一：每條介面背後是哪個使用者看得到的能力，新版要找到對應的做法，不能漏。
> 互查：`00-boundaries.md` 的 B5、B6（也碰到 B2、B9）；`02-architecture.md`（新版改成 POST＋SSE，見 §4、§5.3、§5.5）。

**這是舊版的介面清單，用途是當功能清單的來源；新版 API 重新設計，不照抄這裡的路徑和欄位（00 決定 2）。**

## 怎麼讀

- 每條端點有編號 `E01`…，統計和「待確認」用這個編號。
- 「誰在呼叫」欄：**手機**＝手機網頁（主頁、看畫面頁、Service Worker）；**螢幕**＝把這台當大腦的另一台 AnyBot；**大腦**＝把這台當手腳的另一台 AnyBot；**外部**＝其他程式或瀏覽器；**—**＝找不到呼叫者。
- 「誰可以呼叫」用下面「角色」那張表的名字。
- 數字都是舊版實際的值；看不準的標「待確認」。

## 0. 共通規則

### 0.1 兩條入口

| 入口 | 怎麼到 | 身分 |
|---|---|---|
| 區網直連 | 手機瀏覽器直接開這台：`https://<這台的位址>:8793`（被占用就往後試，到 8802）。設定「同一個網路直接連」打開才有。預設聽這台所有位址；環境變數 `ANYBOT_PHONE_BIND` 可以只聽一個位址，`ANYBOT_PHONE_PORT` 換起始埠 | 用「手機密碼」登入換到的 Bearer token；角色一律是 owner |
| 經帳號伺服器 | 手機打 `<帳號伺服器>/h/<這台的 id>/api/...`，帳號伺服器經長連線（見 §4）交給這台；另一台 AnyBot 用長連線的 `call`，或 HTTP `/relay/hosts/<id>/api/...` | 帳號伺服器在每個請求附上 `x-anybot-user`（帳號）、`x-anybot-role`（角色）、`x-anybot-from`（發問的那台電腦 id；手機是空的） |

**區網直連的連線規則**

- 同一個埠同時講 HTTPS 和 HTTP。HTTPS 用這台自己做的憑證（800 天；設定裡顯示指紋讓人比對）。純 HTTP 只接受從這台自己和 Tailscale 來的連線；其他位址的 HTTP 一律 `308` 轉到同一網址的 https。憑證做不出來時整個退回純 HTTP，設定裡會說。
- `Host` 標頭只能是空的、`localhost`、`*.local`、`*.ts.net` 或 IP，否則 `403 forbidden host`（防 DNS rebinding）。
- 不是 GET／HEAD 的請求必須同源（`Origin` 或 `Sec-Fetch-Site` 對得上 `Host`），否則 `403 forbidden origin`。
- 連線保留 30 秒等下一個請求（回 `Keep-Alive: timeout=25`）。文字類回應（`text/*`、JSON、manifest、SVG）超過 1 KB、瀏覽器收 gzip 時就壓。
- 請求限制：標頭 64 KB 內、15 秒內到齊；整個請求 90 秒內到齊（否則 `408`）；body 最多 30 MB，沒帶登入的請求最多 256 KB（`413`）；所有連線同時在收的 body 合計超過 512 MB 時回 `503`；同時最多 512 條連線（`503`「伺服器忙碌中，請稍後再試」），同一個公網位址最多 48 條（`429`「同一個位址的連線太多了，請稍後再試」；私有網路、Tailscale、本機不限）；WebSocket 另算，最多 8192 條。
- 設定頁列出手機可以開的網址（每個網卡位址一行；Tailscale 位址給 http、其他給 https）。開不起來的原因（位址還沒出現、埠被占、沒權限）寫在設定和紀錄檔，每 5 秒再試。

**經帳號伺服器時，帳號伺服器先擋掉的**（帳號伺服器那邊的規則，列出來是因為它決定誰叫得到什麼）

- 手機只能到 `/h/<id>/api/...`；其中 `/api/device/...` 一律 `404`；查詢字串的 `t` 會被拿掉。靜態檔（`/`、`/screen`…）由帳號伺服器自己給同一份，不轉給 App。
- 另一台電腦只能叫 `/api/sync…`（角色 mirror）和 `/api/device/run`（角色 device）；用配對碼加入的手腳不能叫別台（`403`「這台電腦是配對加入的手腳，不能控制其他電腦或修改大腦」）；不同帳號 `403`；一台同時最多 32 個（`429`）。
- 等回答：一般 35 秒，`/api/device/run` 610 秒。對方不在線立刻 `503`（「那台 AnyBot 目前離線…」）；收到了但逾時 `504`「那台 AnyBot 收到了，但太久沒有回應（可能太忙，或網路很慢）」。

### 0.2 角色

| 角色 | 是誰 | 能叫的 |
|---|---|---|
| owner | 擁有者的手機；區網直連一律是 owner | 除了 `/api/sync/*`、`/api/device/run` 以外全部 |
| admin | 團隊管理員（經帳號伺服器） | 和 owner 同樣的路徑；但 `/api/act` 裡只能做「看」和少數聊天動作（見 E18 的表），記憶、傳送、ChatGPT 登入、連接器市集不行 |
| member | 團隊成員（經帳號伺服器） | 只有：GET `/api/state`、GET `/api/bots/<id>`、GET `/api/pages/…`、GET `/api/file`、GET `/api/preview…`、POST `/api/bots/<id>/` 的 `send`、`read`、`seen`、`upload`、POST `/api/voice/transcribe`、`/api/voice/speech`、POST `/api/diagram`。而且只碰得到「分享給團隊」的 Bot 和它的頁面、檔案，其他回 `404`「找不到（或這個 Bot 沒有分享給團隊）」 |
| mirror | 同帳號的另一台 AnyBot，把這台當大腦（螢幕電腦） | 只有 `/api/sync/*` |
| device | 同帳號的另一台 AnyBot（大腦），要用這台當手腳 | 只有 `/api/device/run` |
| none | 帳號伺服器沒說角色 | 都不行 |

角色不符：`403`「你在這台 AnyBot 的角色不能做這件事（請擁有者處理）」。`/api/act` 裡 owner 專屬的事：`400`「這件事只有這台 AnyBot 的擁有者能做」。

### 0.3 回應與錯誤

- 回應是 JSON；錯誤是 `{"error": "白話說明"}` 加狀態碼。沒有的路徑 `404`「沒有這個網址」（螢幕電腦看到這句會改說「<大腦名>的 AnyBot 是舊版，還沒有這個功能。請更新…」）。
- 很多動作要交給 App 主畫面去做再回答。等不到回 `503`「大腦太忙，沒有及時回應」。預設等 20 秒；同步拉取和同步改動 10 秒；其他寫在各列。
- 這台本身正把別台當大腦時：手機的動作回「這台 AnyBot 連著別的大腦，請在大腦上操作」；同步要求回「這台 AnyBot 本身連著別的大腦」。

## 1. 頁面與靜態檔

區網直連時由 App 給；經帳號伺服器時由帳號伺服器自己給同一份（兩邊各編進一份）。

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E01 | GET | `/`、`/index.html` | 任何人 | — | 手機網頁（單一 HTML）；帶 `ETag`，瀏覽器已有同一版回 `304`；CSP 只准頁面裡自己的腳本 | `Cache-Control: no-cache` | 手機 App 本體 | 手機網頁 | 手機 |
| E02 | GET | `/screen` | 任何人 | 網址 `#` 後面：`lan=1` 或 `host=<id>`、`ticket`、`bot`、`name`、`control=1`、`embed=1` | 看畫面頁（noVNC） | 可被同源頁框住 | 看、接管 Bot 的電腦 | 看 Bot 的電腦 | 手機（主頁把它開成內嵌框） |
| E03 | GET | `/novnc.js` | 任何人 | — | noVNC 1.6.0（MPL-2.0） | 快取 1 天 | 看畫面頁用的 VNC 程式 | 看 Bot 的電腦 | 手機 |
| E04 | GET | `/trackpad.js` | 任何人 | — | 虛擬觸控板和按鍵列 | 不快取 | 兩個頁面共用 | 從手機操作畫面 | 手機 |
| E05 | GET | `/manifest.webmanifest` | 任何人 | — | App 名稱、圖示、兩個捷徑：「主要 Bot 對話」（`./?sc=main`）、「未讀對話」（`./?sc=unread`） | — | 加入主畫面用 | 安裝成 App | 手機 |
| E06 | GET | `/sw.js` | 任何人 | — | Service Worker | — | 頁面和圖示存在手機上（每次打開背景問有沒有新版）；收推播、顯示通知（需要你的那種不自動消失、震兩下）；通知上的「允許／拒絕」；App 圖示上的數字 | 推播通知、快速打開 | 手機 |
| E07 | GET | `/icon.svg` | 任何人 | — | 圖示 | 快取 1 天 | — | 圖示 | 手機 |
| E08 | GET | `/icon-180.png`、`/icon-192.png`、`/icon-512.png`、`/icon-maskable-192.png`、`/icon-maskable-512.png`、`/badge-96.png` | 任何人 | — | PNG | 快取 1 天 | iPhone 主畫面、Android 遮罩圖示、Android 狀態列的單色貓 | 圖示 | 手機 |

## 2. 帳號與登入（App 這一端）

帳號登入、註冊、兩步驟驗證、配對碼、推播訂閱都在帳號伺服器，不在 App（見附錄 A、§5）。App 自己只有區網直連的「手機密碼」。

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E09 | GET | `/api/hello` | 任何人 | — | `{app:"anybot", host, relay, needs_password}` | — | 告訴手機這是 AnyBot 本身（帳號伺服器回 `app:"anybot-relay"`），決定登入畫面要問密碼還是帳號 | 登入畫面 | 手機（只有區網直連時到 App） |
| E10 | POST | `/api/login` | 只限區網直連 | `{password, device}`（device 最多 40 字，預設「手機」） | `{token}` | 同一個來源位址 1 分鐘內錯 5 次，或所有位址合計 30 次 → `429`「密碼錯太多次，請一分鐘後再試」；錯了多等 0.6 秒再回 `401`「密碼不對」；沒設密碼回 `401`「這台 AnyBot 還沒設定手機密碼」 | 用手機密碼登入，發一個 token（只存雜湊），這支手機列進「手機與帳號」的清單 | 區網手機登入 | 手機 |
| E11 | POST | `/api/logout` | 只限區網直連、已登入 | — | `{ok}` | — | 這支手機的 token 作廢 | 登出這支手機 | 手機 |

區網直連的其他 `/api/*` 都要 `Authorization: Bearer <token>`，沒有或失效回 `401`「請先登入」。token **30 天沒用就失效**；最後使用時間最多每分鐘記一次。擁有者可以在手機或電腦上把某支手機登出（E18 的 `device_remove`）。

## 3. 聊天

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E12 | GET | `/api/state` | owner、admin、member | `since`（頁面手上的版本，0＝第一次）、`tz`（手機時區）、`ssig`（頁面手上那份的指紋）；正在看聊天時加 `bot`、`chat_sig`、`chat_delta=1`、`w0`（手上那段從第幾則開始） | 見下方「狀態的內容」 | `since>0` 時最多等 20 秒，有變化馬上回；同一時間最多每 0.4 秒回一次；串流中的聊天最多每 1 秒重送一次；第一次開聊天只給最新 30 則 | 首頁和打開中的聊天的即時狀態（長輪詢）；順便回報手機時區（大腦的時區還是伺服器預設 UTC 時才採用） | 首頁 Bot 清單、誰在工作、等你核准、聊天即時更新 | 手機 |
| E13 | GET | `/api/bots/<id>` | owner、admin、member | `chat_sig`、`chat_delta=1`、`w0`，或 `first=1`（只要最新 30 則） | `{bot, sig, messages 或 unchanged, deliveries, version}`；部分更新時帶 `base, drop, from` | 不節流；找不到 `404`「找不到這個 Bot」 | 一次拿一段聊天 | 打開聊天 | 手機 |
| E14 | GET | `/api/bots/<id>/older` | owner、admin | `from`（訊息編號）、`count`（預設 60，1–120） | `{bot, messages, deliveries, more}` | 等 20 秒 | 往回載入更早的訊息（App 平常只發最新 120 則） | 載入更早的對話 | 手機 |
| E15 | POST | `/api/bots/<id>/<動作>` | 看下表 | 看下表 | `{ok}` 或下表 | 不認得的動作 `404`「沒有這個動作」 | 聊天上的動作 | 見下表 | 手機 |

**E15 的動作**

| 動作 | 誰可以 | 請求重點 | 規則 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|
| `send` | owner、admin、member | `{text, attachments:[附件路徑], id, memories?}` | 空的 `400`「訊息是空的」。同一個 `id`（≤64 字）24 小時內再來只回 `{ok, again:true}`，不做第二次。區網直連時，會放寬權限的聊天指令（例如 `/always-approve`）`403`（見 E18 的區網規則）。不是擁有者時，訊息前面加「【帳號】」（管理員打的 `/goal`、`/plan` 除外）。`memories`：輸入框勾的記憶（最多 20；空陣列＝這次不帶；沒有這欄＝大腦自己挑；只有擁有者的有效）。附件只收這台附件資料夾裡的檔案。成員的訊息不插進擁有者正在跑的回合，排隊等（最多 8 則）。`/goal`、`/plan`、`/doctor` 等 App 指令也從這裡進 | 傳訊息、App 指令 | 手機 |
| `stop` | owner、admin | — | — | 停止 | 手機 |
| `answer` | owner、admin | `{answer, text, call, from}` | 見 §3.1 | 回答核准卡、問題卡、交接卡 | 手機、通知 |
| `read` | owner、admin、member | — | — | 標為已讀 | 手機 |
| `seen` | owner、admin、member | `{att, kind, why, device}`，或一批 `{reports:[…], device}`（一次最多 200） | `att` 最多 32 個十六進位字；`device` 只認 `iPhone`、`iPad`、`Android`、`瀏覽器`，其他叫「手機」，隊友的叫「<帳號> 的 iPhone」；隊友的回報只記錄、不叫醒 Bot；不認得 `400`「不認得這個回報」 | 交付確認：手機顯示了、打開了、或顯示不了 Bot 給的檔案（✓／✓✓） | 手機 |
| `new` | owner、admin | — | 目前的對話收進「過去的對話」（最多留 50 段） | 開新對話 | 手機 |
| `regenerate` | owner、admin | — | — | 重新產生 | 手機 |
| `restore_answer` | owner、admin | — | — | 還原上一個答案 | 手機 |
| `restore` | owner、admin | `{index}` | 目前的對話收回過去的對話 | 回到過去的某段對話 | 手機 |
| `set` | owner、admin | `{provider, model, computer}`（空的不改） | — | 換這個 Bot 的模型或電腦 | 手機 |
| `upload` | owner、admin、member | `?name=`（最多 80 字，去掉 `/`、`\`），body 是檔案原始位元組 | 回 `{path, name, image}`；空檔 `400`「檔案是空的」；最多 30 MB；手機上傳不設逾時 | 附件先上傳，送訊息時帶路徑 | 手機 |

**狀態（E12）的內容**

- 最上層：`version, host, person, accent, pin_order, newer`（有新版）、`always`（一律放行開著）、`bots, tasks, approvals, box`（Bot 的電腦狀態文字）、`box_available, mac`（可以看這台 Mac）、`role, pages`（最新 12 個）、`models`（可以換的模型：每家已設好的供應商和它會聊天的模型）、`sections`（側邊欄分區順序）、`theme, palettes{light, dark, backdrop}`（桌面的外觀，手機照穿）、`voice{listen, speak}`（這台聽得懂／唸得出來嗎）、`ssig`。
- 帶了 `ssig` 時只送變了的鍵（`base` 說是接在哪份上），沒變的 Bot 只送 `{id, same:true}`。
- 每個 Bot：`id, name, description, color, shape, busy, status`（正在做什麼，和電腦上輸入框上方同一句）、`unread, last`（最後一句）、`members`（是群組）、`pinned, can_restore, team, skills, member_names`（@ 點名清單）、`primary, time, via`（供應商 · 模型）、`runs, runs_words`（模型在本機／區網／雲端）、`provider, model, computer, computers[{name, kind, online, person}]`、`routines[{id, title, paused, when}]`、`threads`（最近 20 段）、`folder, color_key, order, bubble, thinking, step_model, queued`（工作中排隊的話）、`plan{task, stage, line, steps}`、`goal{condition, reason, active, line}`。
- 每則訊息：`role`（user／assistant）、`text, via, error, summary, interrupted, wake, note, author, reactions, attachments, at, index, tools, memories, check`。`tools` 每一項是一般步驟 `{label, detail（最多 120 字，祕密遮掉）, status}`，或卡片 `{card}`、計畫 `{plan}`、計畫更新 `{plan_update}`、同事對話 `{talk{ask, to, text}}`、頁面卡 `{page}`；`status` 是 `waiting/running/done/denied/failed`。附件 `{name, image, path, bytes, kind, sha}`，Bot 給的檔再加 `{id, delivery（chat/shown/opened）, seen}`。`check` 是「未驗證」的原因。
- 非擁有者拿不到 `memories`；成員另外拿不到 `thinking` 和步驟結果；成員看到的 Bot 沒有 `routines, threads, queued, goal, plan`，也拿不到 `approvals`、`models`，`always` 一律 false。

### 3.1 核准

沒有專屬端點：卡片在 E12 的 `approvals`、螢幕在 E40 的 `approvals`；回答走 E15 `answer`、螢幕走 E43 `answer`。

- **卡片**：`{bot, title, reason, tool, can_always, can_scope, kind, computer, key, call_id}`；`kind` 是 `ask`、`handoff`（交接：需要你）、`question`（問題卡，多 `questions`）。標題長這樣：「<Bot> 想要<動作>」「允許 <Bot> 使用電腦工具嗎？」「允許 <Bot> 在你的電腦上執行任務嗎？」「<Bot>：需要你」「<Bot> 問你」。支線的卡顯示在主 Bot 的聊天，標題前加「支線：」，多一個 `target`（支線自己的 id）。`can_scope` 只在智慧模式、有授權類別時出現。
- **回答**：`answer` 是 `once`（這次）、`scope`（智慧模式「這輪都准」，範圍用卡片自己的）、`always`（一律允許，類別用卡片自己的，不採用手機傳的字）、`answered`＋`text`（問題卡），其他一律當拒絕（`reject`）。問題卡只收 `answered`，其他都算拒絕。
- **只落在那張卡上**：`call` 必須等於卡片的 `key`。對不上（回答過了、換成別張了、頁面太舊說不出是哪張）就什麼都不做，並在聊天裡留一句說明（手機、螢幕、通知各有一句，例如「通知上的「允許／拒絕」沒有用上：那個請求已經不在等了…」）。
- **通知上的允許／拒絕**：`from:"notice"`，只能是 `once` 或 `reject`，而且只對有給按鈕的那種卡有效；交接卡的「我完成了」、問題卡的「略過」不能從通知按。
- **推播怎麼送出去**：App 不直接推播，而是把通知交給帳號伺服器（`/relay/host/notify`：`{title, body, bot, kind, badge, ask, tag}`；`kind` 是 `need`／`done`／`routine`／`file`；`badge`＝等你的 Bot 數；`ask`＝卡片 key，有它通知才出現「允許／拒絕」）。使用者的 hooks 也同時收到 Notification。點通知打開 `./#host=…&bot=…&answer=…&call=…`。

## 4. Bot 設定與 App 設定

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E16 | GET | `/api/bots/<id>/detail` | owner、admin | — | Bot 的全部可編設定：`name, description, instructions, shape, color, provider, model, thinking, step_model, fallbacks, workdir, speak_replies, voice, voice_rate, voice_lang, strengths, not_for, abilities[], skills[], may_use[], computer, computers[], tools_enabled, context_tokens, folder, pinned, primary, unread, team, members[], memories, allow_rules, demos[], routines[]`（每個含排程、觸發、狀態、最近 5 次結果）、`threads[]`、`files[]`（聊天裡的檔，最多 80） | 20 秒；`memories`、`allow_rules` 只給擁有者 | Bot 的編輯畫面和聊天旁邊的面板 | Bot 設定、例行任務、過去的對話、這個 Bot 的檔案 | 手機 |
| E17 | GET | `/api/settings` | owner、admin | — | 設定頁的全部內容，**沒有任何金鑰**（只說有沒有設）：版本、主題、外觀（強調色、對比、風格、語言）、時區、通知、直接操作（和是否被管理員鎖住）、智慧模式、一律放行、智慧模式等多久、自動更新、整理報告、本機語意模型、文件資料夾、自動審核、無需詢問／一律詢問規則、語音（聽／說）、每家模型供應商（是否就緒、預設模型、網址、看圖小幫手、測試狀態）、技能和技能市集、Bot 範本、形狀、顏色、分區、訊息管道（狀態）、連接器（狀態、工具）、Bot 的電腦（狀態、版本、隔離方式、快照、自動更新、最近 40 行紀錄、機器資訊）；擁有者另有手機清單、Docker 主機 | 20 秒 | 設定頁 | 設定 | 手機 |
| E18 | POST | `/api/act/<動作>` | 看下表 | 看下表 | 成功 `{ok, text, …}`（多附 `version`）；失敗 `400 {error}` | 20 秒；區網直連另有規則（見下） | 桌面能做的事，手機也能做 | 見下表 | 手機 |
| E19 | GET | `/api/mcp/registry` | owner | `q` | `{ok, servers}` | 失敗 `502` | 在官方 MCP Registry 搜尋連接器（手機頁只能問自己的伺服器，所以由 App 代查） | 連接器公用市集 | 手機 |

**E18 的區網直連規則**：區網直連只用手機密碼登入，所以**只能收緊、不能放寬** Bot 能做的事。會放寬的一律 `403`：「這會放寬 Bot 能做的事：同一個網路直接連的裝置只能收緊、不能放寬。請在電腦上的 AnyBot 改，或用帳號連線。」設定名字要一字不差，不認得的一律 `400`「不認得這個設定」（不猜）。從手機改連接器名稱一律 `403`「連接器的名稱只能在電腦上的 AnyBot 改」。下表「區網」欄寫哪些情況被擋。

**E18 的動作**

| 動作 | 誰可以 | 請求重點 | 回應／規則 | 區網直連 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|
| `search` | owner、admin | `{q}` | `{hits[{bot, bot_name, thread, thread_title, index, at, mine, file, snippet}], more}` | 可 | 搜尋所有對話 | 手機 |
| `detail` | owner、admin | `{bot}` | 同 E16 | 可 | （手機走 E16） | — 走 E16 |
| `settings` | owner、admin | — | 同 E17 | 可 | （手機走 E17） | — 走 E17 |
| `stats` | owner | — | 同 E32 | 可 | （手機走 E32） | — 走 E32 |
| `pages` | owner、admin | — | `{pages}`（全部，新的在前） | 可 | 頁面清單 | 手機 |
| `workflows` | owner、admin | — | `{workflows}`：每個的步驟、怎麼開始、跑過的結果、在等的問題 | 可 | 工作流清單 | 手機 |
| `workflow` | owner | `{wf, do}`，`do` 是 `run`／`stop`／`go`（繼續）／`halt`（停下） | 沒步驟、正在跑時拒絕；回「開始執行「…」」等 | 可 | 執行／停止工作流、回答它的確認 | 手機 |
| `schedule` | owner、admin | — | `{today, items, coming}`：前 3 天到後 7 天的例行任務和工作流（最多 400 筆），狀態 `done/failed/stopped/running/coming/paused`；即將執行 6 筆 | 可 | 日程 | 手機 |
| `library` | owner、admin | — | 所有聊天裡的檔案 | 可 | 所有檔案 | 手機 |
| `settings_find` | owner、admin | `{q}` | 符合的設定和它在手機上的位置 | 可 | 搜尋設定 | 手機 |
| `export_bot` | owner、admin | `{bot}` | `{filename:"<名字>.anybot", content}` | 可 | 匯出 Bot 檔案 | 手機（只給擁有者看按鈕） |
| `export_transcript` | owner、admin | `{bot}` | `{filename:"<名字> 對話.md", content}` | 可 | 匯出文字記錄 | 手機（只給擁有者看按鈕） |
| `import_bot` | owner | `{content}` | `{bot, text:"已匯入「…」"}`；匯入的 Bot 不會自動分享給團隊 | 可 | 匯入 Bot 檔案 | 手機 |
| `section_move` | owner | `{folder, by:-1 或 1}` | — | 可 | 調整側邊欄分區順序 | 手機 |
| `memory` | owner | 同 E20 | 同 E20 | 可 | （手機走 E20） | — 走 E20 |
| `older` | owner、admin | `{bot, from, count}` | 同 E14 | 可 | （手機走 E14） | — 走 E14 |
| `app` | owner | `{name, args, bot}` | 只准 17 個：`create_bot, update_bot, remember, forget, save_skill, delete_skill, skillify, skill_install, create_routine, update_routine, delete_routine, run_routine_now, create_group, move_to_group, rename_group, delete_group, update_settings`；其他「手機不能做這件事」；回 `{text, bot}`（新建的 Bot id） | 可（一律算不放寬，見待確認 Q13） | 新增／改 Bot、記住／忘記、教我做（skillify）、從網址裝技能、例行任務的增改刪和立即執行、分區增改刪和移動、改主題／風格／強調色／時區 | 手機（`save_skill`、`delete_skill` 沒有） |
| `bot` | owner（`unread` 管理員也可） | `{bot, key, value}`，`key` 見下 | 回「好了」；不認得 `400`「不認得這個設定」 | 見下 | Bot 的各項開關 | 手機（`folder` 沒有） |
| `group_chat` | owner | `{members:[id], name}` | `{bot}`；至少一個成員 | 可 | 建群組聊天 | 手機 |
| `delete_bot` | owner | `{bot}` | 先停下再刪 | 可 | 刪除 Bot | 手機 |
| `restart_bot` | owner、admin | `{bot}` | — | 可 | 重新啟動 Bot | 手機 |
| `react` | owner、admin | `{bot, index, emoji}`（emoji 最多 8 字） | 再按一次取消 | 可 | 對訊息按表情 | 手機 |
| `reply` | owner、admin | `{bot}` | 最後一則是使用者的才動 | 可 | 重新傳送 | 手機 |
| `resume` | owner、admin | `{bot}` | — | 可 | 當機後的「繼續」 | 手機 |
| `clear` | owner | `{bot}` | 停下、清掉目標、清空聊天 | 可 | 清除聊天記錄 | 手機 |
| `delete_page` | owner | `{id}` | — | 可 | 刪除頁面 | 手機 |
| `setting` | owner | `{key, value, list?, bot?}`，`key` 見下 | — | 見下 | 設定 › 一般、權限、外觀、更新、記憶與文件 | 手機（`smart_mode` 沒有） |
| `voice` | owner | `{which:"listen" 或 "speak", kind, base_url, api_key, model, voice}` | 金鑰只寫不讀 | 填網址會被擋 | 設定 › 語音 | 手機 |
| `provider` | owner | `{provider, key, default_model, base_url, cloud_eyes, fetch}` | 金鑰只寫不讀；`fetch:true` 在背景測試連線並抓模型清單 | 打開看圖小幫手、填網址會被擋 | 設定 › 模型 | 手機 |
| `device_remove` | owner | `{name, created}` | 「已登出那支手機」 | 可 | 登出某支區網手機 | 手機 |
| `channel` | owner | `{id, enabled 或 remove}` | 改完重啟訊息管道 | 重新打開會被擋 | 訊息管道開關、移除 | 手機 |
| `mcp` | owner | `{add:{name, url, header}}`，或 `{name, reconnect 或 remove 或 enabled}`（`rename` 從手機一律拒絕） | 網址要 http(s)，不能還有 `{…}` 沒換；名字重複自動改 | 新增、重新打開會被擋 | 連接器：從市集加入、開關、重連、移除 | 手機 |
| `skill` | owner | `{builtin}`，或 `{install:網址}`，或 `{package:{…}}`，或 `{name, remove 或 enabled}` | — | 可 | 技能：加內建的、從網址裝、整包交給大腦、開關、刪除 | 手機（`builtin`、`remove`、`enabled`）；`install`、`package` 由螢幕走 E50 |
| `box` | owner（`wake`、`disk` 管理員也可） | `{op, value}`，`op` 是 `wake`、`snapshot`、`disk`、`update`、`reset`、`runtime`、`auto_update` | `update`、`reset` 先停下所有 Bot；`reset` 有快照就從快照復原，沒有就換全新的；`auto_update` 的時間要 `HH:MM`，空的＝關 | 從強隔離改回標準會被擋 | 設定 › 電腦：喚醒、快照、檢查用量、更新、重設／復原、容器隔離、自動更新時間 | 手機 |
| `chatgpt` | owner（管理員 `403`「只有擁有者能登入或登出 ChatGPT」） | `{action:"login" 或 "logout" 或 "status", callback?}` | 同 E49 | 可 | ChatGPT 方案登入、登出 | 手機 |

**`bot` 的 `key`**

| key | 值 | 區網直連 |
|---|---|---|
| `pinned`、`pin_at`（位置）、`primary`、`unread`、`shape`、`color`（預設色名或 `#rrggbb`）、`strengths`、`not_for`（各最多 400 字）、`fallbacks`（備用模型清單）、`speak_replies`、`voice`、`voice_rate`（0.5–2）、`voice_lang`、`context`（4,000–2,000,000）、`thinking`（空、`low`、`medium`、`high`）、`step_model`、`allow_remove`（移除一條一律允許）、`skills`、`folder` | 各自的型別 | 可 |
| `abilities`、`may_use`（整份勾選清單） | 勾選的 key 陣列；全勾＝沒有清單（全部）；「可以用的電腦」只動列出來的那幾台，關機中的維持原狀；預設電腦不在清單裡時改回 Bot 的電腦 | 勾了原本沒勾的會被擋 |
| `tools` | 開／關電腦工具 | 打開會被擋 |
| `team` | 分享給團隊（只有擁有者） | 打開會被擋 |
| `members` | 群組成員 id 陣列（至少一個） | 群組已分享給團隊又加新成員會被擋 |
| `workdir` | 起始資料夾 | 一律被擋 |

**`setting` 的 `key`**

| key | 值 | 區網直連 |
|---|---|---|
| `direct_mode` | 開關直接操作 | 打開會被擋 |
| `auto_review` | 開關自動審核 | 打開會被擋 |
| `always_approve` | 一律放行（同聊天裡的 `/always-approve`；`bot` 是哪個聊天）；被管理員鎖住時拒絕 | 打開會被擋 |
| `smart_timeout_secs` | 智慧模式卡片等多久（下限到 3600 秒） | 一律被擋 |
| `smart_mode` | 智慧模式一律開著，傳 false 回錯誤 | 一律被擋（不在區網判定清單） |
| `notify`、`auto_update`、`garden_reports`、`language`、`shell_accent`、`shell_contrast`（0.75–1.35）、`embed`（`local`、`off`、`ollama`） | 各自的值 | 可 |
| `doc_folder_add`、`doc_folder_remove` | 文件索引資料夾 | 加入會被擋 |
| `rule_add`、`rule_remove` | `list` 是 `allow`（無需詢問）或 `ask`（一律詢問）；`value` 是規則文字；加入時規則看不懂會附提醒 | 加「無需詢問」、刪「一律詢問」會被擋 |

## 5. 記憶

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E20 | POST | `/api/memory` | owner（螢幕走 E53，當作擁有者） | `{op, bot, …}`；`bot: 0` 是「關於你」（所有 Bot 共用） | 讀：`list` → `{items, total, ms}`；`overview`／`stats` → `{stats, engine, files, suggestions?}`；`suggest`／`search` → 相關記憶；寫：`{ok, text, id?}` | 其他角色 `403`「記憶只有擁有者能看和改」；讀立刻回，寫等 20 秒 | 記憶頁和輸入框的記憶小卡片 | 記憶 | 手機 |

`op` 的值：

- 讀：`suggest`、`search`（同 `suggest`）、`list`（`filter` 是空、`pinned`、`fact`、`event`、`rollup`、`trash`；`q`、`offset`、`limit` 預設 50、最多 200）、`overview`、`stats`（同 `overview` 但不附建議）。
- 寫：`add`（`text`、`pinned`）、`pin`（`id`、`on`）、`edit`（`id`、`text`；和別條一樣就合併）、`forget`（`id` 或 `ids`；30 天內可在「最近刪除」還原）、`restore`（`id`）、`merge`（`keep`、`drop:[id]`、`text`）、`tidy`（把做過的事壓成每週、每月一行）、`empty_trash`。
- 手機實際用到：`suggest`、`list`、`overview`、`add`、`pin`、`edit`、`forget`、`restore`、`merge`、`tidy`、`empty_trash`；`search`、`stats` 沒人用。

## 6. 檔案（附件與預覽）

所有路徑都只限這台的附件資料夾裡的檔案；成員只限分享給團隊的 Bot 的資料夾。

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E21 | GET | `/api/file` | owner、admin、member | `path` | 檔案本身，依副檔名給型別（圖、PDF、音訊、影片、文字，其他 octet-stream）；CSP `sandbox` | 不在附件資料夾 `404`「找不到這個檔案」 | 顯示或下載聊天裡的檔案 | 看附件、下載 | 手機 |
| E22 | GET | `/api/preview` | owner、admin、member | `path` | `{kind, name, label, bytes, …}`；`kind` 是 `sheet`（多分頁表格 `sheets[{name, rows}]`）、`markdown`、`html`、`text`（`ext`）、`pdf`（`pages`）、`image`、`audio`、`video`、`other`（`message`） | 同上 | 檔案預覽（試算表、Markdown、網頁、文字、PDF…） | 檔案預覽 | 手機 |
| E23 | GET | `/api/preview/pdf` | owner、admin、member | `path`、`page` | 那一頁的 JPEG（寬 1400）；`X-Pages` 標頭是總頁數 | 畫不出來 `422` | PDF 一頁一頁看 | 檔案預覽 | 手機 |
| E24 | GET | `/api/preview/image` | owner、admin、member | `path` | PNG／JPG／GIF／WebP 原樣給，其他格式轉成 PNG | 解不開 `422`「這張圖片解碼失敗」 | 手機看不懂的圖片格式代為轉檔 | 看圖片 | 手機 |

上傳在 E15 `upload`（手機）和 E44（螢幕）。

## 7. 傳送到其他裝置

大腦是中繼站：手機或螢幕把檔案、文字交給大腦，大腦再一段一段推給目的地電腦（E39 的 `drop_put`），或留著給手機下載。

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E25 | GET、POST | `/api/drop/<動作>` | owner（螢幕走 E54） | 見下表 | 見下表 | 其他角色 `403`「只有擁有者能在裝置之間傳送」；錯誤 `400` | 傳送到其他裝置 | 傳檔案、傳文字 | 手機 |

| 動作 | 方法 | 請求重點 | 回應重點 | 規則 | 誰在呼叫 |
|---|---|---|---|---|---|
| `targets` | GET | — | `{targets[{id, name, online, kind}], you}`；`kind` 是 `brain`、`computer`、`phone` | 目的地：大腦自己、把它當大腦或借它手腳的電腦、手機（所有手機算一個）；向帳號伺服器問，快取 10 秒；沒登入帳號時只有「這台」和「手機」 | 手機、螢幕 |
| `list` | GET | — | `{items[{id, name, bytes, text?, from, from_id, to, to_name, at, state, note, sent}], you}` | 最近 7 天，新的在前；`state` 是 `uploading`（交給大腦中）、`waiting`（等對方上線）、`sending`、`ready`（手機可以下載）、`done`、`failed` | 手機、螢幕 |
| `new` | POST | `{name, bytes, to}` 或 `{text, to}` | `{id, chunk:2097152}` | 沒選目的地「請選要傳到哪一台」；選自己「這就是你現在用的這台，選另一台」；在路上的最多 64 個；文字最多 100,000 字；檔案不能空、最多 2 GB；目的地不在帳號「帳號裡找不到這台裝置（可能剛被移除）」 | 手機、螢幕 |
| `chunk` | POST | `?id=&at=`，body 是原始位元組（一段 2 MB） | `{have}` | `at` 不是對方已有的位置就不寫入，只回目前有多少；傳超過說好的大小拒絕 | 手機、螢幕 |
| `done` | POST | `{id}` | `{ok}` | 還沒收完「還沒傳完（x / y）」 | 手機、螢幕 |
| `remove` | POST | `{id}` | `{ok}` | 在路上的也取消 | 手機、螢幕 |
| `retry` | POST | `{id}` | `{ok}` | 只對失敗的有用 | 手機 |
| `file` | GET | `?id=&at=&len=` | 那一段的位元組 | 只給「給手機」的檔；一段最多 8 MB（手機用 4 MB）；讀到最後一段就算已下載 | 手機（螢幕路 E54 也收，沒人用） |

送達的規則：大腦每 20 秒（有新東西時馬上）看一次清單；目的地離線就等，15 秒後再試，最多 7 天；推給電腦時先問對方已經有多少，從那裡接著傳（2 MB 一段）。電腦收到的檔案放「下載/AnyBot」（重名加「 (2)」），文字放進剪貼簿，跳通知「<來源> 傳來的」；給手機的會推播「「名稱」（大小）可以下載了」（`kind:"file"`）。

## 8. Bot 做的網頁與圖表

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E26 | GET | `/api/pages/<id>` | owner、admin、member | `rev`（看舊版） | 頁面全文和中繼資料，加 `versions`、`latest` | 刪了 `404`「這個頁面已經刪除了」 | 看 Bot 做的頁面和它的歷史版本 | 頁面 | 手機 |
| E27 | GET | `/api/pages/<id>/chart/<n>` | owner、admin、member | `rev`、`dark=1` | 頁面裡第 n 張 Mermaid 圖的 SVG | 找不到 `404`「找不到這張圖」；畫不出來或 SVG 裡有腳本、事件屬性、外來內容 `422`「這張圖畫不出來」 | 頁面裡的圖表在大腦畫好給手機 | 頁面圖表 | 手機 |
| E28 | POST | `/api/diagram` | owner、admin、member | `{src, dark}`（Mermaid 原文，最多 20,000 字） | SVG | 空的或太大 `400`「這張圖太大或是空的」；不安全 `422` | 聊天裡的 Mermaid 圖畫成圖片 | 聊天圖表 | 手機 |

## 9. 語音

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E29 | POST | `/api/voice/transcribe` | owner、admin、member | body 是錄音；`name`（副檔名決定格式：m4a、mp4、webm、wav、ogg、opus、mp3、flac，預設 m4a）、`lang`、`ui`（介面語言） | `{text}` | 沒設「聽」`409`「還沒設定語音：請到「設定 → 語音」選一個服務。」；失敗 `502` | 手機錄音轉文字（用大腦設好的語音服務） | 麥克風輸入 | 手機 |
| E30 | POST | `/api/voice/speech` | owner、admin、member | `{text, voice, lang, speed}` | MP3 | 沒設「說」`409`；失敗 `502` | 把文字唸出來 | 朗讀回答 | 手機 |

（狀態 E12 的 `voice` 告訴手機要不要顯示麥克風和朗讀。）

## 10. 紀錄與統計

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E31 | GET | `/api/bots/<id>/audit` | owner、admin | — | 這個 Bot 每一輪做了什麼（30 天的稽核紀錄）；沒有時改用工作紀錄排成時間軸 | — | 稽核時間軸 | 操作紀錄 | 手機 |
| E32 | GET | `/api/stats` | owner | — | 一年的數字（用量、各 Bot…） | 20 秒；非擁有者「這件事只有這台 AnyBot 的擁有者能做」 | 個人資料與用量 | 用量統計 | 手機 |

## 11. 畫面（Bot 的電腦、這台 Mac）

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E33 | GET | `/api/box/thumb` | owner、admin | `bot`（看那個 Bot 自己的畫面；沒開就共用的） | JPEG，寬 480 | 沒開 `404`「Bot 的電腦沒有開著」 | Bot 的電腦的小圖 | 電腦卡片上的縮圖 | 手機 |
| E34 | POST | `/api/box/hold` | owner、admin | `{bot, on}` | `{ok}` | 一次抓住 30 秒；看畫面頁每 10 秒再說一次，手機斷線就自己放開 | 「接管」：人在操作那個畫面時，那個 Bot 在那個畫面上的步驟先等；「交還給 Bot」放開 | 接管 Bot 的電腦 | 手機（看畫面頁） |
| E35 | POST | `/api/vnc/ticket` | 只限區網直連、已登入 | `{}` | `{ticket}` | 60 秒內、用一次；同時最多 64 張（`429`「要求太頻繁了，請稍後再試」） | 換一張連畫面用的票，登入 token 不放進網址 | 看 Bot 的電腦 | 手機（看畫面頁） |
| E36 | WebSocket | `/api/vnc` | 只限區網直連 | `ticket=`（或舊頁面的 `t=<token>`）、`bot=` | 二進位訊息是 VNC；文字訊息是看的人打的字 | 沒登入 `401`「請先登入」；每 20 秒 ping，60 秒沒聲音就結束；每 3 秒重查一次登入；單則最多 16 MB | 直接看、操作 Bot 的電腦（App 自己用密碼登入 Bot 的電腦，給看的人「不用密碼」）；打的字貼進那個畫面；人點或打字時，那個畫面的 Bot 停下，最後一次操作後 8 秒才繼續 | 看、接管 Bot 的電腦 | 手機（看畫面頁） |
| E37 | GET | `/api/screen/mac` | owner、admin | — | 這台螢幕的 JPEG；`X-Size` 標頭是 `寬x高` | 沒開「從手機操作」`403`「這台 Mac 沒有開放從手機操作（AnyBot 設定 › 手機與帳號）」；拍不到 `503`；其他畫面名稱 `404`「沒有這個畫面」 | 手機看這台電腦本身的螢幕（一張一張抓） | 看這台 Mac | 手機 |
| E38 | POST | `/api/screen/mac` | owner、admin | `{kind, …}`：`click{x, y, button, count(1–3), modifiers}`、`move{x, y}`、`drag{path（最多 200 點）, modifiers}`、`type{text}`、`key{key，可以是組合如 "cmd+tab"}`、`scroll{x, y, direction, amount(1–20)}`；`modifiers` 只認 `cmd`、`shift`、`option`、`ctrl`、`alt` | `{ok}` | 沒開「從手機操作」`403`；不認得 `400`「不認得的操作」 | 手機以人的身分點、拖、打字、捲動這台電腦（座標是那張圖的像素） | 從手機操作這台 Mac | 手機 |

經帳號伺服器看 Bot 的電腦走帳號伺服器的 `/relay/vnc/<host>`，到 App 這邊變成長連線的 `vopen/vdata/vtext/vclose`（§12.2）。

## 12. 裝置（手腳）

| # | 方法 | 路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E39 | POST | `/api/device/run` | device（只能經帳號伺服器；直接連 `403`） | `{tool, args（JSON 字串）, who, sig?}` | `{text, ok, image?}`，或下表 | 見下 | 大腦在這台跑一個工具 | 「加一台電腦」當手腳、傳送到其他裝置 | 大腦 |

**前提（依序檢查）**

1. 用配對碼加入的手腳，只執行大腦簽過的指令：`sig{t（時間）, n（一次性編號）, macs（大腦用每把配對金鑰算的簽章）}`；兩台時鐘差 10 分鐘內、同一個編號只收一次。錯誤：「這台電腦只接受它的大腦簽過的指令（大腦請更新到最新版，或重新配對）」「指令的時間對不上：請確認兩台電腦的時鐘都是準的」「指令的簽章不對：不是這台電腦的大腦發的」「這個指令已經執行過了」（都是 `403`）。帳號伺服器只轉送，做不出簽章。
2. `drop_put` 不需要開「裝置連結」，但只收這台自己的大腦推來的（`403`「這台電腦只收它自己的大腦傳來的東西」）。
3. 其他工具要開「裝置連結」（`403`「這台電腦沒有開放給帳號裡的其他 AnyBot 操作」）；配對時限定了大腦的，別的大腦 `403`「這台電腦只讓大腦「…」使用」。
4. 管理員政策鎖住本機操作：`{ok:false, text:"這台電腦已被系統管理員政策鎖定，不允許本機操作。"}`。

**`tool` 的值**

| tool | args 重點 | 回應 | 規則 | 使用者功能 |
|---|---|---|---|---|
| `drop_put` | `{id, name, bytes, from}` ＋ `{probe:true}`（問有多少）或 `{at, data（base64，一段）}` 或 `{text}` | `{ok, have, done, text}` | 收過的直接回 done；`at` 接不上回目前有多少；檔名只留最後一段；最多 2 GB、文字 100,000 字 | 傳送到其他裝置（收的那端） |
| `model_fetch` | `{url, headers[[名, 值]], body}` | `{status, text}` 或 `{error}` | 只幫大腦連這台網路裡的私有位址（不含這台自己的 loopback、不含 link-local），否則 `403`「只幫大腦連這台電腦網路裡的模型」；只轉 `authorization, x-api-key, anthropic-version, anthropic-beta, x-title, x-opencode-session, accept` 這幾個標頭；等 600 秒；回應最多 50 MB。大腦連不到（1.5 秒試連，記 5 分鐘）而且模型在私有位址時才走這條 | 雲端大腦用你家／公司區網裡的模型 |
| 螢幕操作工具（`computer`、`computer_use_*`：截圖、放大、點、打字、按鍵、捲動、移動、拖、開 App、選單、填表、等待…） | 工具自己的參數 | `{text, ok, image}` | 圖片走網路時改用 WebP（小約四分之一） | Bot 操作那台電腦的畫面 |
| 這台的 AnyBot 瀏覽器工具（`browser_open`、`browser_read`、`browser_profiles`、`browser_*` 點擊、輸入、分頁、截圖、上傳、下載…；`web_fetch` 除外） | 工具自己的參數（拿掉 `computer`） | `{text, ok, image}` | 登入狀態是那台的；`browser_upload` 不傳機密檔（「這個檔案是機密的，不能上傳到網頁。」） | Bot 用那台的瀏覽器 |
| `browser_vault_fill` | 保險庫那一筆的內容和工作分頁 | 結果 | 大腦規劃、人核准後才送；在那個工作的分頁上填；頁面網域檢查照做 | 保險庫自動填密碼（在手腳上） |
| Mac App 工具（提醒事項、行事曆、聯絡人、備忘錄、郵件、傳訊息） | 工具自己的參數 | `{text, ok}` | 只在 macOS | Bot 用那台 Mac 的 App |
| `fetch_file` | `{path, workdir}` | `{ok, name, data（base64）}` | 最多 20 MB；資料夾「請先壓縮成一個檔案」；機密檔「這個檔案是機密的，不能傳到聊天。」 | Bot 把那台的檔案傳進聊天 |
| `shell`、`read_file`、`edit_file`、`list_dir`、`search`、`read_terminal`、`write_terminal` | 工具參數，含 Bot 的工作資料夾 | `{text, ok}` | 在那台的工作資料夾跑（資料夾不存在就用家目錄下同名的或家目錄）；一律拒絕的指令和私密檔照樣擋：「被安全規則擋下：…」 | Bot 在那台跑指令、讀寫檔 |
| 其他 | — | `400`「這個工具不能遠端使用」 | — | — |

**大腦什麼時候叫 E39**：Bot 的工具指定了另一台電腦；讀那台的「電腦現況」（一段唯讀小腳本，結果快取 30 分鐘）；瀏覽器分頁、保險庫填入；傳送到其他裝置；模型只有那台連得到。大腦這邊等 50 秒（長連線）或 45 秒（HTTP 退路）；`model_fetch` 等 600 秒。

## 13. 同步（大腦 ↔ 螢幕電腦）

螢幕電腦（有視窗、把別台當大腦的 AnyBot）所有的讀寫都打這一組，角色 mirror，只能經帳號伺服器。

| # | 方法 | 路徑 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|
| E40 | GET | `/api/sync` | `since`（版本，0＝全部重來）、`client`（螢幕自己的編號）、`config=1`（剛啟動，設定一定要給）、`retry=1`（上一次沒收到） | 見下方「拉取的內容」 | `since>0` 且不是 `config` 時最多等 20 秒；大腦處理等 10 秒；螢幕這邊等 90 秒；大腦一天沒聽到某台螢幕就忘了它 | 拿大腦的變化（長輪詢） | 在任何一台電腦看、改同一份 Bot 和聊天 | 螢幕 |
| E41 | GET | `/api/sync/ping` | — | `{ok:true}` | 不經過大腦主畫面 | 連線體檢量「帳號伺服器 ↔ 大腦」這一段 | `/doctor`、連線體檢 | 螢幕（走 HTTP 路，量 3 次） |
| E42 | POST | `/api/sync/edit` | `{ops[], client, seq}` | `{ok, applied}`；同一個 `seq` 再來 `{ok, applied:0, again:true}` | 大腦處理等 10 秒 | 螢幕上改的設定變成小操作送回大腦（見下表） | 在螢幕電腦改設定、新增／刪除／排序 Bot | 螢幕 |
| E43 | POST | `/api/sync/act` | `{bot, action, id, …}` | `{ok}`；同一個 `id` 24 小時內再來 `{ok, again:true}` | 不認得 `404`「沒有這個動作」 | 聊天動作由大腦執行（見下表） | 在螢幕電腦聊天、核准 | 螢幕 |
| E44 | POST | `/api/sync/upload/<bot>` | `?name=`，body 是檔案 | `{path, name, image}` | 螢幕等 120 秒；最多 30 MB | 附件、會議錄音先放到大腦 | 附件、會議錄音 | 螢幕 |
| E45 | GET | `/api/sync/file` | `path` | 檔案位元組 | 只限大腦的附件資料夾；`404`「大腦上找不到這個檔案」；等 120 秒 | 螢幕打開聊天裡的檔案時才去大腦拿，存一份在自己這邊 | 看附件 | 螢幕 |
| E46 | GET | `/api/sync/screen` | `bot`、`thumb`（被忽略） | JPEG，寬 320 | 沒畫面 `404`「還沒有畫面」；等 30 秒 | Bot 的電腦的小縮圖（不會叫醒它） | 電腦頁、畫面縮圖 | 螢幕 |
| E47 | POST | `/api/sync/screen` | `{kind:"hold", on, bot}` 或 `{kind:"wake"}` | `{ok}` | 不認得 `400`「不認得的操作」 | 「接管」時那個畫面的 Bot 步驟等你；`wake` 叫醒 Bot 的電腦 | 接管、喚醒 | 螢幕（只用 `hold`） |
| E48 | POST | `/api/sync/models` | `{provider}` | `{models}` 或 `{error}` | 大腦等 45 秒；螢幕等 60 秒 | 用大腦的金鑰抓模型清單（螢幕沒有金鑰） | 設定 › 模型 | 螢幕 |
| E49 | POST | `/api/sync/chatgpt` | `{action:"status" 或 "login" 或 "logout" 或 "switch_account", callback?}` | `{signed_in, plan_ok, email, registered, status, limited, ok, text, url}`；`login` 沒帶 `callback` 時給登入頁 `url`，瀏覽器在別台時把登入後的網址貼回來（`callback`） | 不給任何 token | ChatGPT 方案的帳號在大腦上 | ChatGPT 登入 | 螢幕 |
| E50 | POST | `/api/sync/skill` | 同 E18 `skill`（`install`：GitHub 或網址由大腦去抓；`package`：螢幕上的資料夾整包送來） | `{ok, text}` | 等 55 秒 | 技能裝在大腦上（Bot 從大腦的磁碟讀） | 安裝、交出、刪除技能 | 螢幕 |
| E51 | POST | `/api/sync/mcp` | `{name}` | `{ok, text}` | 等 10 秒 | 連接器重新連線（清單本身走 E42） | 連接器重連 | 螢幕 |
| E52 | POST | `/api/sync/mcp-rename` | `{name, rename}` | `{ok}` | 等 10 秒；名字空白或重複拒絕 | 改連接器名稱（金鑰、位置、登入、一律允許都跟著） | 連接器改名 | 螢幕 |
| E53 | POST | `/api/sync/memory` | 同 E20 | 同 E20 | 同 E20 | 螢幕的記憶頁 | 記憶 | 螢幕 |
| E54 | GET、POST | `/api/sync/drop/<動作>` | 同 E25（寄件人是發問的那台電腦，用 `x-anybot-from` 認） | 同 E25 | 等 60 秒 | 螢幕的「傳送到其他裝置」 | 傳檔案、傳文字 | 螢幕（`file` 沒人用） |
| E55 | POST | `/api/sync/voice/transcribe` | 同 E29 | 同 E29 | 等 60 秒 | 螢幕的麥克風用大腦的語音服務 | 語音輸入 | 螢幕 |
| E56 | POST | `/api/sync/voice/speech` | 同 E30 | MP3 | 等 60 秒 | 螢幕的朗讀 | 朗讀 | 螢幕 |
| E57 | POST | `/api/sync/voice/voices` | `{}` | `{voices:[[id, 名稱]]}` | 沒設 `409`；失敗 `502`；等 30 秒 | 「說」服務有哪些聲音 | 選聲音 | 螢幕 |
| E58 | GET | `/api/sync/stats` | — | 同 E32 | 大腦等 40 秒 | 個人資料與用量（螢幕當作擁有者） | 用量統計 | 螢幕 |
| E59 | GET | `/api/sync/audit/<id>` | — | 30 天稽核紀錄（沒有退回工作紀錄的那一步） | 等 20 秒 | 操作紀錄 | 操作紀錄 | 螢幕 |

**拉取（E40）的內容**：`version, brain`（大腦名字）、`edit_seq`（大腦已經收到這台的第幾號改動）、`config`（設定有變才給；金鑰、訊息管道 token、連接器的環境變數和標頭換成「••••（已設定，在大腦上）」）、`bots[{id, from, messages}]`（每個 Bot 只送第一個不同的訊息之後的部分）、`threads[{id, threads}]`（有變才給）、`tasks`（有變才給）、`live{<bot>:{status, paused}}`（誰在工作）、`approvals[{bot, call_id, tool, reason, grant, kind, key}]`（`kind` 是 `action`、`tools`、`local`、`handoff`、`question`）、`mcp`（每個連接器在大腦上的連線狀態）、`chatgpt`（同 E49 的狀態）、`box{label, running, available, busy, screens}`、`computers{<bot>:{names, all, current, missing}}`（只有大腦知道帳號裡的其他電腦）。

- 螢幕沒收到上一次的回答時（`retry=1`），大腦從再前一次算起，不會漏。
- 螢幕送回來的設定裡如果是那個替代字，大腦保留原本的真值（清單裡依 id 或名字對上）。
- 不同步、各台自己的設定：`remote, theme, accent, sidebar_collapsed, sidebar_width, selected, machine_label, local_execution, local_never, messages_send, messages_no_ask`。
- 螢幕不能改的 Bot 欄位：`id, messages, threads, unread`（只能用 `bot_read`）、`last_turn, tasks, context_tokens, goal, plan, regen_undo, branches, branch_of`。

**E42 的 `ops`**

| op | 欄位 | 規則 | 誰在呼叫 |
|---|---|---|---|
| `set` | `key, value` | 換掉一個設定；`next_*` 計數只會變大；工作流的執行紀錄和下次執行時間以大腦為準 | 螢幕 |
| `patch` | `key, patch` | 清單或對照表逐項改（格式見下），兩台同時改同一份清單的不同項目兩邊都留 | 螢幕 |
| `merge` | `key, value` | 「把這台的 Bot 搬過去」：只補大腦沒有的（`providers` 補沒填的金鑰；`skills`、`mcp_servers` 依名字補） | 螢幕 |
| `bot_add` | `bot` | 新增 Bot；id 撞到就換 | 螢幕 |
| `bot_del` | `id` | 先停下再刪 | 螢幕 |
| `bot_set` | `id, key, value` | 改 Bot 的一個欄位 | 螢幕 |
| `bot_patch` | `id, key, patch` | 逐項改 Bot 的清單欄位；例行任務保留大腦記錄的執行時間，螢幕給的例行任務編號會推高大腦的計數 | 螢幕 |
| `bot_read` | `id` | 標為已讀 | 螢幕 |
| `bot_order` | `ids` | Bot 的順序 | 螢幕 |

`patch` 的格式：`{"value": …}` 換掉整個值；`{"map": {"put": {…}, "del": […]}}` 對照表；`{"list": {"put": […], "del": […], "order": […]}}` 清單（項目的鍵是 `id`，沒有就 `name`，純值就是值本身）。有一個操作被拒，大腦下次拉取會把整份設定重送給那台螢幕，讓它丟掉被拒的改動。螢幕的改動編號用時鐘（毫秒）起算，客戶端編號存在本機副本旁，重開後沿用。

**E43 的 `action`**

| action | 欄位 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|
| `send` | `text, attachments, memories?, seen?`（旁邊開著的工作流畫布，最多 300 字） | 傳訊息；發問的那台變成「使用者正在用的電腦」（「我的電腦」、區網模型都看它） | 螢幕 |
| `stop` | — | 停止 | 螢幕 |
| `answer` | `answer, text, call, from` | 核准（規則同 §3.1） | 螢幕 |
| `read` | — | 標為已讀 | —（螢幕改用 `bot_read`） |
| `seen` | `att, kind, why` | 交付確認（裝置名稱用那台電腦的名字） | 螢幕 |
| `new` | — | 開新對話 | 螢幕 |
| `regenerate` | — | 重新產生 | 螢幕 |
| `restore_answer` | — | 還原上一個答案 | 螢幕 |
| `reply` | — | 重新傳送 | 螢幕 |
| `resume` | — | 當機後繼續 | 螢幕 |
| `restore` | `index` | 回到過去的對話 | 螢幕 |
| `clear` | — | 清除聊天記錄 | 螢幕 |
| `run_routine` | `routine` | 立即執行例行任務 | 螢幕 |
| `workflow` | `wf, do`（`run`、`stop`、`go`、`halt`） | 工作流 | 螢幕 |
| `meeting` | `path`（先用 E44 上傳的錄音）、`lang` | 會議錄音轉文字再交給 Bot | 螢幕 |

螢幕送的規則：訊息、改動、附件排進一個佇列依序送（App 關掉也還在）；一定要送到的東西，線斷時同一個內容重送最多 100 秒（每次等 20 秒，間隔 0.5 秒起倍增到 8 秒）；停止、重新產生、回答卡片這類動作只有連著（或剛斷一下）才能做，否則提示「連不到大腦：這個動作要連上才能做（訊息可以照樣傳，連上後送出）。」

## 14. 這台電腦上的小服務

不在手機伺服器那個埠上，各自開在這台電腦上。

| # | 方法 | 位址與路徑 | 誰可以呼叫 | 請求重點 | 回應重點 | 限制 | 做什麼 | 使用者功能 | 誰在呼叫 |
|---|---|---|---|---|---|---|---|---|---|
| E60 | POST | `http://127.0.0.1:8792/hooks/<例行任務 id>`（被占用就往後試到 8811） | 知道那個例行任務密鑰的程式（`Authorization: Bearer <密鑰>`） | body 任意文字 | `202`（收下）、`401`（密鑰不對）、`404`（沒有這個任務）、`400` | 只聽 127.0.0.1；讀 10 秒、最多 1 MB；只取前 4000 字 | 觸發那個例行任務，Bot 收到「Webhook 被呼叫，內容：…」；設定上顯示「等待呼叫：POST http://127.0.0.1:<埠>/hooks/<id>」 | 例行任務的 Webhook 觸發 | 外部 |
| E61 | CONNECT、GET `http://…` | 代理伺服器，`127.0.0.1:8791`（被占用往後試到 8810）；Linux 上另外聽 Docker bridge 的閘道位址 | 只限這台和 Docker 的 bridge 網段（`172.17.x.x` 和綁定時偵測到的 bridge） | 標準 HTTP 代理請求 | 連上就轉送；不准的 `403`；連不到 `502`；請求看不懂 `400` | 只准連公網位址（本機、區網、link-local 一律 `403`「禁止連往本機或內部網路」）；連線 10 秒逾時；閒置 30 分鐘斷；標頭最多 64 KB | Bot 的電腦裡新開的連線改走這台電腦的網路（Bot 的電腦拿到 `HTTP(S)_PROXY`） | 「網路流量改走你的電腦」（管理員可以關；Docker 在別台主機時不能用） | Bot 的電腦 |
| E62 | GET | `http://127.0.0.1:1455/auth/callback?code=&state=`（被占用就換一個） | 登入完回來的瀏覽器 | `code`、`state` | 小網頁：「已用 ChatGPT（<帳號>）登入，可以關掉這頁，回到 AnyBot。」或「沒有登入成功：…」；其他路徑 `404` | 等 10 分鐘；一次只一個登入 | ChatGPT 方案登入的 OAuth 回來的地方；瀏覽器在別台時改成把網址貼回來 | ChatGPT 登入 | 外部（瀏覽器） |
| E63 | GET | `http://127.0.0.1:<隨機埠>/callback?code=&state=` | 登入完回來的瀏覽器 | `code`、`state` | 小網頁；其他路徑 `404` | 等 10 分鐘 | 連接器（MCP）OAuth 登入回來的地方；也可以把網址貼回聊天 | 連接器登入 | 外部（瀏覽器） |

另外有兩條內部轉接（不是對人或其他程式的介面）：Bot 的電腦裡瀏覽器的 DevTools 埠轉到這台 127.0.0.1 的隨機埠；Bot 的電腦的 VNC 經 `docker exec` 接到 E36／長連線。

## 15. 長連線（App ↔ 帳號伺服器）

### 15.1 連線與框架

- 每台 AnyBot（大腦、螢幕、手腳）登入帳號後主動連 `wss://<帳號伺服器>/relay/host/ws`（http 網址就用 `ws://`），帶 `Authorization: Bearer <這台的金鑰>`。不用開任何連接埠。
- http:// 的帳號伺服器只在「不經過公開網路」時才連：伺服器就是這台（所有位址都是 loopback；`localhost` 只認 loopback）、一個字的名字（同一個 Docker 網路裡的服務，如 `relay`）、或走這台的 Tailscale。其他一律不連，說「帳號伺服器 <主機> 用的是 http://，這台到它的路不經過 Tailscale：帳號密碼和這台電腦的金鑰會在網路上明文傳送，所以先不連。請改用 https://，或等 Tailscale 連上」。
- 握手結果：`401`／`403` →「這台電腦的帳號登入失效了，請到「設定 › 手機與帳號」重新登入」；`400`／`404`／`405`／`426` → 帳號伺服器是舊版沒有 WebSocket，改用舊式長輪詢（§15.4）；其他 →「帳號伺服器暫時無法使用，稍後會自動再試」；憑證不被信任 →「這台電腦還不信任伺服器的憑證（伺服器自己簽的）：先在這台電腦安裝並信任它的憑證，再試一次」。TCP 連線 10 秒逾時。接受公開憑證加上這台電腦自己信任的憑證（區網自簽伺服器信任一次就好）。
- 每則訊息是一則 WebSocket 二進位訊息：`[4 位元組表頭長度，big-endian][表頭 JSON][內容原始位元組]`。內容不做 base64。表頭最多 1 MB；整則最多 64 MB。表頭的 `t` 說是哪一種。
- 設定裡顯示的狀態：「連線中…」「已連線」「連不上帳號伺服器：<原因>」「連線中斷，正在重新連線…」「另一份 AnyBot 正用這台電腦的登入連著帳號伺服器（同一台開了兩份，或資料夾被複製到別台）：先讓它用，它斷了這份才連回去（每 5 分鐘看一次）」。狀態變了會寫進紀錄檔。

### 15.2 訊息種類

| t | 方向 | 表頭欄位 | 內容 | 意思 | 使用者功能 |
|---|---|---|---|---|---|
| `hello` | App → 伺服器 | `brain`（這台在看的大腦 id；空＝自己就是大腦）、`hands`（開了裝置連結） | 無 | 一連上就送，換大腦時再送；帳號伺服器用它告訴手機「這台用哪個大腦」、哪些電腦是手腳 | 手機自動開對的大腦、手腳清單 |
| `job` | 伺服器 → App | `id, method, path, query{}, user, role, from, z, z_ok` | 請求的 body | 給這台的一個 HTTP 請求（手機、螢幕、大腦發的），App 當成一般請求處理（同 §1–§13 的路徑） | 所有遠端操作 |
| `reply` | App → 伺服器 | `id, status, ctype, headers[[名, 值]], z` | 回應的 body | 回答 `job` | 同上 |
| `call` | App → 伺服器 | `id`（隨機）、`to`（帳號裡另一台的 id）、`method, path, query{}, z` | 請求的 body | 問帳號裡另一台；`path` 只能是 `/api/sync…`（對方看到角色 mirror）或 `/api/device/run`（角色 device），其他帳號伺服器回 `404` | 螢幕同步、手腳 |
| `ret` | 伺服器 → App | `id, status, ctype, z, headers` | 回應的 body | `call` 的回答；帳號伺服器自己的錯誤也走這裡（`403` 配對的手腳不能叫別台、不同帳號；`404`；`429` 同時太多；`503` 對方離線；`504` 逾時） | 同上 |
| `vopen` | 伺服器 → App | `sid, bot` | 無 | 有人要看 Bot 的電腦（`bot` 那個 Bot 的畫面，沒開就看共用的）：App 接上那個畫面的 VNC，先回 VNC 開頭 `RFB 003.008\n` | 看 Bot 的電腦 |
| `vdata` | 雙向 | `sid` | VNC 位元組 | 畫面串流 | 看、操作 Bot 的電腦 |
| `vtext` | 伺服器 → App | `sid`（帳號伺服器另附 `bot`，App 不看） | 看的人打的字（UTF-8） | 貼進那個看的人連上的畫面（不是另外開一個） | 在畫面上打中文等文字 |
| `vclose` | 雙向 | `sid`；App 送的有 `why`（例如「這台電腦的畫面關了」） | 無 | 結束這條畫面串流 | — |
| `bye` | 伺服器 → App | `why`（`replaced` 或沒有） | 無 | `replaced`：另一份用同一把金鑰的 AnyBot 接手了連線 → 這份不搶，先等 15 秒，之後每 5 分鐘問一次帳號伺服器（`/relay/host/peers` 的 `me_online`）位子空了才連回去；沒有 `why`：這台被移出帳號，或要它重新連線 → 照常重連 | 同一台開兩份不會互搶 |

另外：WebSocket 本身的 ping／pong 當心跳（§15.3）。

### 15.3 保活、壓縮、重連、錯誤

- **心跳**：App 每 15 秒送 ping；**90 秒**沒有收到任何位元組就當斷線重連（大回答正在一點一點進來也算活著）。帳號伺服器那邊每 20 秒 ping、60 秒沒聲音就斷。
- **延遲**：有動靜時幾毫秒內送出；安靜 2 秒後最多慢 40 毫秒；安靜 20 秒後最多慢 0.2 秒。
- **壓縮**：內容超過 1 KB、不是已經壓過的（JPEG、PNG、GIF、WebP、PDF、zip／docx、gzip 開頭的不試）、而且能省 10% 以上才用 deflate 壓，表頭標 `z:true`；`job` 的 `z_ok:true` 表示問的人看得懂壓過的回答。解壓最多 64 MB（「收到的壓縮資料太大了」）；解不開回 `400`「收到的資料壞掉了」。
- **同時處理**：App 同時最多處理 64 個 `job`，再多立刻回 `503`「這台 AnyBot 忙碌中，請稍後再試」。
- **`call` 的等待**：各自的時間（見各端點）；時間到了但回答還在進來（3 秒內有位元組）就繼續等，最多 600 秒；真的等不到「等太久沒有回應：網路很慢或斷斷續續」。長連線沒開時，`call` 改用一般 HTTP（§15.4 的退路）。
- **斷線**：App 這邊還在等的 `call` 立刻收到「和帳號伺服器的連線斷了，正在重新連線」，不用等逾時；所有畫面串流結束。帳號伺服器那邊還在等的 `job` 立刻回 `503`「那台 AnyBot 斷線了（網路中斷，或 AnyBot 關掉了）」。
- **重連**：失敗後等 1、2、4…最多 30 秒；連線撐過 30 秒才斷的從 1 秒重來；每台多錯開 0.5–2 秒，伺服器重開時不會全部同時湧回。
- **Tailscale 自己連自己**：帳號伺服器的 `*.ts.net` 名字其實就是這台時，改連 127.0.0.1（憑證照驗），避開繞遠的中繼。

### 15.4 舊式長輪詢（帳號伺服器沒有 WebSocket 時）

- App 開 4 個工作者，每個反覆 `GET /relay/host/next`（帶這台金鑰；自己等 40 秒，伺服器最多扣 25 秒）：`204` 沒事；`200` 是一個 job JSON `{id, method, path, query, user, role, from, body（base64）, z}`；`401`「這台電腦的金鑰失效了，請重新登入帳號」並停止；其他錯誤等 1 秒起倍增到 30 秒。
- 回答：`POST /relay/host/reply/<id>`，`{status, ctype, body（base64）, headers}`，等 60 秒。
- 這個模式下沒有畫面串流；問別台改用 HTTP：`GET/POST /relay/hosts/<id>/api/sync…`（同步，GET 失敗會快速重試一次）、`POST /relay/hosts/<id>/api/device/run`（等 45 秒，body 重新簽一次）。長連線開著時也會用到這條退路（`call` 送不出去時）。

## 16. 互查：手機頁面實際呼叫了什麼

對照 `mobile/index.html`、`mobile/screen.html`、`mobile/sw.js`。

**手機呼叫的 App 端點（38 條）**：E01–E38 全部。

- 主頁：E01、E05–E08（頁面和圖示）、E09–E11、E12–E15、E16–E20、E21–E24、E25、E26–E28、E29–E30、E31–E32、E33、E37–E38。
- 看畫面頁（主頁開成內嵌框）：E02、E03、E04、E34、E35、E36。
- Service Worker：E06 本身；它快取 `""`、`index.html`、manifest、圖示；推播點下去用 E15 `answer`（`from:"notice"`）。
- `/api/act/<動作>`：手機用到 30 種（見 E18 表），沒用 `detail`、`settings`、`stats`、`memory`、`older`（同一件事改走專屬端點）。
- `/api/act/app`：手機用到 15 個名字；`save_skill`、`delete_skill` 沒用（刪技能改走 `skill` 的 `remove`）。
- 手機在帳號模式時，路徑都加上 `h/<host>/`；區網模式直接打 App。

**手機另外直接叫帳號伺服器的（不在本檔，列出來對照）**：`relay/login`、`relay/signup`、`relay/logout`、`relay/2fa/verify`、`relay/2fa/enroll`、`relay/2fa/disable`、`relay/me`、`relay/invites`、`relay/pair`、`relay/sessions/logout`、`relay/sessions/drop`、`relay/push/key`、`relay/push/subscribe`、`relay/push/unsubscribe`、`relay/push/state`、`relay/push/kinds`、`relay/push/test`；看畫面頁另有 `relay/screen-ticket`、`relay/screen-ticket/renew`、WebSocket `relay/vnc/<host>`。

**其他電腦呼叫的（21 條）**：E39（大腦 → 手腳）；E40–E59（螢幕 → 大腦）。

**外部呼叫的（4 條）**：E60（其他程式）、E61（Bot 的電腦）、E62、E63（OAuth 回來的瀏覽器）。

**整條沒人呼叫的端點**：沒有。

**子命令沒人呼叫的（9 個）**：

| 在哪 | 子命令 | 說明 |
|---|---|---|
| E18 `app` | `save_skill`、`delete_skill` | 手機沒用（可能只給 Bot 自己用） |
| E18 `bot` | `folder` | 手機改用 `app` 的 `move_to_group` |
| E18 `setting` | `smart_mode` | 智慧模式一律開；手機沒傳；區網直連還會被擋 |
| E20／E53 | `search`、`stats` | 分別等於 `suggest`、`overview`，沒人用 |
| E43 | `read` | 螢幕改用 E42 的 `bot_read` |
| E47 | `wake` | 螢幕只送 `hold` |
| E54 | `file` | 程式收，但沒有螢幕會下載「給手機」的包裹 |

另外有一個被忽略的參數：E46 的 `thumb`（螢幕傳 0 或 1，回來的都是 320 寬縮圖）。

## 17. 和舊文件的差異（`docs/protocol.md`、`docs/architecture.md`）

| # | 文件說的 | 實際 |
|---|---|---|
| 1 | protocol.md 列了 App 端約 20 條 | 實際 63 條。沒列的：E01–E11 大部分、E14、E16–E19、E21–E24、E26–E28、E29–E30、E31–E33、E34、E35、E41、E49–E52、E55–E59、E60–E63；`/api/act/*` 整組（35 種動作）完全沒寫 |
| 2 | `POST /api/bots/<id>/<動作>`：send、stop、answer、read、new、upload… | 另有 `seen`、`regenerate`、`restore_answer`、`restore`、`set` |
| 3 | `/api/sync/act`：send、stop、answer、read、new、regenerate、reply、restore | 另有 `seen`、`restore_answer`、`resume`、`clear`、`run_routine`、`workflow`、`meeting`；`read` 實際沒人用 |
| 4 | `/api/sync/drop/<動作>`「沒有 `file`」 | 程式其實收 `file`（沒人用） |
| 5 | `POST /api/sync/screen`：`{kind:"hold", on}`、`{kind:"wake"}` | `hold` 還帶 `bot`（每個 Bot 一個畫面）；`wake` 沒人用 |
| 6 | WebSocket `/api/vnc?t=` | 現在先用 E35 換一次性的票 `?ticket=`，`t=` 只留給舊頁面；也帶 `bot=` |
| 7 | `/api/device/run` `{tool, args, who}`，`drop_put` | 另有簽章 `sig`（配對的手腳只收簽過的）、`model_fetch`、`fetch_file`、`browser_vault_fill`、Mac App 工具、管理員政策鎖定；文件沒寫 |
| 8 | 長連線訊息：hello、job、reply、call、ret、vopen、vdata、vtext、vclose | 另有 `bye`（`why:"replaced"`）；`vopen`、`vtext` 帶 `bot` |
| 9 | 「有東西在跑時每 2 毫秒、安靜時每 40 毫秒」 | 還有第三段：安靜 20 秒後 200 毫秒 |
| 10 | architecture.md：「雙方每 15–20 秒互相確認，45–60 秒沒聲音就重連」 | App 改成 90 秒（只要有位元組在進來就算活著）；帳號伺服器仍是 20 秒 ping、60 秒 |
| 11 | protocol.md：AnyBot 的介面「同一個網路、用手機密碼登入」 | 區網直連現在是 HTTPS（自簽憑證、設定裡比對指紋），純 HTTP 只限本機和 Tailscale，其他 308 轉 https；區網直連只能收緊權限（E18 規則）——文件都沒寫 |
| 12 | protocol.md：`GET /api/sync/screen` 寬 320 | 符合；但螢幕傳的 `thumb` 被忽略 |
| 13 | 兩份文件都沒提 | 這台電腦上的小服務 E60–E63（Webhook、出站代理、兩個 OAuth 回呼） |

## 附錄 A：App 呼叫帳號伺服器的介面（只列名稱，細節見帳號伺服器的清單）

| 方法 | 路徑 | 什麼時候 | 重點 |
|---|---|---|---|
| GET | `/api/hello` | 填帳號伺服器網址時、登入後 | 確認是 AnyBot 帳號伺服器（`app:"anybot-relay"`，15 秒）；`brains:true` 表示會替每個帳號開雲端大腦（登入後最多等 90 秒等它開好） |
| GET | `/api/speed?bytes=262144` | 連線體檢 | 量下載速度（慢於每秒 300 KB 算慢；來回超過 400 毫秒算慢） |
| POST | `/relay/signup` | 「建立新帳號」 | `{user, password, invite}`；帳號已存在就改成登入 |
| POST | `/relay/login` | 電腦登入帳號、還原備份 | `{user, password, device}` → `{token}` 或 `{need_2fa, challenge}` |
| POST | `/relay/2fa/verify` | 兩步驗證 | `{challenge, code}` |
| POST | `/relay/hosts` | 登入後把這台登記成帳號的電腦 | `{name, was?{id, token}}` → `{id, token}`；之後馬上登出人的 session，只留這台的金鑰 |
| POST | `/relay/logout` | 同上 | — |
| POST | `/relay/host/remove` | 在設定裡登出這台 | 這台從帳號移除，金鑰作廢 |
| GET | `/relay/host/ws` | 一直 | 長連線（§15） |
| GET／POST | `/relay/host/next`、`/relay/host/reply/<id>` | 帳號伺服器沒有 WebSocket 時 | 舊式長輪詢（§15.4） |
| GET | `/relay/host/peers` | 選大腦、手腳清單、傳送目的地、連線體檢、確認位子空了沒 | `{hosts[{id, name, online, uses, hands}], me_online}` |
| GET／POST | `/relay/host/members` | 團隊 | `{user, role:"member" 或 "admin" 或 "remove"}`；`{members[{user, role, pending}]}` |
| POST | `/relay/host/notify` | 推播 | 見 §3.1 |
| POST | `/relay/host/sites/<名稱>` | 發布網站 | `{files[{path, data（base64）}]}` → `{url}` 或 `{path}`；最多 500 個檔、25 MB、要有 `index.html`；機密檔和連到資料夾外的捷徑不送 |
| PUT／GET | `/relay/backup` | 自動備份、新電腦還原 | 加密的整份資料（上傳 120 秒；下載最多 200 MB；`404`「這個帳號還沒有備份」） |
| POST | `/relay/pair` | 「加一台電腦」 | `{for}` → `{code}`（App 在 6 位數後面接一段只留在兩台之間的金鑰） |
| POST | `/relay/pair/claim` | 新電腦用配對碼加入 | `{code, name}` → `{id, token, user, for, for_name}`（只送 6 位數） |
| POST | `/relay/screen-ticket` | 在電腦上開瀏覽器看大腦的 Bot 電腦 | `{host}` → `{ticket}`（10 分鐘） |
| GET／POST | `/relay/hosts/<id>/api/sync…`、`/relay/hosts/<id>/api/device/run` | 長連線不通時的退路 | 同 §15.4 |
| POST、GET | `/relay/manager/brain`、`/relay/manager/users` | 伺服器上的「大腦管理員」模式 | 替每個帳號開雲端大腦（`{user, name:"雲端大腦"}`） |

## 盤點統計

| 項目 | 數量 |
|---|---|
| App 自己服務的 HTTP／WebSocket 端點（方法＋路徑樣式） | **63**（手機伺服器 59 條，含 1 條 WebSocket；這台電腦上的小服務 4 條）。圖示 PNG 6 個檔算 1 條 |
| 其中有子命令的端點 | E15（11 種動作）、E18（35 種動作；`bot` 25 個 key、`setting` 16 個 key、`app` 17 個名字）、E20／E53（13 種 op）、E25／E54（8 種動作）、E38（6 種輸入）、E39（8 類工具，其他一律拒絕）、E42（9 種 op）、E43（15 種動作）、E47（2 種） |
| 長連線的訊息種類 | **10**（`hello, job, reply, call, ret, vopen, vdata, vtext, vclose, bye`），另加 WebSocket ping／pong；舊式長輪詢另用 2 條帳號伺服器端點 |
| 手機呼叫的 App 端點 | **38**（E01–E38） |
| 其他電腦呼叫的 | **21**（E39–E59） |
| 外部呼叫的 | **4**（E60–E63） |
| 整條沒人呼叫的端點 | **0** |
| 沒人呼叫的子命令 | **9**（見 §16） |
| App 當客戶端呼叫的帳號伺服器路徑 | 20 組（附錄 A） |

## 待確認

| # | 問題 |
|---|---|
| Q1 | E47 的 `wake` 找不到呼叫者：是舊版殘留，還是有別的呼叫路徑沒找到。 |
| Q2 | E54 的 `file`：程式收、protocol.md 說沒有、也沒人用。新版要不要讓電腦也能下載「給手機」的檔案。 |
| Q3 | E43 的 `read` 沒人用（螢幕改用 `bot_read`）：可以當作已淘汰。 |
| Q4 | E18 `app` 的 `save_skill`、`delete_skill` 手機沒用：是不是只給 Bot 的工具用、手機不需要。 |
| Q5 | E18 `bot` 的 `folder` 手機沒用（改走 `move_to_group`）：兩條路是否同一件事。 |
| Q6 | E18 `setting` 的 `smart_mode`：智慧模式不能關，區網直連還會當成不認得的設定擋下；這個 key 是否已淘汰。 |
| Q7 | 記憶的 `search`、`stats` 沒人用：是否只是相容舊版的別名。 |
| Q8 | E46 的 `thumb` 被忽略：螢幕要大圖時是否本來應該給大圖。 |
| Q9 | E18 的 `export_bot`、`export_transcript` 程式允許管理員呼叫，手機只給擁有者看到按鈕；匯出內容（指示、記憶、規則）給管理員是否刻意。 |
| Q10 | E37／E38 只檢查「從手機操作」開關，沒檢查這台是不是 Mac：Linux、Windows 開了會發生什麼。 |
| Q11 | 手腳上的長指令：architecture.md 說超過 25 秒移到背景；介面這邊大腦經長連線等 50 秒、經 HTTP 等 45 秒，帳號伺服器等 610 秒。這幾個數字怎麼配合沒對完。 |
| Q12 | E60 的 Webhook 只聽 127.0.0.1：GitHub 這類外部服務要怎麼打得到（使用者自己轉發？舊版另有轉送？）。 |
| Q13 | E18 的 `app` 在區網直連一律算「不放寬」；其中 `update_bot` 能改哪些欄位（會不會碰到工具、電腦、分享）沒有逐項核對。 |
