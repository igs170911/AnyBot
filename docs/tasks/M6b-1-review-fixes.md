# M6b-1 第 1、2 段的審核修正

第 1 段（相容檢查、遷移檢查）和第 2 段（設定合併）的審核結果：下面 5 件擋住合併，要先修；後面幾件是同一批檔案裡順手修掉的。修完這一份，第 1 段才能合進 `dev`（M6b-2a 在等它）。

先讀 `spec/09-decisions.md` K118 ⑥（相容的承諾、破壞性變更紀錄、遷移只加不刪）和 ⑩（設定的時間、同一刻怎麼比）。

## 擋住合併的 5 件

### 1. 破壞性變更紀錄檔沒有進 git

`.gitignore` 第 9 行的 `*.log` 把 `api/breaking-changes.log` 擋掉了：它不在任何 commit 裡，本機能過只是因為檔案剛好在資料夾裡。GitHub 上拿不到這個檔，`make check` 會紅。

- 讓它被 git 追蹤（換一個不會被擋的副檔名，或在 `.gitignore` 加例外；你選，理由寫進回報）。
- 相容檢查自己確認這個檔被 git 追蹤（不在 `git ls-files` 裡就擋）。
- 基準那一邊（比對用的 ref）讀不到紀錄檔：只有「那個 ref 上根本還沒有這個檔」才當成空的；其他讀不到的情況（ref 不存在、git 出錯）一律擋，不准 `unwrap_or_default()`。

### 2. 第一個正式版之前，破壞性的變更要「加一行就過」

K118 ⑥：第一個正式版之前 API 和協定**可以**做破壞性變更，條件是重新產生基準的那個 commit 在紀錄加一行。現在的寫法對每一個破壞性變更都直接擋，連說明文字（`description:`）改一個字都算破壞性；selftest 還把「記錄了的破壞性變更照樣擋」寫死了。

- 第一個正式版之前：基準有任何變化（破壞性的、相容的、只改說明）→ 紀錄要多一行，多了就過。破壞性的項目照樣印出來（讓寫紀錄的人知道改了什麼），但不擋。
- 「正式版」照 K118 ⑥ 的定義（`main` 上第一個沒有 `-rc` 這種尾巴的 `v*` 標籤）。這次還不用做正式版之後的規則：偵測到已經有正式版時，一律擋，說「正式版之後的相容規則還沒做」（寧可擋錯，不可放過）。
- 拿掉永遠不會成立的那一段（`!breaking.is_empty() && !schema_changed`）。
- `schema::breaking` 漏了「已經有的 `required` 清單多了一項」：這是破壞性的。

### 3. OpenAPI 根本沒跟基準比

只有 AsyncAPI 改了才檢查紀錄有沒有變長；OpenAPI 唯一的關卡是「紀錄最後一行的雜湊等於現在的基準」。改 `api/openapi.yaml`、把最後一行的雜湊改掉、不加新的一行 → 過。把紀錄清空 → 直接過。

- OpenAPI 和 AsyncAPI 用同一套規則：跟 ref 比，有變化就要多一行。
- 紀錄只能往後加：ref 上的紀錄要一字不差地是現在紀錄的開頭，舊的行不准改、不准刪。
- 紀錄是空的、或最後一行的雜湊對不上現在的兩份基準 → 擋。

### 4. 遷移檢查用字串比對，繞得過去

現在是「有 ADD 和 COLUMN 兩個字就放行」，再用一串子字串黑名單補洞。審核者找到的繞法（每一個都要變成測試，而且要被擋）：

- `ALTER TABLE t ADD COLUMN default_model TEXT NOT NULL;`：欄位名稱裡有 `DEFAULT`，就被當成有預設值。
- `ALTER TABLE t ADD COLUMN a TEXT NOT NULL DEFAULT '', ADD COLUMN b TEXT NOT NULL;`：一句裡有兩個動作。
- `ALTER TABLE t ADD COLUMN c INT, ALTER d TYPE BIGINT;`：PostgreSQL 的 `COLUMN` 可以省略。
- `ADD` 和 `COLUMN` 分在兩行；`SET  NOT NULL` 中間兩個空白。
- 一行兩句：`CREATE TABLE a (id INT); CREATE UNIQUE INDEX i ON old (c);`，以及同一行 `CREATE TABLE` 之後接 `ALTER TABLE old ADD CONSTRAINT …`。

