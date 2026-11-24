# 06 · 技術選型

> 來源：2026-11-24 使用者的規模目標（**5,000 人同時在線、每人 50 個 Bot、重度使用**），加上 `02` 的架構與 `05` 的開發原則。使用者授權：「可以就直接用」。
> 用途：新版用什麼語言、框架、資料庫、部署方式，以及為什麼。
> 互查：`02-architecture.md`（因為這份修訂了 A10–A12）、`05-engineering-principles.md`（E1 由這份回答）、`07-repo-layout.md`（資料夾結構）。

## 0. 規模粗估（假設都寫在旁邊，上線前用壓力測試驗證）

| 項目 | 粗估 | 依據／假設 |
|---|---|---|
| Bot 總數 | 250,000 | 5,000 人 × 50 個 |
| 同時在跑的回合 | 25,000–50,000 | 假設尖峰時 10–20% 的 Bot 在工作 |
| 寫資料庫 | 每秒 2,500–10,000 筆 | **只在每個步驟完成時寫一筆**（大約每 5–10 秒一步）；Bot 正在打的字只走即時通道，不寫資料庫 |
| SSE 連線 | 1–2 萬條 | 每人 1–3 台裝置開著，加上執行端 |
| 同時對模型的串流 | 25,000–50,000 | **真正的瓶頸是供應商的速率限制和費用**，不是我們的伺服器 |
| Bot 的電腦 | 尖峰上萬個容器 | 每人 50 個重度使用的 Bot，一台容器撐不住，要按需要加開（A2） |

**結論**：
- 伺服器 API 和資料庫：用 PostgreSQL ＋ 多個 API 副本，撐得住。
- 執行服務：做成無狀態的工作者，水平擴充。
- **成本大頭是 Bot 的電腦**：改成閒置休眠（S9）。第一階段用 Docker；量大了再換 Kubernetes，只要換電腦供應者的實作。
- 模型：每個使用者要有同時回合數的上限，避免打爆供應商的速率限制。

## 1. 因為規模而修訂的決定

| 原決定 | 修訂成 | 為什麼 |
|---|---|---|
| A10「全部用 SQLite」 | **伺服器用 PostgreSQL（加 pgvector、pg_trgm）；客戶端和執行端的本機資料用 SQLite** | 每秒上萬筆寫入、多個 API 副本共用一個資料庫，SQLite 單機做不到。個人部署本來就跑在 Docker 裡（A13），多開一個 PostgreSQL 容器成本很低，**所以全部部署都用 PostgreSQL，只有一套 SQL** |
| A12「HA 先做可恢復（SQLite＋Litestream）」 | **第一階段**：PostgreSQL 定時備份＋持續備份，可恢復。**之後要擴充時**：API 多副本＋PostgreSQL 高可用＋NATS 叢集 | 5,000 人同時在線時不能停幾分鐘，但第一階段先求能恢復 |

**A11「伺服器全部 Docker」維持不變**：使用者決定（2026-11-24）「部署規模先可以擴充就好，一開始先用 docker 不用到 k8s」。第一階段只用 Docker（compose）；Kubernetes 等量真的大了再加。

**架構上保留擴充的路**，之後換實作，上層不用改：
- API 是無狀態的，之後可以多開副本。
- 資料庫一開始就用 PostgreSQL，之後不用換資料庫。
- 通知匯流排、檔案儲存、電腦供應者都是介面：現在用「程式內部／本機檔案／Docker」，之後換成「NATS／S3／Kubernetes」。

`02` 的其他決定不變；A10、A12 已在 `02` 標註修訂。

## 2. 選型總表

### 2.1 語言

