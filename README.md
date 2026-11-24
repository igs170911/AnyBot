# AnyBotNext

AnyBot 新版的 repo。需求規格在 `spec/`（唯一的需求來源），給寫程式 agent 的規則在 `AGENTS.md`。

## 跑檢查關卡

```sh
make check
```

依序跑（任何一步失敗就整個失敗）：

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`（含 `clippy.toml` 的禁用清單）
3. `cargo nextest run --workspace`（沒裝 nextest 時退回 `cargo test --workspace`）
4. `cargo deny check`（授權、漏洞、禁用套件；`deny.toml`）
5. `cargo machete`（沒在用的依賴）
6. depcheck（`cargo run -p depcheck`）：依賴方向照 `spec/07` §3
7. spec-trace（`cargo run -p spec-trace`）：規格功能追蹤表（`spec/10-behavior/`＋`tools/spec-trace/baseline.txt`）。只掃產品程式碼（crates/apps/tests/web/adapters，不掃 tools/）、標記只認註解；基準檔缺失＝關卡失敗，重新產生是明確指令 `make spec-baseline`

需要的工具：rustup（版本由 `rust-toolchain.toml` 鎖住）、`cargo-deny`、`cargo-machete`、`cargo-nextest`（自選，可退回 `cargo test`）。

```sh
rustup component add rustfmt clippy
cargo install --locked cargo-deny cargo-machete cargo-nextest
```

CI（`.gitea/workflows/ci.yml`）跑同樣的 `make check`。

## 各資料夾在做什麼

完整說明見 `spec/07-repo-layout.md`：

| 資料夾 | 內容 |
|---|---|
| `spec/` | 唯一的需求來源（規格＋功能追蹤表） |
| `crates/` | 18 個 Rust 函式庫套件，照 `spec/02` 分層；依賴方向由 `tools/depcheck` 擋 |
| `apps/` | 執行檔：server、runner-service、agent、cli（main 極簡）；desktop、web 是 M2 |
| `web/` `adapters/` | TypeScript 畫面與轉接器（M2） |
| `tools/` | 自家檢查工具：depcheck（依賴方向）、spec-trace（功能追蹤表） |
| `api/` | openapi.yaml（由程式產生、提交）＋執行事件協定 schema |
| `migrations/` | server（PostgreSQL）、client（SQLite）的資料庫遷移 |
| `tests/` | 跨套件端對端、壓力測試、遷移固定測資（OLD 最後一版的真實資料） |
| `images/` `deploy/` | Docker 映像與 compose／helm 部署 |
| `docs/` | 給使用者和維運的文件；`docs/tasks/` 是里程碑任務 |
