# `apps/desktop` · 桌面殼（Tauri 2，骨架）

照 `spec/06` §2.4 和 `spec/07` §2、§3：

- **視窗只是畫面。** 做事的是背景程式 `apps/agent`；桌面殼不准依賴
  `server`、`store-server`、`runner`、`models`、`tools`、`computers`、`mcp`、
  `external-agents`（由 `tools/depcheck` 擋）。
- 畫面和手機共用同一套：前端只載入 `web/ui` 的外殼（M2 骨架，沒有頁面、路由、功能）。
- `src-tauri/` 是 Rust 套件，加進 Cargo workspace（`anybot-app-desktop`）。

## 跑起來（M2 骨架沒有功能，只開一個視窗）

```sh
pnpm install
cargo install --locked tauri-cli   # AGENTS.md 准許的開發工具
cargo tauri dev                    # 前端 vite（port 1420）＋ Tauri 視窗
cargo tauri build                  # 打包（目前 tauri.conf.json 的 bundle.active=false）
```

`make check` 不含 Tauri 的打包：桌面殼的 Rust 部分走 M1 原本的關卡
（fmt、clippy、nextest、deny、machete、depcheck），前端走 TypeScript 的關卡
（biome、tsc、vitest、vite build）。

## 要注意（`spec/06` §2.4）

- Linux 的 WebKitGTK 比較吃資源、顯示比較不一致，要多測（CI 裝系統套件）。
- 長聊天只畫看得到的訊息、不放常駐動畫：這些是之後寫畫面時的規則。
- 資源上限的關卡（視窗閒置 CPU 接近 0、記憶體 200 MB 以內）等桌面真的有東西再做。
