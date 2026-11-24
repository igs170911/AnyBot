# 03b · 帳號伺服器的介面、命令列、環境變數（舊版盤點）

**這是舊版的介面清單，用途是當功能清單的來源；新版 API 重新設計、不讀環境變數（00 決定 2、7）。**

> 來源：看過舊版的一方盤點 OLD 最後一版的帳號伺服器（`anybot-relay`）實際收的每條路由與長連線訊息、`anybot` 和 `anybot-relay` 的命令列、安裝與更新腳本、Docker compose 與 `.env` 範本、程式實際讀的環境變數，並和舊文件《介面與長連線格式》（OLD 的 `docs/protocol.md`）、帳號伺服器的 README 對照（2026-11-24 初稿）；**不含舊程式結構**。
> 用途：功能清單的來源之一：每條介面、每個旗標、每個環境變數背後是哪個使用者看得到的能力或設定，新版要找到對應的做法（或明確砍掉）。
> 互查：`00-boundaries.md` 的 B3、B4、B6、B14；`01-data-inventory.md` E（帳號伺服器的資料卷，欄位見 `01b`）；App 那一端的介面和長連線見 `03a-api-app.md`（§15、附錄 A）。

## 怎麼讀

- 「舊版」指 OLD 最後一版。路徑、JSON 鍵、狀態碼、旗標、畫面上的字照舊版原樣寫，只當線索。
- 時間、上限照舊版數值寫（B15）。
- 路由編號 `R01`…、長連線訊息編號 `W01`…，統計和「待確認」用這些編號。
- **不在本檔**：AnyBot 本機的 `/api/…`（手機經帳號伺服器轉送到的那些）、大腦同步的改動格式，見 `03a`。本檔只寫帳號伺服器**怎麼轉送**（§1.6）。
- 「誰能呼叫」的寫法：
  - **公開**：不用登入。
  - **人**：帶工作階段（手機頁面、App 登入帳號那一刻）。
  - **主機**：帶一台 AnyBot 的主機金鑰。**主機（非配對）**：配對加入的手腳不行。
  - **管理員金鑰**：伺服器內部的「大腦管理員」程式。

---

## 1. 帳號伺服器（`anybot-relay`）

### 1.1 身分

| 身分 | 怎麼證明 | 從哪拿到 | 格式 | 伺服器怎麼存 |
|---|---|---|---|---|
| 人（工作階段） | `Authorization: Bearer <工作階段>`；不能帶標頭的（EventSource、圖片）改用網址參數 `?t=<工作階段>` | R12 登入；開了兩步驟驗證再加 R13 | 64 個十六進位字元 | 只存雜湊 |
| 主機（一台 AnyBot） | `Authorization: Bearer <主機金鑰>` | R21（登入後 15 分鐘內）、R24（配對碼）、R52（大腦管理員代開） | 金鑰 64 個十六進位字元；主機編號 16 個十六進位字元 | 只存雜湊 |
| 配對加入的手腳 | 同上，伺服器記著它是「配對加入」 | R24 | 同上 | 能做的事比較少（§1.4.5） |
| 大腦管理員 | `Authorization: Bearer <ANYBOT_RELAY_MANAGER_TOKEN>`，**而且請求沒有 `X-Forwarded-For`** | 安裝腳本產生 | 至少 32 字元 | 環境變數 |
| 看畫面的瀏覽器 | 網址參數 `?ticket=<畫面票>` | R48、R49 | 48 個十六進位字元 | 只在記憶體 |

### 1.2 共同規則

| 項目 | 舊版行為 |
|---|---|
| 回應格式 | JSON；錯誤是 `{"error": "白話說明"}` 加狀態碼。不認得的路徑：404「沒有這個網址」。 |
| 請求本文上限 | 「看起來有登入」（`Authorization` 超過 16 字元，或網址有 `t=`）：30 MB；沒登入：256 KB；`PUT /relay/backup`：200 MB。超過回 413。 |
| 送完請求的時間 | 標頭 15 秒內送到、最多 64 KB；整個請求 90 秒內（備份 240 秒）。超過 408。 |
| 同時處理量 | 一般請求同時最多 512 個，超過 503「伺服器忙碌中，請稍後再試」；WebSocket 另算，最多 8192 條；同時在收的本文合計 512 MB，超過 503。 |
| 同一位址的連線 | 從網際網路直接來的位址同時最多 48 條，超過 429「同一個位址的連線太多了，請稍後再試」。loopback、私有網段、100.64.0.0/10（tailnet）不限（反向代理、tailnet 後面大家共用一個位址）。這條永遠看 TCP 的對方，不看 `X-Forwarded-For`。 |
| 回應送出期限 | 30 秒，加上「大小 ÷ 每秒 50 KB」。 |
| 連線 | 每回一次就關掉連線（不保持連線）。 |
| 跨站保護 | GET、HEAD 以外的請求，`Origin` 跟 `Host` 對不上，或 `Sec-Fetch-Site: cross-site` → 403「forbidden origin」。WebSocket 升級時 `Origin` 跟 `Host` 對不上 → 403。 |
| 傳輸編碼 | 只收 `chunked`；同時有 `Transfer-Encoding` 和 `Content-Length` → 400。 |
| 壓縮 | 對方接受 gzip 時，1 KB 以上的文字、JSON 由帳號伺服器自己壓（`tailscale serve` 不壓）。 |
| 固定回應標頭 | `X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer`；沒有自己指定快取的一律 `Cache-Control: no-store`。 |

### 1.3 路由總表

#### 1.3.1 頁面與公開檔案

| # | 方法與路徑 | 誰 | 回應重點 | 備註 |
|---|---|---|---|---|
| R01 | GET `/`、`/index.html` | 公開 | 手機頁面（HTML），帶 `ETag`、`Cache-Control: no-cache`、只放行頁面自己腳本的 CSP、`Strict-Transport-Security`、禁止被嵌入 | 瀏覽器已有同一版時 304 |
| R02 | GET `/screen` | 公開 | 看 Bot 的電腦的頁面（noVNC），本身沒有資料 | |
| R03 | GET `/novnc.js` | 公開 | noVNC 1.6.0（MPL-2.0），快取 1 天 | |
| R04 | GET `/trackpad.js` | 公開 | 虛擬觸控板與按鍵列（畫面頁和手機頁共用） | no-cache |
| R05 | GET `/manifest.webmanifest` | 公開 | 安裝成主畫面 App 用 | |
| R06 | GET `/sw.js` | 公開 | 推播用的 Service Worker；`Service-Worker-Allowed: /` | |
| R07 | GET `/icon.svg` | 公開 | 圖示，快取 1 天 | |
| R08 | GET `/icon-180.png`、`/icon-192.png`、`/icon-512.png`、`/icon-maskable-192.png`、`/icon-maskable-512.png`、`/badge-96.png` | 公開 | iPhone 主畫面、Android（含遮罩版）、狀態列的單色貓；快取 1 天 | 6 個檔算 1 條 |
| R09 | GET `/api/hello` | 公開 | `{"app": "anybot-relay"}`；大腦管理員 60 秒內來問過時多 `"brains": true`（App 註冊後就等自己的雲端大腦，不自己當大腦） | App、安裝腳本用它認「這是 AnyBot 帳號伺服器」 |
| R10 | GET `/api/speed?bytes=` | 公開 | 隨機、壓不小的位元組（連線體檢量下載速度）；`Cache-Control: no-store` | 預設 256 KB；1 位元組到 1 MB |

#### 1.3.2 帳號、登入、兩步驟驗證、工作階段

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R11 | POST `/relay/signup` | 公開 | `{user, password, invite}` | `{ok: true}`（不會順便登入） | 帳號 2–64 字，只能英數和 `. _ @ -`，一律轉小寫，不合 400「帳號只能用英文字母、數字和 . _ @ -（2–64 個字）」；密碼至少 8 字 400；同一位址 5 分鐘最多 5 次（成功也算）429「註冊太頻繁了，請 5 分鐘後再試」；邀請碼規則 §1.4.1，403「這台伺服器不開放註冊」或「邀請碼不對」；同名 409「這個帳號已經有人用了」 |
| R12 | POST `/relay/login` | 公開 | `{user, password, device}`（device 最多 40 字，顯示在裝置清單） | `{token}`；開了兩步驟驗證時 `{need_2fa: true, challenge}` | 錯：等 0.5 秒後 401「帳號或密碼不對」（不存在的帳號花一樣的時間）；同位址＋同帳號 5 分鐘 8 次、同帳號不分位址 5 分鐘 40 次 → 429「密碼錯太多次，請 5 分鐘後再試」；每次先算一次，成功才抵銷 |
| R13 | POST `/relay/2fa/verify` | 帶 `challenge`：公開（登入第二步）；不帶：人（設定的最後一步） | 登入：`{challenge, code}`；設定：`{code}` | 登入：`{token}`，用恢復碼時多 `used_recovery: true`；設定：`{ok: true}` | §1.4.3 |
| R14 | POST `/relay/2fa/enroll` | 人 | 已經開著時要 `{code}`（現在那個驗證器的碼） | `{secret, uri, recovery_codes}`；`uri` 是 `otpauth://totp/AnyBot:<帳號>?secret=…&issuer=AnyBot&period=30&digits=6`；恢復碼 8 組、每組 10 個十六進位字元，只顯示這一次 | 要 R13 成功才生效；已開著時碼錯 401、5 分鐘 8 次 429 |
| R15 | POST `/relay/2fa/disable` | 人 | `{code}`（現在的碼） | `{ok: true}`（本來沒開也回 ok） | 碼錯 401「驗證碼不對：輸入驗證器 App 上現在的那個六位數」；429 同上 |
| R16 | POST `/relay/password` | 人 | `{old, new}` | `{ok: true}`；**這帳號的其他工作階段全部登出**，只留這一個 | 新密碼至少 8 字 400「新密碼至少 8 個字」；舊密碼錯 401「目前的密碼不對」；同帳號 5 分鐘 8 次 429。**找不到用戶端呼叫（Q1）** |
| R17 | POST `/relay/logout` | 公開（帶自己的工作階段） | — | `{ok: true}` | 只結束這一個；沒帶或不存在也回 ok |
| R18 | POST `/relay/sessions/logout` | 人 | — | `{ok: true}` | 「登出所有裝置」：這帳號的工作階段（含自己）、畫面票、正在看的畫面全部結束；主機金鑰不受影響 |
| R19 | POST `/relay/sessions/drop` | 人 | `{id}`：裝置清單上的代號（前綴比對，至少 6 字元） | `{ok, left, was_self}`；`was_self` 為真時手機自己登出 | 不到 6 字元 400「要裝置清單裡的裝置代號（6 個字以上）」 |
| R20 | GET `/relay/me` | 人 | — | 見下表 | 401「請先登入」 |

`/relay/me` 的回應：

| 鍵 | 內容 |
|---|---|
| `user` | 帳號 |
| `two_factor` | 兩步驟驗證開了沒 |
| `hosts[]` | 這帳號能用的每台 AnyBot：`id`、`name`（不是自己的加「（擁有者 的）」）、`online`、`managed`（大腦管理員開的）、`uses`（它正在用哪個大腦的編號）、`hands`（它說自己是手腳）、`last_seen_text`（「在線上」／`MM/DD HH:MM UTC`／「從未連線」）、`why`（離線原因，伺服器重開後就沒有）、`role`（owner／admin／member） |
| `invites[]` | 等我回答的團隊邀請：`host`、`name`、`owner`、`role` |
| `sessions[]` | 登入過的裝置：`id`（8 字元代號）、`device`、`created`、`last`（Unix 秒） |
| `backup` | 有沒有加密備份 |

#### 1.3.3 主機（電腦）與配對

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R21 | POST `/relay/hosts` | 人，**而且是 15 分鐘內才登入的工作階段** | `{name, was: {id, token}}`；`name` 最多 60 字，預設「AnyBot」；`was` 是這台原本那一列的編號和金鑰（重新登入時證明身分） | `{id, token, name}`（`name` 可能變成「名字 (2)」） | 不是剛登入 403「要加入一台電腦，請在那台電腦上重新登入帳號（登入後 15 分鐘內有效）」；跟伺服器開的雲端大腦同名 409「這個帳號已經有伺服器開的「名字」，這台換個名字」；同名規則 §1.4.5；每帳號最多 100 台，滿了最久沒出現的那台被擠掉 |
| R22 | POST `/relay/hosts/<id>/remove` | 人（擁有者） | — | `{ok: true}`；那台的長連線立刻收到 `bye` 斷掉 | 不是自己的就什麼都不做，也回 ok。**找不到用戶端呼叫（Q1）** |
| R23 | POST `/relay/pair` | 人，或主機（非配對） | `{for}`：新電腦要當哪台的手腳（主機編號，可以空） | `{code, expires_in: 600}`；`code` 是 6 位數字 | `for` 不是自己的主機 403「只有這台 AnyBot 的擁有者能加電腦」；配對加入的主機 403「配對加入的手腳電腦不能再產生配對碼」；同時等配對的碼超過 10,000 個 503「現在等待配對的電腦太多了，請幾分鐘後再試」 |
| R24 | POST `/relay/pair/claim` | 公開 | `{code, name}`（`code` 只取數字；`name` 最多 60 字） | `{id, token, user, for, for_name}` | 錯或過期：等 0.3 秒 404「配對碼不對，或已經過期（10 分鐘內有效，只能用一次）」；同一位址 5 分鐘錯 8 次、整台伺服器 5 分鐘錯 200 次 → 429「配對碼錯太多次，請 5 分鐘後再試」 |
| R25 | GET `/relay/host/ws`（WebSocket） | 主機 | §1.5.1 | — | 金鑰無效 401「這台電腦的金鑰無效」 |
| R26 | GET `/relay/host/next` | 主機 | — | 有要求就回一個 job（JSON，本文 base64 放在 `body`，其他欄位同 W02）；25 秒沒有回 204 | 舊式長輪詢（伺服器沒有 WebSocket 時 App 才用）；呼叫本身也算「在線」 |
| R27 | POST `/relay/host/reply/<job id>` | 主機（只有被交辦的那台） | `{status, ctype, headers: [[名, 值]…], body（base64）, z}` | `{ok: true}` | 不是那台的回答默默丟掉 |
| R28 | GET `/relay/host/peers` | 主機 | — | `{hosts[], me_online}`：同帳號所有主機（欄位同 R20 的 `hosts`，沒有 `role`）；`me_online`：這台（可能是另一份）現在連著 | App 用它確認登入還有效、換網址時確認是同一台伺服器、看位子空了沒 |
| R29 | POST `/relay/host/remove` | 主機 | — | `{ok: true}`：這台把自己移出帳號（App 登出時），長連線立刻斷 | |

