# 基礎建設：讓多個 agent 可以並行

使用者要多個 agent 同時做不同的里程碑。每個 agent 在自己的 git worktree 和分支工作。

這台電腦的硬碟只剩大約 12 GB，`target/` 卻有 40 GB。使用者選擇不清硬碟，讓幾個工作區**共用一份編譯輸出**，最多同時兩條線。

這次要做三件事：
1. 讓編譯輸出瘦身。
2. 讓所有找編譯輸出的地方不假設它在 `target/`。
3. 加一個跨工作區的測試鎖。

做完 `make check` 全綠，commit 一次。

先讀：`AGENTS.md`（第七節「並行工作」是新的）、`docs/testing.md`、`Makefile`、`.config/nextest.toml`。

## 1. 編譯輸出瘦身

- 在根目錄 `Cargo.toml` 調整 profile：
  - 我們自己的套件只留行號除錯資訊，例如 `debug = "line-tables-only"`。panic 和 backtrace 還是要看得到檔名、行號。
  - 第三方套件不帶除錯資訊，例如 `[profile.dev.package."*"] debug = false`。
- 增量編譯的快取現在佔 13 GB。量過「關掉增量編譯」對重新編譯時間的影響再決定，數字寫進回報。
- 量「改之前」和「改之後」兩組數字：
  - `cargo clean` 後跑一次 `make check` 的時間和 `target/` 大小。
  - 改一個小檔後再跑一次 `make check` 的時間。
- 目標是 `target/` 在 12 GB 以內。

## 2. 找編譯輸出的地方不准假設是 `target/`

- 編譯輸出的位置可能被 cargo 設定改到別的資料夾（共用編譯輸出時就是這樣）。所有要找編譯輸出的地方都改成問 cargo，例如 `cargo metadata --format-version 1 --no-deps` 的 `target_directory`。這些地方包括：
  - iron-laws 的「沒被編譯到的檔案」關卡（現在讀 `target/debug/deps` 的 dep-info）。
  - `make e2e` 和 Playwright 設定裡啟動 `testenv` 的路徑。
  - `make live-model-check`。
  - harness 裡找執行檔的地方。
  - Makefile 和其他腳本。
- 測試自己的暫存和輸出，照舊放在**這個工作區自己的** `target/`，不放進共用的編譯輸出。例如 `target/tests/`、`target/e2e-screens/`、`target/live-model/`、`target/pg-install/`。共用的話，兩個工作區的同名測試會互相蓋掉。
- 自我測試：在 `target/` 底下建一個暫時的 git worktree，它的上一層放 `.cargo/config.toml`，設 `[build] target-dir` 指到另一個資料夾。在那個 worktree 跑 `make check` 要全綠，跑完把暫時的 worktree 收掉。這個自我測試的步驟寫進 `docs/testing.md`，不用每次 `make check` 都跑。

## 3. 跨工作區的測試鎖

- 有幾個測試一定要用 8787 埠（伺服器的預設埠就是要測這個）。兩個工作區同時跑 `make check` 會搶埠。
- `make test`、`make test-fast`、`make e2e` 要先拿一個鎖，鎖檔放在這個 repo 所有工作區共用的位置：`git rev-parse --git-common-dir` 那個資料夾裡，例如 `anybot-tests.lock`。
  - macOS 用 `lockf`，Linux 用 `flock`，兩個都沒有就不鎖（CI 一次只跑一份）。
  - 拿不到鎖時印一行說在等哪個鎖，然後等，不要放棄。
- 自我測試：兩個工作區同時跑 `make test-fast`，第二個要等第一個跑完，兩個都要過。

## 4. `docs/testing.md`

加一節「並行工作」，寫清楚：
- worktree 放哪裡。
- 共用編譯輸出怎麼設定。
- 測試鎖怎麼運作。
- 第 2 點的自我測試怎麼跑。

## 這次不做

- 不建正式的工作區、不改 `AGENTS.md`、不碰 repo 以外的資料夾。正式的工作區由監督者建。

## 回報

照 `AGENTS.md` 第六節，逐項對照實際檔案。另外附上：
- 瘦身前後的 `target/` 大小、`make check` 時間、改一個小檔後重新編譯的時間。
- 改了哪些「找編譯輸出」的地方。
- 兩個自我測試的實際輸出。
