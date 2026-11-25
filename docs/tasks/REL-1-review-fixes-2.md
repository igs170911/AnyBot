# REL-1 審核修正（第 2 輪）

第二次預演比第一次前進很多：版本和模式的檢查過了、macOS 兩種晶片都把 App 編出來了、Linux 的 .deb 和 .rpm 包出來了、amd64 的伺服器映像編好而且 `--version` 對了。

這一輪要修的分成「預演跑出來的」和「我讀程式看到的」。**全部修完再回報。**紀錄在你工作區的 `target/ci-logs/rel1-release-2-failed.log`。

先做：`git merge dev`。`dev` 多了一個重要的修正（`docs/tasks/HOTFIX-make-test.md`：以前測試失敗時 `make check` 還是會過，現在修好了，iron-laws 也多了一道掃 `Makefile` 的檢查）。你加在 `Makefile` 的目標可能會被新檢查擋到，照它的要求改。

## 一、預演跑出來的

1. **`ci.yml` 整個不能跑**（最嚴重）：GitHub 直接拒絕這個檔，「This run likely failed because of a workflow file issue」，一個工作都沒開始。
   - 我判斷的原因：`release-dev` 是「呼叫另一個工作流程」的工作，這種工作不能寫 `timeout-minutes`（只能有 `name`、`uses`、`with`、`secrets`、`strategy`、`needs`、`if`、`concurrency`、`permissions`）。你是為了過 iron-laws「每個工作都要有 `timeout-minutes`」才加的。請你查官方文件確認，不要只照我說的改。
   - 修法：拿掉它；iron-laws 那一條改成「有 `runs-on` 的工作才要 `timeout-minutes`，用 `uses:` 呼叫別的工作流程的不准寫」。被呼叫的那個工作流程裡，每個工作照舊要有。
   - **這個檔如果就這樣合進 `dev`，所有分支的自動檢查都會停擺。**所以再加一道本機就擋得到的檢查：工作流程的語法檢查（例如 actionlint；固定版本、驗 sha256，照 gitleaks 的做法；工具你照 `AGENTS.md` 查過再選），放進 `make check`。附「塞進去會被擋」的測試：在呼叫的工作上寫 `timeout-minutes` 要被擋。
2. **macOS（兩種晶片）**：App 編出來了，死在「collect installers」：
   ```
   no bundle directory under apps/desktop/src-tauri/target
   ```
   Tauri 的輸出在 repo 最上層的 `target/<目標>/release/bundle/`（紀錄裡 Linux 那一段看得到實際路徑），不在 `apps/desktop/src-tauri/target/`。`collect-bundles.sh` 找錯地方。把目標三元組傳進去，只找那一個資料夾。
3. **Linux**：.deb、.rpm 都包出來了，AppImage 失敗：
   ```
   couldn't find a square icon to use as AppImage icon
   ```
4. **Windows**：
   ```
   error RC2176 : old DIB in ...\apps\desktop\src-tauri\icons\icon.ico
   ```
   第 3、4 項是同一個原因：`apps/desktop/src-tauri/icons/` 裡的圖示是不合格的佔位檔。
   - 用程式產生一張簡單的暫用圖（單色底加一個字母這類，1024×1024），再用 Tauri 自己的工具產生整套圖示。產生的方式寫成腳本放在 repo 裡，之後有正式的圖再換。**不准從任何別的地方複製圖示。**
   - 規格（`install-n24` S1）要的 Linux 格式是 .deb、.rpm 和**壓縮檔**。補上壓縮檔（裡面是執行檔和必要的檔案）。AppImage 規格沒要求：圖示修好後包得出來就留著，包不出來就拿掉，不要讓它擋住其他格式。
5. **arm64 的伺服器映像用模擬器編太慢**：amd64 幾分鐘就好，arm64 用 QEMU 跑了三十多分鐘還沒完，我把那次預演取消了。發佈那一步也是用模擬器編兩種架構，會直接超過時間上限。
   - 改成兩種架構各自在原生的 runner 上編（GitHub 有免費的 arm64 Linux runner，名稱你查官方文件確認並寫死版本），各自驗 `--version`。
   - 發佈時：各自推上去，再合成一個兩種架構的標籤。預演時：只編、只驗，不推。

## 二、讀程式看到的

1. `ci.yml` 呼叫發佈流程時寫了 `secrets: inherit`：把所有金鑰都傳過去。這條流程只用 GitHub 自動給的 token，用不到。拿掉。
2. `release.yml` 最上面的 `env: IMAGE` 沒有轉小寫（紀錄裡是 `ghcr.io/igs170911/AnyBot`），`guard` 另外算了一個小寫的。只留一個來源：全部用 `guard` 算出來的那個，把上面那個拿掉。
3. iron-laws「`runs-on` 不准用 `latest`」：你讓 `runs-on: ${{ matrix.runner }}` 這種寫法直接跳過檢查。改成也檢查矩陣裡列出來的值；矩陣裡出現 `latest` 一樣要擋。附測試。
4. Linux 打包裝的系統套件裡有 `file-roller`（一個桌面的壓縮程式），看起來用不到。確認每一個套件都是需要的，不需要的拿掉。
5. **Windows、macOS 的基本測試上一次完全沒跑到**（因為第 1 項）。這次預演會第一次跑。你先自己想一遍它們在那兩種系統上會遇到什麼：`make` 和 `bash` 在 Windows runner 上的路徑、`Makefile` 裡用到的指令（`lockf`／`flock`、`jq`、`sed` 的寫法）在 Windows 的 Git Bash 上有沒有、測試鎖在 Windows 上怎麼辦。能先處理的先處理。不准為了過關跳過測試或放寬檢查。

## 驗證和回報

- `make check` 全綠（現在測試失敗會真的擋下來）。commit 一次。
- 回報（簡短）：每一項怎麼改的、查了哪些官方文件；新加的工具和它的版本、雜湊；你預期 Windows、macOS 的基本測試會卡在哪。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。不准在這台電腦上跑 Docker。
