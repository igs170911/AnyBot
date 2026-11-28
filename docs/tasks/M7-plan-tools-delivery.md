# M7 規劃：電腦與工具、交付（F 系列切片）

這份是**規劃**，不是派工單本身：A 段把這一區 `tools/spec-trace/baseline.txt` 裡還沒驗的 459 個編號逐一分到里程碑；B 段是 M7 要做的切片索引；C 段是每一片的藍圖（照 `docs/tasks/_template.md`，派工時整段複製成 `docs/tasks/M7-F<n>.md`）；D 段是要使用者決定的問題（每題附建議）；E 段是跟其他三個規劃區（回合／任務引擎、信任／核准、記憶／上下文、手機畫面）共用的介面。

讀法：

- 「M7（本區）」＝這份 C 段的切片會拿掉的編號。「M7e／M7f／M7j／M7k」＝M7 裡別的區負責的段（卡片、停止／接續、接上伺服器、手機畫面），沿用 `docs/STATUS-huskyVer.md` 和 M7c／M7d 藍圖用過的字母。
- 「未能測（實機）」＝規格種類寫「實機」的列，要真的 Mac／Linux／Windows，CI 跑不了，留在基準檔，由使用者手動驗。
- 「M9（正式映像）」＝規格 README 說「預期要真的系統行為的（`sudo`、`pgrep`、輸入關著、快照還原）等有正式映像再驗」。
- 一列能不能在 M7 拿編號，照 `spec/09` K86（整列預期都驗到）和 K119 ⑰（執行端這一層：「任務狀態」「證據清單」「模型收到的請求」「檔案…」「交付報告」算；「畫面出現」「推播送出」「按〔按鈕〕」不算；伺服器停機、兩台裝置、斷網的等 M7j）。

現況（寫這份時 `dev` 上有的，C 段的介面都接在這上面）：

- 電腦介面 `GatedComputer` 在 `crates/trust/src/runner.rs`（同步的 trait；`computers` 套件只有轉出口）。操作：`capabilities`、`run_command`、`read_file`、`write_file`、`edit_file`、`path_info`。
- 實際結果的型別在 `crates/protocol/src/computer_results.rs`：只有 `Command`、`Write`、`Read`、`Path`、`Caps`；沒有列資料夾、搜尋、比對、背景指令、拿檔的型別。
- 工具只有三個（`crates/tools/src/registry.rs`：`edit_file`、`read_file`、`shell`）。**產品程式裡沒有任何地方產生給模型的工具清單**：`RunnerParts.tools` 由測試傳進來。
- `shell` 的參數名在 `crates/tools/src/arguments.rs` 是 `line`，規格（`files-shell` S1 和每一列）是 `command`。
- `read_file` 的上限 `MAX_READ_BYTES = 8_000_000`，規格（`files-edit` S3）是 8,388,608。
- 長輸出：`tools/src/shell.rs` 留頭尾各 6,000 字，訊息 `tool-shell-trimmed` 給的是雜湊，不是 `read_result` 的 id；`crates/context/src/read.rs` 已經有 `read_result` 的純函式（分段、找字）。
- 逾時：`tool-shell-timed-out`（「操作本身失敗」）跟 `files-shell` S2（不中斷、移到背景）不一致。
- 交付報告 `DeliveryReportRecord`（`protocol/src/turn_records.rs`）沒有「改了但不能自動還原」那一段。
- 測試資料夾裡有一個殘檔 `crates/runner/tests/turn_files.rs-e`（sed 的備份，不會被編譯，內容跟 `turn_files_edit.rs` 重複）。

---

## A. 範圍分類（459 列）

每一格「編號」只寫最後的數字；同一家族同一理由的併成一列。

### A1. 電腦介面（`computer-*`，30 列）

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| computer-caps | 1 | M8 | 預期有「打開設定的電腦頁，畫面出現…不支援（第一版）」；請求那一半 F1 做掉 |
| computer-caps | 2 | M8 | 要 Bot 的能力開關（第 3 組 `profile-abilities`，R28），現在 `BotProfile` 沒有 |
| computer-caps | 3, 4 | M9 | 現況檢查和 30 分鐘快取要「電腦供應者記下現況檢查」 |
| computer-caps | 5 | M7j | 背景指令上限要「管理員設定」（伺服器設定）；預設 24 的行為 F12 做 |
| computer-caps | 6 | M10 | 使用者 Linux 的瀏覽器工具（桌面功能）才會有 browser |
| computer-caps | 7 | 未能測（實機） | 真的 Mac、Linux |
| computer-ops | 1 | M7（本區 F6） | 兩種假電腦端的結果格式一樣；Linux 上那條要問時用 M7e 的回答介面 |
| computer-ops | 3 | M9 | 預期「電腦供應者沒有記下截圖」；「這一輪沒有這個工具」的行為 F1 先做（描述性測試） |
| computer-ops | 4 | M7j | 步驟有斷網 |
| computer-ops | 5 | M8 | 介面版本比伺服器舊 7 個月＋電腦清單和手機畫面 |
| computer-platforms | 1 | M7（本區 F1） | 請求的工具清單照能力算、Windows 的固定指示 |
| computer-platforms | 2 | M7e | 要「在這台電腦上執行＝一律允許」的設定（`safety-local`）和卡片只有兩個按鈕 |
| computer-platforms | 3, 4 | 未能測（實機） | 真的 Windows／Mac／Linux 桌面 |
| computer-platforms | 5 | M10 | Linux 桌面的系統捷徑和 `mac_` 工具（桌面功能） |
| computer-rewind | 1, 2, 4, 5, 8 | M9 | Bot 的電腦的快照與 `/rewind`（正式映像、電腦供應者） |
| computer-rewind | 3, 7 | M7（本區 F9） | 使用者電腦上指令前後的比對、交付報告列「改了但不能自動還原」；7 是純函式 |
| computer-rewind | 6 | M10 | SSH 電腦（見 D-16） |
| computer-route | 1 | M7j | 手機發起→Bot 的電腦，要伺服器裁判和信箱 |
| computer-route | 2, 6 | M7k | 改派＋「畫面出現改派那一行」／推播 |
| computer-route | 3 | M10 | Mac App 工具 |
| computer-route | 4 | M7（本區 F1） | 帶 `computer` 參數一律回「這一輪的工具都在「這台」做」（見 D-1） |
| computer-route | 5 | M9 | 電腦供應者設成開機失敗 |

### A2. 帳號裡的電腦（`computers-*`，52 列）

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| computers-allowed | 1–4 | M8 | Bot 的「可以用的電腦」設定（`profile-n4`）和清單畫面 |
| computers-default | 1 | M8 | 端對端，手機對話詳細資訊 |
| computers-default | 2 | M10 | 截圖工具在使用者電腦（桌面功能） |
| computers-default | 3, 5 | M7j | 交辦子任務的「最近 3 則」規則、不在線改派，都要伺服器信箱（見 D-1） |
| computers-default | 4 | M7k | 「畫面出現沒有「工作機」這台電腦」 |
| computers-default | 6 | M10 | 遷移 |
| computers-kinds | 1, 4 | M8 | 端對端，電腦清單畫面 |
| computers-kinds | 2 | M10 | Windows 上叫截圖（螢幕工具） |
| computers-kinds | 3 | M7j | 指名另一台跑指令＝交辦到那台（見 D-1） |
| computers-local | 1 | M7k | 開關卡出現＋按一律允許後設定改變 |
| computers-local | 2 | M8 | `update_settings` 工具 |
| computers-local | 3, 6 | M8 | 「步驟詳情出現…」、每台電腦各自的設定不同步 |
| computers-local | 4 | M9 | 例行任務 |
| computers-local | 5 | 未能測（實機） | 管理員政策、真的 Mac |
| computers-n1 | 1–4 | M10 | SSH 電腦（見 D-16） |
| computers-n2 | 1, 4 | M8 | 設定頁說明、管理員政策的畫面 |
| computers-n2 | 2 | M7j | 交辦開關關著時手機發起要改派 |
| computers-n2 | 3 | M7k | 卡片出現在手機 |
| computers-n5 | 1, 3 | M7j | 只有某台連得到本機模型→派到那台，要伺服器派工 |
| computers-n5 | 2 | M7k | 端對端，推播＋畫面 |
| computers-n6 | 1, 2 | M7j | 改派時工作資料夾的對應，發起在手機 |
| computers-n7 | 1–3 | M7j | 伺服器記「最後在哪台打字」 |
| computers-n7 | 4 | M10 | 遷移 |
| computers-n8 | 1 | M8 | 端對端，改名後清單和卡片 |
| computers-n8 | 4 | M10 | 遷移 |
| computers-n9 | 1, 2 | M9 | 現況檢查次數（同 computer-caps S4） |
| computers-n10 | 1, 2 | 未能測（實機） | 真的 Docker |
| computers-online | 3 | M7k | 手機的離線橫幅 |
| computers-online | 6 | M7j | 兩份安裝共用登入、連線狀態 |
| computers-pair | 1 | 未能測（實機） | 全新 Linux 跑配對 |
| computers-pair | 8 | M10 | 要「發布網站」「匯出全部資料」這些入口都存在才能整列驗 |
| computers-send | 1–5 | M10 | 傳送到裝置（R38 排在後面；4 要訊息頻道） |

### A3. 交付（`deliver-*`，43 列）

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| deliver-send | 1 | M8 | 端對端，桌面副標和手機預覽頁 |
| deliver-send | 2, 6 | M7（本區 F10） | `send_file` 的三種拒絕、工作資料夾外的讀檔卡 |
| deliver-send | 3 | M9 | Bot 的電腦休眠、電腦供應者沒被叫醒 |
| deliver-send | 4 | M7j | 伺服器的附件暫存每晚清 |
| deliver-send | 5 | M10 | SSH 電腦 |
| deliver-send | 7 | M8 | 要 Bot 的能力開關才能設「全部關著」的前提（見 D-5） |
| deliver-confirm | 1, 7 | M7k | 端對端，交付記號 |
| deliver-confirm | 2–5, 8 | M7j | 裝置回報經伺服器、叫醒；8 的純函式跟著裝置回報一起做才有地方用 |
| deliver-confirm | 6 | M8 | 使用者在桌面點開的回報 |
| deliver-n1 | 1–3 | M8 | 檔案預覽器 |
| deliver-n2 | 1–8 | M10 | 傳送到其他裝置（R38） |
| deliver-n3 | 1–6 | M10 | 同上，收件端 |
| deliver-open | 1–4 | M10 | 在使用者正在用的電腦打開（桌面功能、信箱交辦） |
| deliver-open | 5 | 未能測（實機） | 真的三種平台 |
| deliver-publish | 1–6 | M10 | 發佈網站（伺服器託管） |

