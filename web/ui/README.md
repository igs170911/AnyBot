# `web/ui` · 共用畫面元件庫（骨架）

照 `spec/06` §2.4、`spec/07` §2：桌面（`apps/desktop`）和手機／瀏覽器（`apps/web`）**共用同一套畫面**，
共用的元件放這裡。

M2 是骨架：只有 `src/shell.tsx` 一個外殼元件，**沒有任何頁面、路由、功能**。
之後的功能畫面都加進這個套件，兩邊一起用。

- 建置：`vite build`（lib mode，輸出 `dist/index.js`）。
- 測試：`vitest run`（用 `react-dom/server` 的 `renderToString`，不需要瀏覽器）。
- 桌面和手機的測試在之後的里程碑用 Playwright 端對端一起測（`spec/06` §2.7）。