#### 1.3.4 團隊分享

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R30 | GET `/relay/host/members` | 主機 | — | `{members: [{user, role, pending}]}` | |
| R31 | POST `/relay/host/members` | 主機（非配對；擁有者那台） | `{user, role}`；`role` 是 `member`、`admin` 或 `remove` | `{ok: true}` | 帳號不存在 404「這台伺服器上沒有這個帳號（請對方先建立帳號）」；給自己 400「這是你自己的帳號」；角色不對 400「角色只能是 member 或 admin」；配對加入 403 |
| R32 | POST `/relay/invites` | 人（被邀請的） | `{host, accept}` | `{ok: true}` | 沒有這個邀請 404「沒有這個邀請」 |
| R33 | POST `/relay/hosts/<id>/leave` | 人 | — | `{ok: true}`：退出別人分享給我的那台 | **找不到用戶端呼叫（Q1）** |

#### 1.3.5 推播

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R34 | GET `/relay/push/key` | 公開 | — | `{key}`：推播用的伺服器公鑰（VAPID，base64url） | |
| R35 | POST `/relay/push/subscribe` | 人 | 瀏覽器給的訂閱 `{endpoint, keys: {p256dh, auth}, kinds}` | `{ok: true}` | `endpoint` 要 https、最多 2048 字；`p256dh` 最多 256、`auth` 最多 128；不完整 400「通知訂閱資料不完整」；同一個 endpoint 先刪舊的；每帳號最多 16 支手機，滿了最舊的被擠掉 |
| R36 | POST `/relay/push/state` | 人 | `{endpoint}` | `{on, kinds}`（沒訂閱：`on: false`、`kinds` 全部） | |
| R37 | POST `/relay/push/kinds` | 人 | `{endpoint, kinds}` | `{ok: true}` | `kinds` 不是陣列 400「要給一份通知種類」；沒訂閱 404「這支手機還沒開通知」 |
| R38 | POST `/relay/push/test` | 人 | `{endpoint}` | `{ok: true}`；手機收到「通知正常：Bot 需要你時，會像這樣跳出來。」 | 沒訂閱 404；推播服務沒收 502「推播服務沒有收下，關掉通知再打開一次試試」 |
| R39 | POST `/relay/push/unsubscribe` | 人 | `{endpoint}` | `{ok: true}` | |
| R40 | POST `/relay/host/notify` | 主機 | `{title, body, bot, kind, badge, ask, tag}` | `{sent: 送出幾則}` | §1.4.8 |

#### 1.3.6 發布網站

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R41 | POST `/relay/host/sites/<名稱>` | 主機（非配對） | `{files: [{path, data（base64）}]}`，或 `{remove: true}`；整個請求受 30 MB 上限 | `{files: 寫了幾個, path: "/s/<網站>/", url}`；`url` 是 ANYBOT_RELAY_PUBLIC_URL 加 `path`，沒設就空字串 | 名稱 1–40 字、英數和 `-`（轉小寫），不合 400「網站名稱只能用英文字母、數字和 -（最多 40 個字）」；跟別的帳號撞名 409「這個網站名稱和別的帳號的網站撞名了，請換一個名稱」；每帳號 50 個 403「這個帳號發布的網站已經到上限了，先移除不用的」；配對加入 403；路徑有空段、`.`、`..` 的檔案略過 |
| R42 | GET `/s/<網站>/<路徑>` | 公開 | — | 檔案；資料夾或沒給路徑時給 `index.html`；類型依副檔名（html／htm、css、js／mjs、json、svg、png、jpg／jpeg、gif、webp、ico、txt／md、pdf、woff2、mp4，其他 `application/octet-stream`）；一律加 CSP `sandbox allow-scripts allow-popups allow-modals allow-downloads; form-action 'none'; connect-src 'none'` | 找不到 404「找不到這個網頁」 |

#### 1.3.7 備份

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R43 | PUT `/relay/backup` | 主機（非配對） | 原始位元組（電腦上加密好的，伺服器打不開） | `{ok: true, bytes}` | 少於 28 位元組 400「備份內容不完整」；超過 200 MB 413「備份太大了（最多 200 MB）」；配對加入 403「配對加入的手腳電腦沒有備份金鑰，不能覆蓋帳號備份」；每帳號一份，新的蓋舊的 |
| R44 | GET `/relay/backup` | 人 | — | 原始位元組 | 沒有 404「還沒有備份」 |

#### 1.3.8 轉送到某台 AnyBot（規則見 §1.6）

| # | 方法與路徑 | 誰 | 轉成 | 限制與錯誤 |
|---|---|---|---|---|
| R45 | 任何方法 `/h/<主機>/api/…` | 人，在那台的角色是 owner／admin／member | 那台的 `/api/…`（網址參數 `t` 拿掉），角色照實帶過去 | 沒權限或沒這台 404「找不到這台 AnyBot」；`/h/<主機>/` 後面不是 `api`，或是 `api/device…` → 404「沒有這個網址」 |
| R46 | POST `/relay/hosts/<目標>/api/device/run` | 主機（非配對），目標是同帳號的另一台 | 目標的 `/api/device/run`，角色 `device` | 配對加入 403「這台電腦是配對加入的手腳，不能控制其他電腦」；不同帳號或自己 403「不是同一個帳號的電腦」 |
| R47 | 任何方法 `/relay/hosts/<目標>/api/sync[/…]` | 同上 | 目標的 `/api/sync…`（帶網址參數），角色 `mirror` | 配對加入 403「這台電腦是配對加入的手腳，不能存取大腦設定」；其他同上 |

#### 1.3.9 看 Bot 的電腦

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R48 | POST `/relay/screen-ticket` | 主機（非配對，要看的是同帳號的主機），或人（在那台是 owner、admin） | `{host}` | `{ticket}` | 配對加入 403「這台電腦是配對加入的手腳，不能要求觀看畫面」；沒這台或沒權限 404「找不到這台 AnyBot」 |
| R49 | POST `/relay/screen-ticket/renew` | 公開（票本身就是證明） | `{ticket}` | `{ticket}`：新票，舊票立刻作廢 | 過期或要票的人已失效 410「這個畫面連結已過期，請從 AnyBot 重新打開畫面」 |
| R50 | GET `/relay/vnc/<主機>?t=…` 或 `?ticket=…`，可加 `&bot=…`（WebSocket） | owner、admin | §1.5.3 | — | 沒登入 401「請先登入」；沒權限 404「找不到這台 AnyBot」；那台沒連著 503「這台 AnyBot 現在沒有連線」 |

#### 1.3.10 大腦管理員

| # | 方法與路徑 | 誰 | 請求 | 回應 | 限制與錯誤 |
|---|---|---|---|---|---|
| R51 | GET `/relay/manager/users` | 管理員金鑰，且沒有 `X-Forwarded-For` | — | `{users: [帳號], created: {帳號: 建立時間}, brains: {帳號: 管理員開的主機編號}}` | 金鑰不對、伺服器沒設金鑰（或不到 32 字）、經過反向代理 → 一律 404「not found」 |
| R52 | POST `/relay/manager/brain` | 同上 | `{user, name}`（`name` 預設「雲端大腦」，最多 60 字） | `{id, token, user, name}`：替這帳號開一台雲端大腦的主機金鑰（同名、自己先前開的那台被登出） | 沒這帳號 404「沒有這個帳號」；帳號裡已有同名、不是管理員開的電腦 409「這個帳號已經有一台叫「名字」的電腦（不是伺服器開的）」 |

### 1.4 生命週期

#### 1.4.1 註冊與邀請碼

- 邀請碼是伺服器的一個設定（ANYBOT_RELAY_INVITE）：大家共用同一組，沒有期限、沒有次數；要換就改設定、重開伺服器。

| 邀請碼有設？ | 已經有帳號？ | 註冊 |
|---|---|---|
| 沒設 | 沒有 | 第一個人不用邀請碼就能註冊 |
| 沒設 | 有 | 一律 403「這台伺服器不開放註冊」 |
| 有設 | 不管 | 一定要填對，**第一個帳號也要**；錯 403「邀請碼不對」 |

- 伺服器安裝腳本（不問問題的裝法）會自動產生 `xxxx-xxxx` 的邀請碼（8 個字，不用容易看錯的字），最後的畫面印出來，並提醒「先自己註冊：第一個註冊的帳號是擁有者」「只給你信任的人」。
- 第一個註冊的帳號是伺服器的擁有者：只有它的雲端大腦拿得到 Docker（§3.6）。
- 啟動時還沒有帳號會印「還沒有帳號：第一個註冊的人會成為這台伺服器的使用者。」（有設邀請碼時也照印）。

#### 1.4.2 工作階段（人的登入）

- 登入成功、或兩步驟的第二步成功，才發。
- 90 天沒用就失效；「用過」最多每分鐘記一次（每分鐘存檔一次）。
- 每帳號最多 50 個，超過時最久沒用的先失效。
- 結束的方式：登出（R17，這一個）、裝置清單移除一個（R19）、登出所有裝置（R18）、改密碼（R16，其他全部）、命令列重設密碼（全部，§2.2）。
- 「剛登入」＝15 分鐘內發的工作階段：只有它能加電腦、拿主機金鑰（R21）。

#### 1.4.3 兩步驟驗證（TOTP）

- 開：R14 拿到 secret、QR 用的 uri、8 組恢復碼 → 驗證器輸入現在的 6 位數 → R13（帶工作階段）→ 生效，恢復碼同時換成這次的 8 組。R13 成功之前不算開。
- 已經開著再 R14（換驗證器）：要先給現在那個驗證器的碼。
- 關：R15 要現在的碼；關掉時恢復碼一起清掉。
- 登入：密碼對 → `{need_2fa, challenge}`；challenge 5 分鐘有效、最多試 3 次，過了 400「這次登入已過期，請重新輸入密碼」。
- 碼：30 秒一格，前後各容許一格；同一組碼 2 分鐘內不能再用（旁邊看到的人不能拿去用）。
- 恢復碼：每組只能用一次；用了回 `used_recovery: true`。
- 登入時碼錯：等 0.3 秒 401「驗證碼不對」；同位址＋同帳號、同帳號，各 5 分鐘 8 次 → 429「驗證碼錯太多次，請 5 分鐘後再試」，challenge 同時作廢。
- 設定時碼錯：401「驗證碼不對：輸入 App 上現在的那個六位數」，5 分鐘 8 次 429；沒有待確認的設定 400「沒有待確認的設定：先點「開始設定」取得QR Code」。

#### 1.4.4 配對碼（「加一台電腦」）

- 誰能產生：人（手機、電腦登入時）、主機（非配對）。
- 6 位數字、10 分鐘、只能用一次；只在記憶體，伺服器重開就沒了。
- 兌換的人不用帳號密碼：拿到主機金鑰、帳號名、要當哪台的手腳。
- 兌換出來的主機永遠是「配對加入的手腳」；名字跟帳號裡別台重複時自動改成「名字 (2)」（不會蓋掉別台）。
- App 給的那一行：`anybot setup 網址 配對碼`；另有 Mac／Linux 的 curl 一鍵安裝和 Windows 的 PowerShell 版本（§2.3）。

#### 1.4.5 主機登記

- 用密碼登入加入（R21）：
  - 帶 `was` 證明是原本哪一列：換掉那一列，保留它的「名字 (n)」和團隊分享。
  - 沒帶 `was`、帳號裡有同名的：那台**現在不在線**，當成是這台以前的樣子（重灌、金鑰掉了）→ 換掉；**在線**，當成另一台同名的電腦（兩台 MacBook Pro）→ 新的這台叫「名字 (2)」。伺服器紀錄寫「…正在線上，新登入的這台改叫「…」（不再把它踢掉）」。
  - 被換掉的那一列的長連線立刻斷。
  - 每帳號最多 100 台，滿了最久沒出現的那台被擠掉。
- 配對加入（R24）：§1.4.4。
- 大腦管理員代開（R52）：一律換掉自己先前開的同名大腦。
- 移除：App 登出時自己移除（R29）、擁有者移除（R22）、被同名換掉。被移除的主機用舊金鑰連來會被拒（401），伺服器紀錄十分鐘最多寫一次「…用這裡不認得的金鑰連線（那台電腦被移出帳號，或被同名的電腦換掉了）：它要重新登入」。
- **配對加入的手腳不能**：呼叫別台（W04、R46、R47）、要畫面票（R48）、產生配對碼（R23）、改團隊（R31）、發布網站（R41）、上傳備份（R43）。

#### 1.4.6 團隊分享

- 擁有者那台（主機金鑰）加人：R31 `{user, role}` → 對方的 R20 `invites` 出現邀請；**接受前看不到、不能用**（以前任何人都能把任何帳號加進自己的 AnyBot，是接管帳號的第一步）。
- 對方接受（R32 `accept: true`）→ 生效；拒絕 → 移除。
- 改角色保留已接受的狀態；`remove` 移除。
- 對方可以自己退出（R33）。
- 角色：owner（擁有者，隱含）、admin（手機上跟擁有者一樣：核准、看畫面、操作畫面）、member（只能聊天、附檔）。帳號伺服器把角色帶給 AnyBot，由 AnyBot 決定 member 能做什麼；看畫面（R48、R50）和收推播只給 owner、admin。

#### 1.4.7 畫面票

- 用途：讓 App 開的瀏覽器（沒有自己的登入）看某台的 Bot 的電腦；手機也用它，免得長期的登入出現在網址裡（網址會進紀錄、瀏覽紀錄）。
- 發出後 10 分鐘內可以開新連線；已經連著的不受影響，只要「要票的人」還有效（那支手機還登入著、那台電腦還在帳號裡）。
- 畫面頁打開時、每次重連前，用 R49 換新票，舊票立刻作廢（網址裡那張在頁面打開後就沒用了）；最後一次使用後 10 分鐘內都能換，超過要從 AnyBot 重新打開。
- 登出所有裝置（R18）：這帳號的票全部作廢、正在看的畫面全部斷。

#### 1.4.8 推播