| 用在哪 | 選 | 為什麼 |
|---|---|---|
| 伺服器、執行端、使用者電腦上的背景程式、命令列、遷移工具 | **Rust**（stable、edition 2024、`rust-toolchain.toml` 鎖版本） | 型別嚴格、lint 工具成熟（`05`）；上萬條連線和串流用非同步很省資源；團隊已經熟 |
| 所有畫面（手機、瀏覽器、桌面） | **TypeScript**（strict 模式） | 一套畫面三個地方用；可以用 Playwright 自動測試（`05`） |
| Claude Code 轉接器 | **TypeScript** | Claude Agent SDK 只有 TypeScript 和 Python 版 |

### 2.2 伺服器與執行端（Rust）

| 項目 | 選 | 備註 |
|---|---|---|
| 非同步執行環境 | tokio | |
| HTTP 伺服器、SSE | axum（加 tower） | 舊版「自己寫 HTTP 伺服器、全部同步」的紅線不沿用：那是為了不改行為的重構，新版要撐上萬條連線 |
| HTTP 客戶端 | reqwest（rustls） | 呼叫模型供應商、外部服務 |
| 資料庫 | sqlx | **SQL 在編譯時就檢查**（`05`）；同一個套件接 PostgreSQL（伺服器）和 SQLite（客戶端） |
| API 規格 | utoipa 從程式產生 OpenAPI 3.1，產出的 `openapi.yaml` 提交進 repo | CI 比對新舊規格，擋破壞性變更（A14）；TypeScript 客戶端由它產生 |
| 錯誤 | thiserror（函式庫）、anyhow（執行檔） | |
| 紀錄與追蹤 | tracing＋OpenTelemetry；指標用 Prometheus 格式 | |
| 密碼雜湊 | argon2id | |
| 加密 | rustls、ring | 規模部署：信封加密，主鑰匙放 KMS；個人部署：主鑰匙放檔案或系統鑰匙圈 |
| MCP | rmcp（官方 Rust SDK） | 接外部工具，也把 AnyBot 的工具提供給外部 agent（`02` §11） |
| 中文搜尋 | 程式端用 jieba-rs 斷詞＋PostgreSQL 的 pg_trgm | 向量用 pgvector |
| 模型供應商 | 自己寫薄的客戶端：OpenAI 相容、Anthropic Messages API、Gemini 等 | Rust 沒有官方的 Anthropic SDK，官方允許直接用 HTTP 接 |
| 即時通知匯流排 | 一個介面兩種實作：單一 API 副本時在程式內部；多副本時用 **NATS** | 通知只是提示，紀錄在 PostgreSQL（`02` §4.1）。漏掉的通知只會慢一點 |
| 檔案儲存 | 一個介面兩種實作：本機檔案系統（個人部署）、S3 相容儲存（規模部署） | |

### 2.3 使用者電腦上的背景程式（Rust）

| 項目 | 選 | 備註 |
|---|---|---|
| 形式 | 背景服務：macOS 用 launchd、Linux 用 systemd 使用者服務、Windows 用服務 | **視窗關掉也照跑**，主動跟進才做得到（`04`） |
| 本機資料 | SQLite（sqlx）：本機副本、還沒送出的匣子、進行中的回合 | |
| 讀 App 的控件 | vercel-labs/agent-desktop（外部函式庫，鎖版本） | 舊版也用它；這是外部套件，不是舊程式 |
| 平台操作 | macOS：objc2；Linux：x11rb、wayland、ashpd；Windows：windows crate | |
| 指令沙盒 | macOS：sandbox-exec；Linux：Landlock；Windows：沒有沙盒，診斷頁照實說 | |
| 終端機 | portable-pty＋vt100 | |
| 瀏覽器自動化 | Chrome DevTools Protocol 客戶端（例如 chromiumoxide，先評估） | |

### 2.4 畫面（TypeScript）

| 項目 | 選 | 備註 |
|---|---|---|
| 框架 | React＋Vite | 最多人會；OpenBot 也用這套 |
| 資料抓取 | TanStack Query；API 客戶端由 `openapi.yaml` 產生 | |
| 手機 | PWA（含 Service Worker、Web Push） | 不做原生 App（A19） |
| 桌面殼 | **Tauri 2** | 桌面和手機共用同一套畫面。舊版「egui 不換」的紅線不沿用：新版畫面和執行端已經分開（A1），畫面不再需要是純 Rust |
| lint／格式 | Biome | |
| 測試 | Vitest（單元）、Playwright（端對端，桌面和手機一起測） | |

