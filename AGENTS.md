# AnyBotNext · 給寫程式的 agent 的規則（每次開始前先讀完）

這是 AnyBot 新版，用 clean room 的方式重寫。`spec/` 是唯一的需求來源。
開始任何工作前先讀：`spec/README.md`、`spec/04-product-core.md`、`spec/05-engineering-principles.md`、`spec/07-repo-layout.md`、`spec/10-behavior/README.md`，再讀這次任務指定的檔案。

## 一、絕對規則（違反就停下來回報，不要自己繞過）

1. **只在這個資料夾裡工作。** 不要讀、不要搜尋、不要列出這個資料夾以外的任何檔案。特別是 `~/Documents/AnyBotRebuild` 和 `~/Documents/AnyBot`：那是舊版，是污染源，一個字都不要看。也不要上網找舊版 AnyBot 的原始碼。例外：可以唯讀查看 `~/.cargo/registry/src/` 裡第三方套件的原始碼（查函式庫的 API 用），不准修改。
2. **規格沒寫的行為，不要猜。** 不要憑印象補「舊版大概怎麼做」。寫進回報的「要問的問題」。
3. **`spec/09-decisions.md` 是最終決定。** 跟其他文件不一致時，以它為準。
4. **不 push、不建遠端 repo。** 只在本地 commit；每個里程碑做完、`make check` 全綠後 commit 一次，訊息用繁體中文寫做了什麼。
5. **不動這個資料夾以外的東西。** 可以安裝的開發工具只有：`rustup component add rustfmt clippy`；`cargo install --locked` 這幾個：`cargo-deny`、`cargo-machete`、`cargo-nextest`、`sqlx-cli`、`tauri-cli`；`npm install -g pnpm`；`brew install gitleaks`、`brew install oasdiff`；`pnpm exec playwright install chromium webkit`；測試用的 PostgreSQL 程式由 harness 下載到 `target/`。**不准用 Docker**（`spec/09` K90，這台電腦的 Docker 上有使用者真的在用的東西）。要裝別的先停下來回報。
6. **不放任何金鑰、帳號、個人資料進 repo。** 新版也不讀環境變數做設定（`spec/02` A10）。
7. **暫存檔、紀錄檔一律放在 repo 的 `target/` 裡**（已經在 `.gitignore`），不要寫到 `/tmp` 或其他地方。
8. **不准改寫 git 歷史**：不准用 `filter-branch`、`rebase`、`reset --hard`、`commit --amend` 改已經存在的 commit，也不准把好幾個 commit 壓成一個。遇到「要改歷史才能過」的情況，停下來回報。
9. **不准按名字或樣式殺程式**：不准用 `pkill`、`killall`、`kill $(pgrep …)` 這類寫法。這台電腦上有使用者真的在用的程式。只准用 PID 收掉你自己啟動的程式。

## 二、寫程式的五條鐵律（使用者定的，`spec/05` §2a、`spec/09` K75–K78、K91）

1. **檢查擋下來的不能關掉，要修到過。** 程式碼裡不准出現 `#[allow`、`#[expect`、`biome-ignore`、`@ts-ignore`、`@ts-expect-error`。改程式讓它通過，不是改檢查讓它閉嘴。`spec/07` §3 規定的套件豁免，寫在那個套件自己的 `clippy.toml`。例外只准兩條（`spec/09` K76、K82）：RUSTSEC-2024-0370；桌面殼產生 Tauri context 的小套件不套 `exit` 規則。其他每個套件都完整繼承 workspace 的 lint。
2. **檔案要小。** 每個原始碼檔案最多 300 行、每個函式最多 60 行。快超過就拆，不要寫成一個大檔。
3. **不寫註解，程式即註解。** 用好的名字、小的函式、清楚的型別表達意思。產生出來的檔案除外。
4. **測試用名稱標出它驗的規格編號**：Rust 測試函式叫 `t_chat_steer_1`（`spec/10-behavior/` 的 `T-chat-steer-1`，`-` 換成 `_`）；TypeScript 測試標題開頭寫 `T-chat-steer-1`。
   - **名稱帶編號＝驗完那一列全部的「預期」**（`spec/09` K86）。只驗一部分、縮小規模、還沒接上畫面的，用描述性的名字，編號留在基準檔。
   - 種類是「整合」「端對端」的，要從外面打真的程式（執行檔、HTTP），不能只呼叫內部函式。
5. **程式碼和測試裡不准有中文**（`spec/09` K91、K92）。
   - 使用者看得到的字，放在 `locales/` 的翻譯檔，用訊息代碼查。
   - 後端只回代碼和參數，由前端照語言顯示。
   - 給開發者看的字直接寫英文：檢查工具的輸出、lint 的原因、斷言訊息、API 文件、README、Makefile。
   - 中文只准出現在 `locales/`、`spec/`、`docs/`、`AGENTS.md`。
   - 測試照代碼比對。測試資料需要中日韓文字時，用 Unicode 跳脫寫法。
   - 回報和 commit 訊息照舊用繁體中文。

## 三、開發原則（`spec/05` 摘要）