### A4. 檔案與指令（`files-*`，77 列）

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| files-shell | 1, 5 | M9（正式映像） | 「輸入關著」「pgrep」要真的系統；5 另外要停止（M7f） |
| files-shell | 2 | M7（本區 F11） | 逾時移到背景＋`read_terminal` |
| files-shell | 3 | M7（本區 F7） | 長輸出頭尾＋`read_result` 的 id |
| files-shell | 6 | M7（本區 F6） | 指令環境拿掉金鑰 |
| files-shell | 8, 9 | 未能測（實機） | 真的 Mac／Windows |
| files-edit | 1, 2 | M7（本區 F3） | `list_dir` |
| files-edit | 3, 4 | M7（本區 F4） | `search` |
| files-edit | 5, 6 | M7（本區 F2） | `read_file` 的分段、上限、資料夾、不是文字檔 |
| files-edit | 9 | M10 | SSH 電腦 |
| files-edit | 10 | M7（本區 F5） | 列資料夾不列、讀檔拒絕 AnyBot 自己的資料夾 |
| files-n1 | 1, 6, 7 | M7（本區 F8） | `read_result` 分段、20 MB 上限、找字 |
| files-n1 | 2 | M7（本區 F7） | 同內容同 id |
| files-n1 | 3, 8 | M7j | 兩台電腦之間、伺服器停機 |
| files-n1 | 4 | M7.5 | 「還被任務、證據、記憶引用」的保存政策要記憶的引用（K103） |
| files-n1 | 5 | M8 | 計畫模式（見 D-17） |
| files-n2 | 1 | M9（正式映像） | `sudo` 後交還檔案 |
| files-n2 | 3 | M7（本區 F5） | Bot 的電腦上碰別的 Bot 的資料夾 |
| files-n2 | 4, 5 | M10 | SSH 電腦 |
| files-n2 | 6 | M7（本區 F1） | 兩種電腦的固定指示 |
| files-terminal | 1–7 | M9（正式映像） | 真的終端機（PTY）；6 的純函式跟終端機一起做才有地方用 |
| files-terminal | 8 | 未能測（實機） | 真的 Windows |
| files-background | 1, 2 | M7（本區 F11） | `read_terminal` 的狀態與 `until` |
| files-background | 3 | M9（正式映像） | `pgrep` |
| files-background | 4, 5, 8 | M7（本區 F12） | 不能打字、上限 24、結束 10 分鐘後忘記 |
| files-background | 6 | M7k | 「按停止」 |
| files-background | 7 | M8 | 「按重新啟動這個 Bot」＋畫面 |
| files-background | 9, 10 | M7j | 伺服器停機、電腦關掉再打開（執行端重開後找回） |
| files-background | 11 | M9 | 電腦供應者沒記下休眠 |
| files-monitor | 1–10, 12 | M9 | 監看叫醒新的一輪＝跟進清單（R32）、例行和叫醒在 M9 |
| files-monitor | 11 | M10 | SSH 電腦 |
| files-rewind | 1, 2, 3, 5, 7, 8, 12 | M8 | `/rewind` 指令（屬信任區，見 D-15）；2 有「輸入框是…」 |
| files-rewind | 4 | M9 | Bot 的電腦的快照還原 |
| files-rewind | 6 | M7（本區 F9） | 超過 100,000 個檔時那一步標「沒比對」 |
| files-rewind | 9 | M7j | 那台不在線＋手機 |
| files-rewind | 10 | M7k | 端對端，手機 |
| files-rewind | 11 | M10 | 遷移 |
| files-workdir | 1, 3, 5 | M8 | 每台的「起始資料夾」和「查看、修改檔案和執行指令」能力都在 Bot 設定 |
| files-workdir | 2, 4 | M7j | 手機發起、改派 |
| files-workdir | 6 | M10 | 遷移 |

### A5. 沙盒（`sandbox-exec`，5 列）

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| sandbox-exec | 1, 2, 3, 5 | 未能測（實機） | 真的沙盒 |
| sandbox-exec | 4 | M8 | 要 /always-approve、直接操作、智慧模式和 /doctor 畫面 |

### A6. Bot 的電腦（`box-*`，83 列）

M7 不需要任何 `box-*`：M7 的「Bot 的電腦」一律用「假電腦端（宣告成 Bot 的電腦）」。

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| box-account-view | 1–9 | M9 | Bot 的電腦（Docker）本身 |
| box-desktop | 1–6 | 未能測（實機） | 實機 |
| box-machine | 1–5 | M9 | 每個使用者一台共用、電腦供應者 |
| box-machine | 6 | 未能測（實機） | 實機 |
| box-manage | 1–4 | M9 | 管理 Bot 的電腦 |
| box-n1 | 1–9 | M9 | 同上（3 是單元，跟著功能做） |
| box-n2, box-n3 | 全部（6＋5） | M9 | 同上 |
| box-n4 | 1–9 | M9 | 使用者密鑰注入（2 單元、7 性質跟著注入一起做） |
| box-n5 | 1–3 | 未能測（實機） | 實機 |
| box-n5 | 4 | M9 | Bot 的電腦 |
| box-n7 | 1–3 | M9 | 同上 |
| box-n8 | 1, 2, 4–7, 9–11 | M9 | 休眠、叫醒 |
| box-n8 | 3, 8 | 未能測（實機） | 實機 |
| box-n10 | 1–4 | M9 | 附件放進 Bot 的電腦（要伺服器的檔案暫存） |
| box-phone-drive | 1–7 | M9 | 手機看 Bot 的電腦的檔案 |

### A7. 瀏覽器與網路（`browser-*`，92 列）

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| browser-actions | 1–10, 12 | M9 | Bot 的電腦上的 AnyBot 瀏覽器 |
| browser-actions | 11 | 未能測（實機） | 實機 |
| browser-close | 1–3 | 未能測（實機） | 實機 |
| browser-close | 4 | M9 | 同上 |
| browser-fetch | 1–9 | M9 | 抓網頁工具＋假網站（見 D-18） |
| browser-gate | 1–9 | M9 | 瀏覽器的把關（第 2 組），跟瀏覽器一起做 |
| browser-n2 | 1–3, 5 | M9 | 網址分類（2 單元、5 性質跟著功能做） |
| browser-n2 | 4 | 未能測（實機） | 實機 |
| browser-n3, browser-n4, browser-n6 | 全部（6＋5＋6） | M9 | Bot 的電腦上的瀏覽器 |
| browser-n5 | 1, 3 | M9 | 同上 |
| browser-n5 | 2 | 未能測（實機） | 實機 |
| browser-own | 1–5, 7–9 | M9 | Bot 自己的瀏覽器、網站登入共用 |
| browser-own | 6 | 未能測（實機） | 實機 |
| browser-profiles | 1–4, 7 | M10 | 使用者 Mac／Linux 上的瀏覽器設定檔（桌面功能） |
| browser-profiles | 5, 6 | 未能測（實機） | 實機 |
| browser-refs, browser-search, browser-type, browser-wait | 全部（3＋5＋6＋3） | M9 | 瀏覽器操作、搜尋工具（假搜尋服務跟著做） |

### A8. 螢幕（`screen-*`，77 列）

先一律排 M9（Bot 的電腦的桌面）；做 M9 規劃時再把「只在使用者 Mac／Linux 才有的」移到 M10（見 D-19）。

| 家族 | 編號 | 分類 | 理由 |
|---|---|---|---|
| screen-act | 1–4 | M9 | 螢幕操作 |
| screen-act | 5, 6 | 未能測（實機） | 實機 |
| screen-batch, screen-bot-screens, screen-handover, screen-targets, screen-redact, screen-n3 | 全部（3＋10＋7＋4＋3＋5） | M9 | Bot 的桌面、接手、塗黑 |
| screen-intent | 1, 2 | M9 | 同上 |
| screen-intent | 3, 4 | 未能測（實機） | 實機 |
| screen-look | 1–5 | M9 | 截圖 |
| screen-look | 6, 7 | 未能測（實機） | 實機 |
| screen-n1 | 1–4 | 未能測（實機） | 實機 |
| screen-n2 | 1–4 | M9 | 同上 |
| screen-n2 | 5 | 未能測（實機） | 實機 |
| screen-n4 | 1–3 | M9 | 同上 |
| screen-n4 | 4 | 未能測（實機） | 實機 |
| screen-n6 | 1–6 | 未能測（實機） | 實機 |
| screen-n7 | 1, 2, 4–6 | M9 | 同上 |
| screen-n7 | 3 | 未能測（實機） | 實機 |
| screen-one-person | 1, 3 | M9 | 同上 |
| screen-one-person | 2 | 未能測（實機） | 實機 |

### A9. 合計

| 分類 | 列數 |
|---|---|
| M7（本區，F1–F12） | 29 |
| M7（他區：M7e 1、M7j 28、M7k 11） | 40 |
| M7.5 | 1 |
| M8 | 35 |
| M9（含正式映像） | 243 |
| M10 之後 | 55 |
| 未能測（實機） | 56 |
| **合計** | **459** |

---

## B. 切片索引

所有切片都**不需要 migration**（都在執行端這一層，存原文、紀錄用測試那一邊的記憶體實作；伺服器、客戶端的保存跟 M7j 一起做）。「協定」一欄指 `crates/protocol` 的型別；照 K119 ⑲，電腦介面現在只是程式裡的介面，不定線上格式，所以這些型別改了不動 `api/openapi.yaml`（目前也不在裡面）。

| 片 | 標題 | 編號 | 依賴 | 可並行 | 套件 | migration | 協定 |
|---|---|---|---|---|---|---|---|
| F1 | 工具清單照能力算、這一輪沒有的工具、不能指定別台 | T-computer-platforms-1、T-computer-route-4、T-files-n2-6 | — | 無（先單獨做） | tools、runner、locales、tests/runner-test | 無 | 無 |
| F2 | `read_file` 的完整規則 | T-files-edit-5、T-files-edit-6 | — | F1 之後跟 F3、F7 並行 | tools、locales | 無 | 無 |
| F3 | `list_dir` | T-files-edit-1、T-files-edit-2 | F1 | F2、F7 | protocol、trust、tools、runner、locales、tests/runner-test | 無 | 加 `ListRequest`／`ListOutcome`／`DirEntry`、`ActualResult::List` |
| F4 | `search` | T-files-edit-3、T-files-edit-4 | F3 | F8 | protocol、trust、computers、tools、runner、locales、tests/runner-test | 無 | 加 `SearchRequest`／`SearchOutcome`、`ActualResult::Search` |
| F5 | 檔案工具的邊界：私密檔、Bot 的電腦的別人資料夾 | T-files-edit-10、T-files-n2-3 | F3；信任區的 `sandbox-private` S2 名單 | F6 | trust、locales、tests/runner-test | 無 | 無 |
| F6 | 指令的環境與各種電腦上的結果格式 | T-files-shell-6、T-computer-ops-1 | F1 | F5 | protocol、trust、tools、tests/runner-test | 無 | `CommandRequest` 加 `env` |
| F7 | 長輸出給 `read_result` 的 id | T-files-shell-3、T-files-n1-2 | F1 | F2、F3 | tools、runner、locales | 無 | 無 |
| F8 | `read_result` 工具 | T-files-n1-1、T-files-n1-6、T-files-n1-7 | F7 | F4 | tools、runner、locales、tests/runner-test | 無 | 無（改 runner 的 `RawStore` trait） |
| F9 | 使用者電腦上指令前後的比對 | T-computer-rewind-3、T-computer-rewind-7、T-files-rewind-6 | F6；信任區的交付報告 | F10 | protocol、computers、trust、runner、tests/runner-test | 無 | `CommandOutcome.changes`、`EvidencePayload::Diffed`、`DeliveryReportRecord.unrevertable` |
| F10 | `send_file` 在執行端這一層 | T-deliver-send-2、T-deliver-send-6 | F5；M7e 卡片 | F9 | protocol、trust、tools、runner、locales、tests/runner-test | 無 | 加 `FetchOutcome`、`PathOutcome.size`、`EvidencePayload::Sent`、一種任務事件（見 D-12） |
| F11 | 逾時移到背景與 `read_terminal` | T-files-shell-2、T-files-background-1、T-files-background-2 | F7、F9 | — | protocol、computers、trust、tools、runner、locales、tests/runner-test | 無 | `CommandOutcome.background`、`TerminalReadRequest`／`TerminalReadOutcome` |
| F12 | 背景指令的上限、忘記、不能打字 | T-files-background-4、T-files-background-5、T-files-background-8 | F11 | — | computers、tools、runner、locales、tests/runner-test | 無 | 加 `TerminalWriteRequest` |