**Tauri 會不會很吃資源（2026-11-24 評估）**

- Tauri 用系統內建的網頁引擎（Mac 的 WKWebView、Windows 的 WebView2、Linux 的 WebKitGTK），不像 Electron 自帶一整個 Chromium。
- **安裝大小**：舊版 egui 的 AnyBot.app 實測 107 MB（字型、手機頁面都編在裡面）；Tauri 大約十幾 MB，中文字型用系統的。
- **CPU**：egui 有動畫就整個視窗重畫。舊版在 2 核、沒有顯示卡的虛擬機上出過事：Bot 工作時每秒 20–30 次整個視窗用 CPU 重畫，畫面一直閃。網頁引擎只重畫有變的區塊，閒置時接近 0。
- **記憶體**：比 egui 多一些（粗估，要實測）。
- **最關鍵**：新版做事的是背景程式（A1），視窗只是畫面。關掉視窗只剩背景程式和選單列圖示，網頁引擎不佔資源。
- **要注意**：
  - Linux 的 WebKitGTK 比較吃資源、顯示比較不一致，要多測。
  - 長聊天只畫看得到的訊息。
  - 不放常駐動畫。
- **資源上限是 CI 關卡**（`05` §2）：視窗閒置 CPU 接近 0、記憶體 200 MB 以內；背景程式閒置 50 MB 以內。
- **退路**：如果哪天要最省，不開桌面視窗，只放選單列圖示，點下去在使用者平常的瀏覽器開畫面。代價是拖檔、全域快捷鍵、原生通知這類整合會少一點。

### 2.5 Bot 的電腦（電腦介面的實作，A18）

| 部署 | 實作 | 隔離 |
|---|---|---|
| 個人／家庭 | Docker | 一般容器；可選 gVisor |
| 多人共用的伺服器（第一階段） | Docker | **gVisor（runsc）一定要開**：不同使用者的容器會在同一台機器上；加上網路隔離，Bot 的電腦連不到伺服器內部的任何東西 |
| 之後量大了 | Kubernetes Pod（gVisor RuntimeClass） | 換電腦供應者的實作就好 |
| 第三方 | Daytona、E2B、Modal 等，各一個轉接器 | 之後再接 |

映像：Debian slim＋Xvfb＋輕量桌面＋Chromium；看畫面先用 noVNC（WebSocket，只在使用中開），之後評估 WebRTC。

### 2.6 部署

| 形態 | 內容 |
|---|---|
| **第一階段：Docker（compose）** | 伺服器、執行服務、PostgreSQL、Bot 的電腦。個人／家庭由桌面 App 或安裝檔自動裝好（A13）；多人共用的伺服器也用同一套，只是 Bot 的電腦要開 gVisor |
| **之後量大了** | Kubernetes＋Helm：伺服器多副本、執行服務多副本、PostgreSQL 高可用、NATS 叢集、S3、Bot 電腦的 Pod 池 |

兩個階段用**同一套映像、同一套程式**，差別只在介面的實作（通知匯流排、檔案儲存、電腦供應者）。

### 2.7 工具鏈與關卡（落實 `05` §2）

| 項目 | 選 |
|---|---|
| Rust 格式與 lint | rustfmt、clippy（`-D warnings`＋`clippy.toml` 的禁用清單） |
| 依賴把關 | cargo-deny（授權、漏洞、禁用套件）、cargo-machete（沒用到的依賴） |
| 依賴方向 | 自己寫的檢查工具，讀 `cargo metadata`（規則見 `07`） |
| 測試 | cargo-nextest、proptest（同步合併的性質測試）、insta（固定輸出比對） |
| 端對端 | Playwright（畫面）＋黑箱 API 測試 |
| 壓力測試 | k6（驗證 §0 的 5,000 人目標） |
| 形式驗證 | TLA+（核准卡片的路徑） |
| CI | Gitea Actions（照舊） |
| 寫程式的 AI 即時回饋 | Claude Code hooks：改檔就跑格式、lint、型別檢查 |

