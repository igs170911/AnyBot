# M7b-speed：上下文編譯器的測試要快（小的）

M7b 已經合進 `dev`，謝謝。合進去之後有一個副作用：`crates/context` 的性質測試很慢，GitHub 上整套 Rust 測試從約 75 秒變成約 514 秒。每一個工作區的每一次 `make check` 都要多等 7 分鐘以上。

GitHub（Linux）上最慢的幾個：

| 測試 | 秒數 |
|---|---|
| `build_tests::t_context_build_1` | 64 |
| `mask_pair_tests::accumulate_margin_between_collapse_events` | 60 |
| `manifest_run_tests::manifest_entries_match_request_blocks` | 60 |
| `budget_prop_tests::shrinking_usable_drops_old_results_then_chat_then_memory` | 59 |
| `mask_prop_tests::budget_shrinks_in_order` | 49 |
| `build_tests::t_context_budget_1` | 44 |

你在工作區 `~/Documents/AnyBotNext-wt/m7b`，分支 `ws/m7b-speed`，剛從 `dev` 切出來。

## 要做到

1. 這個套件的**每一個測試在 GitHub 的 Linux 上 10 秒內跑完**，整個套件加起來 60 秒內。本機量到的時間寫進回報（GitHub 的機器大約比這台 Mac 慢 2 倍）。
2. **帶編號的測試照樣 1,000 組**（那一列寫的數字不准改）；描述性名字的測試可以少一點，但每一種情況都要產生得到。
3. 先量時間花在哪，再改。可能的方向（你判斷）：
   - 每一組產生的內容太大（字串很長、步數很多）：照那一列真正要的範圍產生，不要更大。
   - 同一組裡每一步都從頭組一次，總共是步數的平方：能只驗最後一次的就只驗最後一次；要驗每一步的，步數控制在能觸發那個規則的最小值。
   - 編譯器本身有沒有明顯的浪費（同樣的東西每一步重算）：有就修，但不准改行為；改了產品程式的話，測試要照樣全過。
   - 只有在量過、確定是除錯模式太慢的時候，才可以讓這個套件在測試編譯時開最佳化（根目錄 `Cargo.toml` 為這一個套件設 `opt-level`），寫出前後各多久。
4. 不准放寬 `.config/nextest.toml` 的逾時，不准把測試標成略過，不准拿掉斷言。

## 驗證和回報

`make check` 全綠就 commit；`git merge dev`、`make check`；簡短回報：每個慢的測試改了什麼、前後各幾秒（本機）、整個套件前後各幾秒。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長（30 分鐘）。不要改 `crates/models` 裡的任何檔。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。遇到「Operation not permitted」是沙箱在擋，不要繞過。