- 伺服器第一次啟動時產生推播用的伺服器金鑰（資料夾裡的 `vapid.pk8`；換掉的話所有手機的訂閱都失效）。
- 種類：`need`（需要你）、`done`（做完了）、`routine`（例行任務）、`file`（收到檔案）；測試通知是 `test`。
- 手機可以挑種類；從沒挑過＝全部。（訂閱時給空陣列也被當「全部」，用 R37 設成空的卻是「都不要」，見 Q8。）
- 主機送通知（R40）：只送給這台 owner、admin 的手機，而且種類對得上；種類不認得就當成空的（每支都送）。
- 通知內容：`title`（最多 80 字，預設「AnyBot」）、`body`（最多 200 字）、`bot`、`host`（伺服器補上）、`kind`、`badge`（最多 999）、`ask`（要核准的那張卡的編號，最多 100 字，讓橫幅上的允許／拒絕只回答那一張）、`tag`（只留英數和 `-`，最多 40 字；同一個 tag 的橫幅互相取代，工作流的「等你確認」才不會被別的蓋掉）。
- 送出：內容加密（aes128gcm），`TTL: 86400`、`Urgency: high`，10 秒逾時、不跟轉址；推播服務的位址要是 https、不能是 localhost 或 `.local`、解析出來要是公開位址；推播服務回 404 或 410 → 刪掉那個訂閱。
- 推播服務要的聯絡方式：ANYBOT_RELAY_CONTACT，沒設用 `mailto:anybot-relay@example.com`。

#### 1.4.9 發布網站

- 網址 `/s/<帳號（非英數換成 ->-<名稱>/`。每次發布整個資料夾重來（先刪後寫）。兩個帳號湊出同一個資料夾名時，屬於先發布的那個。
- 公開頁面在沙盒裡：不能送表單、不能連網、拿不到同網址手機頁面的登入，不能冒充這個網址的登入頁。

#### 1.4.10 備份

- 每帳號一份加密檔（`backups/<帳號雜湊>.bin`）：主機上傳、人下載；伺服器打不開。鑰匙由密碼產生，所以命令列重設密碼後要在電腦上按一次「立刻備份」（會提醒）。
- 新版不再讀這些舊備份（00 決定 5）。

### 1.5 長連線

#### 1.5.1 主機的長連線（R25）

- 每台 AnyBot 主動連出去、一直開著（電腦不用開連接埠）。給它的要求立刻從這條下來；它問帳號裡別台也從這條上去。
- 格式：每則一個 WebSocket 二進位訊息＝「4 位元組表頭長度（big-endian）＋表頭 JSON＋本文原始位元組」。本文不做 base64。表頭最大 1 MB；整則最大 60 MB。
- `z: true`：本文用 deflate 壓過；`z_ok: true`：問的人看得懂壓過的回答。

| # | t | 方向 | 表頭欄位 | 意思 |
|---|---|---|---|---|
| W01 | `hello` | AnyBot → 伺服器 | `brain`、`hands` | 這台在看哪個大腦（空＝自己）、是不是手腳；顯示成 R20／R28 的 `uses`、`hands` |
| W02 | `job` | 伺服器 → AnyBot | `id, method, path, query, user, role, from, z, z_ok` | 交給這台的要求；`role` 是 owner／admin／member（手機）、`mirror`（大腦同步）、`device`（裝置連結）；`from` 是問的那台主機編號（手機是空字串） |
| W03 | `reply` | AnyBot → 伺服器 | `id, status, ctype, headers, z` | 回答 job；只有被交辦的那台能回 |
| W04 | `call` | AnyBot → 伺服器 | `id, to, method, path, query, z` | 問帳號裡另一台；只能是 `/api/sync…`（mirror）或 `/api/device/run`（device） |
| W05 | `ret` | 伺服器 → AnyBot | `id, status, ctype, z, headers` | call 的回答（錯誤也用它回，本文是 `{"error": …}`） |
| W06 | `vopen` | 伺服器 → AnyBot | `sid, bot` | 有人要看這台的 Bot 的電腦；`bot` 是看哪個 Bot 的螢幕（只有數字） |
| W07 | `vdata` | 雙向 | `sid` | VNC 位元組（本文）；只有開這條畫面的那台能寫 |
| W08 | `vtext` | 伺服器 → AnyBot | `sid, bot` | 看的人打的字（UTF-8），打進那個 Bot 的螢幕 |
| W09 | `vclose` | 雙向 | `sid`（AnyBot 送的可能有 `why`） | 結束這條畫面 |
| W10 | `bye` | 伺服器 → AnyBot | `why`（可省略；`replaced`＝被同一把金鑰的另一條連線取代） | 伺服器要結束這條連線 |

W04 的錯誤（用 W05 回）：配對加入 403「這台電腦是配對加入的手腳，不能控制其他電腦或修改大腦」；路徑不對 404「沒有這個網址」；不同帳號或自己 403「不是同一個帳號的電腦」；同一台同時進行超過 32 個 429「這台電腦同時進行的請求太多了，稍後再試」。

保活與換手：

- 伺服器每 20 秒送 WebSocket ping；60 秒完全沒收到任何位元組就斷（大檔案慢慢傳的途中也算活著）。
- 每 10 秒檢查這台還在帳號裡、這條還是最新的一條：
  - 被移出帳號或被同名換掉 → 斷。
  - 同一把金鑰又連進來一條 → 舊的那條收到 `bye`（`why: replaced`）；舊的那端 3 秒內關閉＝真的有兩份在跑（紀錄寫「同一台電腦開了兩份 AnyBot，或資料夾被複製到別台」，十分鐘寫一次），沒關＝同一台換了網路，新的接手。
  - 取代它的那條也斷了 → 收到沒有 `why` 的 `bye`，請它重新連。
- 斷線時：這條上還在等的要求立刻回 503「那台 AnyBot 斷線了（網路中斷，或 AnyBot 關掉了）」；在這條上開的畫面全部結束；記下離線原因和時間（§1.9）。
- App 那一端的行為（細節見 `03a` §15）：斷了自動重連；收到 `bye`（`replaced`）先等 5 分鐘，再用 R28 的 `me_online` 看位子空了沒，空了才連；伺服器沒有 WebSocket 就改用 R26／R27。

#### 1.5.2 舊式長輪詢（R26、R27）

- 主機一直問 R26，伺服器最多壓 25 秒；有要求就回 W02 的欄位加 `body`（base64）。回答用 R27。
- 轉送時，要求 8 秒內沒被拿走就當那台不在了：回 503 離線，並把要求丟掉（不讓它晚到才執行）。

#### 1.5.3 看畫面的 WebSocket（R50）

- 認證：`?t=<工作階段>` 或 `?ticket=<畫面票>`；`bot=` 只留數字、最多 20 字。
- 看的人送：二進位＝VNC 資料（轉成 W07）、文字＝要打進去的字（轉成 W08）。伺服器送：二進位 VNC 資料。
- 單則最大 16 MB；看的人落後 256 則就被斷（不是在看即時畫面了）。
- 每 5 秒重新確認：角色還是 owner／admin、工作階段還在；用票的：要票的人還有效、票沒被收回。
- 每 20 秒 ping；60 秒沒任何回應就斷（ping 的回應也算；noVNC 畫面不動時什麼都不送）。
- 那台換了新的長連線時，只斷在舊連線上開的畫面。

### 1.6 轉送規則

| 從誰 | 路徑 | 轉成 | 角色 | 最多等 |
|---|---|---|---|---|
| 手機（人） | R45 `/h/<主機>/api/…` | 那台的 `/api/…` | owner、admin、member | 35 秒 |
| 主機 | R47、或 W04 的 `/api/sync…` | 目標的 `/api/sync…` | `mirror` | 35 秒 |
| 主機 | R46、或 W04 的 `/api/device/run` | 目標的 `/api/device/run` | `device` | 610 秒（Bot 經別台叫模型、跑長指令） |

- 轉送時附上 `user`（帳號）、`role`、`from`（問的那台主機編號；手機是空）。
- 對方不在線 → 立刻 503「那台 AnyBot 目前離線（電腦可能關機、睡眠、斷網，或 AnyBot 沒開）」；伺服器看過它斷線的，改成「那台 AnyBot 目前離線：<原因>」。
- 長連線送不出去 → 503「那台 AnyBot 斷線了…」；收到了但時間到還沒回 → 504「那台 AnyBot 收到了，但太久沒有回應（可能太忙，或網路很慢）」。
- 回答的過濾（對方可能是別人分享來的大腦）：
  - `Content-Type` 只放行 `application/json`、`text/plain`、`text/markdown`、`text/csv`、`text/event-stream`、`application/pdf`、`application/octet-stream`、`image/png`、`image/jpeg`、`image/gif`、`image/webp`、`image/bmp`、`image/avif`、`image/heic`、`image/x-icon`、`audio/*`、`video/*`；其他（HTML、SVG、XML、不認得的）一律改成 `application/octet-stream`（變成下載）。
  - 一律加 `X-Content-Type-Options: nosniff`、`Content-Security-Policy: sandbox; default-src 'none'`。
  - 回應標頭只放行 `X-Size`（主機對主機的 W05 則原樣帶回）。
  - 壓過的回答解開後最大 60 MB；解不開 502「那台 AnyBot 傳來的資料壞掉了」。手機接受 gzip 時再壓一次給它。

### 1.7 次數與數量限制總表

| 什麼 | 算法 | 上限 | 窗口 | 超過時 |
|---|---|---|---|---|
| 登入密碼錯 | 位址＋帳號 | 8 | 5 分鐘 | 429 |
| 登入密碼錯 | 帳號（不分位址） | 40 | 5 分鐘 | 429 |
| 註冊 | 位址（成功也算） | 5 | 5 分鐘 | 429 |
| 配對碼兌換錯 | 位址 | 8 | 5 分鐘 | 429 |
| 配對碼兌換錯 | 整台伺服器 | 200 | 5 分鐘 | 429 |
| 登入第二步碼錯 | 位址＋帳號；帳號 | 各 8 | 5 分鐘 | 429，challenge 作廢 |
| 同一個 challenge | 次數 | 3 | 5 分鐘有效 | 400 |
| 設定兩步驟碼錯 | 帳號 | 8 | 5 分鐘 | 429 |
| 換、關兩步驟碼錯 | 帳號；位址＋帳號 | 各 8 | 5 分鐘 | 429 |
| 改密碼 | 帳號（每次都算） | 8 | 5 分鐘 | 429 |
| 一台主機同時問別台 | 同時進行 | 32 | — | 429 |
| 一個公開位址的連線 | 同時 | 48 | — | 429 |
| 每帳號工作階段 | 個數 | 50 | — | 最久沒用的失效 |
| 每帳號主機 | 台數 | 100 | — | 最久沒出現的被擠掉 |
| 每帳號網站 | 個數 | 50 | — | 403 |
| 每帳號推播訂閱 | 支數 | 16 | — | 最舊的被擠掉 |
| 等待中的配對碼 | 整台伺服器 | 10,000 | — | 503 |

「位址」怎麼算見 §1.8。計數只在記憶體，伺服器重開就歸零。

### 1.8 反向代理與來源位址

- 預設：用 TCP 連線的對方位址。
- `RELAY_TRUST_PROXY` 是 `1`、`true` 或 `yes`：改信 `X-Forwarded-For` 的**最後一個**值（代理自己加的；最前面的是客戶端自己寫的，不信），最多取 64 字元。
- 再設了 `ANYBOT_RELAY_PROXY_SECRET`：只有帶 `X-AnyBot-Proxy: <同一個暗號>` 的請求才信 `X-Forwarded-For`。同一個 Docker 網路上的雲端大腦直接連進來、自己亂寫 `X-Forwarded-For` 的，不信。沒設暗號就照上一條。
- 影響：註冊、登入、兩步驟、配對碼兌換的次數限制，和伺服器紀錄裡的位址。**不影響** §1.2「同一位址 48 條連線」（那條永遠看 TCP 的對方）。
- 大腦管理員（R51、R52）：只要請求帶 `X-Forwarded-For`，就當成從外面來的（404），所以只能在伺服器內部直接連。
- 為什麼要設（舊版 README 的說法）：放在代理後面卻沒設，所有人都算成代理的位址，任何人連錯幾次密碼就把那個帳號對所有人鎖住。只有在外面連不到這個連接埠、一定要經過代理時才能設。
- 伺服器安裝腳本的做法：Caddy 在前面時自動產生暗號，在 Caddyfile 每個轉到帳號伺服器的區塊加 `header_up X-AnyBot-Proxy {$ANYBOT_PROXY_SECRET}`，並用 Caddy 檢查設定；用 Tailscale serve 時帶不了暗號，就不設（照舊相信）；Caddyfile 有別的轉法補不上時警告「Caddyfile 沒辦法自動補上送暗號的那行…註冊、登入的次數限制擋不住偽造的」。

### 1.9 在線、離線

- 在線：長連線連著，或 60 秒內用 R26 問過。
- 最後出現時間：每分鐘存一次，斷線時立刻記。
- 離線原因（給人看，伺服器重開後就忘了）：
  - 「它那邊關掉了連線（AnyBot 關了、電腦睡眠，或網路換了）」
  - 「連線被切斷（AnyBot 被強制關掉，或中間的網路、代理把它切了）」
  - 「連線斷了（網路中斷）」
  - 「送資料給它時斷了（網路中斷）」
  - 「超過 60 秒沒有它的任何訊號（網路斷了，或電腦睡著了）」
  - 「這台被移出帳號了（或被同名的電腦換掉）」
  - 「後來那條連線也斷了，請它重新連上」

### 1.10 伺服器存了什麼

| 放在 | 內容 | 重開後 |
|---|---|---|
| 資料夾的 `data.json` | 帳號、工作階段、主機、推播訂閱、網站對照（欄位見 `01b`） | 保留 |
| `backups/`、`sites/`、`vapid.pk8` | §1.4.10、§1.4.9、§1.4.8 | 保留 |
| 只在記憶體 | 配對碼、畫面票、兩步驟 challenge、2 分鐘內用過的驗證碼、次數計數、誰在線、離線原因、主機說自己看哪個大腦／是不是手腳、等待中的轉送 | 消失 |

- `data.json` 讀不了或內容壞掉 → 伺服器不啟動（結束碼 2），不會當成空的蓋掉：「…的內容壞了（…）。為了不蓋掉它，先停下來；修好或移開這個檔案再啟動。」
- 存檔失敗會印「AnyBot 帳號伺服器：存不了 data.json（…）」。
- 資料夾和檔案只給擁有者讀寫。

### 1.11 舊版附的反向代理設定（新版不附，`02` A11）

| 範本 | 用在 | 重點 |
|---|---|---|
| 網域版 | 有網域、DNS 指到這台 | Caddy 自動申請、更新憑證；`Strict-Transport-Security` |
| 區網 IP 版 | 沒網域，只在區網 | `https://<IP>`，Caddy 自己簽憑證；`http://<IP>/anybot-ca.crt` 下載根憑證，其他 http 轉 https |
| tailnet／區網 http 版 | 不問問題的裝法、tailnet 沒憑證 | `http://<位址>` 直接用（不轉 https），同時有自簽的 `https://<位址>` 給手機推播用；任何名字打到 80 埠都是同一個帳號伺服器 |

