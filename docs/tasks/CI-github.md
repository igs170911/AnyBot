# CI-1：GitHub 上的自動檢查

repo 現在放在 GitHub（公開）。CI 從 Gitea Actions 改成 GitHub Actions（`spec/09` K110）：公開 repo 用標準 runner 免費。

這次只做「每次推上去跑 `make check`」。版本標籤觸發的編譯和發佈是下一個任務（REL-1），這次不做。

兩個背景：
- `.gitea/workflows/ci.yml` 從來沒有真的跑過。
- 到目前為止，`make check` 只在 macOS 上跑過，**Linux 上一次都沒跑過**。這次第一次上 Linux，預期會有幾處要修。

你在並行工作（`AGENTS.md` 第七節）：自己的工作區和分支。

先讀：
- `AGENTS.md`。
- `spec/09` K110；`spec/06` 的「CI 和發佈」那一列；`spec/07` 的 `.github/workflows/`。
- `Makefile`、`.gitea/workflows/ci.yml`、`docs/testing.md`、`.config/nextest.toml`。

## 要做的

1. **`.github/workflows/ci.yml`**：
   - 觸發：推任何分支、任何 PR。
   - 一個工作，在 Linux 的標準 runner 上跑 `make check`。runner 的系統版本要寫死（例如 `ubuntu-24.04`），不用 `latest`。
   - 同一個分支有新的推送時，取消還在跑的舊的那次。
   - 把 `.gitea/` 刪掉；`docs/testing.md` 和其他提到 `.gitea` 的地方一起改。
2. **安全規則**（公開 repo，`spec/09` K110 ④）：
   - 工作流程最上面寫 `permissions`，只給 `contents: read`。
   - 每個工作都有 `timeout-minutes`。
   - 第三方 action 一律釘在**完整的 commit 雜湊**（40 個十六進位字），後面用註解寫版本號。優先用 GitHub 官方的 action（`actions/*`）；要用別人的，照 `AGENTS.md` 先查維護者和使用量，寫進回報。
   - 直接下載的工具（gitleaks、oasdiff 這類）固定版本，下載後**驗 sha256**。
   - 這個工作流程不用任何金鑰。
3. **用工具擋**：iron-laws 加一道檢查，掃 `.github/workflows/*.yml`：
   - 檔案是合法的 YAML。
   - 最上面有 `permissions`。
   - 每個工作有 `timeout-minutes`。
   - 每個 `uses:` 都釘在 40 個十六進位字的雜湊（`./` 開頭的本地 action 除外）。
   - 附「塞進去會被擋」的測試：沒釘雜湊、沒有 `permissions`、沒有 `timeout-minutes` 各一個。
4. **速度**：
   - 快取：cargo 的下載和編譯輸出、pnpm 的下載、Playwright 的瀏覽器、測試用 PostgreSQL 的下載。
   - cargo 的工具（nextest、deny、machete）用現成的執行檔，不要每次從原始碼編。
   - 公開 repo 的 Linux runner 是 4 核，比開發機慢。目標：有快取時 15 分鐘以內。實際數字由監督者跑完告訴你。
5. **spec-trace 比對的基準**（`SPEC_TRACE_REF`）：
   - 推到 `main`：跟推之前的那個 commit 比（`github.event.before`）。第一次推、或那個 commit 不存在時，退回跟 `HEAD` 比。
   - 其他分支和 PR：跟 `origin/main` 的共同祖先比。
   - checkout 要抓完整歷史。
6. **先自己找 Linux 上會不一樣的地方**，能改的先改成兩邊都能跑。至少檢查：
   - `Makefile` 和腳本裡 macOS 專用的寫法：`sed -i ''`、`stat -f`、`date`、`mktemp` 的參數、`lockf`／`flock`。
   - 測試用 PostgreSQL 的 socket 路徑：Linux 的上限大約 108 個字元，runner 的工作目錄比較長（`/home/runner/work/<repo>/<repo>/…`）。算一下最長的那個測試會不會超過，會的話縮短。
   - 伺服器預設同時聽 `127.0.0.1` 和 `::1`：runner 上 `::1` 不一定能用。不准改產品行為；先寫進回報，由監督者看實際跑的結果。
   - Tauri 在 Linux 要的系統套件（`.gitea` 那份已經列了，確認還對）。
   - Playwright 的 WebKit 在 Linux 要系統套件（`--with-deps`）。

## 怎麼驗證

- **你不能 push，也不要用 `gh`。**你在本機能做的：
  - `make check` 全綠（macOS）。
  - iron-laws 的新檢查和它的測試。
  - 自己逐行讀過工作流程。
- 你回報後，由監督者把你的分支推上 GitHub 實際跑。失敗的紀錄會放進你工作區的 `target/ci-logs/`，再請你接著修。所以第一次回報不用保證 Linux 一次就過，但要把你擔心的地方列出來。

## 這次不做

- 版本標籤觸發的編譯、GitHub Release、Docker 映像、簽章：REL-1。
- Windows、macOS 的 runner：REL-1。
- 不改產品行為。

## 回報

照 `AGENTS.md` 第六節，逐項對照實際檔案。另外附上：
- 工作流程每一步在做什麼。
- 每個 action 和下載的工具：版本、commit 雜湊或 sha256、雜湊從哪裡查到的。
- 你改了哪些 macOS 專用的寫法。
- 你擔心 Linux 上會失敗的地方。
