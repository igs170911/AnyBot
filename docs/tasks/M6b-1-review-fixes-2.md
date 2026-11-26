# M6b-1 第 1、2 段的審核修正（第 2 輪）

## 1. 目標

上一輪修正之後的審核又找到 5 件擋住合併的問題。這一輪照下面寫死的做法修掉，不自己換做法；做法行不通就停下來回報。

## 2. 開工條件

工作區 `~/Documents/AnyBotNext-wt/m6b1`，分支 `ws/m6b-1`。先 `git merge dev`。注意：監督者把一個用錯身分的 commit 改回了 repo 的身分（內容不變），所以你看到的 commit 編號跟上一輪不同，屬於正常。

## 3. 只准改的範圍

`tools/api-compat/`、`tools/iron-laws/src/{main.rs,migrations*.rs}` 和它們的測試、`crates/domain/src/settings_merge*.rs`、用到 `corrected_time` 的地方、`Makefile` 第 22 行附近的註解、`Cargo.toml`（只改 `sqlparser` 的 features）、`docs/testing.md`。

## 4. 定好的介面

```rust
// crates/domain/src/settings_merge.rs
pub enum CorrectedTimeError { Overflow, BeforeEpoch }
pub fn corrected_time(device_time: TimestampMs, offset_ms: i64, received_ms: TimestampMs)
    -> Result<u64, CorrectedTimeError>;
```

- 校正後的時間在**建立一筆設定修改時算一次、存在那一筆上**（建構函式回 `Result`），合併時比存著的值，不在合併裡重算（K118 ⑩：伺服器算好記在那一筆上，每台都用它比）。
- 整個檔案不准有 `unwrap_or`、`unwrap_or_else`、`saturating_*` 用來把算不出來的值變成某個值；`MillisCast` 這類藏起來的轉換一起拿掉。

## 5. 行為表

| 編號 | 情況 | 預期 |
|---|---|---|
| A1 | 相容檢查拿到的比對基準是 `<ref>` | 比 `<ref>` 本身，**不是** `<ref>~1`。基準上的紀錄有 L1、L2；分支改了 `openapi.yaml`、把 L2 的雜湊改掉、沒加新行 → 擋 |
| A2 | 同上，分支多加一行 L3（雜湊對得上現在的兩份基準）、L1、L2 一字不差 | 過 |
| A3 | 分支沒改任何基準、紀錄也沒變 | 過 |
| M1 | 基準之後 commit 了一個**新的**遷移檔 | 過 |
| M2 | 改了基準上已經有的遷移檔的內容 | 擋 |
| M3 | 刪掉基準上已經有的遷移檔 | 擋 |
| M4 | 把基準上已經有的遷移檔改名 | 擋（做法：`git diff --quiet --no-renames --diff-filter=DMT <ref> -- <資料夾>`） |
| M5 | `iron-laws` 沒拿到比對基準（參數沒給或是空的） | 失敗，說「沒有比對基準」，結束碼不是 0；不准退回 `HEAD` |
| M6 | `ADD COLUMN x INT NOT NULL DEFAULT NULL` | 擋（`DEFAULT NULL` 不算有預設值） |
| M7 | `INSERT … VALUES ((SELECT …))`、`VALUES (now())`、任何不是常數的值 | 擋（固定資料只准常數） |
| M8 | 遷移檢查回報違規 | 寫那一句所在的行號，不是一律第 1 行 |
| M9 | 判斷「基準上沒有這個檔」 | 用 `git cat-file -e <ref>:<路徑>` 的結果，不看 git 的錯誤訊息文字 |
| T1 | `device_time` ≥ 2^63 毫秒、或 `received` ≥ 2^63 | `Err(Overflow)` |
| T2 | `device_time + offset` 溢位 | `Err(Overflow)` |
| T3 | `device_time + offset` < 0 | `Err(BeforeEpoch)` |
| T4 | 加起來比 `received` 晚 | `Ok(received)` |
| T5 | 加起來等於 `received` | `Ok(received)` |
| T6 | 加起來比 `received` 早 | `Ok(加起來的值)` |
| S1 | 清單類的設定（K118 ⑩、`sync-settings` S4） | 每一份清單至少 3 項，不同裝置各改不同的項，每一項各自照「晚的贏、同一刻裝置大的贏、同一台紀錄流大的贏」合併 |