三個共同：gzip；請求本文最大 210 MB；`/relay/manager/*` 直接回 404；帶 `X-AnyBot-Proxy`；讀取逾時 620 秒；重載設定時舊連線 5 分鐘後才斷；`nosniff`、`X-Frame-Options: SAMEORIGIN`、`no-referrer`。

Tailscale 用法：帳號伺服器只開在 `127.0.0.1:8780`，由 `tailscale serve --bg --https=<埠> http://127.0.0.1:8780` 加上 HTTPS（埠取自 ANYBOT_RELAY_PUBLIC_URL，沒寫就 443）。

### 1.12 跟舊文件的差異

《介面與長連線格式》（OLD 的 `docs/protocol.md`）：

| 項目 | 文件寫的 | 實際行為 |
|---|---|---|
| 註冊 | 第一個帳號不用邀請碼，之後要 | 設了邀請碼時第一個也要；沒設邀請碼而已有帳號時完全不能註冊 |
| 登入鎖定 | 位址＋帳號 8 次／5 分 | 另有同帳號不分位址 40 次／5 分 |
| R21 加電腦 | 同名的主機會換新金鑰 | 同名但在線的不換，新的叫「名字 (2)」；要 15 分鐘內登入的工作階段；可帶 `was`；跟管理員開的雲端大腦同名 409 |
| R14 兩步驟 | 沒提換驗證器 | 已開著時要現在的碼 |
| R23／R24 配對碼 | `for` 看起來必填；同位址錯 8 次停 5 分 | `for` 可以空；配對加入的主機不能產生；另有整台伺服器 200 次 |
| 轉送逾時 | 35 秒沒回 504 | `/api/device/run` 等 610 秒；長輪詢的主機 8 秒沒拿走就 503 |
| R48 畫面票 | 只有同帳號的主機能要 | 人（owner、admin）也能要；有 R49 換新；「10 分鐘」是「發出後 10 分鐘內能開新連線」 |
| 看畫面 | 沒提 | R50 多 `bot=`；W06、W08 帶 `bot` |
| 長連線訊息 | 9 種 | 還有 W10 `bye`（含 `why: replaced`） |
| W04 上限 | 沒提 | 同時 32 個，429 |
| R43／R44 備份 | `PUT/GET` | PUT 要主機（非配對）、GET 要人；最大 200 MB |
| 團隊 | `GET/POST members` | 加人是「邀請」，要對方 R32 接受 |
| 沒寫到的路由 | — | R16、R19、R32、R33、R29、R34–R39（推播 6 條）、R49、R51、R52、R09、R10、R01–R08 |

帳號伺服器的 README：

| 項目 | README 寫的 | 實際 |
|---|---|---|
| 密碼雜湊 | PBKDF2-SHA256 12 萬輪 | 新密碼用 argon2id（19 MiB、2 輪）；舊的 PBKDF2 照樣能登入，改密碼時換新（protocol.md 寫對了） |
| 環境變數表 | 5 個 | 少了 ANYBOT_RELAY_MANAGER_TOKEN、ANYBOT_RELAY_PROXY_SECRET |
| 開放註冊 | 「在 docker-compose.yml 設定 ANYBOT_RELAY_INVITE」 | 帳號伺服器資料夾那份 compose 把它寫死成空字串：照它架的，第一個帳號之後誰都不能註冊 |

---

## 2. 命令列、安裝與更新

### 2.1 `anybot`

#### 2.1.1 跑法總表

| 跑法 | 做什麼 | 輸出 | 結束碼 |
|---|---|---|---|
| `anybot`（不帶參數） | 開視窗。資料夾被另一份 AnyBot 用著（等 5 秒）→ 改開一個 460×240 的小視窗「AnyBot 已經在執行」，寫出原因（哪個程序、哪個資料夾）和「關閉」鈕 | 視窗 | 0 |
| `anybot --serve` | 不開視窗：Bot、例行任務、觸發、訊息頻道、手機頁面繼續跑（一直開著的 Mac、伺服器）。啟動時讀一串環境變數（§3.2）；帳號伺服器還沒好、帳號還沒建時每 30 秒重試；收到停止訊號就收尾（停掉 Bot 開的東西、關連接器、存聊天），第二次訊號立刻結束 | 紀錄：第一行是版本；「透過帳號 … 連線（…）」；沒手機密碼也沒帳號時警告「還沒有設定手機密碼或帳號。用 ANYBOT_PHONE_PASSWORD=… 或 ANYBOT_ACCOUNT=網址\|帳號 + ANYBOT_ACCOUNT_PASSWORD=… 啟動。」 | 資料夾被占用 1；收尾完 0；第二次訊號 1 |
| `anybot --ask 問題…` | 問一個 Bot 一句，等這一輪（`/goal` 的話等到整個目標做完）；不開視窗；會先接上連接器；也能下 AnyBot 自己的 `/` 指令；要核准的事預設自動拒絕；這一輪夠長時等它學技能（最多 180 秒） | §2.1.2 | §2.1.2 |
| `anybot setup …` | 在終端機把這台接到帳號（手腳或大腦） | §2.1.3 | §2.1.3 |
| `anybot --install-skill 來源` | 安裝技能（GitHub 網址、`owner/repo/路徑`、SKILL.md 網址、本機資料夾或檔案） | 「已安裝技能「名字」（在 設定 › 整合 › 技能 可看）」 | 0；沒給來源 2；失敗 1；資料夾被占用 1 |
| `anybot --update-box` | 更新 Bot 的電腦（＝設定 › 電腦 › 更新：先存快照、有 Bot 在用就排隊、檔案和登入保留）。有 AnyBot 在跑就請它做，最多等 2 分鐘；沒有就自己做，一步一步印出 | 進度、結果；等不到時說「正在執行的 AnyBot 還沒接手…」 | 0 成功；1 失敗或沒接手 |
| `anybot --doctor` | 連線體檢（同聊天裡的 `/doctor`）：這台 ↔ 帳號伺服器的來回時間、下載速度、帳號伺服器還認不認得這台、帳號伺服器 ↔ 大腦 | 第一行「連線體檢：<網址>（大腦「名字」）」，之後一行一項，前面 ✓ 或 ✗；沒登入印「這台沒有登入帳號：沒有帳號伺服器可以量」 | 全部 ✓ 0；有 ✗ 1；沒登入 0 |
| `anybot --screenshot 檔名` | 開視窗並把畫面畫成一張圖（不占資料夾，可以跟正在跑的 AnyBot 並存） | 圖檔 | — |
| `anybot --banner 標題 內文` | 用 Bot 的方式送一則系統通知（檢查通知有沒有開） | 印出送出的結果 | 0 |
| `anybot --version`、`-V` | 印「AnyBot v<版本>-<commit> · 建置時間」，不碰資料夾 | 一行 | 0 |
| `anybot --help`、`-h` | 印用法 | 用法 | 0 |
| 不認得的 `--旗標` | 不開視窗 | 「AnyBot：不認得「…」。」加用法 | 2（單一破折號的 macOS 系統參數不管） |
| `anybot --brains` | 大腦管理員：替帳號伺服器上每個帳號開一台雲端大腦（伺服器的 `brains` 服務用） | 紀錄「大腦管理員：每個帳號一個雲端大腦（映像…，網路…）」 | 設定不全 2；否則一直跑 |
| `anybot --sample-state [檔名]` | 開發用：寫一份範例設定（預設 `state.json`，不含帳號） | 檔案 | 寫不進 1 |
| `anybot --write-icon 輸出 [像素] [svg]` | 把圖示畫成 PNG（預設 256；安裝、打包用） | 檔案 | 失敗 1 |
| `anybot --tool 工具 JSON …` | 開發用：依序跑畫面工具，印出 Bot 會聽到的結果；回來的圖存在暫存資料夾 `anybot-tool-N.jpg` | 文字 | 全部成功 0，否則 1 |
| `anybot --desktop 指令 JSON` | 開發用：試桌面操作（預設指令 `apps`） | JSON | 失敗 1 |
| `anybot __anybot_sandbox …` | 內部：Linux 上包住單一指令的沙盒，AnyBot 自己叫自己，不是給人用的 | — | — |

資料夾：

- 同一個資料夾一次只能一份 AnyBot（視窗、`--serve`、`--ask`；`setup`、`--install-skill` 寫設定時也要拿）。
- **只有 `--ask` 認得 `--data-dir`**；其他跑法只能用環境變數 ANYBOT_DATA_DIR 換資料夾（`01` A 的寫法要改，見 Q11）。

#### 2.1.2 `anybot --ask`

| 選項 | 意思 |
|---|---|
| `--json` | 一行一個 JSON（給腳本） |
| `--bot 名字` | 問哪個 Bot（先比完全相同，再不分大小寫；預設主要 Bot） |
| `--data-dir 資料夾` | 用另一個資料夾 |
| `--approve once` | 要核准的事自動允許一次（改 hooks 的、問題卡、交給人做的除外）；`--approve reject`＝預設，自動拒絕 |

- 選項可以放在問題前後；其他字全部接成問題。沒有問題 → 2「要問什麼？例如：anybot --ask --json --bot 小幫手 今天天氣如何」。
- 不連帳號、訊息頻道、例行任務。
- 預設最多等 900 秒（ANYBOT_ASK_SECS）。
- 輸出（給人看）：👤 使用者的話、🤖 Bot 的話（有工具呼叫時截到 300 字）、🔧 工具和參數、↳ 結果（截到 400 字）、「⚠️ 要求核准（自動允許／自動拒絕）：工具 — 原因」、「📚 學到…」、最後「—— N 個工具步驟，M 秒（時間到了，還沒做完）」。
- 輸出（`--json`）：`{"type":"message", role, text, error, tools[{id, name, args}], tool_call_id}`、`{"type":"approval", tool, reason, answer: once|reject}`、`{"type":"learned", skill, description}`、`{"type":"done", ok, code, steps, secs, timed_out, answer}`。文字裡的秘密遮掉。
- 結束碼：0 這一輪順利；1 失敗或時間到；130 按了 Ctrl-C；2 指令寫錯、沒有這個 Bot（「沒有叫「…」的 Bot（有：…）」）、資料夾還沒有 Bot。

#### 2.1.3 `anybot setup`

| 寫法 | 做什麼 |
|---|---|
| `anybot setup 網址 配對碼` | 「加一台電腦」給的那一行：不用回答任何問題；兌換配對碼、加入帳號、當配對碼指定的那台的手腳 |
| `anybot setup` | 終端機互動畫面（沒有終端機 → 1「請用：anybot setup 網址 配對碼…」） |
| `--brain` | 這台當大腦，不當手腳 |
| `--no-service` | 不設開機自動執行 |
| `--real-screen` | 讓 Bot 用這台登入中的桌面（預設：設定時沒有螢幕或是 Wayland，就給 Bot 自己的虛擬螢幕 1440×900） |
| `--name 名字` | 這台在帳號裡的名字（最多 60 字；預設照主機名） |
| `--server 網址 --user 帳號 [--brain-name 名字]` | 給程式用：密碼放在 ANYBOT_ACCOUNT_PASSWORD；`--brain-name` 指定當哪台的手腳，帳號裡沒有就失敗「帳號裡沒有叫「…」的 AnyBot」 |
| `--seed` | 寫入第一次的模型（讀 ANYBOT_PROVIDER、ANYBOT_API_KEY、ANYBOT_BASE_URL、ANYBOT_MODEL、ANYBOT_BOT_NAME）：還沒有 Bot 就建第一個（預設叫「Bot」、模型用該服務建議的），已經有就只寫金鑰；不問問題 |
| `--move-to 新網址` | 帳號伺服器換了網址（換機器、換 VPN）：確認新網址是同一台（認得這台的金鑰）就只換網址，不用密碼、不重新登入；AnyBot 開著時失敗並說怎麼先關掉（或在設定按「換網址」） |
| `--refresh-service` | 更新後重寫開機自動執行的啟動檔並重新啟動（安裝、更新腳本叫的）；沒設開機執行就說「這台沒有設成開機自動執行，不用更新服務。」 |
| `--help`、`-h` | 印說明 |

互動畫面：

| 步驟 | 標題 | 要填、要選 | 按鍵 |
|---|---|---|---|
| 1 | 「把這台電腦接到你的 AnyBot」（`--brain`：「把這台設成大腦」） | 網址、配對碼（6 位數字）；可切換「沒有配對碼？用帳號密碼登入 ›」→ 網址、帳號、密碼（「改用配對碼 ›」切回）；可以直接貼上整行 `anybot setup 網址 配對碼` 或 App 給的整段，自動填好 | Enter 連線、↑↓ 換欄位、Esc 離開 |
| 2 | 「要給哪個大腦用？」（帳號裡有好幾台時） | 每台一行：● 在線上／○ 離線（時間） | ↑↓ 選擇、Enter 確定 |
| 3 | 「正在設定…」 | 依序：儲存設定、開機自動執行、連上帳號伺服器、大腦「名字」 | Ctrl+C 離開 |
| 4 | 「好了」或「沒有完成」 | 提示：試試看的一句話、看狀態、看紀錄、改設定（`anybot setup`）；Linux 提醒 `sudo loginctl enable-linger <使用者>`；Mac 提醒開螢幕錄製、輔助使用；Windows 說「Bot 操控的是你登入中的桌面…」 | Enter 離開 |

- 欄位錯：「配對碼是 6 位數字」「請輸入帳號和密碼」；伺服器的錯照原文顯示（例如配對碼過期）。
- 中途離開：「沒有改任何設定。」結束碼 1。
- 要設開機自動執行時，先停掉在背景跑的那份；寫設定時要拿到資料夾（等 10 秒）；拿不到：「關掉另一個 AnyBot 之後，再執行一次 anybot setup。」
- 結束碼：0 成功；1 失敗；`--seed` 少了 ANYBOT_PROVIDER 或需要的 ANYBOT_API_KEY 是 2。

開機自動執行（`setup` 寫的）：

| 平台 | 寫什麼 |
|---|---|
| macOS | `~/Library/LaunchAgents/com.anybot.serve.plist`：跑 `--serve`、開機執行、掛了重開，紀錄在 `~/Library/Logs/anybot-serve.log` |
| Linux | systemd 使用者服務 `anybot.service`，啟動檔 `~/.local/bin/anybot-run`（帶 ANYBOT_DATA_DIR、手腳要的輔助使用設定、設定時記下的 DISPLAY；沒螢幕時開虛擬螢幕＋openbox）；舊的 `anybot-hands.service` 會被關掉 |
| Windows | 工作排程器的 `AnyBot`，登入時執行（操控的是登入中的桌面） |

### 2.2 `anybot-relay`

