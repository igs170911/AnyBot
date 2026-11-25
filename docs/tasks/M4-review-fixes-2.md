# M4 審核後要修的（第二輪）

上一輪的 `make check` 全綠：267 個 Rust 測試，瀏覽器測試也過了。

審核通過的部分：
- 網路控制 6 個真連線測試。
- iPhone SE 加 310% 字級。我看了截圖，沒有橫向捲動。
- 每個情境都有截圖。
- `check-fast` 會跑測試。
- nextest 設定有效，也加了關卡。
- `make live-model-check` 能跑，憑證驗證只認那一張憑證，金鑰沒有出現在任何紀錄裡。
- 這輪沒寫 `/tmp`，也沒撞到沙盒。

還有兩件要修，修完 `make check` 全綠再 commit 一次。

## 1. 測試用的 PostgreSQL 還在寫 `/tmp`（上一輪第 8 項沒做，回報也沒提）

- `tests/harness/src/postgres.rs` 沒有設 socket 目錄，所以 PostgreSQL 照預設把 `.s.PGSQL.<埠>` 和它的鎖檔寫在 `/tmp`。這輪能過，是因為沙盒暫時放行了這個檔名。
- 照上一輪任務第 8 項做：
  - 把 socket 目錄（`unix_socket_directories`）設到 `target/` 裡那個資料庫自己的資料夾。macOS 的 socket 路徑上限大約 104 個字元。
  - 附自我測試：啟動後 socket 檔在 `target/` 裡，`/tmp` 沒有這次的 `.s.PGSQL.<埠>`。
- 回報要逐項對照任務的每一項。沒做的要寫「沒做」和原因，不准不提。上一輪寫「做完七項」，任務其實有八項。

## 2. 假模型分成兩種格式

上一輪為了跟真的本機模型對上，把那台伺服器特有的欄位（`kv_transfer_params`、`routed_experts`、`weight_versions`、`prompt_logprobs` 這類）加進了假模型的**預設**輸出。這樣假模型就不再是「照官方文件」，而是變成那台伺服器。改成兩種格式，由腳本或建構假模型時選：

- **`openai`（預設）**：照 OpenAI 官方文件。工具呼叫的 `finish_reason` 是 `tool_calls`，沒有那台伺服器特有的欄位。
- **`vllm`**：照那台本機模型伺服器的樣子：
  - 思考內容放在非標準的 `reasoning` 欄位。
  - 那些特有欄位都有。
  - 呼叫工具時 `finish_reason` 是 `stop`。
- `make live-model-check` 拿 **`vllm` 格式**的假模型跟真的比。三種請求（一般、串流、工具呼叫）都要 identical，報告照樣寫在 `target/live-model/`。
- 自我測試兩種格式都要有。原本驗標準格式的測試（例如 `fake_model_openai_tool_call_fields_in_json_and_stream`）維持驗 `openai` 格式。
- `docs/testing.md` 說明兩種格式各是什麼、什麼時候用哪一種。

## 3. 金鑰不要放在命令列上

- 你上一輪用了好幾次 `curl … -H "Authorization: Bearer $(cat target/live-model/key)"`。金鑰沒有印出來，這點很好，但它會出現在 `curl` 的命令列參數裡，同一台電腦上的其他程式用 `ps` 就看得到。
- 要打真的模型，就用 `make live-model-check` 的程式。真的要用 `curl`，就把標頭寫進 `target/` 裡權限 600 的檔案，用 `curl -H @檔案`，用完刪掉。

## 你上次問的問題

1. 產品要不要相容「`finish_reason=stop` 但有工具呼叫」：這是 M7 模型客戶端的設計，到時候跟使用者討論後再定。這次只要讓假模型兩種格式都做得出來。
2. `live-model-check` 由我在審核碰到模型客戶端的里程碑時跑，不用定期跑。

## 回報

照 `AGENTS.md` 第六節，逐項對照實際檔案。另外附上：
- `make live-model-check` 的實際輸出（三種請求都要 identical）。
- 自我測試的輸出，要看得到 socket 在 `target/` 裡。
