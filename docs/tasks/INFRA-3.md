# INFRA-3：完整檢查先跑快的，測試放最後（很小）

`make check` 的步驟照 `Makefile` 裡 `check:` 那一行的順序跑，現在是：`fmt clippy gate-selftest test deny machete depcheck sqlx-check spec-trace ironlaws …`。完整的 Rust 測試（`test`）排在第四個，後面才是 `depcheck`、`spec-trace`、`ironlaws` 這些幾秒鐘就跑完的檢查。

2026-11-26 M6b-1 的一次 `make check` 跑了 8.9 分鐘，最後在 `depcheck` 失敗；一個 5 秒的檢查，等了整段測試才告訴你。那一段工作 47 分鐘裡，有 36 分鐘在等完整檢查。

你在工作區 `~/Documents/AnyBotNext-wt/infra2`（INFRA-2 用的同一個），分支 `ws/infra-2`。

## 要做到

1. `check:` 的順序改成「先快後慢」：先跑不用跑測試的檢查（格式、clippy、相依套件、iron-laws、規格編號、遷移、版本、語言檔、API 文件和 lint、授權、biome、型別檢查……），最後才跑 Rust 測試、前端測試、建置、e2e。一個快的檢查沒過，`make check` 要在一兩分鐘內就停下來。
2. 先確認 `make` 是照順序一個一個跑前置步驟（沒有 `-j`、沒有 `MAKEFLAGS` 讓它平行）；如果不是，用別的方法保證順序。
3. `check-fast` 也照同樣的原則排。
4. 不准拿掉任何一個檢查，也不准把檢查從 `check` 移到別的地方；只改順序。改完列出改前改後兩行，確認集合一樣（腳本比對，不是用看的）。
5. `docs/testing.md` 寫一句：`make check` 先跑快的，失敗的時候只重跑那一步（例如 `make depcheck`），全部過了再跑一次完整的。

## 驗證和回報

暫時弄壞一個快的檢查（例如讓 `depcheck` 失敗，不要 commit），量 `make check` 多久停下來，貼秒數，再改回來。`make check` 全綠就 commit，訊息開頭 `INFRA-3：`。`git merge dev`、`make check`；簡短回報。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長（30 分鐘）。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。遇到「Operation not permitted」是沙箱在擋，不要繞過。