| 旗標 | 意思 | 預設 |
|---|---|---|
| `--port N` | 連接埠 | 環境變數 PORT，沒有就 8080 |
| `--bind 位址` | 聽哪個位址 | `0.0.0.0` |
| `--data 資料夾` | 資料放哪 | `./relay-data` |
| `--reset-password 帳號` | 忘記密碼：在伺服器主機上重設（要先停伺服器） | — |

- 不認得的旗標直接忽略（沒有用法說明，也沒有 `--help`）。
- 啟動（標準錯誤輸出）：「AnyBot 帳號伺服器：http://<位址>:<埠>（資料在 …）」；沒有帳號時加一行（§1.4.1）。
- 結束碼：連接埠用不了 1「無法使用連接埠 N：…」；`data.json` 讀不了或壞掉 2；平常一直跑。
- `--reset-password`：
  - 先試連 `127.0.0.1:<埠>`，連得上 → 「帳號伺服器還在執行（埠 N）。先停掉它再重設密碼，之後再啟動。」結束碼 1。只看 127.0.0.1，綁在別的位址時檢查不到（見 Q12）。
  - 帳號不存在 →「<資料夾> 裡沒有帳號「名字」。」1。
  - 問兩次（不顯示）：「「帳號」的新密碼（至少 8 個字）：」「再輸入一次：」；太短「密碼至少 8 個字，沒有改。」、不一樣「兩次不一樣，沒有改。」→ 1。
  - 成功：這帳號全部工作階段登出；印「已改好「帳號」的密碼。啟動帳號伺服器後，用新密碼登入。」和「（帳號備份是用舊密碼加密的，之後要先在電腦上按一次「立刻備份」。）」→ 0。
- Docker 映像：以 `relay` 使用者跑 `anybot-relay --port 8080 --data /data`，資料卷掛在 `/data`。

### 2.3 一鍵安裝

#### 2.3.1 `install.sh`（Mac、Linux）

取得方式：`curl -fsSL <倉庫>/install.sh | sh`、`… | sh -s -- <模式> [參數]`、已經下載原始碼時 `./install.sh [模式]`。在 Git Bash、MSYS、Cygwin 裡跑會自動交給 `install.ps1`。

| 模式 | 做什麼 |
|---|---|
| 不給、`auto` | 有終端機：顯示這台的系統、架構、注意事項和建議，問「選 1/2/3/4，直接 Enter 用建議」（1 桌面 App、2 手腳、3 伺服器、4 只裝編譯需要的）；沒終端機：直接用建議（有桌面→app；Linux 沒桌面→hands） |
| `guide` | 同 auto，但一定問 |
| `app` | 桌面 App：系統套件 → Rust（至少 1.95，用 rustup 裝在 `~/.cargo`，不用 root）→ 下載或更新原始碼 → 編譯 → 安裝 → 第一次設定 |
| `hands [網址 配對碼]`、`headless` | 手腳：同上，Linux 另裝虛擬螢幕、視窗管理員、輔助使用、dbus、中文字型；給了網址和配對碼就接著 `anybot setup 網址 配對碼`；已經裝好 anybot 又給了網址，就跳過編譯直接接上 |
| `server` | 伺服器（只限 Linux）：基本套件、Docker（沒有就用 get.docker.com 裝、設開機啟動、把使用者加進 docker 群組，這次先用 sudo）、原始碼，然後交給伺服器設定腳本（§2.5），不問問題 |
| `deps` | 只裝編譯需要的系統套件和 Rust |
| `--print`、`print` | 不安裝，印一行 JSON（給代理程式）：`ok, os, arch, arch_id, rust_host, distro, distro_id, libc, pm, desktop, headless, wsl, systemd, recommended, server_ok, modes, notes, one_click{app, hands, server, windows, print}` |
| `doctor` | 同上，給人看的文字（發行版、Rust 目標、套件管理員、桌面、systemd、建議、能不能架伺服器、注意事項、一鍵指令） |
| `-h`、`--help`、`help` | 印說明 |

會問的問題（有終端機、沒有 ANYBOT_NONINTERACTIVE、不是 CI 時才問）：

| 時機 | 問題 |
|---|---|
| 模式沒給 | 「選 1/2/3/4，直接 Enter 用建議」 |
| app 裝好後，沒給配對資訊也沒給 ANYBOT_PROVIDER | 「第一次設定（可略過…）」：1 稍後在 App 裡設定、2 貼上 API 金鑰建立第一個 Bot、3 接到大腦當手腳；選 2 → 「服務 openai / anthropic / xai / gemini / openrouter / opencode / ollama / custom：」「API 金鑰（ollama 可直接 Enter）：」（不顯示）；選 3 → 「網址：」「配對碼：」 |
| hands 沒給網址 | 「網址（直接 Enter 先跳過）：」「配對碼：」 |
| 需要時 | 系統的 sudo 密碼；Mac 沒有 Xcode Command Line Tools 時打開安裝視窗，請裝好再跑一次 |

會自動做的事：

- 偵測系統、架構（含 Rosetta、musl、WSL、有沒有桌面、SSH 連進有桌面的機器）、發行版、套件管理員（apt、dnf、yum、pacman、zypper、apk、xbps；NixOS、Gentoo 不自動裝）、systemd。
- 家目錄剩不到 1.5 GB → 停（ANYBOT_IGNORE_DISK=1 硬裝）；不到 4 GB 警告。
- 沒終端機而 sudo 要密碼 → 停，請先 `sudo true` 或用 root 或 ANYBOT_SKIP_PACKAGES=1。
- apt 的套件鎖被占用時每 15 秒重試，最多 8 次。
- 編譯同時幾件：每 2 GB 記憶體（含一半 swap）一件、不超過 CPU 數；不到 3 GB 提醒加 swap。
- 更新來源的簽章：第一次把 `deploy/allowed_signers` 記到 `~/.config/anybot/allowed_signers`，之後原始碼要由它簽過才用（ANYBOT_UNSIGNED=1 跳過一次）。
- 已經在背景跑的 AnyBot 換成新版（`anybot setup --refresh-service`）。
- Mac：打開「隱私權與安全性 › 螢幕錄製」。root 安裝時打開 linger。

裝在哪：

| 平台 | 程式 | 其他 |
|---|---|---|
| macOS | `~/Applications/AnyBot.app`，`~/.local/bin/anybot` 指向它 | 第一次打開按右鍵 › 打開 |
| Linux | `~/.local/bin/anybot` | 選單 `~/.local/share/applications/anybot.desktop`、圖示 |
| 原始碼 | `~/AnyBot`（ANYBOT_DIR） | 再跑一次同一行＝更新 |

結束碼：成功 0；任何失敗 1（「✗ 原因」加下一步怎麼做）；`--print` 支援 0、不支援 1；不認得的模式 1「不認得「…」。可以用：app、hands、server、deps、--print」。

#### 2.3.2 `install.ps1`、`install.cmd`（Windows x64、arm64）

- 用法：`irm <倉庫>/install.ps1 | iex`；`powershell -NoProfile -ExecutionPolicy Bypass -File install.ps1 [模式] [網址] [配對碼]`；`-Print`。`install.cmd` 可以雙擊，參數原樣交給 `install.ps1`。
- 模式：`app`（沒終端機時的預設）、`hands`、`deps`、`auto`／`guide`（互動時問「1 桌面 App、2 手腳、3 只裝編譯需要的」，預設 1）、`--print`／`print`／`-Print`（JSON；`modes` 只有 app、hands、deps）、`doctor`；`server` → 1「Windows 不架帳號伺服器。」；其他 → 1。
- 自動：用 winget 裝 Git、Visual Studio Build Tools（C++）；rustup；原始碼放 `%USERPROFILE%\AnyBot`；編譯；複製到 `%LOCALAPPDATA%\AnyBot\anybot.exe` 並加進使用者 PATH；簽章檢查同 2.3.1（記在 `%USERPROFILE%\.config\anybot\allowed_signers`）。
- 問題：第一次設定同 2.3.1（金鑰用隱藏輸入）；hands 問網址、配對碼。
- 不支援的架構 → 1「不支援的 Windows 架構：…」。

#### 2.3.3 發行包

- Linux 桌面、無螢幕版的 tar.gz 裡的 `install.sh`（沒有參數）：不編譯，放 `~/.local/bin/anybot`、有桌面時加選單和圖示；缺 libasound2 時提醒；最後說「接到你的大腦：在手機或電腦的 AnyBot 按「加一台電腦」，把那一行貼到這裡執行。」
- 伺服器版 tar.gz：用預先編好的程式，走 §2.5 同一支設定腳本。
- Mac：`.app` 的 zip。

### 2.4 Makefile（在原始碼資料夾跑）

| 目標 | 給誰 | 做什麼 |
|---|---|---|
| `make`、`make all`、`make build` | 開發者 | 編譯 AnyBot 和帳號伺服器 |
| `make app`、`make relay` | 開發者 | 只編其中一個 |
| `make run` | 使用者 | 不安裝直接打開 |
| `make install` | 使用者 | Mac → `$(APPDIR)/AnyBot.app`（預設 `~/Applications`）並連到 `~/.local/bin/anybot`；Linux → `~/.local/bin/anybot` 和選單；Windows（MINGW）→ `install.ps1 app` |
| `make uninstall` | 使用者 | 移除程式；「你的 Bot、聊天和設定還在」 |
| `make update` | 使用者 | 這台裝了什麼就更新什麼（§2.6）；`PULL=0` 不拉新版 |
| `make update-app`、`update-hands`、`update-server` | 使用者、管理員 | 只更新那一種（§2.6） |
| `make update-box` | 使用者、管理員 | 請這台的 AnyBot 更新 Bot 的電腦（`anybot --update-box`）；伺服器上：單一帳號裝法進大腦容器跑，否則對每個拿得到 Docker 的雲端大腦跑；不認得旗標時說「裝好的 AnyBot 是舊版：先 make update」 |
| `make migrate` | 管理員（Mac） | §2.7 的搬帳號伺服器 |
| `make deps` | 使用者 | `install.sh deps` |
| `make doctor` | 使用者 | 列出這台缺什麼（git、C 編譯器、Rust、ALSA、Docker、Xcode 工具、是否已編譯、Bot 的電腦的資料卷、快照數） |
| `make setup` | 使用者 | `anybot setup` |
| `make serve` | 使用者 | `anybot --serve` |
| `make hands` | 使用者 | Linux：`hands.sh --deps` |
| `make relay-run` | 開發者 | 在這台跑帳號伺服器（埠 8080，資料在 `./relay-data`） |
| `make server` | 管理員 | `install.sh server` |
| `make server-logs` | 管理員 | `setup.sh --logs` |
| `make test`、`lint`、`lint-sh` | 開發者 | 測試、檢查 |
| `make dist` | 開發者 | 打包發行檔 |
| `make sign-setup` | 作者 | 讓之後的 commit 都簽章（`KEY=公鑰`） |
| `make clean`、`make help` | 開發者 | 清掉建置結果、列出目標 |
| `check-rust` | （內部） | Rust 要 1.95 以上，不然叫你 `make deps` |

### 2.5 伺服器設定（`make server`、`update-server`、`server-logs`）

| 寫法 | 做什麼 |
|---|---|
| `./setup.sh`（＝`make server`） | 第一次：不問問題（簡單裝法）；已經設定過又在終端機：問要不要保留 |
| `./setup.sh --ask` | 改成問問題（網域、一個帳號和它的大腦） |
| `./setup.sh --start`（`make update-server` 用） | 用現有的 `.env` 啟動，不問 |
| `./setup.sh --logs [服務]`（`make server-logs`） | 看紀錄；預設看大腦，簡單裝法看大腦管理員 |
| `./setup.sh --dry-run` | 問完、寫好檔案，但不啟動 |
| 不問問題時給答案 | ANYBOT_SETUP_USER、ANYBOT_SETUP_PASSWORD、ANYBOT_SETUP_DOMAIN、ANYBOT_SETUP_LAN_IP、ANYBOT_SETUP_VOICE（§3.9） |

- 伺服器的設定只放在 `deploy/server/.env`（權限 600）；改了就 `make update-server`。不要直接 `docker compose up`（有兩個 Docker 時可能連到另一個，起一台空的伺服器）。
- 進度列：檢查 → 網址 → 帳號 → 啟動。
- 檢查：Docker 有沒有裝、docker compose、有沒有權限（沒有就教 `sudo usermod -aG docker $USER`）、用哪個 Docker（釘在 `.env` 的 DOCKER_SOCKET）、root 跑的 Docker 提醒、開機自動啟動（systemd 開機啟用；rootless 加 linger）、80／443 埠、記憶體至少 2 GB、硬碟至少 10 GB。
- 已經設定過（有 `.env` 和 Caddyfile）→ 問「這台已經設定過了（網址）」：用現有設定更新並重新啟動／重新設定／加上（或拿掉）語音服務。
- 簡單裝法（第一次、沒 `--ask`、沒給帳號）：
  - 網址：ANYBOT_SETUP_DOMAIN → 有憑證的 Tailscale 名字（https）→ 沒憑證的 tailnet IP（http）→ 區網的私有 IP（自簽 https）；位址是公開的又沒網域 → 停「這台的位址 … 在網路上是公開的：不把帳號用明文開在網路上」，教你給網域或裝 Tailscale。
  - 產生邀請碼、管理員金鑰、代理暗號；寫 `.env` 和 Caddyfile；不建帳號（帳號在 App 註冊，每個帳號自動有雲端大腦）。
- `--ask` 的問題：

| 問題 | 選項、檢查 |
|---|---|
| 「手機和其他電腦用哪個網址連到這台？」 | 依環境：用 Tailscale／Tailscale＋區網 IP／只用區網 IP／用自己的網域；tailnet 沒憑證時：用 tailnet 位址／只用區網 IP／用自己的網域；什麼都沒有：用區網 IP／用自己的網域 |
| 「這台在區網的 IP」 | 要像 10.0.0.25 |
| 網域 | 要像 anybot.example.com；查 DNS：沒指到任何位址 → 「重新輸入／仍然繼續」；指到別台時提醒（在 NAT、雲端主機後面是正常的） |
| 「你的 AnyBot 帳號」 | 帳號（2–64 字，英數和 . _ @ -）、密碼、再一次（至少 8 字、兩次一樣、不能有單引號） |
| 「要在這台架語音服務嗎？」 | 要（第一次下載約 1.6 GB 模型）／先不要 |

