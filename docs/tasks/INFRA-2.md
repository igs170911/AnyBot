# INFRA-2：完整檢查的測試不要單執行緒（小的）

`Makefile` 的 `_test-body` 用 `cargo nextest run --workspace --test-threads 1` 跑全部的測試。這是 M3.5 時為了避開埠衝突加的。後來 `.config/nextest.toml` 已經有測試分組，會撞埠、用 PostgreSQL、改網路、量時間的測試各自在一組裡排隊（`ports`、`pg`、`net`、`timing`，每組 `max-threads = 1`）。

現在完整的 Rust 測試有 720 個，單執行緒在 GitHub 上要 4–5 分鐘；本機好幾個工作區又要輪流等同一把測試鎖（`tools/testlock.sh`），一次完整檢查常常要等 15 分鐘。

你在工作區 `~/Documents/AnyBotNext-wt/infra2`，分支 `ws/infra-2`，剛從 `dev` 切出來。

## 要做到

1. 拿掉 `--test-threads 1`，讓 nextest 照機器的核心數平行跑；會互相干擾的測試靠測試分組排隊，不靠全部單執行緒。
2. **證明穩定**：拿掉之後，連續跑 3 次 `make test`（完整的 Rust 測試）全部通過，沒有一個重跑才過的。有失敗的，找出是哪兩個測試互相干擾（共用的埠、共用的檔案、共用的資料庫、量時間被拖慢），把它們放進對的測試分組，或修掉共用的東西；**不准**放寬逾時、不准略過、不准加重試。三次的總結行貼進回報。
3. 量時間：改之前、改之後各跑一次 `make test`，本機的秒數寫進回報。
4. 測試鎖照留（不同工作區的測試還是會撞同一個固定的埠）。
5. `docs/testing.md` 跟著改：為什麼不需要單執行緒、新的測試要放哪個分組。

## 驗證和回報

`make check` 全綠就 commit；`git merge dev`、`make check`；簡短回報（前後時間、三次的總結行、你放進分組或修掉的測試）。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check`、`make test` 用前景跑、timeout 設長（30 分鐘）。同時有別的 agent 在別的工作區做事，測試會互相等，屬於正常。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。遇到「Operation not permitted」是沙箱在擋，不要繞過。