## 3. 舊版「選型紅線」為什麼不沿用

舊版的紅線（egui、同步不用 async、自寫 HTTP 伺服器、rusqlite）是為了「重構時行為零變更」而鎖的。新版是重做，而且目標規模不同：
- 上萬條 SSE 連線和模型串流，需要非同步。
- 畫面和執行端分開了（A1），畫面可以用網頁技術。
- 多副本要共用資料庫，需要 PostgreSQL。

**沿用的外部套件**：rustls、ring、argon2、jieba-rs、agent-desktop、portable-pty、平台相關的 crate、sandbox-exec／Landlock。這些是外部函式庫，不是舊程式碼，沒有污染的問題。

## 4. 已定的決定（2026-11-24）

| # | 決定 |
|---|---|
| S1 | 後端 Rust、畫面 TypeScript；桌面用 Tauri 2，跟手機共用畫面。 |
| S2 | 伺服器資料庫 PostgreSQL（加 pgvector、pg_trgm）；客戶端和執行端本機用 SQLite；都透過 sqlx，SQL 在編譯時檢查。 |
| S3 | axum＋tokio；API 規格由程式產生、提交進 repo、CI 擋破壞性變更。 |
| S4 | **第一階段只用 Docker（compose）**，個人和多人共用的伺服器都是；架構保留擴充的路，量大了再加 Kubernetes＋Helm，同一套映像。 |
| S5 | 多人共用的伺服器上，Bot 的電腦一律開 gVisor（Docker 用 runsc），再加網路隔離。 |
| S6 | 正在打的字只走即時通道；每個步驟完成才寫一筆進資料庫。 |
| S7 | 每個使用者設同時回合數上限，保護模型供應商的速率限制。 |
| S8 | **兩種經營方式都支援：你經營的雲端服務，和企業自己架。安全預設照「雲端服務、多租戶」最嚴的情況設計**（原 T1）：不同使用者的資料和 Bot 電腦嚴格隔離、金鑰和保險庫用 KMS 信封加密。模型金鑰由誰出、要不要收費，之後另外討論。 |
| S9 | **Bot 的電腦閒置就休眠，要用時幾秒內恢復**（原 T2，修訂 `02` A2 的「一直開機」）：有事就開；閒置一段時間就存檔休眠；例行任務到點前先叫醒。使用者感覺不到，成本少很多。 |

| S10 | **第一階段只用使用者自己的模型金鑰**（自己的 API 金鑰或 ChatGPT 帳號）；收費之後再談（原 T3，`09` K5）。 |
| S11 | **每個使用者的資源上限由管理員在管理頁設，有預設值**（照壓力測試結果定），可以針對個別使用者調整（`09` K7）。 |
| S12 | **發佈做好、簽章過的 Docker 映像和桌面安裝檔**，不再從原始碼編譯（`09` K8）。 |
| S13 | **代理伺服器在設定裡設，沒設就照作業系統的代理設定**；自簽憑證也能在設定裡加（`09` K9）。 |
| S14 | **第一版平台**：Bot 操作使用者自己的電腦（螢幕、點擊、瀏覽器、App）支援 **Bot 的電腦＋Mac＋Linux**；Windows 的螢幕操作和瀏覽器延後，在電腦的能力宣告裡照實寫「不支援」。聊天、檔案、指令三個平台都有（`09` K38）。 |
| S15 | **ChatGPT 方案留**：每個人自己登入，權杖加密存伺服器；經營雲端服務前先確認 OpenAI 的條款（`09` K45）。 |

## 5. 待決定

（T1、T2、T3 已在 2026-11-24 決定，見 S8–S10。）