- 要問卻沒有終端機 → 1「這次要回答問題（…）：請在終端機執行…」。
- 啟動：有 git 就先拉新版；建置（第一次約 5–15 分鐘）；停大腦管理員 → 啟動；用 Tailscale 時設好 `tailscale serve`；等大腦管理員就緒（或單一帳號的大腦登入）；檢查 `https://<網域>/api/hello`、區網的 https（用 Caddy 的根憑證）、http 位址。
- 其他自動做的：之前用 sudo 留下的 root 檔案改回給你（必要時問密碼）；有 ANYBOT_LAN_IP 時設 `net.ipv4.ip_nonlocal_bind=1`（`/etc/sysctl.d/90-anybot.conf`），開機時網卡還沒好也綁得上；依設定寫 COMPOSE_PROFILES（owner-brain、voice）。
- 完成畫面：網址、邀請碼（含「先自己註冊」「只給你信任的人：每個帳號的雲端大腦能在這台伺服器的網路裡跑程式」）、電腦怎麼接、手機怎麼開（可以「加入主畫面」）、信任憑證的步驟（iPhone、Mac、Linux）、root Docker 對外公開時的安全提醒（建議 rootless）、開機會不會自己起來、AI 金鑰在 App 的 設定 › 模型 設、看紀錄、更新、改設定。
- 結束碼：檢查失敗 1；`--dry-run` 0；`--start`：大腦（或大腦管理員）起來 0，否則 1；完成 0。
- 容器組成（舊版，新版重做，B16 軟契約）：帳號伺服器、Caddy、單一帳號的大腦（`owner-brain`）、大腦管理員（`brains`）、語音服務（`voice`，speaches，沒有對外埠）、Docker API 過濾代理（`docker-socket-proxy`）；網路 `docker-api`（內部，只有大腦、管理員、代理）、`manager`（內部，只有帳號伺服器和管理員）；資料卷 `relay-data`、`caddy-data`、`anybot-data`、`brains-data`、`voice-models`；每個雲端大腦另有自己的容器 `anybot-brain-<帳號>` 和同名資料卷（`01` D）。

### 2.6 更新（`make update…`、`deploy/update.sh`）

- 寫法：`sh deploy/update.sh [auto|app|hands|server]`；`PULL=0` 不拉新版，裝現在這份原始碼；`-h`／`--help`／`help` 印說明；其他 → 1「不認得「…」。」
- `auto`：有伺服器的 `.env` → 更新伺服器；Mac 有 `~/Applications/AnyBot.app` → App；Linux 有 `~/.local/bin/anybot` → 手腳（啟動檔是手腳的）或 App；都沒有 → 1「這台還沒裝 AnyBot，沒有東西可以更新。」
- 拉新版：有釘簽章時先抓、驗新版簽章再合併（不過 → 1「新版沒有通過簽章檢查，沒有更新。」，ANYBOT_UNSIGNED=1 跳過）；只快轉合併，衝突 → 1；列出新的 commit（最多 10 個）；更新腳本自己改了就換成新版繼續。
- App：編譯並安裝（紀錄在 `target/update-build.log`）；有背景服務就重新啟動；視窗開著 → 提醒「AnyBot 開著：結束再打開，才會換成新版」。
- 手腳（Linux）：同上，重啟後 3 秒檢查服務在跑「手腳在跑，大腦看得到這台」，沒起來 → 1 並教看 `journalctl --user -u anybot -n 50`。Mac 的手腳就是 App。
- 伺服器：用 `.env` 的 DOCKER_SOCKET，必要時用 sudo；跑 `setup.sh --start`。
- 完成：「更新好了。你的 Bot、聊天和設定都還在。」0。Windows：再跑一次 `install.ps1`。

### 2.7 其他腳本與檔案

| 檔案 | 寫法 | 做什麼 |
|---|---|---|
| `deploy/linux/install.sh` | `[--deps \| --deps-only \| --rebuild]` | 從原始碼裝到 `~/.local/bin`、選單、圖示；`--deps` 先裝套件、`--deps-only` 只裝套件、`--rebuild` 重新編譯；Wayland 時說明第一次要按「允許遠端操作」「分享」 |
| `deploy/linux/hands.sh` | `[--deps \| --stop]` | 裝好後開 `anybot setup`；`--deps` 裝虛擬螢幕等；`--stop` 關掉並移除 `anybot`、`anybot-hands` 服務 |
| `deploy/migrate-relay.sh`（`make migrate`） | 不帶參數，只在 Mac | 把帳號伺服器搬到大腦那台：問 SSH 目標（必要時裝 SSH 金鑰、Rust、Tailscale）；在那台編譯、用 systemd 使用者服務跑在 `127.0.0.1:8780`；問要不要開 `https://<名字>:8443`（[Y/n]）；問密碼兩次、在新伺服器建帳號（已存在就驗密碼）；叫大腦改接；改這台 App 的網址（先備份 `state.json.pre-migrate.bak`、重開 App）；印收尾步驟和怎麼退回。可以重跑，已經做過的步驟會跳過 |
| `deploy/sign-setup.sh`（`make sign-setup`） | `KEY=公鑰` | 作者用：之後的 commit 都簽章，公鑰寫進 `deploy/allowed_signers` |
| `deploy/mac/com.anybot.relay.plist` | 照檔頭說明手動放進 LaunchAgents | Mac 上跑帳號伺服器（`127.0.0.1:8780`，搭 `tailscale serve`），裡面寫了 ANYBOT_RELAY_PUBLIC_URL |
| `deploy/mac/com.anybot.serve.plist` | 同上 | Mac 不開視窗跑 `--serve`（要先在視窗裡登入帳號，兩者不能同時跑） |
| `deploy/mac/gvisor-vm.sh` | `[--rm]` | Mac 上建 Lima VM＋Docker＋gVisor（強隔離）；之後在 設定 › 電腦 填 Docker 主機 `ssh://lima-anybot-gvisor`、容器隔離選「強隔離（gVisor）」 |
| `deploy/mac/gvisor-migrate.sh` | `[--check]` | 把 Bot 的電腦的家目錄資料卷整個搬到那台 VM（本機那份保留當備份）；`--check` 只比對 |

---

## 3. 環境變數

### 3.0 讀法

- 「誰設」用五類：**使用者**、**管理員**、**安裝腳本**、**Docker compose**、**測試或除錯用**。有好幾個來源時，第一個是統計用的主要來源。伺服器上的「大腦管理員」程式替每個雲端大腦設的，歸在 Docker compose。
- 「建議」三種：
  - 【安裝】＝轉成安裝時的設定（安裝程式問、或當安裝程式的參數，寫進資料庫）。
  - 【畫面】＝轉成設定畫面裡的設定。
  - 【不保留】＝不保留，再分三種原因：**測**（測試、除錯、開發用）、**架**（新架構沒有這個角色，`02`）、**建**（建置、容器細節，新版重做）。
- 「秘密」標「是」的，遷移和紀錄都要當秘密處理。
- 依決定 7，標【安裝】【畫面】的，舊版使用者或管理員設過的值，遷移時轉進新版的設定。

### 3.1 帳號伺服器讀的

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_RELAY_INVITE | 邀請碼（§1.4.1） | 空白 | 管理員（`.env`）；簡單裝法由安裝腳本產生 | 是 | 【安裝】 | App 的 `--serve` 建帳號時也讀（§3.2） |
| ANYBOT_RELAY_PUBLIC_URL | 對外網址：發布網站時回傳完整網址；安裝腳本也拿它決定 Tailscale 的 HTTPS 埠和做連線檢查 | 空白（回傳的 `url` 是空的） | 安裝腳本 | 否 | 【安裝】 | |
| ANYBOT_RELAY_CONTACT | 推播服務要的聯絡方式，`mailto:` 或 `https://` 開頭才算 | `mailto:anybot-relay@example.com` | 管理員 | 否 | 【安裝】 | 安裝腳本沒問，大多數伺服器用的是假信箱 |
| ANYBOT_RELAY_MANAGER_TOKEN | 大腦管理員的金鑰；不到 32 字當作沒設（R51、R52 一律 404） | 空白 | 安裝腳本（產生 64 個十六進位字元） | 是 | 【不保留·架】 | 新版沒有每帳號一台雲端大腦（`02` §10）；大腦管理員也讀它 |
| ANYBOT_RELAY_PROXY_SECRET | 只相信帶這個暗號的代理說的來源位址（§1.8） | 空白（不檢查暗號） | Docker compose（由 `.env` 的 ANYBOT_PROXY_SECRET 帶入） | 是 | 【安裝】 | 對應 `02` §5.4「只相信安裝時設定的反向代理」 |
| RELAY_TRUST_PROXY | `1`／`true`／`yes`：相信 `X-Forwarded-For` 的最後一個值 | 不信 | Docker compose（固定 `1`） | 否 | 【安裝】 | 同上 |
| PORT | 連接埠（沒給 `--port` 時） | 8080 | 管理員 | 否 | 【安裝】 | |

### 3.2 AnyBot 不開視窗（`--serve`）啟動時讀的

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_ACCOUNT | `網址\|帳號`：加入這個帳號 | 沒設就不加入 | Docker compose（`http://relay:8080\|<ANYBOT_USER>`）；大腦管理員給每個雲端大腦；管理員 | 否 | 【安裝】 | 已經有主機金鑰就不再用 |
| ANYBOT_ACCOUNT_PASSWORD | 上面那個帳號的密碼 | — | Docker compose（`.env` 的 ANYBOT_PASSWORD）；管理員；`anybot setup --server`；搬帳號伺服器的腳本 | 是 | 【安裝】 | 只在第一次加入時用；Bot 跑的指令、hooks、連接器的環境裡會拿掉它 |
| ANYBOT_ACCOUNT_CODE | 帳號開了兩步驟驗證時用的恢復碼（用一次就沒了） | — | 管理員 | 是 | 【安裝】 | 一次性 |
| ANYBOT_ACCOUNT_CREATE | `1`：帳號不存在就建（帶 ANYBOT_RELAY_INVITE），成功印「已建立帳號」 | 不建 | Docker compose（固定 `1`） | 否 | 【安裝】 | |
| ANYBOT_RESTORE_FROM_ACCOUNT | `1`：第一次加入帳號後，從帳號最新的加密備份接手 Bot、聊天、例行任務、金鑰；做過就在資料夾留 `restored-from-account`，不再做 | 不做 | 管理員（`.env`） | 否 | 【安裝】 | 見 Q7（決定 5） |
| ANYBOT_HOST_NAME | 這台在帳號裡的名字 | 照主機名 | Docker compose（預設「雲端 AnyBot」）；大腦管理員（「雲端大腦」）；管理員 | 否 | 【安裝】 | |
| ANYBOT_HOST_ID | 大腦管理員代開的主機編號（跟下一個一起有，就不用密碼加入，而且蓋過資料夾裡存的登入） | — | Docker compose（大腦管理員建容器時給） | 否 | 【不保留·架】 | |
| ANYBOT_HOST_TOKEN | 同上的主機金鑰 | — | Docker compose（大腦管理員） | 是 | 【不保留·架】 | |
| ANYBOT_PHONE_PASSWORD | 手機直接連這台（不經帳號伺服器）的密碼，至少 8 字；改了會登出已連的手機，並打開手機直接連線 | — | 使用者（README、Mac 的 plist 範例教的） | 是 | 【不保留·架】 | `02` A13 一律有伺服器；見 Q10 |
| ANYBOT_PHONE_BIND | 手機頁面聽哪個位址；不是位址就報錯、不開 | 所有位址 | 管理員 | 否 | 【不保留·架】 | 新版伺服器聽哪個位址在安裝時選（`02` §5.4）；見 Q10 |
| ANYBOT_PHONE_PORT | 手機頁面的起始連接埠 | 8793 | 管理員 | 否 | 【不保留·架】 | 見 Q10 |
| ANYBOT_DEVICE_LINK | `1`：打開「讓大腦的 Bot 用這台」（裝置連結） | 不改 | 管理員 | 否 | 【畫面】 | |
| ANYBOT_BRAIN | 大腦的名字：這台改顯示那台的 Bot、自己當手腳（每 20 秒找一次） | — | 管理員 | 否 | 【不保留·架】 | 新版沒有「大腦／螢幕」之分（`02` §1） |
| ANYBOT_VOICE_URL | 這台伺服器的語音服務網址；啟動時填進 設定 › 語音 一次，之後以使用者改的為準 | — | 安裝腳本（`http://voice:8000/v1`） | 否 | 【安裝】 | |
| ANYBOT_VOICE_LISTEN_MODEL | 聽寫的模型；啟動時請語音服務先下載 | 安裝腳本給 `deepdml/faster-whisper-large-v3-turbo-ct2` | 安裝腳本 | 否 | 【安裝】 | |
| ANYBOT_VOICE_SPEAK_MODEL | 朗讀的模型 | — | 管理員 | 否 | 【畫面】 | |
| ANYBOT_VOICE_VOICE | 朗讀的聲音 | — | 管理員 | 否 | 【畫面】 | |

### 3.3 AnyBot 一般會讀的

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_DATA_DIR | 資料夾的位置 | macOS `~/Library/Application Support/AnyBot`；Linux `~/.local/share/AnyBot`（有 XDG_DATA_HOME 就在那下面）；Windows `%APPDATA%\AnyBot` | 使用者；伺服器映像固定 `/data`；`anybot setup` 把它寫進開機啟動檔；`--ask --data-dir` 也是設它 | 否 | 【安裝】 | 新版改成命令列 `--data-dir`（`02` §6），所有跑法都要認（Q11） |
| ANYBOT_LOG | 紀錄多詳細：`off`、`error`、`warn`（`warning`）、`info`、`debug`、`trace`；看不懂當 `info`；別的元件只記 warn 以上 | `info` | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_HF_BASE | 記憶搜尋的本機模型從哪下載（Hugging Face 或它的鏡像，例如 hf-mirror.com） | `https://huggingface.co` | 管理員（`.env`） | 否 | 【畫面】 | 大腦管理員會原樣傳給每個雲端大腦 |
| XAI_API_KEY | 模型金鑰的後備：設定裡沒存金鑰才用，而且只送到該服務自己的網址 | — | 使用者；Docker compose（`.env`） | 是 | 【畫面】 | 遷移：設定沒金鑰而環境有，轉進新版的金鑰（`02` A9） |
| OPENAI_API_KEY | 同上 | — | 使用者；Docker compose | 是 | 【畫面】 | 同上 |
| ANTHROPIC_API_KEY | 同上 | — | 使用者；Docker compose | 是 | 【畫面】 | 同上 |
| GEMINI_API_KEY | 同上 | — | 使用者；Docker compose | 是 | 【畫面】 | 同上 |
| OPENROUTER_API_KEY | 同上 | — | 使用者 | 是 | 【畫面】 | 同上 |
| OPENCODE_API_KEY | 同上 | — | 使用者 | 是 | 【畫面】 | 同上 |
| ANYBOT_VAULT_DIR | 保險庫檔放哪；同時加進 Bot 不能讀寫的地方 | 資料夾 | 測試或除錯用 | 否 | 【不保留·測】 | Q6 |
| ANYBOT_VAULT_KEY_FILE | 保險庫的鑰匙改放在這個檔，不用系統鑰匙圈；同時加進 Bot 不能讀寫的地方 | 系統鑰匙圈 | 測試或除錯用 | 是（指向鑰匙） | 【不保留·測】 | Q6 |
| WGPU_POWER_PREF | 圖形卡偏好；有設就照它，沒設 AnyBot 用省電的內建顯卡（雙顯卡 Mac 不發熱） | 省電 | 測試或除錯用 | 否 | 【不保留·測】 | |