**順序（一波一波派）**：

1. F1（單獨，因為它把 `RunnerParts.tools` 拿掉，所有回合測試的組裝都會動）
2. F2、F3、F7
3. F4、F8
4. F5、F6
5. F9、F10
6. F11
7. F12

**共用的熱檔**（同一波裡不准兩片都改，派工時照這張排）：

- `crates/protocol/src/computer_results.rs`：F3、F4、F6、F9、F10、F11
- `crates/trust/src/runner.rs`（`GatedComputer`）和 `crates/trust/src/pipeline.rs`（`RequestedAction`）：F3、F4、F6、F10、F11、F12
- `crates/runner/src/runner.rs`：F1；`crates/runner/src/action.rs`：F1、F8
- `crates/tools/src/registry.rs`、`arguments.rs`：每一片加工具的都會動
- `tests/runner-test/src/fake_computer*.rs`、`fake_builder.rs`、`fake_scripts.rs`：F3、F4、F6、F9、F10、F11、F12
- `locales/en.ftl`、`locales/zh-TW.ftl`：幾乎每一片（只加新行、不重排，衝突好解）

---

## C. 切片藍圖

每一片的「驗證和回報」都一樣，不再重寫：本機跑改到的套件的 clippy 和相關測試（`cargo nextest run -p <套件>`，給執行工具設逾時）；送雲端前 `make check-fast` 一次；雲端檢查 `python3 ~/.claude/skills/acp-agents/bin/cloud_check.py`（前景，timeout 50 分鐘）全綠才回報；回報照 `AGENTS.md` 第六節，驗收清單逐條寫「做到了，在哪個 commit、哪個檔」。每一片都要附「變異紅」表：把行為表的每一條規則拿掉，哪個測試會紅。

測試資料裡的中文一律用 `\u{…}` 跳脫（K92）；比對照訊息代碼和參數（K119 ⑯）。

### F1：工具清單照能力算、這一輪沒有的工具、不能指定別台

**1. 目標**：給模型的工具清單由程式照這台的能力宣告算出來（不再由呼叫的人傳）；模型叫了這一輪沒有的工具、或帶了選電腦的參數，回規格寫的字，不執行。

**2. 開工條件**：`dev` 上有 M7c、M7d、M7e 第一波。分支 `ws/m7-f1`。

**3. 只准改的範圍**：`crates/tools/src/`（新增 `offer.rs`，改 `registry.rs`、`arguments.rs`）、`crates/runner/src/`（`runner.rs`、`action.rs`、新增 `tool_specs.rs`）、`tests/runner-test/src/turn*.rs`、`crates/runner/tests/`（新增 `turn_offer.rs`；刪掉 `turn_files.rs-e`）、`locales/*.ftl`。

**4. 定好的介面**

```rust
// crates/tools/src/offer.rs
pub struct ToolDef { pub name: &'static str, pub description_code: &'static str, pub parameters: serde_json::Value }
pub fn offered_tools(caps: &anybot_protocol::ComputerCaps) -> Vec<ToolDef>;   // 依 registry 的順序
pub const COMPUTER_PARAM: &str = "computer";
// crates/tools/src/arguments.rs 的 ArgsRejection 加兩種
NotInTurn { tool: String, available: Vec<String> },
OtherComputer,
pub fn check_offered(tool: &str, json: &str, offered: &[&str]) -> Result<(), ArgsRejection>;
// crates/runner/src/tool_specs.rs
pub(crate) fn turn_tools(lang: anybot_messages::Lang, caps: &ComputerCaps) -> Vec<anybot_models::ToolSpec>;
```

- `RunnerParts` 拿掉 `tools` 欄位；一輪開始時用 `gated.declared_caps()` 算一次，同一輪不變（K119 ⑳：把關設定改了下一個動作生效，但工具清單照規格 `computer-caps` S2 是「這一輪」的）。
- 能力對工具：`run_commands`→`shell`；`read_files`→`read_file`；`write_files`→`edit_file`。後面的切片各自在這張表加一列。
- `shell` 的 JSON 參數名改成 `command`（`files-shell` S1）；Rust 欄位名可以不改。
- 新訊息代碼：`tool-not-in-turn`（`tool`、`available`）、`tool-other-computer`（`computer`）、每個工具的說明 `tool-desc-shell`、`tool-desc-read-file`、`tool-desc-edit-file`（措辭見 D-4）。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 宣告 Windows：指令、讀、寫支援，螢幕、瀏覽器不支援 | 工具＝`edit_file`、`read_file`、`shell`；沒有 browser、computer；固定指示是 `turn-fixed-computer-windows` | computer-platforms S1、S4；computer-caps S2 |
| R2 | 只宣告讀檔 | 工具只有 `read_file` | computer-caps S2 |
| R3 | 什麼都沒宣告 | 工具清單是空的，照樣問模型 | computer-caps S2、sandbox-n4 S1 |
| R4 | 模型叫了清單外的工具（例如 `computer`） | 那個動作記失敗，模型收到 `tool-not-in-turn`，`available` 照清單順序；電腦沒有收到任何操作；不換電腦 | computer-ops S3 |
| R5 | 參數裡有 `computer`（不管值是不是這台） | 不執行，模型收到 `tool-other-computer`，`computer`＝這台的名字 | computer-route S4、`02` A4 |
| R6 | Bot 的電腦、Linux 各一輪 | 請求的固定指示分別是 `turn-fixed-computer-bot-machine`、`turn-fixed-computer-linux` | files-n2 S1 |
| R7 | 舊的 `line` 參數 | 當成缺 `command`，回參數錯誤 | files-shell S1 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_computer_platforms_1` | R1 | 假電腦端宣告 Windows，送 `"\u{6253}\u{958b} example.test"`；請求的工具名稱＝`["edit_file","read_file","shell"]`，不含 `browser`、`computer`；系統提示含 `turn-fixed-computer-windows` 照 zh-TW 渲染的字 | 整合 |
| `t_computer_route_4` | R5 | 在名叫 `Linux` 的假電腦端；假模型 `shell {"command":"ls","computer":"Bot \u{7684}\u{96fb}\u{8166}"}`；模型收到代碼 `tool-other-computer`、參數 `computer=Linux`；另一個宣告成 Bot 的電腦的假電腦端 `logged()` 是空的；這一台的 `logged()` 也沒有 `ls` | 整合 |
| `t_files_n2_6` | R6 | 兩輪各一句；第一個請求含 `turn-fixed-computer-bot-machine`，第二個含 `turn-fixed-computer-linux`，兩者互不包含 | 整合 |
| `offered_tools_follow_each_capability` | R1–R3 | 逐一把三個能力改成不支援，清單各少恰好那一個 | 單元 |
| `unknown_tool_lists_this_turn` | R4 | 只宣告讀檔、模型叫 `edit_file`：`available=["read_file"]`；證據清單沒有 `Changed` | 整合 |
| `shell_rejects_old_line_parameter` | R7 | `{"line":"ls"}` → 缺 `command` | 單元 |

**7. 不准做的**：不做 Bot 的能力開關（第 3 組，見 D-5）；不做交辦子任務；不改固定指示的組法（`context-fixed` 屬記憶／上下文區），只改 zh-TW `turn-fixed-computer-bot-machine` 的字讓它寫出「不用問」（`files-n2-6` 的預期）時，要在回報寫明；不加 browser、computer 的假工具。

**8. 驗收清單**

1. `RunnerParts` 沒有 `tools` 欄位，產品程式裡算工具清單的只有 `offered_tools`。
2. 三個帶編號的測試和三個描述性測試都在，都綠。
3. `shell` 的參數是 `command`，舊測試都改用 `command`。
4. `turn_files.rs-e` 刪了。
5. 兩個新代碼在 `en.ftl`、`zh-TW.ftl` 都有，`make locale` 綠。
6. 變異紅表：拿掉 R4、R5、能力對工具表任一列，各有測試紅。

### F2：`read_file` 的完整規則

**1. 目標**：`read_file` 的上限、分段提示、資料夾和非文字檔的訊息改成規格的樣子。

**2. 開工條件**：F1 合進 `dev`（F1 和 F2 都改 `locales`，F2 跟 F1 之後的 F3、F7 並行）。分支 `ws/m7-f2`。

**3. 只准改的範圍**：`crates/tools/src/read.rs`、`read_tests.rs`、`arguments.rs`（只動 `read_file` 的預設值常數）、`crates/runner/tests/turn_files_read.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
pub const MAX_READ_BYTES: u64 = 8_388_608;
pub const NOT_TEXT_PROBE_BYTES: usize = 8_000;
pub const DEFAULT_READ_LIMIT: usize = 2_000;
pub struct NumberedLines { /* 現有欄位 */ pub remaining: usize, pub next_offset: usize }
```

- 新代碼 `tool-read-more`（`remaining`、`next_offset`）：「…還有 { $remaining } 行（用 offset={ $next_offset } 繼續讀）」。
- 改字（代碼不變）：`tool-read-too-big`→「{ $path } 太大了，最多讀 8 MB」；`tool-read-directory`→「{ $path } 是資料夾，請用 list_dir」。
- `offset` 照現在的意思（跳過幾行，從 0 起）。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 3,000 行，沒給 offset、limit | 第 1–2,000 行，每行帶行號，結尾 `tool-read-more`，`remaining=1000`、`next_offset=2000` | files-edit S3 |
| R2 | 同上，offset=2000 | 第 2,001–3,000 行，沒有 `tool-read-more` | files-edit S3 |
| R3 | 剛好 2,000 行 | 沒有 `tool-read-more` | 邊界 |
| R4 | 8,388,608 bytes | 讀得到 | files-edit S3 |
| R5 | 8,388,609 bytes；9 MB（9,437,184） | `tool-read-too-big` | files-edit S3 |
| R6 | 第 8,000 個 byte（索引 7,999）是空字元 | `tool-read-not-text` | files-edit S3 |
| R7 | 第 8,001 個 byte 才有空字元 | 當文字讀 | 邊界 |
| R8 | 路徑是資料夾 | `tool-read-directory` | files-edit S3 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_edit_5` | R1 | `big.txt` 每行 `line-N`，3,000 行；模型收到的結果第一行和第 2,000 行照 `numbered_lines` 現在的格式寫死（抄現有格式），沒有 `line-2001`，結尾代碼 `tool-read-more`、`remaining=1000`、`next_offset=2000` | 整合 |
| `t_files_edit_6` | R5、R6、R8 | 9,437,184 bytes 的檔、資料夾 `d`、前段有空字元的 `bin`；三個結果的代碼依序 `tool-read-too-big`、`tool-read-directory`、`tool-read-not-text` | 整合 |
| `read_limit_is_exactly_8_mib` | R4、R5 | 8,388,608 讀得到；8,388,609 不行 | 單元 |
| `nul_probe_stops_at_8000_bytes` | R6、R7 | 同上兩種 | 單元 |
| `offset_continues_where_hint_says` | R2、R3 | 照 R2、R3 | 單元 |

