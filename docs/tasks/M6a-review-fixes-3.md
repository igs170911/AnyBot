# M6a 審核修正（第 3 輪，小的，修完就合）

第 2 輪的回報很誠實，安全那幾項也都證明了改壞會變紅。審查的結論是「小修之後就合」：帳號和登入沒有剩下的安全洞，遷移只加不刪，`returned.txt` 沒辦法用來藏東西，`t_safety_n10_6`、`t_account_login_9`、`t_safety_n10_2` 站得住。

這一輪只做下面三節。其他想到的寫在回報裡，不要做。行號是 `ws/m6a` 現在的位置，動手前再對一次。

先做：`git merge dev`。`spec/09` 的 K116 ⑤ 改了：還沒登入、但請求裡指名帳號的（登入、註冊），`Idempotency-Key` 綁正規化之後的帳號名稱，就是你現在的做法，不用改程式。

## 一、GitHub 上的檢查（擋著合併）

`make check` 在 GitHub 上停在「install sqlx-cli」：工具的原始碼解壓縮到專案資料夾裡面，cargo 以為它屬於這個 workspace，拒絕編譯（`current package believes it's in a workspace when it's not`）。解壓縮到專案外面（例如 `$RUNNER_TEMP`），用完照樣清掉。紀錄在 `target/ci-logs/m6a-ci-2-failed.log`。

## 二、規格編號放回基準檔

下面 6 個放回 `tools/spec-trace/baseline.txt`（照你做的 `returned.txt` 規則），測試改描述性的名字：

| 編號 | 為什麼還不算 |
|---|---|
| `T-conn-post-1` | 那一列是整合：同一張卡片的答案重送三次。測試是單元測試，只驗指紋不含密碼 |
| `T-account-login-1` | 那一列是端對端、經過畫面；測試只打 API |
| `T-account-n10-1` | 同上；另外密碼用 3 個字（那一列是 7 個字）、沒驗「兩次輸入不一樣」、把一個登入的工作階段當成「電腦 A」 |
| `T-server-n1-2` | 轉送的標頭從頭到尾沒變，伺服器完全不看 `X-Forwarded-For` 也會過（用 loopback 代替 `10.0.0.9` 沒問題，K108 ⑤ 允許） |
| `T-safety-n11-2` | 唯一分得出「取最後一個值」和「取第一個值」的那一步之前，時間窗被清掉了，取第一個值的伺服器也會過 |
| `T-server-invite-2` | 用產品自己的檢查函式驗邀請碼，它跟產生器讀同一份字母表。要嘛形狀和不准用的字元寫死在測試裡、證明會失敗，要嘛放回去 |

`T-server-n1-2`、`T-safety-n11-2`、`T-server-invite-2` 如果你這一輪就能改到整列、證明會失敗，可以不放回；改不到就放回。

## 三、收殘留程序的那一段（擋著合併）

`tests/harness/src/reap.rs` 大約第 86 行：讀不到那個程序的執行檔路徑時，當成「對得上」，照樣送結束訊號。pid 檔在正常結束時沒有刪掉；如果舊的 PID 之後被同一個使用者的別的程式重複使用，而查詢剛好失敗（`lsof` 寫死在 macOS 的路徑、沙箱也可能擋），那個不相干的程式就會收到結束訊號。

- 讀不到執行檔路徑 → 跳過這個 PID，不送訊號。
- 測試工具正常結束時刪掉自己的 pid 檔。
- 加測試：pid 檔裡記的 PID 是一個不相干的程序（測試自己另外啟動一個無害的程序、PID 寫進 pid 檔、讓查詢失敗）→ 那個程序沒有收到訊號。證明會失敗。

## 驗證和回報

`make check` 全綠就 commit；`git merge dev`、`make check`；簡短回報（三節怎麼改的、放回了哪幾個編號、「會失敗」的表）。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長（30 分鐘）。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`（要停程序只能指定 PID）。遇到「Operation not permitted」是沙箱在擋，不要繞過。