### 3.4 第一次的模型設定（`anybot setup --seed`）讀的

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_PROVIDER | 模型服務：openai、chatgpt、anthropic、xai、gemini、openrouter、opencode、ollama、custom | 沒設結束碼 2 | 安裝腳本（問完傳過來）；也可以使用者、代理程式自己給 | 否 | 【安裝】 | chatgpt 不用金鑰，裝好再到設定登入；安裝腳本看到它就自動跑 `--seed` |
| ANYBOT_API_KEY | 金鑰（需要金鑰的服務沒給 → 2） | — | 安裝腳本 | 是 | 【安裝】 | |
| ANYBOT_BASE_URL | 服務網址（ollama、自訂服務） | 服務的預設 | 使用者 | 否 | 【安裝】 | |
| ANYBOT_MODEL | 預設模型，也是第一個 Bot 的模型 | 該服務建議的模型 | 使用者 | 否 | 【安裝】 | |
| ANYBOT_BOT_NAME | 第一個 Bot 的名字 | 「Bot」 | 使用者 | 否 | 【安裝】 | |

### 3.5 問一輪（`--ask`）讀的

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_ASK_SECS | 最多等這一輪幾秒 | 900 | 測試或除錯用 | 否 | 【不保留·測】 | 新版若保留「問一輪」，改成命令列旗標 |
| ANYBOT_ASK_FULL | 印出完整文字，不截短 | 截短 | 測試或除錯用 | 否 | 【不保留·測】 | 同上 |
| ANYBOT_ASK_HERE | 讓 `--ask` 也能用這台電腦（試畫面步驟） | 不行 | 測試或除錯用 | 否 | 【不保留·測】 | |

### 3.6 大腦管理員（`--brains`）讀的

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_RELAY_INTERNAL | 管理員連帳號伺服器的網址（自己的網路） | `http://relay-manager:8080` | Docker compose | 否 | 【不保留·架】 | |
| ANYBOT_BRAIN_RELAY | 雲端大腦連帳號伺服器的網址 | `http://relay:8080` | Docker compose | 否 | 【不保留·架】 | |
| ANYBOT_BRAIN_IMAGE | 雲端大腦用的映像 | 管理員自己的映像 | 測試或除錯用 | 否 | 【不保留·架】 | |
| ANYBOT_BRAINS_DOCKER | 除了擁有者，還有哪些帳號的雲端大腦拿得到 Docker（逗號分隔） | 只有擁有者 | 管理員 | 否 | 【不保留·架】 | 新版每個使用者都有 Bot 的電腦（`02` A2） |
| ANYBOT_BRAINS_ALL | `1`：管理員第一次啟動之前就有的帳號也開雲端大腦 | 不開 | 管理員 | 否 | 【不保留·架】 | |
| ANYBOT_BRAINS_SKIP | 不開（停了也不重開）雲端大腦的帳號（逗號分隔） | compose 自動放 ANYBOT_USER | Docker compose | 否 | 【不保留·架】 | |
| ANYBOT_BRAINS_MAX | 雲端大腦最多幾個；到了新帳號先不開（「雲端大腦已經 N 個，到上限…」） | 20 | 管理員 | 否 | 【不保留·架】 | Q9 |
| ANYBOT_BRAIN_MEMORY | 每個雲端大腦的記憶體上限（docker 的寫法，只收英數），不另給 swap | `4g` | 管理員 | 否 | 【不保留·架】 | Q9 |
| ANYBOT_BRAIN_CPUS | 每個雲端大腦的 CPU（不超過機器有的；Docker 不能限 CPU 時不限） | 2 | 管理員 | 否 | 【不保留·架】 | Q9 |

- 管理員也讀 ANYBOT_RELAY_MANAGER_TOKEN（§3.1；沒有 → 結束碼 2「沒有 ANYBOT_RELAY_MANAGER_TOKEN（32 個字以上，跟帳號伺服器的一樣）」）、HOSTNAME（§3.13）、DOCKER_HOST。
- 它替每個雲端大腦設：ANYBOT_ACCOUNT、ANYBOT_HOST_NAME（「雲端大腦」）、ANYBOT_HOST_ID、ANYBOT_HOST_TOKEN；原樣傳 ANYBOT_VOICE_*、ANYBOT_HF_BASE；拿得到 Docker 的另傳 DOCKER_HOST、ANYBOT_BOX_NAME（`anybot-box-<帳號代號>`）、ANYBOT_BOX_VOLUME（`anybot-box-home-<帳號代號>`）。每個雲端大腦還有固定限制：不能偽造網路封包、不能提權、最多 2048 個程序。

### 3.7 Bot 的電腦、瀏覽器、桌面

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_BOX_NAME | Bot 的電腦的容器名字 | `anybot-box` | Docker compose（伺服器上單一帳號的大腦用 `anybot-brain-box`）；大腦管理員；測試 | 否 | 【不保留·建】 | 遷移要找得到改過名字的舊容器（`01` D）；Q5 |
| ANYBOT_BOX_VOLUME | 它的家目錄資料卷 | `anybot-box-home` | Docker compose（大腦管理員）；測試 | 否 | 【不保留·建】 | 同上 |
| ANYBOT_BOX_IMAGE | 它的映像 | `anybot-box:1` | 測試或除錯用（跑效能測試的一次性電腦） | 否 | 【不保留·測】 | |
| DOCKER_HOST | 用哪個 Docker；設定 › 電腦 的「Docker 主機」空白時用它 | Docker 的預設 | Docker compose（`tcp://docker-socket-proxy:2375`）；使用者 | 否 | 【畫面】 | 「Docker 主機」本來就是設定畫面的一欄 |
| ANYBOT_BROWSER_ENGINE | `firefox`：一定用 Firefox | 自動選 | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_BROWSER_FLAGS | 多給瀏覽器的啟動參數 | — | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_BROWSER_PORT | 「預設」瀏覽器設定檔的除錯埠 | 9333 | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_FIREFOX_HEADLESS | Firefox 不開視窗 | — | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_DESK | `x11`：Linux 一定走 X11 的畫面操作 | 自動判斷 | 測試或除錯用 | 否 | 【不保留·測】 | |

### 3.8 安裝腳本讀的（`install.sh`、`install.ps1`）

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_MODE | 模式（同第一個參數） | 第一個參數，或 auto | 使用者（代理程式） | 否 | 【安裝】 | |
| ANYBOT_NONINTERACTIVE | 有設就不問問題 | 有終端機就問 | 使用者（代理程式） | 否 | 【安裝】 | |
| CI | 有設就不問問題 | — | 測試或除錯用（CI 環境） | 否 | 【安裝】 | 併入「不問問題」 |
| ANYBOT_PAIR_URL | 配對網址：裝完直接 `anybot setup 網址 配對碼` | — | 使用者（「加一台電腦」給的那一行） | 否 | 【安裝】 | |
| ANYBOT_PAIR_CODE | 配對碼 | — | 使用者（同上） | 是（一次性，10 分鐘） | 【安裝】 | |
| ANYBOT_PRINT | `1`：`install.ps1` 只印報告 | — | 使用者 | 否 | 【安裝】 | |
| ANYBOT_SKIP_PACKAGES | `1`：不裝系統套件（NixOS、Gentoo、自己裝好了） | 會裝 | 使用者 | 否 | 【安裝】 | Q4 |
| ANYBOT_IGNORE_DISK | `1`：家目錄不到 1.5 GB 也硬裝 | 會檢查 | 使用者 | 否 | 【不保留·測】 | |
| ANYBOT_UNSIGNED | `1`：這次跳過更新來源的簽章檢查 | 會檢查 | 使用者 | 否 | 【安裝】 | 更新腳本也讀；Q4 |
| ANYBOT_DIR | 原始碼放哪 | `~/AnyBot`（Windows `%USERPROFILE%\AnyBot`） | 使用者 | 否 | 【安裝】 | Q4 |
| ANYBOT_REPO | 原始碼倉庫 | `https://code.30cm.net/daniel.w/AnyBot.git` | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_BRANCH | 分支 | `main` | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_PS1_URL | 在 Git Bash 交棒時去哪下載 `install.ps1` | 倉庫 main 分支的檔 | 測試或除錯用 | 否 | 【不保留·測】 | |
| ANYBOT_INSTALL_URL | `--print` 報告裡一鍵指令用的網址 | 同上 | 測試或除錯用 | 否 | 【不保留·測】 | |

另外安裝腳本也讀 §3.4 的 ANYBOT_PROVIDER、ANYBOT_API_KEY（決定要不要跑 `--seed`），伺服器模式把 §3.9 的 ANYBOT_SETUP_* 傳給設定腳本。

### 3.9 伺服器的 `.env` 與 Docker compose

`.env` 由設定腳本寫（權限 600），是「伺服器的設定唯一放的地方」。下表是只出現在 `.env`／compose 的鍵；`.env` 另外還放 §3.1–§3.6 的 ANYBOT_HOST_NAME、ANYBOT_RESTORE_FROM_ACCOUNT、ANYBOT_RELAY_INVITE、ANYBOT_RELAY_MANAGER_TOKEN、ANYBOT_RELAY_PUBLIC_URL、四個模型金鑰、ANYBOT_HF_BASE、ANYBOT_VOICE_*、ANYBOT_BRAINS_*、ANYBOT_BRAIN_MEMORY、ANYBOT_BRAIN_CPUS。

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| ANYBOT_SETUP_USER | 不問問題時用的帳號（要跟密碼一起給才算） | — | 管理員（腳本、Ansible） | 否 | 【安裝】 | |
| ANYBOT_SETUP_PASSWORD | 它的密碼 | — | 管理員 | 是 | 【安裝】 | |
| ANYBOT_SETUP_DOMAIN | 網域或 IP（100.64.0.0/10 的 IP → tailnet 的 http；其他 IP → 區網自簽 https） | 自己找 | 管理員 | 否 | 【安裝】 | |
| ANYBOT_SETUP_LAN_IP | 有 Tailscale 時另外開區網 IP | — | 管理員 | 否 | 【安裝】 | |
| ANYBOT_SETUP_VOICE | `1`：不問問題時也架語音服務 | 不架 | 管理員 | 否 | 【安裝】 | |
| ANYBOT_USER | 單一帳號裝法的帳號（第一次啟動自動建）；也決定要不要起 `owner-brain`、讓大腦管理員別替它重開 | 簡單裝法沒有 | 安裝腳本（`--ask` 問的） | 否 | 【安裝】 | |
| ANYBOT_PASSWORD | 它的密碼（明文放在 `.env`） | — | 安裝腳本 | 是 | 【安裝】 | 新版不該再存明文 |
| ANYBOT_PROXY_SECRET | 給 Caddy 和帳號伺服器的暗號 | 產生 64 個十六進位字元；用 Tailscale serve 時不設 | 安裝腳本 | 是 | 【安裝】 | |
| ANYBOT_TAILSCALE | `1`：用 Tailscale（不起 Caddy，帳號伺服器只開 `127.0.0.1:8780`） | — | 安裝腳本 | 否 | 【安裝】 | |
| ANYBOT_LAN_IP | 區網 IP：Caddy 只聽這個位址的 80、443 | — | 安裝腳本 | 否 | 【安裝】 | 對應 `02` §5.4「要聽哪個位址」 |
| COMPOSE_FILE | 疊哪幾個 compose 檔（Tailscale、區網） | `docker-compose.yml` | 安裝腳本 | 否 | 【不保留·建】 | |
| COMPOSE_PROFILES | `owner-brain`（單一帳號的大腦）、`voice`（語音服務），手加的照留 | — | 安裝腳本 | 否 | 【不保留·建】 | |
| DOCKER_SOCKET | 這台伺服器用哪個 Docker（rootless 是 `/run/user/<uid>/docker.sock`），第一次設定時釘住 | `/var/run/docker.sock` | 安裝腳本 | 否 | 【安裝】 | |
| ANYBOT_BUILD_JOBS | 編譯同時幾件（每 2 GB 記憶體一件） | 依記憶體 | 安裝腳本 | 否 | 【不保留·建】 | |

compose 寫死的值：帳號伺服器 `RELAY_TRUST_PROXY=1`、`ANYBOT_RELAY_PROXY_SECRET=${ANYBOT_PROXY_SECRET}`；單一帳號的大腦 `ANYBOT_ACCOUNT=http://relay:8080|${ANYBOT_USER}`、`ANYBOT_ACCOUNT_PASSWORD=${ANYBOT_PASSWORD}`、`ANYBOT_ACCOUNT_CREATE=1`、`ANYBOT_BOX_NAME=anybot-brain-box`、`DOCKER_HOST=tcp://docker-socket-proxy:2375`；大腦管理員 `ANYBOT_RELAY_INTERNAL`、`ANYBOT_BRAIN_RELAY`、`ANYBOT_BRAINS_SKIP=${ANYBOT_USER},${ANYBOT_BRAINS_SKIP}`；大腦映像 `ANYBOT_DATA_DIR=/data`。

### 3.10 Makefile、更新腳本

| 名稱 | 意思 | 預設 | 誰設 | 秘密 | 建議 | 備註 |
|---|---|---|---|---|---|---|
| PULL | `0`：不拉新版，裝現在這份原始碼 | `1` | 使用者 | 否 | 【不保留·建】 | |
| ANYBOT_UPDATE_PULLED | 更新腳本換成新版自己時帶的記號 | — | 安裝腳本（更新腳本自己設） | 否 | 【不保留·建】 | |
| ANYBOT_GIT_COMMIT | 版本字串裡的 commit（`-dirty`＝有沒提交的改動） | 問 git | 安裝腳本（Makefile） | 否 | 【不保留·建】 | |
| CARGO_BUILD_JOBS | 編譯同時幾件 | 依記憶體 | 安裝腳本（更新腳本） | 否 | 【不保留·建】 | |
| APPDIR | Mac 的 App 裝在哪 | `~/Applications` | 使用者 | 否 | 【安裝】 | |
| KEY | `sign-setup` 用哪把公鑰 | `~/.ssh/id_ed25519.pub` | 測試或除錯用（作者） | 否 | 【不保留·測】 | |
| CARGO、CARGO_TARGET_DIR、VERSION、MAKE、KEEP_APP（5 個） | 建置工具、輸出位置、版本字串、make 指令、打包時留下 `.app` 不壓縮 | — | 測試或除錯用（開發者） | 否 | 【不保留·建】 | |
| ANYBOT_VERSION | 建置時寫進程式的版本字串（執行時不讀） | — | 測試或除錯用（建置） | 否 | 【不保留·建】 | |