**7. 不准做的**：不改讀檔的把關（工作資料夾外先問屬信任區）；不改假電腦端讀檔的語意；不做 `list_dir`（F3）。

**8. 驗收清單**

1. 三個常數照第 4 節，`8_000_000` 不再出現。
2. 五個測試在、都綠；兩個帶編號的在 `crates/runner/tests/`，從回合的公開介面起。
3. 兩個改字的代碼兩種語言都改了。
4. 變異紅表：上限、探測長度、預設行數各改 1，各有測試紅。

### F3：`list_dir`

**1. 目標**：新工具 `list_dir`，走電腦介面的新操作，照規格排序、截斷、空資料夾、不列私密檔。

**2. 開工條件**：F1 合進 `dev`。分支 `ws/m7-f3`。

**3. 只准改的範圍**：`crates/protocol/src/computer_results.rs`、`crates/trust/src/`（`runner.rs`、`pipeline.rs`、`op_ticket.rs`、把關分類的那一個檔）、`crates/tools/src/`（新增 `list.rs`，改 `registry.rs`、`arguments.rs`、`calls.rs`、`offer.rs`）、`crates/runner/src/action.rs`（只加 `view_of` 的一支）、`tests/runner-test/src/fake_*.rs`、`crates/runner/tests/turn_list_dir.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
// protocol
pub struct ListRequest { pub path: String }
pub struct DirEntry { pub name: String, pub kind: TargetKind, pub size: u64 }
pub struct ListOutcome { pub path: String, pub canonical: String, pub entries: Vec<DirEntry> }
ActualResult::List(ListOutcome)
// trust
OpKind::ListDir;  RequestedAction::ListDir { path: String };
fn list_dir(&self, ticket: &OpTicket) -> Result<ActualResult, OpRejection>;   // GatedComputer，寫法照 read_file
// tools
pub const TOOL_LIST_DIR: &str = "list_dir";      // ToolLevel::ReadOnly，能力 read_files
pub const MAX_LIST_ENTRIES: usize = 2_000;
pub fn render_listing(entries: &[DirEntry]) -> ListingText;
```

- 新代碼：`tool-list-more`（`count`）「…還有 { $count } 項」；`tool-list-empty`「（空資料夾）」；`tool-desc-list-dir`。
- 把關照 `read_file`：同一套工作資料夾內外的規則（屬信任區，不改規則，只讓 `ListDir` 走跟 `ReadFile` 一樣的分類）。
- 電腦回全部項目；排序、截斷、格式在 `tools`；私密項目由 `tools` 用 `anybot_trust` 的判斷函式濾掉（名單本身是信任區的，F5 補齊）。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | b/、A.txt（3 bytes）、c.txt | 依序 `b/`、`A.txt (3 B)`、`c.txt (0 B)`：資料夾在前加「/」，不分大小寫排序，檔案帶大小 | files-edit S1 |
| R2 | 2,005 項 | 列前 2,000 項＋`tool-list-more` `count=5` | files-edit S1（項數算法見 D-8） |
| R3 | 空資料夾 | 只有 `tool-list-empty` | files-edit S1 |
| R4 | 裡面有 `.env`、`id_rsa` | 不列 | files-edit S1、sandbox-private S1 |
| R5 | 路徑是檔案 | 操作失敗，回電腦的原因（`tool-operation-failed`） | computer-ops S4 |
| R6 | Bot 的電腦上列別的 Bot 的資料夾 | 拒絕（信任區現有規則） | sandbox-n4 S2 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_edit_1` | R1 | 結果的三行依序 `b/`、`A.txt (3 B)`、`c.txt (0 B)`（大小格式見 D-7，照決定改寫死值） | 整合 |
| `t_files_edit_2` | R2、R3 | 第一次：2,000 行項目＋代碼 `tool-list-more`、`count=5`；第二次 `list_dir("e")`：代碼 `tool-list-empty` | 整合 |
| `listing_sorts_dirs_first_case_insensitive` | R1 | `["b/","a/","C.txt","a.txt"]` 這種混排排成 `a/`、`b/`、`a.txt`、`C.txt` | 單元 |
| `listing_hides_private_names` | R4 | `.env`、`id_rsa` 不在結果 | 單元 |
| `list_dir_is_read_only_in_gate` | R6 | Bot 的電腦列 `/home/bot/bot-2` 被拒 | 整合 |

**7. 不准做的**：不做 `search`（F4）；不改信任區的把關規則和私密名單（F5、信任區）；不在電腦那一邊排序或截斷。

**8. 驗收清單**

1. 第 4 節的型別、函式、常數都在，名字一樣。
2. `list_dir` 只在宣告讀檔的電腦出現在工具清單（F1 的表加一列）。
3. 五個測試都綠；兩個帶編號的從回合起。
4. 假電腦端的 `list_dir` 照記憶體裡的檔案和 `.directory()` 回項目。
5. 變異紅表：排序、資料夾加「/」、上限、空資料夾、濾私密，各有測試紅。

### F4：`search`

**1. 目標**：新工具 `search`，有 ripgrep 和沒有 ripgrep 兩種，規則照規格。

**2. 開工條件**：F3 合進 `dev`。分支 `ws/m7-f4`。

**3. 只准改的範圍**：`crates/protocol/src/computer_results.rs`、`crates/trust/src/`（同 F3 那幾個檔）、`crates/computers/src/`（新增 `search_rules.rs`）、`crates/tools/src/`（新增 `search.rs`，改 registry、arguments、calls、offer）、`tests/runner-test/src/fake_*.rs`（`regex` 只准當 `tests/runner-test` 的依賴）、`crates/runner/tests/turn_search.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
// protocol
pub struct SearchRequest { pub pattern: String, pub path: String, pub files_only: bool }
pub enum SearchEngine { Ripgrep, Plain }
pub struct SearchHit { pub path: String, pub line: u32, pub text: String }
pub enum SearchCut { None, MaxLines, MaxTime }
pub struct SearchOutcome { pub engine: SearchEngine, pub hits: Vec<SearchHit>, pub files: Vec<String>, pub cut: SearchCut }
ActualResult::Search(SearchOutcome)
// computers/src/search_rules.rs
pub const SEARCH_MAX_MS: u64 = 25_000;  pub const SEARCH_MAX_LINES: usize = 10_000;
pub const SEARCH_RETURN_LINES: usize = 2_000;  pub const SEARCH_LINE_CHARS: usize = 300;
pub fn skipped(engine: SearchEngine, path: &str) -> bool;
pub fn clip_line(text: &str) -> String;
// tools
pub const TOOL_SEARCH: &str = "search";   // ReadOnly，能力 read_files；參數 pattern、path（預設 "."）、files_only
```

- 用哪種由電腦回報（`SearchOutcome.engine`）；假電腦端 `FakeComputerBuilder::ripgrep(bool)`，有就用 `regex`、沒有就不分大小寫找子字串，兩種都呼叫 `search_rules` 的略過和截斷。
- 新代碼 `tool-desc-search`；找不到和截斷的字見 D-9，決定前用 `tool-empty` 和描述性的 `tool-search-cut`（`shown`、`found`）。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 有 ripgrep，`foo[0-9]+` | 正規表示式；`a.txt:1:foo123`；略過 `.git`、`.env`、`.pem`、`.key` | files-edit S2 |
| R2 | 沒有 ripgrep，`foo` | 不分大小寫，`FOO` 也算；略過 `node_modules`、`target` | files-edit S2 |
| R3 | 一行超過 300 字 | 只留 300 字 | files-edit S2（兩種都套，見 D-9） |
| R4 | 找到 10,000 行 | 停止；只回前 2,000 行 | files-edit S2 |
| R5 | 25 秒到了 | 停止，回已找到的 | files-edit S2 |
| R6 | `files_only=true` | 只列檔名 | files-edit S2 |
| R7 | 沒有 ripgrep 時的私密檔 | 照樣略過 `.git`、`.env*`、`*.pem`、`*.key` | sandbox-private S4（見 D-9） |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_edit_3` | R1 | 有 ripgrep；`a.txt`＝`foo123`、`.env`＝`foo9`；結果含 `a.txt:1:foo123`，不含 `.env` | 整合 |
| `t_files_edit_4` | R2 | 沒有 ripgrep；`a.txt`＝`FOO`、`node_modules/x.js`＝`foo`；結果含 `a.txt:1:FOO`，不含 `node_modules` | 整合 |
| `search_clips_long_lines` | R3 | 400 個 `x` 的行回 300 個 | 單元 |
| `search_stops_at_limits` | R4、R5 | 12,000 行符合：回 2,000 行、`cut=MaxLines`；假時鐘 25,001 ms：`cut=MaxTime` | 單元 |
| `search_lists_files_only` | R6 | 兩個檔各有兩行符合：只回兩個檔名 | 單元 |
| `plain_search_skips_private` | R7 | 沒有 ripgrep，`.env` 有符合：不在結果 | 單元 |

**7. 不准做的**：不呼叫真的 `rg`（產品裡也不准執行程式，只有 `computers` 在通行證後面可以，M7 沒有真的電腦端）；不把正規表示式套件加進產品套件；不做 `.gitignore` 的語意。

**8. 驗收清單**

1. 第 4 節的型別、常數、函式都在。
2. 六個測試都綠。
3. 產品套件的 `Cargo.toml` 沒有多 `regex`。
4. 變異紅表：兩份略過清單、300、2,000、10,000、25 秒，各有測試紅。

### F5：檔案工具的邊界

**1. 目標**：檔案工具碰 AnyBot 自己的資料夾、Bot 的電腦上別的 Bot 的資料夾，回規格的字。

**2. 開工條件**：F3 合進 `dev`；信任區的 `sandbox-private` S2 名單那一片已經合進 `dev`（裡面要有「AnyBot 自己的資料夾」由電腦宣告的位置）。如果信任區還沒做，這一片先停（見 E）。分支 `ws/m7-f5`。

