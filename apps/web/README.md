# `apps/web` · PWA（手機和瀏覽器，骨架）

照 `spec/06` §2.4：手機不做原生 App（A19），用 PWA（含 Service Worker、Web Push）。

M2 是骨架：畫面只有 `web/ui` 的外殼，**沒有頁面、沒有路由、沒有功能**；
資料抓取（TanStack Query）和 API 客戶端等 `api/openapi.yaml` 產生的那個里程碑再補。

```sh
pnpm --filter @anybot/web build     # dist/ ＋ sw.js ＋ manifest.webmanifest
pnpm --filter @anybot/web preview   # 本機看 PWA
```

Service Worker 用 `vite-plugin-pwa`（`registerType: "autoUpdate"`），
Web Push 的實作留給之後（規格沒寫之前不猜）。
