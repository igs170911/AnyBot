# `web/api-client` · API 客戶端（骨架）

照 `spec/06` §2.4 和 `spec/07` §2：這個套件是**由 `api/openapi.yaml` 產生的** TypeScript 客戶端，
資料抓取用 TanStack Query（之後的里程碑補）。

## 產生來源

- **來源**：`api/openapi.yaml`（唯一來源）。
- 那份 openapi.yaml 本身**由 `crates/protocol` 產生後提交**（`spec/07` §2：`api/` = 由程式產生、提交）。
  目前 `api/` 還沒有 openapi.yaml，要在之後的里程碑由 protocol 產生出來，這裡才會有真正的客戶端。
- M2 的骨架只有型別佔位（`src/index.ts`）：`createApiClient()` 在產生之前一律失敗，
  避免有人在骨架階段假裝有 API 可用。

## 指令（產生客戶端）

客戶端產生之後，每次 `api/openapi.yaml` 更新就要重新產生，並把產生的檔一起提交：

```sh
# 型別（openapi-typescript 產生到 src/generated/types.ts）
pnpm exec openapi-typescript api/openapi.yaml -o web/api-client/src/generated/types.ts
```

> 這個指令要用的工具（`openapi-typescript`）和產生出點的 wrapper，會在「API 開放」的那個里程碑
> 加進這個套件的 `devDependencies` 與 `generate` script；M2 骨架階段不預裝、不預宣告
> （照 `AGENTS.md` 二：只加真的用到的）。
>
> 同一個里程碑也要補 `spec/05` §2 的兩條關卡：每個寫入 API 帶 `Idempotency-Key`、
> API 相容 6 個月（比對新舊 openapi.yaml）。

## 測試

`vitest run`（M2 只測佔位行為：產生之前不能建立客戶端）。