**3. 只准改的範圍**：`crates/trust/src/`（`gate_flow.rs` 的訊息參數、`message_codes.rs`）、`tests/runner-test/src/fake_builder.rs`（宣告 AnyBot 資料夾的位置）、`crates/runner/tests/turn_file_bounds.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

- `GateConfig` 裡 AnyBot 自己的資料夾的欄位用信任區那一片定的名字；這一片不另外加。
- `gate-private-file` 改字：「「{ $path }」是私密或系統檔案，Bot 不能讀取。」（sandbox-private S1）。
- `gate-bot-machine-outside` 參數改成 `bot`、`path`，字照 D-3 的決定（建議 files-n2 S2）：「{ $bot } 看不到這台電腦上的其他檔案（{ $path }）。要處理你自己電腦上的檔案，請在那台叫我，或交辦到那台。」

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | Linux，工作資料夾＝家目錄，AnyBot 資料夾在家目錄底下；列它的上一層 | 清單沒有 AnyBot 的資料夾 | files-edit S1、S7；sandbox-private S2 |
| R2 | 讀 AnyBot 資料夾裡的檔 | 拒絕，`gate-private-file` | sandbox-private S1 |
| R3 | Bot 1 在 Bot 的電腦讀 `/home/bot/bot-2/x.txt` | 拒絕，`gate-bot-machine-outside`，`bot`＝Bot 1 的名字 | files-n2 S2、sandbox-n4 S2 |
| R4 | Bot 1 讀自己資料夾裡的檔 | 直接讀 | files-n2 S2 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_edit_10` | R1、R2 | 家目錄 `/home/alice`＝工作資料夾，AnyBot 資料夾宣告在 `/home/alice/.local/share/AnyBot`，裡面有 `keys.db`；`list_dir("/home/alice/.local/share")` 的結果不含 `AnyBot/`；`read_file` 那個檔的結果代碼 `gate-private-file`、`path` 是那個路徑 | 整合 |
| `t_files_n2_3` | R3 | Bot 名字 `Bot 1`、工作資料夾 `/home/bot/bot-1`；結果代碼 `gate-bot-machine-outside`、`bot=Bot 1`、`path=/home/bot/bot-2/x.txt`；證據清單有一筆 `Rejected` | 整合 |
| `bot_reads_own_folder_directly` | R4 | 讀 `/home/bot/bot-1/a.txt` 沒有卡、讀得到 | 整合 |

**7. 不准做的**：不改私密名單本身（信任區）；不碰指令層（`sandbox-private` S5 是沙盒的事）。

**8. 驗收清單**

1. 兩個代碼的字和參數照第 4 節。
2. 三個測試都綠。
3. 變異紅表：拿掉 AnyBot 資料夾的判斷、拿掉 Bot 的電腦外面的拒絕，各有測試紅。

### F6：指令的環境與各種電腦上的結果格式

**1. 目標**：指令的環境由程式決定要拿掉哪些變數，電腦照做；同一個 `shell` 在 Bot 的電腦和使用者電腦上回給模型的格式一樣。

**2. 開工條件**：F1 合進 `dev`。分支 `ws/m7-f6`。跟信任區確認 `sandbox-n1` 那一片由誰做（見 D-10）；這一片只做 S1、S2 的「拿掉」。

**3. 只准改的範圍**：`crates/protocol/src/computer_results.rs`（`CommandRequest`）、`crates/trust/src/`（新增 `sandbox_env.rs`）、`crates/tools/src/calls.rs`（`build_request` 帶 env）、`tests/runner-test/src/fake_*.rs`、`crates/runner/tests/turn_command_env.rs`（新增）。

**4. 定好的介面**

```rust
// protocol
pub struct EnvPlan { pub unset_exact: Vec<String>, pub unset_prefix: Vec<String> }
pub struct CommandRequest { pub command: String, pub timeout_ms: u64, pub env: EnvPlan }
// trust/src/sandbox_env.rs
pub const MODEL_KEYS: [&str; 6] = ["XAI_API_KEY","OPENAI_API_KEY","ANTHROPIC_API_KEY","GEMINI_API_KEY","OPENROUTER_API_KEY","ANTHROPIC_AUTH_TOKEN"];
pub const ACCOUNT_KEYS: [&str; 1] = ["ANYBOT_ACCOUNT_PASSWORD"];
pub const RISKY_EXACT: [&str; 5] = ["LD_PRELOAD","LD_LIBRARY_PATH","BASH_ENV","ENV","PROMPT_COMMAND"];
pub const RISKY_PREFIX: [&str; 1] = ["DYLD_"];
pub const COMMAND_ONLY: [&str; 2] = ["SSH_AUTH_SOCK","DOCKER_HOST"];
pub fn command_env_plan() -> EnvPlan;
```

- 假電腦端：`FakeComputerBuilder::inherited_env(&[(&str, &str)])`；`CommandScript::ok_expanding(template)` 把 `$名字` 換成「繼承的環境去掉 `EnvPlan` 之後」的值（沒有就是空字串）；`FakeComputer::env_seen(command)` 回那次看到的變數名。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 繼承的環境有六個模型金鑰和帳號密碼 | 指令看不到 | sandbox-n1 S1；files-shell S4 |
| R2 | 有 `DYLD_INSERT_LIBRARIES`、`LD_PRELOAD` 等 | 看不到 | sandbox-n1 S2 |
| R3 | 有 `SSH_AUTH_SOCK`、`DOCKER_HOST` | 指令看不到 | sandbox-n1 S2 |
| R4 | 一般變數 `PATH`、`HOME` | 照樣在 | sandbox-n1（只拿掉列出的） |
| R5 | Bot 的電腦、Linux 各跑 `echo hi; exit 3` | 兩次模型收到的結果字串完全一樣；證據各有指令原文、電腦名稱、結束碼 3、秒數 | computer-ops S1、S2 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_shell_6` | R1 | 名叫 `Linux` 的假電腦端繼承 `ANTHROPIC_API_KEY=sk-test-1`；`shell(command="echo [$ANTHROPIC_API_KEY]")`；模型收到 `[]` | 整合 |
| `t_computer_ops_1` | R5 | 兩台腳本都是輸出 `hi\n`、結束碼 3、花 1,200 ms；兩次結果字串相等，都含 `hi` 和代碼 `tool-shell-failed`、`code=3`；兩筆 `Executed` 證據的 `command="echo hi; exit 3"`、`computer` 各是那台的名字、`exit_code=Some(3)`、`elapsed_ms=1200`。Linux 上若要問，用 M7e 的回答介面答允許這次再比 | 整合 |
| `env_plan_lists_every_key` | R1–R3 | `command_env_plan()` 含上面每一個名字和 `DYLD_` 前綴 | 單元 |
| `env_plan_keeps_ordinary_vars` | R4 | 套用後 `PATH`、`HOME` 還在 | 單元 |

**7. 不准做的**：不做 S3（帶 `ANYBOT_CONVERSATION_ID`、`ANYBOT_REQUEST_ID`，值是什麼見 D-10）和 S4（使用者密鑰注入，M9）；產品程式不讀自己的環境變數（A10）；不在產品程式執行任何程式。

**8. 驗收清單**

1. 名單照第 4 節逐字。
2. 四個測試都綠。
3. 變異紅表：名單每一類拿掉一個，各有測試紅。

### F7：長輸出給 `read_result` 的 id

**1. 目標**：指令輸出太長時，模型看到頭尾和「用 read_result 讀：id=…」；結束碼不是 0、認得出錯誤位置時改成三段。

**2. 開工條件**：F1 合進 `dev`。分支 `ws/m7-f7`。

**3. 只准改的範圍**：`crates/tools/src/shell.rs`（和它的測試）、新增 `crates/tools/src/result_id.rs`、`crates/runner/src/action_events.rs`（拿存原文回來的編號）、`crates/runner/tests/turn_long_output.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
pub fn result_id(digest: &ContentDigest) -> String;   // 格式見 D-11，建議 "r" + SHA-256 前 16 個十六進位字
pub struct Trimmed { pub head: String, pub tail: String, pub omitted: usize, pub error_window: Option<String> }
pub const SHELL_RESULT_CHARS: usize = 12_000;          // 頭尾各 OUTPUT_EDGE_CHARS（6,000）
```

- `tool-shell-trimmed` 參數改成 `omitted`、`id`，字：「…（中間省略 { $omitted } 個字；完整內容用 read_result 讀：id={ $id }）…」（files-shell S3、files-n1 S1）。
- 錯誤位置：用 `anybot_context` 已經有的那一份樣式（`context-mask` S2，M7b 做的），三段各 4,000 字（K114 ②、⑮）；那個函式不是公開的就停下來回報，不准在 `tools` 再寫一份。
- id 由存原文回的 `ContentDigest` 算，所以同內容同 id。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 結束碼 0、輸出 588,895 字 | 頭 6,000、尾 6,000，`omitted=576,895`，帶 id | files-shell S3 |
| R2 | 一樣的輸出跑兩次 | 同一個 id | files-n1 S2 |
| R3 | 結束碼 2，中間有一行 `error: boom` | 開頭、那一行前後、結尾三段，總長不超過 12,000 | files-shell S3；K114 ②、⑮ |
| R4 | 結束碼 0 但有 `error` 字樣 | 只頭尾 | context-mask S2 |
| R5 | 12,000 字以下 | 原樣，沒有 id 提示 | files-shell S3 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_shell_3` | R1 | 腳本 `seq 1 100000` 的輸出在測試裡組出來（588,895 字）；模型收到的結果開頭是輸出的前 6,000 字、結尾是最後 6,000 字、代碼 `tool-shell-trimmed`、`omitted=576895`、`id` 等於測試自己用 `sha2` 算的 `r`＋前 16 個十六進位字 | 整合 |
| `t_files_n1_2` | R2 | 50,000 個字的輸出跑兩次，兩個結果的 `id` 相同 | 整合 |
| `error_window_three_parts` | R3 | 三段各 4,000 字，中段含 `error: boom` | 單元 |
| `exit_zero_ignores_error_words` | R4 | 只有頭尾 | 單元 |
| `short_output_untouched` | R5 | 11,999 字原樣 | 單元 |

**7. 不准做的**：不做 `read_result` 工具（F8）；不改上下文編譯器的收起規則；不在 `tools` 重寫錯誤樣式。

**8. 驗收清單**

1. `tool-shell-trimmed` 的參數是 `omitted`、`id`，不再是雜湊全文。
2. 五個測試都綠。
3. 變異紅表：6,000、三段、結束碼 0 不找、同 id，各有測試紅。

### F8：`read_result` 工具

**1. 目標**：`read_result` 能分段讀、找字；原文超過 20 MB 只存前 20 MB 並註明。

**2. 開工條件**：F7 合進 `dev`。分支 `ws/m7-f8`。

**3. 只准改的範圍**：`crates/tools/src/`（新增 `read_result.rs`，改 registry、arguments、offer）、`crates/runner/src/`（`record.rs` 的 `RawStore`、`action.rs` 分出不碰電腦的工具、新增 `read_result_run.rs`）、`tests/runner-test/src/mem_store.rs`、`crates/runner/tests/turn_read_result.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
pub const TOOL_READ_RESULT: &str = "read_result";  // ReadOnly；不看能力，每個 Bot 都有（files-n1 S3）
pub const READ_RESULT_MAX_CHARS: usize = 20_000;
pub const RAW_MAX_BYTES: usize = 20 * 1024 * 1024;  // 見 D-11
pub const FIND_CONTEXT_CHARS: usize = 200;  pub const FIND_MAX_HITS: usize = 20;
// runner/src/record.rs
pub struct RawBlob { pub bytes: Vec<u8>, pub original_len: u64 }
pub trait RawStore { fn put(&self, bytes: Vec<u8>) -> …<ContentDigest>; fn get(&self, id: ContentDigest) -> …<Option<RawBlob>>; }
```