### 3.11 測試、除錯、截圖用（全部【不保留·測】）

| 名稱 | 個數 | 用途 | 秘密 |
|---|---|---|---|
| ANYBOT_SHOT_*：ARGS、CALM、CLICK、DARK、DRAFT、DRAG、EDITOR_COLOR、EVERY、FILE、FIND、FRAMES、GOAL、HOVER、KEYS、LANG、MCP、MCP_PAGE、MCP_TOOLS、MOOD、MOTION、NEWER、NOPOPUP、PAGE、PAGE_BODY、PAGE_JSON、PAL、PANEL、PINS、PLAN、REAL、RESIZE、SCREEN、SCROLL、SHELF、SHELL、SIDEBAR、SIZE、SKILL_PAGE、SPLASH、STATUS、STYLE、SUB、TAB、THEME、TOOL、TYPE、VIEW、ZOOM | 48 | 截圖模式（`--screenshot`）：視窗大小、放大、主題、語言、要開哪頁、點哪、捲到哪…，做文件和畫面測試的圖 | 否 |
| ANYBOT_TEST_*：ADMIN_HOOKS、CERT_OUT、CLIP、OAI_BASE、OAI_LISTEN、OAI_SPEAK、OAI_VOICE、OUT、REAL_BOX、RESTORE、SANDBOX_EXE、TOKEN、XAI_KEY | 13 | 測試：假的管理員 hooks 位置、憑證輸出、語音實測、用真的 Bot 電腦、還原、沙盒程式 | TEST_XAI_KEY 是；TEST_TOKEN 是測試值 |
| ANYBOT_LIVE_BASE、ANYBOT_LIVE_KEY、ANYBOT_LIVE_MODEL、ANYBOT_LIVE_PROMPT | 4 | 對真的模型跑一輪的測試 | LIVE_KEY 是 |
| ANYBOT_DROP_BRAIN、ANYBOT_DROP_FILE、ANYBOT_DROP_STATE、ANYBOT_DROP_TO | 4 | 實際傳檔的測試 | 否 |
| ANYBOT_LOCK_TEST、ANYBOT_LOCK_UPDATE、ANYBOT_MENU_PID、ANYBOT_FIXTURES、ANYBOT_DIAGRAM_OUT、ANYBOT_DUMP_AVATARS、ANYBOT_TLS_URL、ANYBOT_RELAY_URL | 8 | 單項測試的輸入或輸出（資料夾鎖、工具清單鎖定、選單、範例檔、圖表、頭像、自簽憑證、真的帳號伺服器） | 否 |
| ANYBOT_MS_GRAPH、ANYBOT_MS_LOGIN、ANYBOT_TELEGRAM_API、ANYBOT_TWILIO_API、ANYBOT_X_API | 5 | 把外部服務換成測試用的假伺服器；預設 `https://graph.microsoft.com/v1.0`、`https://login.microsoftonline.com`、`https://api.telegram.org`、`https://api.twilio.com`、`https://api.x.com` | 否 |
| E2E_APP、E2E_SCREENSHOT、E2E_TYPE | 3 | 遠端端對端測試腳本 | 否 |
| GITHUB_SERVER_URL、GITHUB_REPOSITORY、RELEASE_TOKEN | 3 | 發行流程上傳檔案 | RELEASE_TOKEN 是 |

### 3.12 AnyBot 交給別的程式的（不是讀進來的設定；不列入統計）

| 名稱 | 給誰 | 意思 | 建議 |
|---|---|---|---|
| ANYBOT_HOOK_EVENT | 管理員和使用者的 hooks | 這次是哪個事件 | 照樣提供（hooks 格式不變，決定 6）；Q3 |
| ANYBOT_CONVERSATION_ID、ANYBOT_REQUEST_ID | Bot 跑的指令 | 屬於哪個對話、哪個請求 | Q3 |
| ANYBOT_JOB | Bot 的電腦裡的每個指令 | 標記，停止時找得到它的整棵程序 | 內部，新版自訂 |
| ANYBOT_SCREEN | Bot 的電腦裡的瀏覽器啟動檔 | 開那個 Bot 自己螢幕的瀏覽器（英數、`_`、`-`，最多 48） | 內部 |
| ANYBOT_BOX_SANDBOX | Bot 的電腦 | `1`：打開 Chromium 自己的沙盒（強隔離 gVisor 時） | 內部 |
| ANYBOT_MAX_SCREENS | Bot 的電腦的螢幕腳本 | 最多開幾個螢幕（預設 6），滿了結束碼 3 | 內部 |

另外，Bot 跑的指令、hooks、連接器的環境裡會**拿掉**：XAI_API_KEY、OPENAI_API_KEY、ANTHROPIC_API_KEY、GEMINI_API_KEY、OPENROUTER_API_KEY、ANTHROPIC_AUTH_TOKEN、ANYBOT_ACCOUNT_PASSWORD；Bot 跑的指令和連接器另外拿掉 DYLD_INSERT_LIBRARIES、DYLD_LIBRARY_PATH、DYLD_FRAMEWORK_PATH、LD_PRELOAD、LD_LIBRARY_PATH、BASH_ENV、ENV、PROMPT_COMMAND（對應 `02` §5.2「環境變數裡的金鑰先拿掉」）。Bot 的電腦設成「流量經過這台 Mac」時（管理員政策 AllowBoxTrafficViaThisMac 允許、用本機的 Docker），Bot 的電腦裡新開的指令會帶 HTTP_PROXY／HTTPS_PROXY（大小寫各一）和 NO_PROXY。

### 3.13 作業系統本來就有的（參考；不列入統計）

| 名稱 | 舊版拿來做什麼 |
|---|---|
| HOME、USER、USERPROFILE、LOCALAPPDATA、APPDATA、TMPDIR | 家目錄、使用者名、安裝位置、暫存 |
| XDG_DATA_HOME、XDG_CONFIG_HOME | Linux 資料夾位置；簽章釘選檔的位置 |
| HOSTNAME | 預設的電腦名字；大腦管理員認自己的容器 |
| SHELL、PATH | 跑指令 |
| DISPLAY、WAYLAND_DISPLAY、XAUTHORITY、XDG_SESSION_TYPE、XDG_RUNTIME_DIR、XDG_SESSION_ID | 找畫面、Wayland、通知、systemd 使用者服務 |
| LANGUAGE、LC_ALL、LC_MESSAGES、LANG | Linux 上的介面語言（認不出來用繁體中文） |
| ProgramData | Windows 管理員 hooks 的位置（`%ProgramData%\AnyBot\hooks.json`） |
| PROCESSOR_ARCHITECTURE、PROCESSOR_ARCHITEW6432 | Windows 安裝腳本認架構 |
| WSL_DISTRO_NAME | 安裝腳本認 WSL |
| SUDO_USER | 伺服器設定腳本用 sudo 跑時，把檔案還給原本的人 |
| SSL_CERT_FILE | 只在一個測試裡明確讀（信任區網自簽的根憑證）；TLS 憑證清單是否也會自動讀，Q2 |
| HTTP_PROXY、HTTPS_PROXY、ALL_PROXY、NO_PROXY（含小寫） | 安裝腳本說「在代理後面請設 https_proxy」（給 git、curl）；AnyBot 自己的 HTTP 客戶端會不會自動讀，Q2 |

第三方容器的設定（容器怎麼建新版重做，不列入統計）：docker-socket-proxy 的 CONTAINERS、EXEC、IMAGES、BUILD、GRPC、SESSION、VOLUMES、NETWORKS、INFO、POST、GET（各 `1`，只開大腦管理 Bot 的電腦要的 API）；語音服務的 WHISPER__COMPUTE_TYPE（`int8`）、WHISPER__TTL（`-1`，模型一直載著）；大腦映像的 NPM_CONFIG_CACHE、UV_CACHE_DIR、UV_PYTHON_INSTALL_DIR（連接器下載的東西放在資料卷 `/data/cache/…`，重建不用重抓）；Caddy 讀 ANYBOT_PROXY_SECRET。

---

## 盤點統計

| 項目 | 數量 |
|---|---|
| 帳號伺服器的路由 | **52**（R01–R52）：HTTP 50 條、WebSocket 2 條（R25、R50）。圖示 PNG 6 個檔算 1 條；R01 含 2 個路徑 |
| 　依區塊 | 頁面與公開檔 10、帳號與登入 10、主機與配對 9、團隊 4、推播 7、發布網站 2、備份 2、轉送 3、看畫面 3、大腦管理員 2 |
| 　找不到用戶端呼叫的 | **3**（R16、R22、R33） |
| 長連線訊息 | 主機長連線 **10** 種（W01–W10）＋ WebSocket ping／pong；看畫面的 WebSocket 2 種（二進位＝VNC、文字＝打字）；舊式長輪詢用 R26／R27（欄位同 W02／W03） |
| 推播種類 | 5（need、done、routine、file、test） |
| `anybot` 的跑法 | **17**（含不帶參數、內部沙盒；其中開發用 4 個） |
| 　`--ask` 的選項 | 4 |
| 　`setup` 的寫法與旗標 | 位置參數 1 種＋旗標 11 個（`--brain`、`--no-service`、`--real-screen`、`--name`、`--server`、`--user`、`--brain-name`、`--seed`、`--move-to`、`--refresh-service`、`--help`） |
| `anybot-relay` 的旗標 | **4**（`--port`、`--bind`、`--data`、`--reset-password`） |
| `install.sh` 的模式 | 10（含別名 `headless`、`print`、`help`） |
| `install.ps1` 的模式 | 7（`server` 一律拒絕不算）＋位置參數網址、配對碼 |
| Makefile 目標 | 29（含內部 `check-rust`） |
| 伺服器設定腳本的寫法 | 5（不帶、`--ask`、`--start`、`--logs`、`--dry-run`） |
| 更新腳本的寫法 | 5（`auto`、`app`、`hands`、`server`、`help`） |
| 其他腳本 | 6 支（§2.7）＋ 2 個 Mac 開機設定檔 |
| 環境變數（§3.1–§3.11） | **190** 個 |
| 　依誰設（主要來源） | 使用者 22、管理員 22、安裝腳本 18、Docker compose 14、測試或除錯用 114 |
| 　依建議 | 【安裝】41、【畫面】11、【不保留】138（測 107、架 16、建 15） |
| 　其中秘密 | 23 個（含指向鑰匙的路徑 1 個、測試或發行流程用 4 個） |
| 　ANYBOT_ 開頭 | 168 個名字：主表 161、§3.12「交給別的程式的」7（跟舊程式裡出現過的 ANYBOT_ 名字逐一對過，沒有漏） |
| 不列入統計 | AnyBot 交給別的程式的 7 個（§3.12）、作業系統本來就有的 31 個（§3.13）、第三方容器設定 16 個 |

## 待確認

| # | 問題 |
|---|---|
| Q1 | R16（改密碼）、R22（從手機移除電腦）、R33（退出別人分享的電腦）伺服器有收，但 App 和手機頁面都找不到呼叫的地方：新版要不要有這三個功能、從哪個畫面做。 |
| Q2 | AnyBot 自己的 HTTP 客戶端、TLS 憑證清單，會不會自動讀 HTTP_PROXY／HTTPS_PROXY／ALL_PROXY、SSL_CERT_FILE：沒核對。有的話，用公司代理或自簽憑證的使用者就靠它們，遷移時要轉成設定。 |
| Q3 | ANYBOT_HOOK_EVENT（給 hooks）、ANYBOT_CONVERSATION_ID／ANYBOT_REQUEST_ID（給 Bot 跑的指令）新版要不要照樣提供。決定 6 說管理員 hooks 的位置和格式不變，hooks 腳本可能靠這個變數分辨事件。 |
| Q4 | 新版的安裝方式還是從原始碼編譯嗎？決定 ANYBOT_DIR、ANYBOT_SKIP_PACKAGES、ANYBOT_UNSIGNED、PULL、更新來源簽章釘選要不要留。 |
| Q5 | ANYBOT_BOX_NAME／ANYBOT_BOX_VOLUME：新版不讀環境變數，遷移工具怎麼找到改過名字的舊 Bot 電腦容器和資料卷（讀舊的 `.env`、問使用者、還是列出所有 `anybot-box*`）。 |
| Q6 | ANYBOT_VAULT_DIR／ANYBOT_VAULT_KEY_FILE 是否有真實安裝在用（例如沒有鑰匙圈的伺服器）。有的話，遷移要從那裡讀保險庫和鑰匙。 |
| Q7 | ANYBOT_RESTORE_FROM_ACCOUNT（新伺服器第一次啟動從帳號備份接手）：決定 5 不再讀舊備份，新版還要不要「第一次啟動從帳號接手」這件事。 |
| Q8 | 推播種類：訂閱時給空陣列被當成「全部」，用 R37 設成空的卻是「都不要」。新版要哪一種意思。 |
| Q9 | 大腦管理員的上限（每台雲端大腦記憶體 4g、CPU 2、最多 20 台、哪些帳號拿得到 Docker）：新版的執行服務和每個使用者的 Bot 電腦要不要對應的上限、誰設。 |
| Q10 | 手機直接連 AnyBot（ANYBOT_PHONE_PASSWORD、ANYBOT_PHONE_BIND、ANYBOT_PHONE_PORT，埠 8793）：`02` A13 之後是否整個不要；已設過手機密碼的使用者遷移時怎麼處理。 |
| Q11 | `01` A 說「命令列 `--data-dir` 可以改資料夾」，但舊版只有 `--ask` 認得 `--data-dir`，其他跑法只能用 ANYBOT_DATA_DIR。`01` 那句要改；新版 `--data-dir` 要所有跑法（視窗、無視窗、setup、問一輪）都認。 |
| Q12 | `anybot-relay --reset-password` 只檢查 `127.0.0.1:<埠>` 有沒有在跑，綁在別的位址時檢查不到，可能被還在跑的伺服器蓋回去。新版（設定在資料庫）忘記密碼要怎麼重設。 |
| Q13 | `anybot --serve` 在帳號伺服器那頭的雲端大腦、單一帳號大腦、一般使用者三種用法讀的環境變數混在一起（§3.2）：新版安裝時要問哪些、哪些在桌面 App 自動裝伺服器時（`02` A13）直接決定，需要逐項對。 |
