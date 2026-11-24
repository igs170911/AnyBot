# 里程碑 M3：foundation、protocol，和 API 文件的產生流程

這是第一個寫真程式的里程碑。完成條件照 `docs/plan.md` 開頭（規格測試、三層測試、API 文件、四條鐵律、`make check`）。
先讀：`spec/02` §6（啟動設定）、§8（API 原則）、§11.1（執行事件協定）；`spec/05`；`spec/06` S3；`spec/07`。

## 要做的

### 1. `crates/foundation`
- **ID**：依時間排序的 UUIDv7。每種東西各一個型別（帳號、裝置、Bot、任務、紀錄…），不能互相混用。能產生、解析、比較先後、序列化。
- **時間**：時鐘的介面，加上真的時鐘。假時鐘留給 M4，但介面這次要定好。
- **錯誤**：共用的錯誤分類。
- **金鑰型別**：
  - 印出來永遠是遮掉的。
  - **沒有**序列化（直接序列化就編譯不過）。
  - 要用明文時，必須呼叫一個明確的方法。
  - 用完清掉記憶體（zeroize）。
- **啟動設定**：
  - 客戶端讀 `--data-dir`。
  - 伺服器讀 `--bootstrap <檔案>`；檔案裡只有資料庫的連線位址，用金鑰型別裝。
  - 不讀環境變數。

### 2. `crates/protocol`
- **執行事件協定**（`spec/02` §11.1）：回合開始／結束、文字片段、思考摘要、工具呼叫開始／結果、要核准（卡片）、證據、任務狀態變更、錯誤。每個事件都有遞增編號，協定有版本號。
- **API 的共用型別**：
  - 錯誤回應用 `application/problem+json`（RFC 9457）。
  - `Idempotency-Key`。
  - 分頁游標。
  - SSE 事件的外框：編號、種類、內容。

### 3. API 文件的產生流程（`spec/02` §8、`spec/06` S3）
- **第一個 API**：`GET /v1/health`。回傳伺服器版本和現在時間；`crates/server` 只放 axum 的骨架和這一個路由，不接資料庫。
- **OpenAPI 3.1 由程式產生**（utoipa）。
  - 寫到 `api/openapi.yaml` 並 commit。
  - 關卡：重新產生的內容跟 commit 的不一樣就擋。
  - **說明文字用 utoipa 屬性裡的字串寫，不用 `///` 註解**（四條鐵律第 3 條）。
- **API 文件的 lint**：`@redocly/cli`，裝成專案的 devDependency。每個 API 都必須有這些，缺一個就擋：
  - `operationId`
  - 摘要、說明、標籤
  - 請求和回應的範例
  - 4xx 和 5xx 錯誤回應（problem+json）
- **HTML 文件**：產生到 `api/docs/index.html` 並 commit；一樣有「不一致就擋」的關卡。
- **TypeScript 客戶端**：`openapi-typescript` 產生型別到 `web/api-client`，用 `openapi-fetch` 包成客戶端；一樣有「不一致就擋」的關卡。
- **相容性檢查**（6 個月相容期，`spec/02` A14）：用 oasdiff（可以 `brew install oasdiff`）跟 `SPEC_TRACE_REF` 那個 ref 的 `api/openapi.yaml` 比，有破壞性變更就擋。
- **AsyncAPI**：
  - 執行事件協定（走 SSE）寫成 `api/asyncapi.yaml`。
  - 事件內容的 JSON Schema 由 Rust 型別產生（schemars）。
  - 關卡兩道：每種事件都要在 AsyncAPI 裡；產生的內容跟 commit 的不一樣就擋。
- 以上全部接進 `make check`；`make check-fast` 至少包含「不一致就擋」那幾道。

### 4. 測試
- **單元**：ID 的先後順序和解析；金鑰型別印出來是遮掉的、不能序列化（用編譯期測試）；啟動設定的各種錯誤；每種事件序列化以後再讀回來，內容不變。
- **整合**：在隨機連接埠啟動伺服器，打 `GET /v1/health`，回應符合 OpenAPI 的定義。
- **規格測試**：到 `spec/10-behavior/` 找這次範圍對得到的編號（例如 `--data-dir`、`--bootstrap`、不讀環境變數、金鑰不能印出來、`Idempotency-Key`、事件編號這類），寫成測試；在回報裡列出你對到哪些編號。對不到的不要硬湊。

## 不要做
- 資料庫、帳號、其他任何 API（M6）。
- 測試驅動程式和假的外部服務（M4）。

## 回報
照 `AGENTS.md` 第六節；另外附上：這次對到的規格編號清單、spec-trace 覆蓋數字的前後變化、API 文件的檔案位置。
