# 07 · 資料夾結構

> 來源：2026-11-24 使用者提供的 Rust workspace 範本（`my-large-project`），照 `02` 的分層和 `06` 的選型改寫。
> 用途：新版 repo 的資料夾結構、每個套件負責什麼、套件之間准許的依賴方向。
> 互查：`02-architecture.md`（分層、三個可插拔介面）、`05-engineering-principles.md`（依賴方向用工具擋）、`06-tech-stack.md`。

## 1. 對範本的評估

**範本的骨架照用**：workspace 根、全專案共用一個 `Cargo.lock`、`main.rs` 保持極簡、根目錄的 `tests/` 放跨套件的端對端測試、`migrations/`。

**要改的三件事**：
1. **不要只切成 core／infra／utils 三大塊。** 一個「core」很快又會長成舊版那種什麼都有的巨檔。改成照 `02` 的分層和三個可插拔介面切，每個套件只做一件事。
2. **不要有「utils」雜物箱。** 共用的東西放 `foundation`，而且範圍寫死（見 §3）。
3. **範本缺的要補上**：TypeScript 的畫面、規格（`spec/`）、映像（`images/`）、兩種部署（`deploy/`）、伺服器和客戶端各自的資料庫遷移、自家的檢查工具。

## 2. 結構

```
anybot/
├── Cargo.toml                 # workspace：members、workspace.dependencies、workspace.lints
├── Cargo.lock
├── rust-toolchain.toml        # 鎖 Rust 版本
├── clippy.toml                # 禁用清單（讀環境變數、執行外部程式、發 HTTP…只准在指定套件）
├── deny.toml                  # cargo-deny：授權、漏洞、禁用套件
├── package.json               # pnpm workspace（畫面、轉接器）
├── pnpm-workspace.yaml
├── biome.json
├── README.md
├── .github/workflows/         # CI 和發佈（GitHub Actions，`09` K110）：跑 05 §2 的全部關卡
│
├── spec/                      # 規格（現在的 ClearRoom/spec 搬進來）＋功能追蹤表
├── api/                       # openapi.yaml（由程式產生、提交）、執行事件協定的 schema
│
├── crates/                    # Rust 函式庫
│   ├── foundation/            # 最底層：ID、時間、錯誤、Secret 型別、啟動設定（唯一可以讀「資料庫在哪」的地方）
│   ├── messages/              # 照訊息代碼和語言，把翻譯檔排成字（命令列、伺服器印的訊息；`09` K91、K92）
│   ├── protocol/              # 執行事件協定、API 的請求和回應型別（伺服器、執行端、畫面共用）
│   ├── domain/                # 任務、Bot、對話紀錄、核准規則、證據、同步合併規則。純邏輯，不碰網路和檔案
│   ├── trust/                 # 信任層：把關、留證據、驗收、產生交付報告
│   ├── models/                # 模型供應商介面＋各家實作
│   ├── tools/                 # 工具介面＋內建工具
│   ├── mcp/                   # MCP 客戶端（接外部工具）、MCP 伺服器（提供工具給外部 agent）
│   ├── computers/             # 電腦介面＋實作：local（使用者的電腦）、docker、kubernetes、第三方轉接
│   ├── runner/                # 執行端：Bot 的回合（問模型 → 經過信任層 → 工具）
│   ├── external-agents/       # 外部 agent 的轉接（Rust 這一側；Claude Code 的另一側在 adapters/）
│   ├── store-server/          # 伺服器資料層（PostgreSQL）
│   ├── store-client/          # 本機資料層（SQLite）：副本、匣子、進行中的回合
│   ├── sync/                  # 同步：送出匣子、拿第 N 號之後的、SSE 客戶端
│   ├── bus/                   # 即時通知匯流排：程式內部／NATS
│   ├── blob/                  # 檔案儲存：本機檔案系統／S3
│   ├── channels/              # 聊天頻道轉接：Slack、Discord、Telegram
│   ├── server/                # 伺服器 API（axum）：帳號、信箱、協調、排程、推播、SSE
│   └── migrate-old/           # 舊資料遷移：只讀 OLD 最後一版的格式（01、01a、01b）
│
├── apps/                      # 可以執行、可以部署的東西（main 都保持極簡）
│   ├── server/                # [bin] anybot-server
│   ├── runner-service/        # [bin] 伺服器上的執行服務
│   ├── agent/                 # [bin] 使用者電腦上的背景程式：執行端＋手腳＋同步
│   ├── cli/                   # [bin] anybot：安裝、遷移、診斷
│   ├── desktop/               # Tauri 2：src-tauri/（Rust 殼）＋載入共用畫面
│   └── web/                   # 手機和瀏覽器用的 PWA
│
├── locales/                   # 翻譯檔：en（基準）、zh-TW；Rust 和 TypeScript 共用，建置時打包（`09` K91）
├── web/                       # 共用的 TypeScript 套件
│   ├── ui/                    # 共用畫面元件與頁面（桌面、手機都用）
│   └── api-client/            # 由 api/openapi.yaml 產生的客戶端
│
├── adapters/                  # 非 Rust 的轉接器
│   └── claude-code/           # TypeScript：包 Claude Agent SDK，講 AnyBot 執行事件協定
│
├── images/                    # Dockerfile：server、runner-service、bot-computer
├── deploy/
│   ├── compose/               # 第一階段：個人／家庭（A13）和多人共用的伺服器都用它
│   └── helm/                  # 之後量大了需要 Kubernetes 時再加（第一階段先不做）
├── migrations/
│   ├── server/                # PostgreSQL
│   └── client/                # SQLite
├── tests/
│   ├── e2e/                   # 跨套件、黑箱：API 測試＋Playwright
│   ├── load/                  # k6 壓力測試（5,000 人目標）
│   └── fixtures/old-data/     # OLD 最後一版產生的真實資料（遷移測試用）
├── tools/                     # 自家的檢查工具：依賴方向、功能追蹤表、API 規格 lint
└── docs/                      # 給使用者和維運的文件
```