- 參數：`id`、`from`（預設 0）、`chars`（預設 20,000，超過當 20,000）、`find`（有就找字，忽略 from、chars）。
- 存的時候超過 `RAW_MAX_BYTES` 只存前面（切在字的邊界），`original_len` 記原長；id 照存進去的內容算（見 D-11）。
- 分段和找字用 `anybot_context::read` 已有的 `ReadQuery`／`read_result`；不碰電腦、不走把關、不發通行證（見 D-14）。
- 新代碼：`tool-read-result-window`（`from`、`to`、`total`）「第 { $from }–{ $to } 個字，共 { $total } 個字」；`tool-read-result-hit`（`at`、`context`）；`tool-read-result-not-found`「沒有找到」；`tool-read-result-missing`「找不到這份結果：可能超過 30 天被清掉了」；`tool-read-result-cut`（`mb`）「只存了前 { $mb } MB」；`tool-desc-read-result`。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 50,000 字，from=6000、chars=30000 | 回第 6,000 到 26,000 個字（20,000 字），`from=6000`、`to=26000`、`total=50000` | files-n1 S3；context-ref-1 的範圍寫法 |
| R2 | 原文 25 MB，from=20000000 | 回那一段，加 `tool-read-result-cut`、`mb=20` | files-n1 S2 |
| R3 | find 有兩處 | 兩筆 `tool-read-result-hit`，各帶前後 200 字 | files-n1 S5 |
| R4 | find 沒有 | `tool-read-result-not-found` | files-n1 S5 |
| R5 | 21 處 | 只回前 20 處 | files-n1 S5 |
| R6 | 亂給的 id | `tool-read-result-missing` | files-n1 S4 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_n1_1` | R1 | 輸出是 `0123456789` 重複 5,000 次；第二步 `read_result(id, from=6000, chars=30000)`：內容等於輸出第 6,000–26,000 個字、參數 `from=6000`、`to=26000`、`total=50000` | 整合 |
| `t_files_n1_6` | R2 | 輸出 26,214,400 個 `a`；`read_result(id, from=20000000)`：含代碼 `tool-read-result-cut`、`mb=20` | 整合 |
| `t_files_n1_7` | R3、R4 | 50,000 字，第 10,000 和 40,000 個字起是 `ERROR 42`；第一次兩筆 hit，`at=10000`、`at=40000`，前後文各 200 字；`find="\u{6c92}\u{6709}\u{7684}\u{5b57}"` 回 `tool-read-result-not-found` | 整合 |
| `find_caps_at_twenty` | R5 | 21 處只回 20 | 單元 |
| `unknown_id_is_missing` | R6 | `id="r-bogus"` → `tool-read-result-missing` | 整合 |

**7. 不准做的**：不做保存期限和引用（files-n1-4，M7.5）；不做向伺服器拿（M7j）；不做計畫模式（files-n1-5）；不讓 `read_result` 經過電腦。

**8. 驗收清單**

1. `RawStore::get` 回 `RawBlob`；測試那一邊的記憶體實作跟著改。
2. 五個測試都綠。
3. `read_result` 在任何能力宣告下都在工具清單裡。
4. 變異紅表：20,000、20 MB、200、20 處，各有測試紅。

### F9：使用者電腦上指令前後的比對

**1. 目標**：使用者的電腦上每條指令前後比對工作資料夾，結果記成證據，交付報告列「改了但不能自動還原」的檔；超過 100,000 個檔標「沒比對」。

**2. 開工條件**：F6 合進 `dev`；跟信任區對好 `DeliveryReportRecord` 加欄位（見 D-13）。分支 `ws/m7-f9`。

**3. 只准改的範圍**：`crates/protocol/src/`（`computer_results.rs`、`evidence_record.rs`、`turn_records.rs`）、`crates/computers/src/`（新增 `workdir_diff.rs`）、`crates/trust/src/`（`evidence.rs`、`report.rs`）、`crates/runner/src/action_events.rs`、`tests/runner-test/src/fake_*.rs`、`crates/runner/tests/turn_workdir_diff.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
// protocol
pub enum WorkdirChanges { NotTracked, NotCompared { limit: u32 }, Compared { modified: Vec<String>, created: Vec<String>, deleted: Vec<String> } }
pub struct CommandOutcome { /* 現有欄位 */ pub changes: WorkdirChanges }
EvidencePayload::Diffed { computer: String, changes: WorkdirChanges }
pub struct UnrevertableFile { pub path: String, pub change: FileChange, pub source: CheckSource }
pub enum FileChange { Modified, Created, Deleted }
pub struct DeliveryReportRecord { /* 現有欄位 */ pub unrevertable: Vec<UnrevertableFile> }
// computers/src/workdir_diff.rs
pub const DIFF_MAX_FILES: usize = 100_000;
pub struct FileStamp { pub path: String, pub size: u64, pub modified_ms: u64, pub digest: Option<ContentDigest> }
pub fn diff(before: &[FileStamp], after: &[FileStamp]) -> WorkdirChanges;
```

- 比對由電腦在 `run_command` 裡做（見 D-13）：Mac、Linux、Windows 回 `NotCompared` 或 `Compared`；Bot 的電腦、SSH 回 `NotTracked`。
- 判斷「改了」：大小或修改時間變了，而且兩邊的雜湊不同（有一邊沒有雜湊也算改了）。
- 新代碼 `tool-diff-skipped`（`limit`）「沒比對（工作資料夾超過 { $limit } 個檔）」。`unrevertable` 的 `source` 一律 `CheckSource::Program`。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | Linux：`sed -i s/a/b/ x.txt; touch y.txt` | 證據 `Diffed{Compared{modified:[x.txt], created:[y.txt], deleted:[]}}`；報告 `unrevertable`＝x.txt（Modified）、y.txt（Created），來源程式檢查 | computer-rewind S4 |
| R2 | 工作資料夾 100,001 個檔 | `NotCompared{limit:100000}`，那一步帶 `tool-diff-skipped` | computer-rewind S4；K66 |
| R3 | 剛好 100,000 個檔 | 照常比對 | 邊界 |
| R4 | 修改時間變了、內容沒變 | 不算改了 | computer-rewind S4 |
| R5 | Bot 的電腦上跑同一條 | `NotTracked`，報告沒有這一段的項目 | computer-rewind S1、S4 |
| R6 | `edit_file` 改的檔 | 不進 `unrevertable`（改檔工具改的能還原） | computer-rewind S4；files-rewind S1 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_computer_rewind_3` | R1 | 名叫 `Linux` 的假電腦端，`x.txt`＝`a`；腳本寫 `x.txt`＝`b`、建 `y.txt`＝空；要問就用 M7e 的回答介面答允許這次；證據和報告照 R1 | 整合 |
| `t_computer_rewind_7` | R2 | `diff` 收 100,001 個 stamp → `NotCompared{limit:100000}` | 單元 |
| `t_files_rewind_6` | R2 | 假電腦端宣告 Mac、100,001 個檔、`touch a`；那一步的 `Diffed` 證據是 `NotCompared{limit:100000}`，模型收到的結果含 `tool-diff-skipped`、`limit=100000` | 整合 |
| `diff_exactly_at_limit_compares` | R3 | 100,000 個 → `Compared` | 單元 |
| `diff_ignores_touched_but_same` | R4 | 同大小、新時間、同雜湊 → 空的 `Compared` | 單元 |
| `bot_machine_does_not_diff` | R5、R6 | Bot 的電腦 → `NotTracked`；`edit_file` 不進 `unrevertable` | 整合 |

**7. 不准做的**：不做快照、`/rewind`（M8、M9）；不做 SSH 的報告字（M10）；不在 `trust` 或 `runner` 掃檔案（只有電腦那一邊）。

**8. 驗收清單**

1. 第 4 節的型別都在；`unrevertable` 照 `CheckSource::Program`。
2. 六個測試都綠。
3. 變異紅表：100,000、雜湊相同不算、Bot 的電腦不比對、報告來源，各有測試紅。

### F10：`send_file` 在執行端這一層

**1. 目標**：新工具 `send_file`：從這台拿檔，記大小和 SHA-256，放進聊天；太大、資料夾、私密檔拒絕；工作資料夾外照讀檔規則先問。

**2. 開工條件**：F5 合進 `dev`（私密檔）；M7e 的卡片在 `dev` 上。D-12 的決定已經有。分支 `ws/m7-f10`。

**3. 只准改的範圍**：`crates/protocol/src/`（`computer_results.rs`、`evidence_record.rs`、`known_task_event_kind.rs`）、`crates/trust/src/`（`runner.rs`、`pipeline.rs`、把關分類）、`crates/tools/src/`（新增 `send.rs`，改 registry、arguments、calls、offer）、`crates/runner/src/`（記錄檔案卡那一支）、`tests/runner-test/src/fake_*.rs`、`crates/runner/tests/turn_send_file.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
// protocol
pub struct PathOutcome { /* 現有欄位 */ pub size: Option<u64> }
pub struct FetchOutcome { pub path: String, pub size: u64, pub digest: ContentDigest, pub bytes: Vec<u8> }
ActualResult::Fetch(FetchOutcome)
EvidencePayload::Sent { path: String, size: u64, digest: ContentDigest }
KnownTaskEventKind::FileShared { call_id: CallId, name: String, size: u64, digest: ContentDigest }  // 名字見 D-12
// trust
OpKind::FetchFile;  RequestedAction::SendFile { path: String };   // 把關照讀檔＋私密檔＋「傳出」
fn fetch_file(&self, ticket: &OpTicket) -> Result<ActualResult, OpRejection>;
// tools
pub const TOOL_SEND_FILE: &str = "send_file";   // 能力 read_files；不屬任何 Bot 能力（S7）；層級見 D-12
pub const SEND_MAX_BYTES: u64 = 52_428_800;      // 見 D-12
```