## 6. 要寫的測試

| 測試名稱 | 驗哪幾列 | 預期值（寫死） | 種類 |
|---|---|---|---|
| `compat_blocks_a_rewritten_hash_against_the_merge_base` | A1 | 結束碼不是 0，訊息提到 `api/breaking-changes.log` | 整合（暫存 repo：基準 commit 加 L2、分支改雜湊） |
| `compat_accepts_an_appended_line_against_the_merge_base` | A2 | 結束碼 0 | 整合（同一個暫存 repo，傳 `git merge-base` 算出來的 ref） |
| `compat_passes_when_nothing_changed` | A3 | 結束碼 0 | 整合 |
| `committed_migration_gate_allows_a_new_committed_file` | M1 | 沒有違規 | 整合（新檔**要 commit**，不是沒追蹤的檔） |
| `committed_migration_gate_blocks_edit_delete_rename` | M2–M4 | 三種各一筆違規 | 整合 |
| `iron_laws_without_a_base_ref_fails` | M5 | 結束碼不是 0 | 整合（跑執行檔） |
| 遷移檢查補 M6、M7 的例子 | M6、M7 | 各一筆違規 | 單元 |
| `migration_violation_reports_the_statement_line` | M8 | 第 3 行的違規回報第 3 行 | 單元 |
| `corrected_time_rejects_and_clamps` | T1–T6 | 六個寫死的輸入和結果 | 單元 |
| `merge_lists_item_by_item` | S1 | 寫死三台、三項的輸入和每一項的贏家 | 單元 |

性質測試：

- 預期模型自己用 `(欄位, 項)` 分組，**不准**呼叫產品的 `edit.key()`、`wins()`、`corrected_time()`。
- 送達順序用固定種子的亂數各自洗牌：要幾種就產生幾種，不先產生一千多種再用三種。
- 每台裝置有一部分修改**送兩次**，結果要跟只送一次一樣（測試名稱裡的 `repeats` 才名副其實）。
- 整個 `anybot-domain` 的測試在本機每一個都要在 2 秒內跑完；回報裡寫收斂測試的秒數。

## 7. 不准做的

- 不准再用 `<ref>~1` 或任何「往前一個」的推算。
- 不准把新的遷移檔加進例外清單來躲 M1。
- 不改 `.github/`（`ci.yml` 的 `|| echo HEAD` 是 INFRA-4 的事）。
- 不做第 3 段以後的事。

## 8. 驗收清單

1. A1–A3 三個測試存在，用的是真的 merge-base，不是 `HEAD`。
2. M1–M9 都有測試，M1 的新檔是 commit 過的。
3. `iron-laws` 沒有基準就失敗；`Makefile` 第 22 行附近的註解照實寫。
4. `corrected_time` 是第 4 節的簽名；`settings_merge.rs` 沒有 `unwrap_or`、`saturating_*`、`MillisCast`；T1–T6 測試存在。
5. 校正後的時間在建立修改時算一次、存在那一筆上。
6. 清單每份至少 3 項、各項分開合併，有 S1 的測試。
7. 性質測試照第 6 節的四點；回報收斂測試的秒數。
8. `sqlparser` 改成 `default-features = false, features = ["std"]`（編不過就回報原因，不要硬改）；沒用到的 `serde` 拿掉。
9. `schema::breaking` 對新加的 `required` 項目照樣印出來（info）。
10. 「有正式版標籤就擋」的判斷規則（哪種標籤算）寫進 `docs/testing.md`。
11. 雲端檢查全綠。

## 9. 驗證和回報

- 本機：改到的套件 `cargo clippy -p <套件> --all-targets -- -D warnings`、相關的測試；送雲端前 `make check-fast` 一次。
- 雲端檢查：`python3 ~/.claude/skills/acp-agents/bin/cloud_check.py`（前景，timeout 50 分鐘），全綠才回報。
- 回報：驗收清單逐條寫「做到了，在哪個 commit、哪個檔、哪一行」或「沒做到，原因」；跟藍圖不一樣的地方和理由；要問的問題。
- commit 用 repo 原本的身分，不准加 `-c user.name`、`-c user.email`、`--author`。
