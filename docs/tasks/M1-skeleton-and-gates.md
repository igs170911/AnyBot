# 里程碑 M1：專案骨架和檢查關卡

對應 `spec/05` §1 的第 1、2 步，和 `spec/07`。先打底：這個里程碑**不寫任何功能邏輯**。

## 要做的

1. **Rust workspace 骨架**（照 `spec/07` §2）
   - `crates/` 下 18 個函式庫套件：foundation、protocol、domain、trust、models、tools、mcp、computers、runner、external-agents、store-server、store-client、sync、bus、blob、channels、server、migrate-old。
   - `apps/` 下 4 個執行檔：server、runner-service、agent、cli。desktop 和 web 留給 M2。
   - 每個套件只放骨架：檔案開頭用註解寫這個套件負責什麼（照 `spec/07` 的說明）；加上 `#![forbid(unsafe_code)]`。`main` 保持極簡，只印版本和 `--help`。
   - 套件之間的依賴照 `spec/07` §3 准許的方向，**只加真的需要的**，不要預先多加。

2. **根目錄的設定**
   - `Cargo.toml`：workspace members、`workspace.dependencies`、`workspace.lints`：clippy 挑一組嚴格的；正式程式碼裡 `unwrap_used`、`expect_used`、`panic` 一律 deny。
   - `rust-toolchain.toml`：鎖一個 stable 版本號，並列出 rustfmt、clippy。
   - `rustfmt.toml`。
   - `clippy.toml`：照 `spec/07` §3 的「用 lint 禁用的寫法」設 disallowed-methods 和 disallowed-types。准許的套件用套件層級的 `allow` 打開。
   - `deny.toml`：授權白名單、漏洞檢查、禁用套件（例如禁用 openssl，一律用 rustls）。

3. **`tools/` 裡的檢查工具**（用 Rust 寫，放進 workspace，都要有單元測試）
   - **depcheck**：讀 `cargo metadata`，照 `spec/07` §3 的表檢查：
     - 每個內部套件只依賴准許的內部套件。
     - domain 不依賴 tokio、sqlx、reqwest 這類碰網路、檔案、資料庫、非同步的套件。
     - server 不依賴 runner、models、tools、computers。
     - `allow(clippy::disallowed_methods)` 只出現在准許的套件。
   - **spec-trace**：讀 `spec/10-behavior/[1-6]-*.md` 表格裡所有 `T-` 開頭的測試編號；掃程式碼裡的 `spec: T-…` 標記。用 `tools/spec-trace/baseline.txt` 記「還沒有測試的編號」，第一次產生時全部放進去。關卡規則：
     - 程式碼裡出現規格沒有的編號 → 擋。
     - 基準檔裡的編號已經有測試了卻還留在基準檔 → 擋（逼基準檔只能變短）。
     - 規格多了新編號，既不在基準檔、也沒有測試 → 擋。
     - 每次印出覆蓋數字：總數、有測試的、還沒有的。

4. **`Makefile`**：`make check` 依序跑下面這些，任何一步失敗就整個失敗：
   1. `cargo fmt --all --check`
   2. `cargo clippy --workspace --all-targets -- -D warnings`
   3. `cargo nextest run --workspace`（沒有 nextest 時退回 `cargo test --workspace`）
   4. `cargo deny check`
   5. `cargo machete`
   6. depcheck
   7. spec-trace

5. **CI**：`.gitea/workflows/ci.yml`，在 CI 上跑 `make check`。

6. **`README.md`**：怎麼跑關卡、各資料夾在做什麼（指向 `spec/07`）。

7. **`.gitignore`**：`target/`、`node_modules/`、`.DS_Store` 這類。

全部做完、`make check` 全綠之後，commit 一次。

## 不要做

- TypeScript 和畫面（M2）。
- 任何功能邏輯、資料庫、API（之後的里程碑）。

## 回報

照 `AGENTS.md` 第四節的格式。
