# 發佈

這篇說明怎麼發一個版本、怎麼預演、開發版在哪裡、出問題怎麼退回。先不簽章（`spec/09` K110）。

工作流程：

| 檔案 | 什麼時候跑 |
|---|---|
| `.github/workflows/ci.yml` | 每次推分支、對 `dev` 的 PR。`dev`、`main`、`ws/rel-*` 另外跑 Windows、macOS。推 `dev` 且這三個都綠，才呼叫發佈流程 |
| `.github/workflows/release.yml` | 由 `ci.yml` 在 `dev` 全綠後呼叫（開發版）；推 `ws/rel-*`（預演）；推 `v*` 標籤（正式版）；手動只能預演；正式版公開之後只移 `latest` |
| `.github/workflows/security.yml` | 每週一，或手動 |
| `.github/workflows/wiki.yml` | `main` 上 `docs/wiki/` 有改動 |

工作流程不准往 `main` 或 `dev` 推 commit。`dev-build` 是標籤，不是分支。

## 開發版在哪裡

開發版只有**固定的一個頁面**。不是每次推 `dev` 開一個新頁。

`dev` 推上去之後，`ci.yml` 先跑 Linux 的 `check`、Windows、macOS 基本測試。三個都綠，才呼叫 `release.yml` 開始打包、發佈。有一個紅，就不打包、不發佈。這是一條看得出先後的流程，不是平行跑完再去問結果。

全綠之後：

1. 把 `dev-build` 這個標籤移到這次的 commit。標籤會被移動，不是新標籤。
2. 刪掉舊的那一頁，再建同一個 `dev-build` 預先發佈頁，把這次的安裝檔整頁換上去。舊檔不留。
3. 把伺服器映像推到 `ghcr.io/` 加上這個 repo 的名稱（轉成小寫），標 `dev` 和該 commit 的 12 字雜湊。名稱不寫死帳號。

頁面在：

https://github.com/igs170911/AnyBot/releases/tag/dev-build

現在下載得到的是空殼。桌面 App 叫 **AnyBot Dev**，識別名稱 `app.anybot.desktop.dev`，跟正式版的資料夾分開。畫面仍是一行「畫面骨架：沒有功能」。伺服器映像只回答 `--version` 和健康檢查。

沒有簽章時系統會警告。放行方式寫在那一頁的說明裡，也寫在 `deploy/release/release-notes.md`。

## 怎麼發一個版本

打版本標籤要使用者說。監督者不能自己打。

1. 確認 `dev` 全綠，而且 `main` 已經停在同一個 commit。
2. 確認三處版本號一致：`Cargo.toml` 的 `[workspace.package]`、`apps/desktop/src-tauri/tauri.conf.json`、`apps/desktop/package.json`、根目錄 `package.json`。`make check` 裡的 `versions` 關卡平常就擋不一致。
3. 在那個 commit 上打標籤，形狀是 `v0.1.0`，或預先發佈 `v0.1.0-rc.1`。標籤必須在 `main` 上，而且那個 commit 的 `check` 是綠的。不過就整個停下，不發佈。
4. 推標籤。`release.yml` 先確認那個 commit 有名稱是 `check`、結論是成功的 check run（GitHub Actions 的工作不會寫 commit status）。對了才編安裝檔和伺服器映像，建 **Release 草稿**。`-rc.N` 標成預先發佈。
5. 打開草稿，對 `SHA256SUMS`，看說明裡的放行步驟還在，再公開。
6. 公開之後（不是草稿、不是 `-rc`），同一個工作流程把映像的 `latest` 移到這個版本。

伺服器映像的標籤是版本號（例如 `0.1.0`）和 commit 短雜湊。`latest` 不在草稿階段移動。

## 怎麼預演

推 `ws/rel-` 開頭的分支。`ci.yml` 會跑 Linux 全部關卡，也跑 Windows、macOS 基本測試。`release.yml` 用預演模式跑：編譯、打包、算雜湊，產物掛在那次執行的附件。預演不等那三個檢查，也不建 Release、不推映像、不動標籤。

也可以在 Actions 頁手動跑 Release。手動只有預演，沒有發佈選項。要發佈只有兩條路：推 `dev`（開發版）、推 `v*` 標籤（正式版）。

## 出問題怎麼退回

開發版：

- 安裝檔：下一次 `dev` 全綠會把固定的那一頁整個換掉，並把 `dev-build` 標籤移到新的 commit。要立刻拿掉，在 GitHub 刪除 `dev-build` 這個 Release（標籤留著也沒有檔案可下）。
- 映像：`dev` 標籤指到壞的那次時，在確認好的舊 commit 上重跑，或等下一次綠的 `dev` 蓋過。不要刪除別人還在用的短雜湊標籤。

正式版：

- 草稿還沒公開：刪除草稿，不要公開。映像若已經推上版本號標籤，在套件頁刪除那個版本標籤，不要移 `latest`。
- 已經公開：不要改寫 `main`，不要刪已經公開的標籤。打下一個修正版本。`latest` 只在下一個非 `-rc` 的 Release 公開後才移動。
- 簽章還沒做。退回不是「把簽章撤掉」，是把下載頁和映像標籤指回你要的那一次。

## 管道

編譯時決定，不是執行時的環境變數。定義在 `crates/foundation/src/release.rs`。

| 管道 | App 名稱 | 識別名稱 | 資料夾結尾 |
|---|---|---|---|
| 正式版 | AnyBot | `app.anybot.desktop` | `app.anybot.desktop` |
| 開發版 | AnyBot Dev | `app.anybot.desktop.dev` | `app.anybot.desktop.dev` |

正式版的資料夾名稱跟這次改動之前一樣。

版本字串是 `v<版本>-<commit> · 建置時間`（`install-n10`）。建置時間是 CI 傳進編譯的 `SOURCE_DATE_EPOCH`，格式 RFC 3339。工作樹有未提交的改動時，本機編譯帶 `-dirty`；CI 拒絕帶 `-dirty` 的描述。本機沒有傳 `ANYBOT_GIT_DESCRIBE` 時，commit 那段是 `unknown`。

伺服器映像的 Dockerfile 複製整個 repo。`.dockerignore` 排掉 `.git`，所以映像裡的版本只能來自三個 build 參數：`ANYBOT_RELEASE_CHANNEL`、`ANYBOT_GIT_DESCRIBE`、`SOURCE_DATE_EPOCH`。執行階段有系統根憑證（`ca-certificates`），程序不使用 root。

## 目前有哪些平台

桌面安裝檔：macOS arm64（`.dmg`）、macOS x86_64（`.dmg`）、Windows x64 NSIS、Linux x86_64 的 `.deb`、`.rpm`、AppImage。

這次沒有：Windows arm64、Linux aarch64 的桌面安裝檔。

伺服器映像：linux/amd64 和 linux/arm64。arm64 在 runner 上用 QEMU 載入映像、確認裡面有 `anybot-server`，並跑 `--version`。本機不跑 Docker。
