# 工作狀態記錄（2026-02-24，由 Claude 直接寫程式的路線）

這是一份給人审查、也給下一個 agent 接續用的快照。程式都由 Claude 直接寫（取代原本 pi-bolt 寫、Claude 督的分工），審查由使用者請人做。

## 分支現況

| 分支 | 狀態 |
|---|---|
| `dev` | M1–M4、M5、M6a、M7a、M7b 已合進。M7c/M7d/M6b-1 前兩段**尚未**合進。 |
| `huskyVer` | 已推到 origin（`ab308d5`）。內容＝dev＋ws/m7c＋ws/m6b-1(第1–2段)＋M7d-1～M7d-4 全部。 |
| `ws/infra-2` | 领先 dev 3 筆，未合（INFRA-2 雲端測試多執行緒）。 |
| `ws/rel-3` | 领先 dev 7 筆、落後 5 筆，未合（Windows 修正）。 |

## 已完成的里程碑（全在 huskyVer）

- **M7c 單一入口＋三個工具**：三輪審查修正全做完，18 個編號驗到。
- **M7d-1**：只有對話的一輪（runner 薄切片、20 個測試）。
- **M7d-2**：一步裡的動作（信任入口接上、T-turn-loop-3/9 等）。
- **M7d-3**：多步迴圈＋沙盒規則（T-turn-loop-1 性質 1000 輪、沙盒三測、context 兩測）。
- **M7d-4**：收尾、完成閘門（last_write_ok）、交付報告（trust::report 純函式）、T-turn-loop-2/12、T-task-done-gate-7、描述性測試四份。
- **M7d 補漏**：「要做什麼」早於假電腦操作的性質測試（500 組，turn_intent_prop.rs）；重型性質測試排在 nextest 的 `prop` 群（依序跑，逾時放寬到 300 秒×2）。
- **M6b-1 第 1–2 段**：相容檢查基準化、遷移檢查 sqlparser、設定合併搬 domain＋修正（在 ws/m6b-1 上，已合進 huskyVer）。
- **M6b-1 第 3 段（網路控制補洞）**、**第 4 段（憑證/防跨站 T-conn-auth-1～6）**：已做完並 commit。
- **M6b-1 第 5 段（版本檢查 T-install-n25、伺服器時間）**：domain 版本表＋中介層＋整合測試 6 個全過，收束 commit（K86 改名、同碼雙裝置測）。
- **M6b-1 第 6 段（Bot 和紀錄流）**：append/after/SSE 三個端點（T-sync-append、T-sync-id、T-conn-sse）。整合測試 24 個（append 11＋after 7＋SSE 6）全過；openapi/lint/client/html/相容基準同步更新。

驗證數字：`make check-fast` 899/899 全綠（含新增 24 個串流測試）；depcheck/iron-laws/locale(156)/spec-trace/api-lint/api-compat/biome 全過。

## 還沒做的（接續順序）

1. **把 huskyVer 合進 dev**（雲端檢查全綠後由監督者做）。
2. **M6b-1 第 7 段起（B 段）**：刪除/清除/恢復（§8）、設定（§9）、任務事件與裁判（§10）。
3. **M6b-2a**：主鑰匙、秘密加密、2FA、救回碼（無分支）。
4. **M6b-2b**：裝置、配對、在線偵測、信箱、runner-service 憑證（無分支）。
5. **M6c**：客戶端同步（store-client 是空殼）。
6. **M7e–M7k、M7.5、M8–M10**：卡片、停止/插話/接續、換模型/壓縮、做完的標準、檔案工具、伺服器整合、畫面、大腦。

## 接手要注意的坑

- 測試群組：`t_turn_loop_1`、`t_turn_loop_9`、`turn_intent_record_precedes_computer_operation` 在 `prop` 群依序跑；新加的慢性質測試要排進去，否則 120 秒會互搶逾時。
- `crates/runner/tests/turn_report*.rs` 是拆過的（loop2/gate7/report12＋runner-test 裡的 `delivery_of`）。
- `FakeComputer.set_mutate_after_write(true)` 用來模擬驗收重讀雜湊不相符。
- GateResult 的 `Ask` 判定也會給模型結果（context_steps 的 result_entry），不然上下文配對失敗。
- 交付報告在 `crates/trust/src/report.rs`（純函式，不看時鐘）；runner 端 emit 在 `report_out.rs`。
- 代碼數 156（§6 串流新增 6 個代碼）。