## 3. 依賴方向（用工具擋，`05` §2）

上層可以用下層，反過來不行。CI 用 `tools/` 裡的檢查讀 `cargo metadata`，不符合就擋。

| 套件 | 准許依賴的內部套件 | 特別禁止 |
|---|---|---|
| `foundation` | 無 | |
| `messages` | 無 | 只做「代碼＋參數＋語言 → 字」；翻譯檔在建置時打包，執行時不讀檔 |
| `protocol` | foundation | |
| `domain` | foundation、protocol | **不准碰網路、檔案、資料庫、非同步執行環境**：純邏輯才好測 |
| `trust` | foundation、protocol、domain | |
| `models`、`tools`、`computers`、`mcp` | foundation、protocol、domain | 彼此不互相依賴 |
| `blob`、`bus` | foundation | |
| `store-server`、`store-client` | foundation、protocol、domain | |
| `sync` | foundation、protocol、store-client | |
| `channels` | foundation、protocol、domain | |
| `runner` | 上面全部，**除了** store-server、server | |
| `external-agents` | foundation、protocol、trust、mcp | |
| `server` | foundation、protocol、domain、store-server、blob、bus、channels | **不准依賴 runner、models、tools、computers**：用 Cargo 就擋住了「伺服器 API 不跑腦」（A1） |
| `migrate-old` | foundation、protocol、domain、store-client | |

**執行服務也是 API 的客戶端**：`apps/runner-service` 和使用者電腦上的 `apps/agent` 一樣，透過伺服器的 API 寫紀錄，不直接連資料庫。這樣伺服器是唯一寫資料庫的地方，伺服器上和使用者電腦上的執行端走同一條路。

**用 lint 禁用的寫法**（`clippy.toml` 的 disallowed-methods，只准在指定套件）：

| 寫法 | 只准在 |
|---|---|
| 讀環境變數 | **新版自己的設定：任何地方都不准**（2026-11-24 修正：`02` A10 和 §6 已定「資料庫在哪」用命令列 `--data-dir`、`--bootstrap`，不讀環境變數）。命令列參數可以讀。**唯一可以讀的是 `migrate-old`**：遷移時讀舊版用過的環境變數，轉進新版的設定、找舊資料夾和資料卷（`00` 決定 7、`migrate-env`、`install-n23` S3；`09` K89）；遷移完就不再讀 |
| 執行外部程式 | `computers`（而且要經過 `trust` 給的通行證） |
| 發 HTTP 請求 | `models`、`mcp`、`sync`、`channels`、`computers` |
| 讀寫任意檔案 | `computers`、`blob`、`store-client`；`foundation` 只准讀 `--bootstrap` 指定的那個啟動檔 |
| 找使用者的家目錄（作業系統的慣例資料夾） | 只有 `foundation`，用來算本機資料夾的預設位置（`09` K85）；不算讀設定 |
| 以上全部（執行外部程式、連網路、讀寫 `target/` 裡的檔案） | 根目錄 `tests/` 底下的測試輔助套件（`09` K87）；產品套件只能把它當開發用依賴 |

**執行檔（`apps/`）的依賴**（2026-11-24 補）：只加真的用到的；而且：

| 執行檔 | 不准依賴 |
|---|---|
| `apps/server` | `runner`、`models`、`tools`、`computers`、`mcp`、`external-agents`（伺服器 API 不跑腦，A1） |
| `apps/runner-service`、`apps/agent` | `server`、`store-server`（它們是 API 的客戶端，不直接碰伺服器資料庫） |
| `apps/cli` | `server`、`store-server` |
| `apps/desktop`（Tauri 殼） | `server`、`store-server`、`runner`、`models`、`tools`、`computers`、`mcp`、`external-agents`（視窗只是畫面，做事的是背景程式 `apps/agent`，`06` §2.4） |

## 4. repo 本身

- **新版開一個新的 git repo**，不帶 OLD 的歷史。OLD 留在原本的 repo，當唯讀的參考。
- `ClearRoom/spec/` 搬進新 repo 的 `spec/`。
- **授權：`MIT OR Apache-2.0`**（`09` K88）。根目錄放 `LICENSE-MIT`、`LICENSE-APACHE`；第三方套件的授權由關卡檢查。
- 「沒有混進舊程式」的黑名單（`05` §2）放在新 repo **外面**，由看過舊程式的一方執行。
