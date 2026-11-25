# REL-1 審核修正（第 1 輪）

REL-1 我看過了，也把 `ws/rel-1` 推上 GitHub 跑了第一次預演。架構是對的：預演不會發佈任何東西、寫入的權限只在需要的工作上、action 和映像都有釘住、管道和版本字串集中在一個地方。紀錄裡沒有越界的動作。

下面分兩部分：預演實際跑出來的失敗，和我讀程式看到的問題。**全部修完再回報**；修完我再推上去跑下一次預演。失敗的紀錄在你工作區的 `target/ci-logs/`。

先做：`git merge dev`。

## 一、預演跑出來的

第一次預演（`target/ci-logs/rel1-release-1-failed.log`）：`guard` 過了，其餘五個工作全部停在同一步「export compile-time release values」：

```
deploy/release/export-build-env.sh: 6: cannot create .../target/release-build-env: Directory nonexistent
```

runner 上是全新的 checkout，沒有 `target/`。

- 修這一個。
- **然後自己把每一份腳本、每一個工作流程的每一步，用「全新 checkout、什麼資料夾都還沒有」的前提從頭走一遍**：要寫檔的資料夾存不存在、要讀的檔是誰產生的、三種系統的 shell 一不一樣（Windows 上 `sh` 是 Git Bash，路徑和換行不同）。這一次是五個工作都死在第一個自己寫的步驟，後面的步驟一個都還沒被驗到；不要一次預演只修一個錯。
- 在本機能先驗的先驗：把 repo 複製一份到自己工作區的 `target/` 底下（乾淨的、沒有 `target/`），在那裡跑這些腳本。

## 二、讀程式看到的

1. **開發版沒有等檢查全綠就發佈**（`spec/09` K111 ②）。
   - 現在推 `dev` 時，`release.yml` 和 `ci.yml` 是平行的：Linux 的 `check`、Windows、macOS 的基本測試就算失敗，開發版還是會掛上去。
   - 改成：`dev` 的開發版一定在「Linux 全部關卡、Windows、macOS 基本測試」三個都綠了之後才開始包、才發佈。做法你選（例如讓發佈的流程由檢查的流程在最後呼叫），但要是一條看得出先後的流程，不要用「等一下再去查」的輪詢。
2. **正式版的「那個 commit 的檢查是綠的」查錯地方。**
   - 現在查的是 commit 的 statuses；GitHub Actions 的工作產生的是 check runs，不是 statuses，所以永遠查不到，打版本標籤時一定被擋。
   - 改查 check runs，名稱 `check`，結論要是成功。
3. **Release 公開時，整條流程會再跑一次。**
   - `release` 事件進來時，`guard`、`desktop`、`image`、`publish` 都會跑（那時 ref 是標籤，會被當成一次新的正式發佈）。
   - `release` 事件只該跑 `latest` 那一個工作。
4. **手動觸發的 `publish` 選項拿掉。**手動觸發只能預演。要發佈只有兩條路：推 `dev`（開發版）、推 `v*` 標籤（正式版）。
5. **Windows、macOS 的基本測試，預演分支也要跑。**
   - `ci.yml` 的 `build-windows`、`build-macos` 現在只在推 `dev`、`main` 時跑，合進 `dev` 之前完全驗不到；第一次跑就是在 `dev` 上，失敗會讓 `dev` 變紅。
   - 推 `ws/rel-` 開頭的分支時也跑這兩個工作。其他工作分支和 PR 照舊只跑 Linux。
6. **伺服器映像**：
   - 執行階段沒有系統的根憑證：伺服器之後要對外連 https（推播服務這類）會全部失敗。裝上 `ca-certificates`。
   - Dockerfile 一個一個資料夾複製（`crates`、`apps`、`tools`、`tests`、`locales`）。另一個里程碑正在加 `migrations/` 和編譯時要用的查詢快取，到時候這裡會漏。改成複製整個 repo，另外加 `.dockerignore` 排掉不該進去的（`target`、`node_modules`、`.git`、`.local`、`docs` 這類）。`.git` 排掉之後，版本資訊只能從 build 參數來，確認是這樣。
   - arm64 的映像至少要驗到「編得出來、裡面有那個執行檔」。能在 runner 上用模擬跑 `--version` 就跑；不行就寫進回報。
7. **映像名稱不要寫死帳號。**從 repo 的名稱算出來（轉成小寫）。
8. **開發版的說明和 `docs/release.md`** 要寫清楚：開發版頁面是固定的一個、每次會整個換掉；`dev-build` 這個標籤會被移動。

## 這次不用做

- Windows arm64、Linux aarch64 的桌面安裝檔：照你回報的，先不做，`docs/release.md` 寫清楚目前有哪些平台。
- 本機編譯時版本字串的 commit 是 `unknown`：先這樣。

## 驗證和回報

- `make check` 全綠，commit 一次。
- 回報（簡短）：每一項怎麼改的；你自己從頭走一遍時另外找到、修掉的問題（列出來）；還擔心哪裡。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。不准在這台電腦上跑 Docker。