- **能用工具擋的就用工具擋，不靠感覺。**
- **「完成」＝`make check` 全綠，而且這次相關的規格測試都通過。** 沒到這個狀態，不要回報「完成」。
- **邊寫邊跑 `make check-fast`**（有了之後）：每改完一批就跑，錯了當場修，不要累積到最後。
- **依賴方向**照 `spec/07` §3，由 `tools/` 裡的檢查工具擋。
- **禁用的寫法**照 `spec/07` §3，由 clippy 的禁用清單擋：讀環境變數、執行外部程式、發 HTTP、讀寫任意檔案，只准在指定的套件。
- 正式程式碼裡不准 `unwrap`、`expect`、`panic!`；測試裡可以（靠 `clippy.toml` 的測試設定，不是在程式碼裡關掉）。
- 金鑰一律用專用型別：不能印、不能直接序列化。
- 根目錄 `tests/` 底下的測試輔助套件可以執行程式、連網路、寫 `target/` 裡的檔案（`spec/07` §3、`spec/09` K87）；產品套件只能把它當開發用依賴。
- **不准為了讓測試過而改正式程式的行為**（例如為了測試不互相干擾，在正式的伺服器開 `SO_REUSEPORT`）。測試不穩就修測試。
- **不准為了測試在正式程式加命令列旗標、環境變數、輸出模式**（例如 `--json-lines`、`--port`）。測試要測使用者真的看到的輸出。需要假的東西時，把伺服器當函式庫在測試裡啟動，假的東西從建構函式傳進去。
- **Rust 測試一律用 `cargo nextest run` 跑**（或 `make` 的目標），不要用 `cargo test`。`cargo test` 在同一個程式裡同時跑好幾個測試，不管 nextest 的分組，用同一個連接埠的測試會互相搶。
- **測試裡等外面的東西都要有期限**：等程式結束、等一行輸出、等連線都一樣。到期就用 PID 收掉自己啟動的程式，回錯誤說明等的是什麼。
- **測試卡死要被自動收掉**：`.config/nextest.toml` 設 `terminate-after`；自己跑測試的指令也要給執行工具設逾時。非同步的等待（例如等假時鐘叫醒）也要包一層真時間的期限，只准在測試和 `tests/harness` 用。
- **測試因為真實的日期、時間、環境變紅，代表程式錯了。** 先找出程式哪裡錯，不准把測試改成固定時鐘、固定值，讓它變綠。只有能說明測試本身寫錯了才改測試，而且要寫進回報。
- **算不出來的值要回錯誤**，不准用 `unwrap_or(0)`、`unwrap_or_default()` 這類寫法悄悄變成某個值，來躲開 lint。
- **日期、時間、網址、連線字串這類有標準的東西，用函式庫處理**，不要手寫曆法或解析。
- 選第三方套件前先查：誰維護、多久沒更新、有沒有 RustSec 紀錄。0.0.x 版、只有一個作者的套件，先在回報裡提出來再用。
- **`spec/` 是唯讀的**：任何人、任何工具都不准改 `spec/` 底下的檔案（包括格式化工具）。

## 四、技術選型（`spec/06`）

Rust stable、edition 2024；tokio、axum、sqlx、utoipa、reqwest（rustls）、thiserror／anyhow、tracing。TypeScript strict、React＋Vite、Biome、Vitest、Playwright；pnpm workspace；桌面 Tauri 2。伺服器資料庫 PostgreSQL，客戶端 SQLite。

## 五、功能里程碑的完成條件

照 `docs/plan.md` 開頭的「每個功能里程碑的完成條件」：規格測試、三層測試（單元、整合、端對端）、API 文件、五條鐵律加上 `make check`、回報。缺一項就不算完成。

## 六、回報格式（每個里程碑結束都要寫）

**回報必須跟實際的檔案一致。** 寫了「做了什麼」，檔案裡就要真的是那樣；審查的人會逐項對照。

用繁體中文：
1. 做了什麼（照任務的項目逐條）。
2. `make check` 最後的輸出（貼最後 30 行）。
3. 偏離規格的地方，和原因。
4. 要問的問題。

## 七、並行工作（好幾個 agent 同時做）

使用者會讓好幾個 agent 同時做不同的里程碑。被派到並行工作時：

1. **你只在自己的工作區**：一個 git worktree，一條從 `dev` 切出來的自己的分支（例如 `ws/<名稱>`）。分支的流程照 `spec/09` K111：工作分支 → `dev` → `main`。這個工作區的資料夾就是第一節第 1 條說的「這個資料夾」。
   - 不准碰別的工作區。
   - 不准切換到別人的分支，不准直接改 `dev` 和 `main`。合進 `dev`、把 `main` 往前推，都由監督者做。
2. **commit 在自己的分支**。回報之前：
   - 先 `git merge dev` 把最新的 `dev` 合進來。不准 rebase，也不准用任何方式改寫歷史。
   - 解決衝突，`make check` 全綠，再回報。
   - 衝突裡別人的改動要保留。看不懂、不確定怎麼合的，停下來寫進回報。
3. **編譯輸出可能跟別的工作區共用**：
   - 出現「Blocking waiting for file lock」是在等別人編譯，屬於正常。
   - 不准刪共用的編譯輸出，不准對它 `cargo clean`。
4. **測試鎖**：`make test`、`make e2e` 會等別的工作區的測試跑完，不要繞過。
5. 暫存檔放**自己工作區的** `target/`。不 push。
