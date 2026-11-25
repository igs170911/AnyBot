# CI-1 第二輪：照新的分支流程改

第一輪審過了：兩個 action 的雜湊、五個工具的 sha256，我都對過官方來源，全部相符；iron-laws 的新關卡和測試也都在。

你做完之後，使用者定了新的分支流程（`spec/09` K111）：工作分支 → `dev` → `main`。這一輪把工作流程改成照它走，順便修幾個小地方。做完 `make check` 全綠，commit 一次。

先做：`git merge dev`（`dev` 有 K111、新的 `AGENTS.md` 第七節）。之後你的分支以 `dev` 為準，不是 `main`。

## 要改的

1. **觸發條件**（K111 ②③⑤）：
   - `push`：所有分支都跑，但**不要**在標籤上跑（標籤是之後發佈用的工作流程在處理）。
   - `pull_request`：只對目標是 `dev` 的 PR 跑。
2. **`SPEC_TRACE_REF`**：
   - 推到 `dev` 或 `main`：跟推之前的那個 commit 比。
   - 其他分支和 PR：跟 `origin/dev` 的共同祖先比。
   - `github.event.before` 不要直接寫進指令裡：先放進步驟的 `env`，指令裡再用環境變數讀。直接把 `${{ … }}` 寫進 shell 是 GitHub 文件說要避免的寫法。
3. **快取要守住 10 GB**：GitHub 每個 repo 的快取上限是 10 GB，而且快取照分支分開存；只有預設分支（`main`）存的快取，所有分支都讀得到。
   - 編譯輸出的快取，key **不要**含 commit 雜湊。現在每個 commit 都存一份 5–6 GB，馬上就會把別的擠掉。
   - 大的快取（編譯輸出）**只在 `main` 存**，其他分支只讀不存。用 `actions/cache` 的 `restore` 和 `save` 兩個子 action 分開做，一樣要釘雜湊。
   - CI 裡關掉增量編譯（`CARGO_INCREMENTAL=0`）：CI 每次都是新的 checkout，增量編譯沒有用，只會讓快取變大。
   - 小的快取（cargo 下載、pnpm、Playwright、PostgreSQL）可以每個分支各存。
4. **你回報裡有一處說錯**：`phone-max-font` 那個情境用的是 iPhone SE，也就是 **WebKit**，不是全部都用 chromium。所以 WebKit 一定要裝，維持現在這樣。回報要對照實際檔案。

## 你上次問的問題

1. repo 叫 `AnyBot`，runner 上的路徑是 `/home/runner/work/AnyBot/AnyBot`，socket 路徑不會太長。
2. 快取照上面第 3 點改。
3. `::1` 等第一次實際跑的結果。
4. `selftest.spec.ts` 只在 `desktop-large` 跑，其他情境略過，是設計如此。
5. `make live-model-check` 不進 CI，你做的對。
6. pnpm 維持現在的裝法（版本寫死）。

## 驗證

跟上一輪一樣：你不能 push、不要用 `gh`。回報後由監督者推上 GitHub 實際跑，失敗的紀錄會放進你工作區的 `target/ci-logs/`。

## 回報

照 `AGENTS.md` 第六節，逐項對照實際檔案。
