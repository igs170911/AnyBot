# M6a 中途修正

你做了 148 分鐘，還沒有任何 commit；最後 30 多分鐘都在重跑同一個測試 `scratch_down_proxy`（14 次，每次 100 秒）。我把你停下來了。工作區的 64 個改動都還在，接著做，但先照下面改方向。

## 1. 不要關掉代理的埠再搶回來

你為了「資料庫連不上、之後恢復」（`T-server-n8-3`），把 `tests/harness/src/netctl.rs` 改成把代理的埠真的關掉、之後再綁回同一個埠（`rebind_with_retry`、`bind_loop`、`std_bind`、`close_listener`）。這個做法不要了：

- 埠關掉之後不保證搶得回來：系統會保留一陣子，別的程式（包括別的工作區的測試）也可能先拿走。
- harness 本來就有這個功能：`net_down` 和 `net_restore`。代理一直開著，停機時新的連線一進來就被關掉、現有的連線被切斷，對外看起來就是連不上。M4 的 `netctl_real` 已經用真的連線測過，Linux 上也過了。

請改成：

1. `tests/harness/src/netctl.rs` 和 `netproxy.rs` 回到 `dev` 上的版本，拿掉你加的關埠、重綁那一套。
2. `T-server-n8-3` 的「資料庫連不上」：在 PostgreSQL 前面放一個代理，用 `net_down` 讓它連不上，用 `net_restore` 讓它上線。
3. 刪掉 `tests/harness/tests/scratch_down_proxy.rs`、`scratch_raw_echo.rs` 這些試驗用的檔。

## 2. 「20 秒後」不准真的等 20 秒

- 「每 5 秒再試」「20 秒後上線」用假時鐘：伺服器當函式庫啟動、傳假時鐘進去，快轉時間。這種測試幾秒內就要跑完。
- `T-server-n8-3` 的前半（資料庫有別的程式的資料 → 結束碼 2）要看執行檔的結束碼，用執行檔測；後半（連不上 → 每 5 秒再試 → 上線後正常啟動）用函式庫加假時鐘測。兩半都驗到，才算這一列驗完。
- 除了啟動 PostgreSQL 本身，任何一個測試都不該跑超過 10 秒。

## 3. 先合 `dev`

`dev` 現在有三樣你沒有的東西，越晚合衝突越多，**現在就 `git merge dev`**：

- GitHub 的自動檢查（`.github/workflows/ci.yml`）。你加的東西如果在 CI 上需要新的工具（例如 sqlx-cli），要一起加進工作流程：固定版本、下載後驗 sha256、第三方 action 釘在完整的 commit 雜湊（iron-laws 會擋）。
- 模型閘道（M7a）：它也改了 `tests/harness`（假模型的腳本、`netctl` 加了 `net_hang_connect`、`net_alias`）。`netctl.rs` 的衝突一律用 `dev` 的版本。
- 新的分支流程（`spec/09` K111、`AGENTS.md` 第七節）：之後回報前要合的是 `dev`，不是 `main`。

## 4. 做事的方式

- **早點 commit。**A 段做完、`make check` 全綠就 commit，不要等。現在的目標是用最短的路讓 A 段全綠。
- **卡住超過 15 分鐘就換做法**：先找 harness 裡現成的功能；沒有的話選最簡單的做法；還是不行就寫進回報，先做別的項目。不准同一個測試一直重跑。
- **你是一次性執行的**：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長，不准結束回合去等通知。
- 測試啟動的程式（PostgreSQL、伺服器）要確定有收掉。這次被停下時留了兩個沒收的。

## 其他照舊

`docs/tasks/M6a-server-base.md` 照做：A 段 commit 一次、B 段 commit 一次，最後回報。回報多加一段「中途修正」。
