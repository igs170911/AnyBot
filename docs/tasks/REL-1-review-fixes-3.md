# REL-1 審核修正（第 3 輪）

第三次預演又前進了一大步。**過了的**：版本和模式的檢查、Linux 的安裝檔（.deb、.rpm、壓縮檔）、macOS Intel 的安裝檔、amd64 和 arm64 兩個伺服器映像（arm64 在原生的機器上編，幾分鐘就好）、Linux 的完整檢查。`ci.yml` 這次是合法的，工作都有開始跑。

剩下三個失敗，都很具體。紀錄在 `target/ci-logs/rel1-release-3-failed.log` 和 `rel1-ci-3-failed.log`。

先做：`git merge dev`。

## 1. Windows 的安裝檔：連結失敗

```
CVTRES : fatal error CVT1100: duplicate resource.  type:VERSION, name:1, language:0x0409
LINK : fatal error LNK1123: failure during conversion to COFF: file invalid or corrupt
error: could not compile `anybot-app-desktop` (bin "anybot-desktop")
```

同一個執行檔被塞了兩份 Windows 的版本資源。桌面的殼分成兩個套件（`apps/desktop/context` 和 `apps/desktop/src-tauri`），看起來兩邊的建置腳本都各自產生了一份。上一次預演的紀錄裡，圖示的錯誤就是出在 `anybot-app-desktop-context` 的建置腳本。

- 只有最後那個執行檔的套件該帶 Windows 的資源。另一個套件不要產生。
- 這個你在本機驗不到（要 Windows 的連結器）。把兩個建置腳本各自做了什麼寫進回報，讓我看得出來為什麼修完只會有一份。

## 2. `ci.yml`：Windows、macOS 的基本測試還是沒跑到

兩個工作都死在測試之前：

- **macOS**：`error: could not create home directory: '/home/runner/.rustup'`。
  `ci.yml` 最上面的 `env` 把 `RUSTUP_HOME`、`CARGO_HOME` 寫死成 Linux 的路徑，整個工作流程的每個工作都吃到。這兩個只該給 Linux 的工作用。
- **Windows**：
  ```
  Move-Item: Cannot find path 'D:\a\AnyBot\AnyBot\cargo-nextest.exe'
  ```
  zip 檔被解到 `expand-<名稱>` 的子資料夾裡，接著卻到上一層去找。另外它也吃到上面那兩個 Linux 路徑的環境變數，一起處理。

這兩個是第一次有機會跑到測試。修完之後請你再照上一輪第二部分第 5 項，把這兩個工作**從第一步到最後一步**在腦中走一遍：每個步驟用的 shell 是哪一個、工具裝在哪裡、`PATH` 裡有沒有、`make test-basic` 會呼叫到哪些東西。

## 3. macOS（Apple 晶片）的安裝檔這次沒跑

```
The job was not acquired by Runner of type hosted even after multiple attempts
Due to capacity constraints, jobs targeting macOS arm64 runners may experience longer queue times.
```

這是 GitHub 那邊機器不夠，不是我們的問題（上一次預演這個工作是過的）。不用改。下次預演會再跑。

## 驗證和回報

- `make check` 全綠，commit 一次。
- 回報（簡短）：三項各怎麼改的；兩個桌面套件的建置腳本現在各做什麼；你走一遍之後另外找到、修掉的問題。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。不准在這台電腦上跑 Docker。
