# REL-1 審核修正（第 4 輪）

第四次預演，**發佈那一半全部通過**：版本和模式的檢查、四個桌面安裝檔（Windows、macOS 兩種晶片、Linux）、兩個伺服器映像。產出的檔案我下載看過了：`AnyBot Dev_0.1.0_x64-setup.exe`、兩個 `.dmg`、`.deb`、`.rpm`、壓縮檔，各附 `SHA256SUMS`。Windows 重複的版本資源修對了。

剩下的都在 `ci.yml` 的基本測試。這次 Windows、macOS 第一次真的跑到。紀錄在 `target/ci-logs/rel1-ci-4-failed.log`。

先做：`git merge dev`。

## 1. macOS：459 個裡 1 個失敗

```
FAIL test-harness::postgres postgres_socket_directory_is_under_target
postgres setup failed: HTTP status client error (403 Forbidden) for url
  (https://api.github.com/repos/theseus-rs/postgresql-binaries/releases?...)
```

- 這個測試要啟動 PostgreSQL，執行時去 GitHub 下載，被 GitHub 以沒登入的請求太多擋了。
- 基本測試的定義是「不需要 PostgreSQL、不需要瀏覽器」。這個測試不該在裡面：`.config/nextest.toml` 的 `quick` 設定沒有篩掉它。
- 把篩選修對：需要 PostgreSQL 的測試一個都不准進基本測試。現在需要 PostgreSQL 的有 `tests/harness/tests/postgres.rs`，和 `apps/server/tests/` 底下 `pg_` 開頭的那些。
- 訂一條看得懂、之後不會漏的規則（例如「需要 PostgreSQL 的測試檔一律 `pg_` 開頭」），寫進 `docs/testing.md`。另一個里程碑（M6a）也有需要 PostgreSQL 的測試，它那邊會照這條規則改名；你不用動它的檔。
- 加一道檢查：`quick` 設定跑的測試裡，沒有任何一個會去啟動 PostgreSQL。做法你想（例如讓啟動 PostgreSQL 的那個函式在基本測試的環境下直接讓測試失敗並說明原因，或用工具掃）。重點是下次有人把這種測試放錯地方，本機就擋得到，不用等到 macOS 的機器上才知道。

## 2. Windows：測試工具編不過

```
error[E0433]: cannot find `unix` in `os`
error[E0599]: no method named `signal` found for reference `&ExitStatus`
error: could not compile `test-harness` (lib)
```

`tests/harness` 用了只有 Unix 才有的東西（訊號、行程群組）。產品本身的套件沒有報錯。

讓測試工具在 Windows 上也能用是另一件有份量的工作，**這次不做**，之後另開任務。這一輪這樣處理：

1. Windows 的工作改成：
   - 編譯**產品**的套件和可執行檔（`crates/` 全部、`apps/` 全部，包括桌面的殼）。
   - 跑**產品套件自己的單元測試**（寫在套件裡面的那些）。
   - 不編、不跑依賴測試工具的東西：`tests/` 底下的套件，和各個 `apps/*/tests/`、`crates/*/tests/` 裡用到測試工具的整合測試。
2. 哪些套件、哪一類測試在 Windows 上沒跑，**明確列在一個地方**（`docs/testing.md`），工作流程裡照那份清單，不要散在指令的參數裡看不出來。
3. 三種系統的工作最後都印一行：這次跑了幾個測試。Windows 的數字會比較小，這是預期的，但要看得到。
4. 產品的程式在 Windows 上有一個警告（`crates/foundation` 有沒用到的匯入，因為檢查檔案權限那一段只在 Unix 編譯）。修掉；之後 Windows 上的編譯警告要當成錯誤。
5. 不准用「在 Windows 上整個不編」來處理產品的套件。產品的套件一個都不能少。

## 3. 小的

1. Linux 那一包多收了兩個檔：`control.tar.gz`、`data.tar.gz`。它們是打 .deb 時的中間檔，被「所有 `.tar.gz`」的規則撈進來了。只收要給人下載的那幾個。
2. 執行結果裡多了 `*.dockerbuild` 的附件（建置映像的 action 自動上傳的紀錄）。關掉。
3. `docs/release.md`、wiki 的「安裝與使用」：把這次實際產出的檔名寫上去。

## 驗證和回報

- `make check` 全綠，commit 一次。
- 回報（簡短）：每一項怎麼改的；Windows 上沒跑的清單；三種系統預期各跑幾個測試。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。不准在這台電腦上跑 Docker。
