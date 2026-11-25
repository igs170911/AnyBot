# CI-1 第三輪審核後要修的

版面的修正方向是對的，本機也做了「紅 → 綠」的驗證，很好。下面是審核後要改的，全部做完 `make check` 全綠，commit 一次。

先做：`git merge dev`。`dev` 有新的規格段落 `phonefit-width`（`spec/10-behavior/5-devices-account.md` 第 8 節）和這份任務。

## 1. action 換成跑在 Node.js 24 的版本（使用者指出的）

GitHub 上的執行有警告：`actions/cache` v4.3.0 跑在已經淘汰的 Node.js 20。我查過官方 repo，請換成下面這些（都是 `using: node24`）：

| action | 版本 | 完整的 commit 雜湊 |
|---|---|---|
| `actions/checkout` | v7.0.1 | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| `actions/cache`、`actions/cache/restore`、`actions/cache/save` | v6.1.0 | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` |
| `actions/upload-artifact` | v7.0.2 | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` |

- 每個 action 跨了大版本，先讀它的 README 和版本說明，確認我們用到的參數（`path`、`key`、`restore-keys`、`fetch-depth`、`name`、`retention-days`、`cache-hit` 輸出）在新版還是一樣的用法。有不一樣的照新版改，寫進回報。
- 以後選 action 的版本，一律用當時最新的正式版，並確認它不是跑在已淘汰的 Node 版本。

## 2. 版面：不准用「藏起來」的方式

- `shell.css` 的 `overflow-x: hidden` 拿掉。它只是把超出的部分裁掉，使用者會看不到內容（`phonefit-width` S5）。靠換行和寬度限制讓內容真的留在畫面裡。
- `expectNoHorizontalScroll` 加強（`phonefit-width` 的「永遠成立」）：除了頁面沒有橫向捲動，還要檢查畫面上每個看得到的元素，左右邊界都在畫面寬度以內。這樣內容被裁掉也會紅。
- 確認拿掉 `overflow-x: hidden` 之後，`phone-max-font` 和 `phone-wide-font` 都還是綠的。

## 3. 規格編號

- 你自己編了 `T-phonefit-width-1`、`T-phonefit-width-2`，當時規格裡沒有。規格沒有的編號不准自己編（`AGENTS.md` 第二節第 4 條、`spec/09` K86）。
- 現在規格補上了 `phonefit-width`，只有一列測試：`T-phonefit-width-1`。把寬字型那個 e2e 測試對到它，照那一列的預期全部驗到。
- `apps/web/src/shell.css.test.ts` 刪掉：它只檢查 CSS 檔裡有沒有某幾個字，不是在驗行為。版面被改壞，e2e 那個測試會紅。
- spec-trace 現在只認「整個標題剛好就是編號」的 TypeScript 測試，跟 `AGENTS.md` 寫的「標題開頭寫編號」不一致。改成：標題以編號開頭，後面接空白、冒號或直接結束，都算。附測試：`T-a-b-1`、`T-a-b-1: 說明`、`T-a-b-1 說明` 都認得；`T-a-b-10` 不會被當成 `T-a-b-1`；小寫的 `t-a-b-1` 不算。
- 做完跑 `make spec-baseline`，覆蓋數字應該多 1。

## 4. 你又按樣式殺程式了

- 你跑了 `pkill -g 0`。`AGENTS.md` 第一節第 9 條寫的是**不准用 `pkill`**，沒有例外，`-g 0` 也不行。你有老實回報，這點是對的。
- 以後要在背景啟動程式（例如預覽伺服器）：啟動時記下 PID，用完用那個 PID 收掉。

## 回報

照 `AGENTS.md` 第六節，逐項對照實際檔案。另外附上：
- 三個 action 新舊版本之間，你確認過的參數差異。
- 加強後的檢查，在還沒修版面時會紅在哪裡（實際跑一次）。
