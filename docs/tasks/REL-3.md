# REL-3：macOS 基本測試裡「編不過才算對」的測試逾時（很小）

`dev` 合進 M7a 之後的那一次推送（CI run `45758210950`），macOS 的「basic tests」有 2 個測試逾時（各 120 秒），發佈因此沒有開始：

- `anybot-foundation secret::tests::test_secret_serialization_is_a_compile_failure`
- `anybot-models::t_models_permit_fail complete_without_permit_does_not_compile`

另外 `iron-laws permit_block::tests::test_iron_laws_permit_mint_is_rejected_by_clippy` 也跑了 30 秒以上。紀錄在主資料夾的 `target/ci-logs/dev-second-failed.log`。

這幾個是「現場編一個違規的小範例、確認它編不過」的測試。GitHub 的 macOS 機器上沒有編譯快取，要從頭編相依套件，所以很慢。它們檢查的是編譯時的規則，三種系統結果一樣；Linux 的完整檢查照樣會跑。

你在工作區 `~/Documents/AnyBotNext-wt/rel3`，分支 `ws/rel-3`，剛從 `dev` 切出來。

## 要做到

1. **定一條命名規則**，讓這一類測試認得出來：例如測試名稱帶 `compile_fail`，或放在固定名字的檔案／模組。照現有的幾個（上面三個、`crates/trust/tests/compilefail` 那一組、其他用 `trybuild` 或現場叫 `cargo`/`rustc`/`clippy` 的測試）整理一次，全部改成符合規則。
2. `.config/nextest.toml` 的 `quick` 設定（macOS、Windows 的基本測試用的）照這條規則排除它們。Linux 的 `make check` 照樣全部跑。
3. `tools/iron-laws` 加一道檢查：測試裡現場叫 `trybuild`、`cargo`、`rustc`、`clippy-driver` 的，名稱一定要符合規則（不符合就擋）。附「塞一個不符合的進去會被擋」的自我測試。
4. `docs/testing.md` 寫這條規則和原因；Windows 的清單文件如果有提到，一起對齊。
5. 不准放寬逾時、不准把測試標成略過。

## 驗證和回報

`make check` 全綠就 commit；`git merge dev`、`make check`；簡短回報（規則是什麼、改了哪幾個測試、iron-laws 怎麼擋）。我會推 `ws/rel-3` 跑預演（這次三種系統的檢查會跑），綠了就合。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長（30 分鐘）。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。不准在這台電腦上跑真的 Docker。遇到「Operation not permitted」是沙箱在擋，不要繞過。
