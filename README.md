# AnyBotNext

AnyBot 新版的 repo。需求規格在 `spec/`（唯一的需求來源），給寫程式 agent 的規則在 `AGENTS.md`。

## 跑檢查關卡

```sh
make check
```

依序跑（任何一步失敗就整個失敗）：

Rust（M1）：

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`（含 `clippy.toml` 的禁用清單）
3. `cargo nextest run --workspace`（沒裝 nextest 時退回 `cargo test --workspace`）
4. `cargo deny check`（授權、漏洞、禁用套件；`deny.toml`）
5. `cargo machete`（沒在用的依賴）
6. depcheck（`cargo run -p depcheck`）：依賴方向照 `spec/07` §3
7. spec-trace（`cargo run -p spec-trace -- --against HEAD`）：規格功能追蹤表
   （`spec/10-behavior/`＋`tools/spec-trace/baseline.txt`）。只掃產品程式碼
   （crates/apps/tests/web/adapters 的 `.rs`、`.ts`、`.tsx`，不掃 tools/）、標記只認註解；
   基準檔缺失＝關卡失敗，重新產生是明確指令 `make spec-baseline`；
   `--against <ref>` 比對那個 ref 的基準檔，**基準檔只能變短**
   （多出來的編號只准是這段變更新增的規格編號；CI 跟主分支的分岔點比，見 `.gitea/workflows/ci.yml`）

TypeScript（M2）：

8. `pnpm install --frozen-lockfile`
9. `pnpm exec biome ci .`（格式＋lint，`biome.json`；`*.md` 和 `pnpm-lock.yaml` 不掃：
   `spec/` 是需求來源、不能被工具重排，鎖檔是工具產物）
10. `pnpm -r --no-bail typecheck`（各套件 `tsc --noEmit`，`tsconfig.base.json` strict 全開）
11. `pnpm -r --no-bail test`（各套件 `vitest run`）
12. `pnpm -r --no-bail build`（各套件 `vite build`）

Tauri 的 Rust 部分（`apps/desktop/src-tauri`）走上面 1–7 的 Rust 關卡（在 workspace 裡）。

### 邊寫邊跑

```sh
make check-fast
```

只跑最快的四個：`cargo fmt --all --check`、`cargo clippy`、depcheck、`biome ci`。
改完一批就跑，錯了當場修（`spec/05` §1 第 3 步、`AGENTS.md` 二）。

## 需要的工具

Rust：rustup（版本由 `rust-toolchain.toml` 鎖住）、`cargo-deny`、`cargo-machete`、
`cargo-nextest`（自選，可退回 `cargo test`）。

```sh
rustup component add rustfmt clippy
cargo install --locked cargo-deny cargo-machete cargo-nextest
```

TypeScript／桌面：Node 22 以上、pnpm、（要真的開桌面視窗時）`cargo install --locked tauri-cli`。

```sh
npm install -g pnpm
pnpm install            # 裝完會產生 pnpm-lock.yaml（要 commit）
pnpm --filter @anybot/desktop dev   # 需要 tauri-cli：cargo tauri dev
```

CI（`.gitea/workflows/ci.yml`）跑同樣的 `make check`；Linux 上會先裝 Tauri 要的
WebKitGTK 等系統套件，並把 `SPEC_TRACE_REF` 設成跟主分支的分岔點。

## 各資料夾在做什麼

完整說明見 `spec/07-repo-layout.md`：

| 資料夾 | 內容 |
|---|---|
| `spec/` | 唯一的需求來源（規格＋功能追蹤表） |
| `crates/` | 18 個 Rust 函式庫套件，照 `spec/02` 分層；依賴方向由 `tools/depcheck` 擋 |
| `apps/` | 執行檔：server、runner-service、agent、cli（main 極簡）；`desktop/`（Tauri 2：`src-tauri/` 是 Rust 殼、前端載入 `web/ui` 的外殼）、`web/`（PWA：手機和瀏覽器） |
| `web/` | 共用的 TypeScript 套件：`ui/`（共用畫面元件，桌面和手機都用）、`api-client/`（由 `api/openapi.yaml` 產生，見該目錄的 README） |
| `adapters/` | 非 Rust 的轉接器（之後的里程碑） |
| `tools/` | 自家檢查工具：depcheck（依賴方向）、spec-trace（功能追蹤表＋基準檔只能變短） |
| `api/` | openapi.yaml（由程式產生、提交）＋執行事件協定 schema |
| `migrations/` | server（PostgreSQL）、client（SQLite）的資料庫遷移 |
| `tests/` | 跨套件端對端、壓力測試、遷移固定測資（OLD 最後一版的真實資料） |
| `images/` `deploy/` | Docker 映像與 compose／helm 部署 |
| `docs/` | 給使用者和維運的文件；`docs/tasks/` 是里程碑任務 |

根目錄的 `package.json`、`pnpm-workspace.yaml`、`biome.json`、`tsconfig.base.json` 是
TypeScript 工作區的設定；工作區成員是 `web/ui`、`web/api-client`、`apps/web`、`apps/desktop`。

## 桌面殼（`apps/desktop`）

照 `spec/06` §2.4、`spec/07` §3：**視窗只是畫面**，做事的是背景程式 `apps/agent`；
桌面殼不准依賴 server、store-server、runner、models、tools、computers、mcp、external-agents
（由 `tools/depcheck` 擋）。M2 只有骨架：一個視窗顯示 `web/ui` 的外殼，沒有頁面、沒有功能。