- 先用 `path_info` 看種類和大小，資料夾或超過上限就不拿檔。
- 拿到的位元組放進存原文（`RawStore::put`），任務事件只記名字、大小、雜湊。
- 新代碼：`tool-send-too-big`「檔案超過 50 MB」；`tool-send-directory`「這是資料夾：先壓縮成一個檔再傳」；`tool-send-done`（`name`）「已放進聊天：{ $name }。不要說使用者已經看到」（S3，措辭見 D-4）；`tool-desc-send-file`。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 51 MB | `tool-send-too-big`，沒有 `FileShared` | deliver-send S4 |
| R2 | 資料夾 `d` | `tool-send-directory`，沒有 `FileShared` | deliver-send S4 |
| R3 | `.env` | `gate-private-file`，沒有 `FileShared` | deliver-send S5 |
| R4 | Mac，工作資料夾外的 `~/x.txt` | 停在一張讀檔的核准卡 | deliver-send S5 |
| R5 | 工作資料夾裡 1 KB 的檔 | 一筆 `FileShared`（大小、雜湊）、一筆 `Sent` 證據、模型收到 `tool-send-done` | deliver-send S1–S3 |
| R6 | 剛好 52,428,800 bytes | 傳得出去 | 邊界 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_deliver_send_2` | R1–R3 | 53,477,376 bytes 的檔、資料夾 `d`、`.env`；三個結果代碼依序 `tool-send-too-big`、`tool-send-directory`、`gate-private-file`；這件任務的紀錄沒有任何 `FileShared` | 整合 |
| `t_deliver_send_6` | R4 | 假電腦端宣告 Mac，工作資料夾 `/Users/alice/work`，`/Users/alice/x.txt` 存在；`TurnOutcome.pending` 有一張卡，卡上的動作是讀檔 `~/x.txt`；模型沒收到 `tool-send-done` | 整合 |
| `send_file_records_size_and_digest` | R5 | `report.md` 1,024 bytes：`FileShared.size=1024`、`digest` 等於測試自己算的 SHA-256；`Sent` 證據一樣 | 整合 |
| `send_limit_is_inclusive` | R6 | 52,428,800 傳得出去、52,428,801 不行 | 單元 |

**7. 不准做的**：不做檔案卡的畫面（M7k、M8）；不做伺服器的附件暫存和每晚清（M7j）；不做交付確認（M7j）；不做 SSH（M10）；不做「在這台電腦上執行＝永不允許」的那一條（`computers-local`，M7e／M8）。

**8. 驗收清單**

1. 第 4 節的型別、常數都在。
2. 四個測試都綠。
3. `send_file` 在宣告讀檔的電腦一定在清單裡，不看別的。
4. 變異紅表：50 MB、資料夾、私密檔、外面先問，各有測試紅。

### F11：逾時移到背景與 `read_terminal`

**1. 目標**：指令超過等待時間不中斷、移到背景並給編號；`read_terminal` 讀狀態、最新輸出，能等某段字。

**2. 開工條件**：F7、F9 合進 `dev`（都改 `CommandOutcome`）。D-15 的決定已經有。分支 `ws/m7-f11`。

**3. 只准改的範圍**：`crates/protocol/src/computer_results.rs`、`crates/computers/src/`（新增 `background.rs`）、`crates/trust/src/`（`runner.rs`、`pipeline.rs`、把關分類）、`crates/tools/src/`（`shell.rs`，新增 `terminal.rs`，改 registry、arguments、calls、offer）、`tests/runner-test/src/fake_*.rs`、`crates/runner/tests/turn_background.rs`（新增）、改到的舊測試、`locales/*.ftl`。

**4. 定好的介面**

```rust
// protocol
pub struct BackgroundId(pub u32);
pub struct CommandOutcome { /* 現有欄位 */ pub background: Option<BackgroundId> }
pub struct TerminalReadRequest { pub id: BackgroundId, pub wait_ms: u64, pub until: Option<String> }
pub enum BackgroundState { Running { processes: u32, cpu_ms: u64, memory_bytes: u64 }, Exited { code: Option<i32> } }
pub struct TerminalReadOutcome { pub id: BackgroundId, pub command: String, pub state: BackgroundState, pub output_tail: String, pub until_seen: Option<bool> }
ActualResult::TerminalRead(TerminalReadOutcome)
// computers/src/background.rs
pub struct BackgroundTable { /* 每台電腦一份，編號從 1 起 */ }
// tools
pub const TOOL_READ_TERMINAL: &str = "read_terminal";  // ReadOnly，能力 run_commands
pub const READ_TERMINAL_MAX_WAIT_MS: u64 = 120_000;
```

- 假電腦端：`CommandScript::emitting(&[(u64, &str)])`（第幾毫秒印哪一行）；`read_terminal` 照假時鐘看 `now + wait`，用掉的時間推進假時鐘；「安靜多久算安靜」照 D-15。
- `tool-shell-timed-out` 拿掉，換成 `tool-shell-backgrounded`（`seconds`、`id`）「指令跑了超過 { $seconds } 秒，已移到背景繼續執行（背景指令 #{ $id }）」；M7c 斷言逾時是失敗的測試改成新行為，回報列出改了哪幾個。
- `read_terminal` 的字：`tool-terminal-header`（`id`、`command`）「背景指令 #{ $id }：{ $command }」；`tool-terminal-running`（`processes`、`cpu_seconds`、`memory_mb`）「還在執行：{ $processes } 個程式，CPU 時間 { $cpu_seconds } 秒，記憶體 { $memory_mb } MB」；`tool-terminal-exited`（`code`）「已結束（代碼 { $code }）」；`tool-terminal-until-seen`「已出現」；`tool-terminal-until-missing`「還沒出現」；`tool-desc-read-terminal`。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 預設 30 秒，指令要 40 秒 | 30 秒時回 `tool-shell-backgrounded`、`seconds=30`、`id=1`；不中斷 | files-shell S2 |
| R2 | 之後 `read_terminal(id=1, wait_ms=20000)` | 收到 `done`、`tool-terminal-exited` `code=0` | files-shell-2 |
| R3 | 每秒印一行的背景指令，`wait_ms=3000` | 標頭、`tool-terminal-running`、最新輸出 | files-background S1 |
| R4 | 第 5 秒印 `ready`，`until="ready"`、`wait_ms=10000` | `tool-terminal-until-seen` | files-background S1 |
| R5 | `until="nope"`、`wait_ms=2000` | `tool-terminal-until-missing` | files-background S1 |
| R6 | `wait_ms=200000` | 當成 120,000 | files-background S1 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_shell_2` | R1、R2 | 腳本 `sleep 40; echo done`：40,000 ms 印 `done`、結束碼 0；第一個結果代碼 `tool-shell-backgrounded`、`seconds=30`、`id=1`；第二個結果含 `done` | 整合 |
| `t_files_background_1` | R3 | 腳本每 1,000 ms 印 `tick N`，跑 60,000 ms，假電腦端回 1 個程式、CPU 2 秒、記憶體 12 MB；先 `shell(timeout_ms=1000)` 進背景；`read_terminal(id=1, wait_ms=3000)` 含 `tool-terminal-header` `id=1`、`tool-terminal-running` `processes=1`、`cpu_seconds=2`、`memory_mb=12` 和 `tick 4` | 整合 |
| `t_files_background_2` | R4、R5 | 第 5,000 ms 印 `ready`；依序回 `tool-terminal-until-seen`、`tool-terminal-until-missing` | 整合 |
| `read_terminal_wait_is_capped` | R6 | 200,000 → 120,000 | 單元 |
| `background_ids_count_per_computer` | — | 兩台各開一個背景指令，編號都是 1 | 單元 |

**7. 不准做的**：不做 `write_terminal`、上限、忘記（F12）；不做互動終端機、監看（M9）；不做執行端重開後找回（M7j）；不讓停止按鈕停背景指令（那是 M7f 的事，規格 S4 說不停）。

**8. 驗收清單**

1. `tool-shell-timed-out` 不在程式和翻譯檔裡。
2. 五個測試都綠；改到的 M7c 舊測試列在回報。
3. 變異紅表：30 秒、不中斷、120 秒上限、until，各有測試紅。

### F12：背景指令的上限、忘記、不能打字

**1. 目標**：每台最多 24 個背景指令；結束 10 分鐘後忘記；一般背景指令不能打字，可以停。

**2. 開工條件**：F11 合進 `dev`。分支 `ws/m7-f12`。

**3. 只准改的範圍**：`crates/protocol/src/computer_results.rs`（只加 `TerminalWriteRequest`）、`crates/computers/src/background.rs`、`crates/trust/src/`（`runner.rs`、`pipeline.rs`）、`crates/tools/src/terminal.rs`、registry、arguments、offer、`tests/runner-test/src/fake_*.rs`、`crates/runner/tests/turn_background_limits.rs`（新增）、`locales/*.ftl`。

**4. 定好的介面**

```rust
pub const BACKGROUND_LIMIT_DEFAULT: usize = 24;       // computer-caps S5；管理員可調是 M7j
pub const FORGET_AFTER_MS: u64 = 600_000;
pub struct TerminalWriteRequest { pub id: BackgroundId, pub text: Option<String>, pub stop: bool }
pub const TOOL_WRITE_TERMINAL: &str = "write_terminal";   // 能力 run_commands；這一片只收 text、stop
```

- 新代碼：`tool-terminal-not-interactive`「不是互動終端機：輸入一開始就關了」（S2，措辭見 D-4）；`tool-background-full`（`limit`）「背景指令已經有 { $limit } 個（上限），先停掉不用的」；`tool-terminal-forgotten`（`id`）「找不到背景指令 #{ $id }（結束超過 10 分鐘）」；`tool-terminal-stopped`（`id`）；`tool-desc-write-terminal`。
- `write_terminal` 打字、按鍵、`close` 是互動終端機的事（M9），這一片收到 `key`、`close` 回參數錯誤。

**5. 行為表**

| 編號 | 情況 | 預期 | 出處 |
|---|---|---|---|
| R1 | 一般背景指令 `write_terminal(text="x")` | `tool-terminal-not-interactive` | files-background S2 |
| R2 | 已有 24 個，再一個超過等待時間 | 不移到背景，直接停掉，回 `tool-background-full` `limit=24`；還是 24 個 | files-background S3 |
| R3 | 已有 23 個 | 第 24 個照常移到背景 | 邊界 |
| R4 | 結束後 11 分鐘讀 | `tool-terminal-forgotten` `id=1` | files-background S5 |
| R5 | 結束後剛好 10 分鐘讀 | 照常讀到 | 邊界 |
| R6 | `write_terminal(stop=true)` | 那個背景指令結束，之後讀到 `tool-terminal-exited` | files-background S2 |

**6. 要寫的測試**

| 測試 | 驗 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `t_files_background_4` | R1 | 一般背景指令 #1；結果代碼 `tool-terminal-not-interactive` | 整合 |
| `t_files_background_5` | R2 | 先用 24 次逾時做出 24 個背景指令；`shell(command="sleep 100")`：30 秒時結果代碼 `tool-background-full`、`limit=24`；假電腦端的背景表還是 24 個 | 整合 |
| `t_files_background_8` | R4 | #1 結束後假時鐘推 660,000 ms；結果代碼 `tool-terminal-forgotten`、`id=1` | 整合 |
| `limit_allows_the_twenty_fourth` | R3 | 23 個時第 24 個進背景 | 單元 |
| `forget_boundary_is_inclusive` | R5 | 600,000 ms 時讀得到 | 單元 |
| `stop_ends_background_command` | R6 | 停了之後讀到已結束 | 整合 |

**7. 不准做的**：不做互動終端機、按鍵（M9）；不做「重新啟動這個 Bot」（M8）；不做管理員改上限（M7j）；不做追 4,096 個子程式（M9 正式映像）。

**8. 驗收清單**