這跟 M7c 認指令遇到的是同一類問題：不要再補黑名單。**改成真的把 SQL 解析出來，照語法樹用「只准這幾種」來判斷，解析不了的一律擋。** 建議用 `sqlparser` 套件（Apache DataFusion 維護）的 PostgreSQL 方言；你要用別的，在回報裡說明維護狀況。只准的（K118 ⑥）：

- `CREATE TABLE`：新表裡的限制、外鍵（包括 `ON DELETE CASCADE`）都可以。
- 不是 `UNIQUE` 的 `CREATE INDEX`。
- `ALTER TABLE … ADD COLUMN`：每一個動作都必須是加欄位，而且那個欄位可以是空的，或者有 `DEFAULT`。
- 固定的起始資料：`INSERT … VALUES`。`INSERT … SELECT` 不算固定資料。

其他全部擋，包括任何 `DROP`、改名、改型別、在已經有的表上加限制、`UPDATE`、`DELETE`、`TRUNCATE`。順便會解掉：底線把字切開造成的誤擋（`last_update`、`drop_reason`）、`$$…$$` 和沒關上的引號把後面整段吃掉。

### 5. 設定合併「同一刻怎麼比」的測試拿掉規則也照樣綠

`apply` 只要不是「確定輸」就換掉（`settings_merge.rs` 第 63 行附近）。`merge_respects_the_tie_break_rules` 每次都把該贏的那筆放在最後套用，所以「後到的贏」也會得到一樣的結果。拿掉「裝置編號大的贏」、拿掉「紀錄流編號大的贏」、把整個比較換成只比時間，測試全部照樣綠。

- 加上「該贏的那筆先到」的情況：同一刻、不同台；同一刻、同一台。
- 性質測試的預期，用測試裡自己寫的模型算，不准呼叫產品的 `corrected_time()`（現在第 87 行附近有呼叫）。
- 收斂測試：每台裝置拿到的是整份清單的一種隨機排列（現在 1,060 種排列只用了 3 種，而且只有開頭是亂的）；清單類的設定每份至少放幾項，驗每一項各自合併。
- 回報裡寫：你暫時拿掉哪一條規則、哪一個測試變紅（三條規則各試一次，試完改回來，不要 commit 壞的版本）。

## 同一批檔案裡順手修

- `migrations_committed` 在本機用的 ref 是 `HEAD`，只抓得到還沒 commit 的改動。本機改用 `git merge-base HEAD dev`。比對用 `git diff --quiet <ref> -- <路徑>`，不要自己比原始位元組和 `from_utf8_lossy` 的結果（Windows 換行、非 UTF-8 會誤報）。
- `corrected_time` 用了兩次 `unwrap_or`（規則不准）；算不出來（溢位、算出來早於 1970）就回錯誤，不要悄悄變成「收到的那一刻」。
- `tests/api-compat-selftest/selftest.sh` 寫死 `target/debug/api-compat`，又沒有 `set -e`：編譯失敗時會跑舊的執行檔。改成編不過就失敗。
- `Makefile` 第 241–242 行附近：oasdiff 的基準和新版放反了，又把 AsyncAPI 交給它（它看不懂），兩個錯都被 `|| true` 蓋掉。修好順序，只給它 OpenAPI。
- `crates/sync/src/lib.rs` 留下一個重複的 `#[cfg(test)]`。

## 驗證和回報

完整檢查改在雲端跑（`AGENTS.md` 已改，`git merge dev` 後就看得到）：本機只跑相關的測試和 `make check-fast`；完整的檢查用

```
python3 ~/.claude/skills/acp-agents/bin/cloud_check.py
```

（前景執行、timeout 設 50 分鐘；它會先掃有沒有私人資料，再把你的分支推上 GitHub、等結果。沒過會印出錯在哪，完整紀錄在 `target/ci-logs/`；在本機只重跑失敗的那一步，修好、commit、再送一次。）

順序：`git merge dev` → 修 → 相關測試和 `make check-fast` 過了就 commit（訊息開頭 `M6b-1 review fixes:`）→ 雲端檢查全綠 → 簡短回報：五件各怎麼修、第 5 件三條規則各拿掉時哪個測試變紅、雲端檢查的最後一行和網址。

你是一次性執行的：回合一結束，背景工作就會被收掉。指令用前景跑，不要結束回合去等通知。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、除了雲端檢查之外不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。遇到「Operation not permitted」是沙箱在擋，不要繞過。
