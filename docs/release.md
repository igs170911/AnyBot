# 發佈

這篇說明怎麼發一個版本、怎麼預演、開發版在哪裡、出問題怎麼退回。先不簽章（`spec/09` K110）。

工作流程：

| 檔案 | 什麼時候跑 |
|---|---|
| `.github/workflows/ci.yml` | 每次推分支、對 `dev` 的 PR。`dev` 和 `main` 另外跑 Windows、macOS |
| `.github/workflows/release.yml` | 推 `dev`、推 `ws/rel-*`、推 `v*` 標籤、手動、正式版公開之後 |
| `.github/workflows/security.yml` | 每週一，或手動 |
| `.github/workflows/wiki.yml` | `main` 上 `docs/wiki/` 有改動 |

工作流程不准往 `main` 或 `dev` 推 commit。`dev-build` 是標籤，不是分支。

## 開發版在哪裡

`dev` 推上去、Linux 的 `check` 加上 Windows、macOS 都綠之後，`release.yml` 會：

1. 編桌面安裝檔，掛到固定標籤 `dev-build` 的預先發佈頁。舊檔拿掉。
2. 把伺服器映像推到 `ghcr.io/igs170911/anybot`，標 `dev` 和該 commit 的 12 字雜湊。

頁面在：

https://github.com/igs170911/AnyBot/releases/tag/dev-build

現在下載得到的是空殼。桌面 App 叫 **AnyBot Dev**，識別名稱 `app.anybot.desktop.dev`，跟正式版的資料夾分開。畫面仍是一行「畫面骨架：沒有功能」。伺服器映像只回答 `--version` 和健康檢查。

沒有簽章時系統會警告。放行方式寫在那一頁的說明裡，也寫在 `deploy/release/release-notes.md`。

## 怎麼發一個版本

打版本標籤要使用者說。監督者不能自己打。

1. 確認 `dev` 全綠，而且 `main` 已經停在同一個 commit。
2. 確認三處版本號一致：`Cargo.toml` 的 `[workspace.package]`、`apps/desktop/src-tauri/tauri.conf.json`、`apps/desktop/package.json`、根目錄 `package.json`。`make check` 裡的 `versions` 關卡平常就擋不一致。
3. 在那個 commit 上打標籤，形狀是 `v0.1.0`，或預先發佈 `v0.1.0-rc.1`。標籤必須在 `main` 上，而且那個 commit 的 `check` 是綠的。不過就整個停下，不發佈。
4. 推標籤。`release.yml` 編三種系統的安裝檔和伺服器映像，建 **Release 草稿**。`-rc.N` 標成預先發佈。
5. 打開草稿，對 `SHA256SUMS`，看說明裡的放行步驟還在，再公開。
6. 公開之後（不是草稿、不是 `-rc`），同一個工作流程把映像的 `latest` 移到這個版本。

伺服器映像的標籤是版本號（例如 `0.1.0`）和 commit 短雜湊。`latest` 不在草稿階段移動。

## 怎麼預演

推 `ws/rel-` 開頭的分支。`release.yml` 用預演模式跑：檢查、編譯、打包、算雜湊，產物掛在那次執行的附件。不建 Release、不推映像、不動標籤。

也可以在 Actions 頁手動跑 Release，模式選 `rehearsal`。選 `publish` 會真的發，不要在工作分支上選。

## 出問題怎麼退回

開發版：

- 安裝檔：下一次 `dev` 全綠會整頁換掉。要立刻拿掉，在 GitHub 刪除 `dev-build` 這個 Release（標籤留著也沒有檔案可下）。
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

版本字串是 `v<版本>-<commit> · 建置時間`（`install-n10`）。建置時間是 CI 傳進編譯的 `SOURCE_DATE_EPOCH`，格式 RFC 3339。工作樹有未提交的改動時，本機編譯帶 `-dirty`；CI 拒絕帶 `-dirty` 的描述。