1. 兩個常數和 `TerminalWriteRequest` 照第 4 節。
2. 六個測試都綠。
3. 變異紅表：24、10 分鐘（兩個邊界）、不能打字，各有測試紅。

---

## D. 要問的問題（每題附建議）

1. **模型能不能用參數指定別台電腦？** `computer-route` S4 和 `02` A4 說一律不行（回「這一輪的工具都在「這台」做」）；`computers-default` S3 說「你最近 3 則訊息沒提到那台」才不行（反過來說有提到就可以），`T-computers-kinds-3` 甚至寫「跑指令(電腦＝W)…跑得成」。**建議**：A4 為準，工具參數永遠不能換電腦（F1 照這個做）；`computers-default` S3 只管「交辦子任務」，`T-computers-kinds-2`、`-3` 改寫成交辦。
2. **`shell` 參數名**：程式是 `line`，規格每一處都是 `command`。**建議**：F1 改成 `command`（這是對齊規格，不是新行為；列在這裡讓審查知道舊測試會一起改）。
3. **Bot 的電腦上碰別人資料夾的字有兩種**：`files-n2` S2「…要處理你自己電腦上的檔案，請在那台叫我，或交辦到那台。」；`sandbox-n4` S2「…要處理你的檔案，請用「你的電腦」。」。兩個測試都只看前半句。**建議**：用 `files-n2` S2 的（跟 D-1 的「交辦」一致）。
4. **AnyBot 寫給模型看的工具說明和幾句沒寫全的字**：K119 ⑯ 說要用訊息代碼，但規格沒給每個工具的說明，也有幾句用「…」省略（`send_file` 的「已放進聊天…」、`write_terminal` 的「不是互動終端機…」）。**建議**：說明每個工具一句話，照各工具 S1 的參數寫；省略號的地方由寫程式的 agent 補成完整句子，放進回報給監督者審，`zh-TW` 為準。
5. **Bot 的能力開關還沒有**（`profile-abilities`，R28）。工具清單＝能力 ∩ 宣告 ∩ 把關，現在只能算後兩個。**建議**：M7 當成能力全開；`T-computer-caps-2`、`T-deliver-send-7`、`T-files-workdir-5` 留到做能力開關的里程碑（建議 M8）。
6. **`read_file` 上限**：程式 8,000,000，規格 8,388,608。**建議**：照規格（F2）。
7. **`list_dir` 的大小怎麼寫**：規格只有「A.txt（帶 3 B）」和檔案卡的「1 KB」。**建議**：跟檔案卡同一個格式——1,024 以下寫「N B」，再來「N KB」「N MB」（1,024 進位、四捨五入到整數）；F3 寫死 `3 B`、`0 B`。
8. **`T-files-edit-2` 的數字**：前提是「2,005 個檔；另一個空資料夾 e」，又要 `list_dir(".")` 回「…還有 5 項」。e 如果也在 `.` 裡就是 2,006 項、還有 6 項。**建議**：項目數含資料夾；測試做 2,004 個檔＋資料夾 e（共 2,005 項），e 排第一個。
9. **`search` 的幾個細節**：(a) 沒有 ripgrep 時要不要略過 `.env`、`.pem`、`.key`？`files-edit` S2 只在有 ripgrep 那一句寫，`sandbox-private` S4 說搜尋一律略過。(b) 每行 300 字只在有 ripgrep 時？(c) 截斷、找不到時回什麼字？(d)「只列檔名」的參數名。**建議**：(a) 兩種都略過私密檔，沒有 ripgrep 時另外略過 `node_modules`、`target`；(b) 兩種都套；(c)「只列前 2,000 行（共找到 N 行）」「沒有找到」；(d) `files_only`。
10. **指令環境由誰做、S3 的值**：`sandbox-n1` 屬信任區，但 `T-files-shell-6` 在這區。`ANYBOT_CONVERSATION_ID`、`ANYBOT_REQUEST_ID` 在新版對到什麼？**建議**：名單放 `trust/src/sandbox_env.rs`（F6 建），信任區的 `T-sandbox-n1-1` 接著用；對話編號用 Bot 的編號（一個 Bot 一段對話），請求編號用這一輪的編號；S3 跟 `T-sandbox-n1-1` 一起做。
11. **`read_result` 的 id、20 MB、範圍寫法**：(a) id 格式只寫「id=r…」；(b) 20 MB 是位元組還是字；(c) 超過 20 MB 時 id 照全文還是照存下來的那段算；(d)「第 a–b 個字」從 0 還是 1 數。**建議**：(a)「r」＋SHA-256 前 16 個十六進位字；(b) 20 × 1,024 × 1,024 位元組，切在字的邊界；(c) 照存下來的那段（存原文本來就照內容給編號）；(d) 照 `T-context-ref-1`：a＝from、b＝from＋實際回的字數。
12. **`send_file` 的幾個介面**：(a) 讀檔只回文字、上限 8 MB，傳檔要位元組、上限 50 MB，要另一個「拿檔」的操作嗎？(b) 檔案卡在紀錄裡是什麼？現在 `RecordEntry` 只有使用者的話和任務事件。(c) 50 MB 是 50,000,000 還是 52,428,800？(d) `send_file` 的工具層級（重送會不會多一張卡）？**建議**：(a) 加 `fetch_file`，把關照讀檔；(b) 一種新的任務事件 `FileShared`（回合／任務引擎區的 `task-events` 要同意）；(c) 52,428,800；(d) `Irreversible`（重送會多一張卡，照「不能重做」處理）。
13. **使用者電腦上的比對放哪裡、報告怎麼記**：`computer-ops` S1 把「比對」列成一個操作，但比對要在指令前後各看一次。**建議**：電腦在 `run_command` 裡自己做前後比對，結果放 `CommandOutcome.changes`（不用另發通行證）；純函式放 `computers/src/workdir_diff.rs`，之後真的電腦端也用它；`DeliveryReportRecord` 加 `unrevertable`，信任區（`report-delivery`）同意欄位名後 F9 才開工。
14. **`read_result` 要不要走把關、留證據？** 它讀的是 AnyBot 自己存的原文，不碰電腦。**建議**：不走電腦的把關、不發通行證、不另外留證據（證據只來自電腦的實際結果，`computer-ops` 永遠成立那句）；照樣記工具的開始、完成事件。
15. **背景指令的細節**：(a) 編號是每台電腦還是每個 Bot？(b)「等到有新東西又安靜」的安靜是多久？(c) `/rewind`（files-rewind）屬信任區，要不要排進 M7？**建議**：(a) 每台電腦，從 1 起，執行端活著時不重用；(b) 跟互動終端機一樣 1.2 秒；(c) 不排 M7，跟 `/rewind` 指令一起在 M8（改檔前的存檔從那時開始存，`files-rewind` S10 本來就說升級後從那一刻起算）。
16. **SSH 電腦排哪個里程碑？** `docs/plan.md` 沒寫。**建議**：M10（R45：SSH 經由握著金鑰的那台使用者電腦連，跟桌面功能一起）。
17. **計畫模式在哪個里程碑？** `T-files-n1-5` 要「計畫模式」。**建議**：跟計畫模式（第 1 組 `plan-*`）一起，暫排 M8。
18. **抓網頁、搜尋工具（`browser-fetch`、`browser-search`）要不要早做？** 它們在執行端發 HTTP，不需要 Bot 的電腦。**建議**：跟 Bot 的電腦的瀏覽器一起排 M9（假網站、假搜尋服務那時做）；如果使用者想早點能查網頁，可以單獨提前，不影響 M7。
19. **`screen-*` 的細分**：這份先全部排 M9。**建議**：做 M9 規劃時逐列分出「只在使用者 Mac／Linux 才有」的移到 M10。
20. **殘檔 `crates/runner/tests/turn_files.rs-e`**：不會被編譯，但內容跟真的測試重複，`grep` 會誤導。**建議**：F1 順手刪掉。

---

## E. 跟其他規劃區共用的介面

**回合／任務引擎（M7f 停止、插話、接續；`task-events`）**

- F1 把 `RunnerParts.tools` 拿掉、在一輪開始時算工具清單：引擎區如果也在改 `RunnerParts`，F1 要先合。
- F8 在 `action.rs` 分出「不碰電腦的工具」那一支（`read_result`）；之後引擎區的停止、沒執行（K119 ⑦）要照樣套到這一支。
- F10 要一種新的任務事件（D-12 的 `FileShared`），`task-events` S2 歸引擎區，要先同意。
- 背景指令（F11、F12）：按停止不停一般背景指令（`files-background` S4），M7f 做停止時不要把它們一起停；執行端重開後找回（S6）跟 `task-journal` 一起在 M7j。

**信任／核准（M7e 卡片、`sandbox-*`、`report-delivery`）**

- `GatedComputer` 會加 `list_dir`（F3）、`search`（F4）、`fetch_file`（F10）、`read_terminal`（F11）、`write_terminal`（F12）；`RequestedAction`、`OpKind` 跟著加。把關怎麼分類（列、搜尋、拿檔、讀背景＝讀；寫背景＝指令）由信任區確認，這區不改規則本身。
- F5 要信任區先把 `sandbox-private` S2 的名單做齊，包括「AnyBot 自己的資料夾」由電腦宣告的位置；`trust_floor.rs` 現在的 `PRIVATE_PATH_PREFIXES` 只有 8 個，不夠。
- F6 建 `sandbox_env.rs`，信任區的 `T-sandbox-n1-1`、`-2` 接著用（D-10）。
- F9 在 `DeliveryReportRecord` 加 `unrevertable`（D-13）；交付確認的那一段（`deliver-confirm` S6）之後也會加在報告裡（M7j）。
- F9、F10、`T-computer-ops-1` 用 M7e 的卡片回答介面在測試裡答「允許這次」。

**記憶／上下文（`context-mask`、`context-ref`、M7.5）**

- 每個工具給模型看的上限（K114 ②）：`shell` 12,000（頭尾各 6,000）、`read_result` 20,000、其他 8,000。F7 用 `anybot_context` 已有的錯誤位置函式，不另寫；那邊如果不是公開的，要記憶區開出來。
- F8 用 `anybot_context::read` 的 `ReadQuery`、`read_result`；`T-context-ref-1`、`-2`、`-3` 是同一個工具從記憶區看，F8 做完後那三列可以由記憶區拿編號（`-3` 要永久刪除，M7j）。
- 原文的保存期限和引用（`files-n1-4`、K103）在 M7.5，`RawStore` 的介面（F8 改成回 `RawBlob`）到時候再加引用計數。

**手機畫面（M7k）與伺服器整合（M7j）**

- 檔案卡（F10 的 `FileShared`）要在手機顯示「名字 · 類型 · 大小 · 點一下開啟」、預覽頁有「檔案核對」（`deliver-send-1`，桌面那一半在 M8）。
- 改派那一行、離線橫幅、交付記號都在 M7k；這區的事件和證據已經帶足欄位，畫面只讀。
- 存原文和附件的正式實作（客戶端 SQLite、伺服器暫存）在 M7j；到時候才需要 migration（伺服器從 0300、客戶端從 `migrations/client/0020` 起），這份的 F1–F12 都不用。
