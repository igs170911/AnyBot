# M1 審查後要修的地方

審查的人重跑了 `make check`（全綠），也檢查了結構和 clean room。下面幾點要修，修完 `make check` 全綠再 commit 一次。

## 必修

1. **spec-trace 把自己的測試資料算成覆蓋了 `T-chat-steer-1`。** 這是假覆蓋，正是 `spec/05` 說的「靠關卡造假」。
   - spec-trace 只掃產品程式碼：`crates/`、`apps/`、`tests/`、`web/`、`adapters/`。**不掃 `tools/`。**
   - 標記只認註解裡的 `spec: T-…`，字串裡的不算。
   - 把 `T-chat-steer-1` 放回基準檔：總數 2538、有測試 0、還沒有 2538。
   - `tools/` 裡工程檢查工具的測試**不用標規格編號**（回答你的問題 1）。`spec/10-behavior/` 只管產品行為。
2. **基準檔不存在時，關卡直接失敗**，不要自動產生（回答你的問題 5）。另外給一個明確的指令 `make spec-baseline`，只在人決定要重新產生時跑。
3. **不讀環境變數，任何地方都不准**（`spec/07` §3 剛修正過，先讀新版）。
   - `clippy.toml` 加上 `std::env::vars`、`std::env::vars_os`；改掉「只准 foundation 讀資料庫在哪」那些說明文字。
   - 因為套件層級的 `allow` 分不出是哪一類禁用，**另外在 depcheck 加一個文字檢查**：`crates/`、`apps/` 裡出現 `env::var`、`env::vars`、`env::var_os`、`env::vars_os`，一律擋。
   - foundation 的豁免只留「讀 `--bootstrap` 指定的啟動檔」。
   - 讀命令列參數（`std::env::args`）**可以**（回答你的問題 3）。
4. **執行檔的依賴**：照 `spec/07` §3 新補的「執行檔的依賴」表改 depcheck（回答你的問題 2；你原本寫的「runner-service 只准依賴 protocol」之後會擋到正常的依賴）。
   - 骨架階段的執行檔**不要為了過 `unused_crate_dependencies` 而去假裝用到某個套件**。用不到的依賴就不要宣告。
5. **runner 的准許依賴表**照你寫的就對（回答你的問題 4）。

## 小事

- 安裝工具的紀錄檔不要寫到 `/tmp`，放在 repo 的 `target/`（已經在 `.gitignore`）裡。

## 回報

照 `AGENTS.md` 第四節，另外列出每一點怎麼改的。
