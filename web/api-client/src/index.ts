/**
 * `web/api-client` · 由 `api/openapi.yaml` 產生的 API 客戶端（`spec/06` §2.4）。
 *
 * M2 是骨架：這裡只有型別佔位，沒有任何端點、沒有任何資料抓取。
 * 產生來源和指令見同目錄的 README.md。
 */

/** 客戶端建立時的選項（之後由 openapi.yaml 產生的實作會用同一個形狀）。 */
export interface ApiClientOptions {
  /** 伺服器 API 的網址。 */
  readonly baseUrl: string;
}

/** 這個套件還沒有產生真正的客戶端（M2 骨架）。產生之後這個值是 true。 */
export const apiClientGenerated: boolean = false;

/**
 * 佔位：客戶端產生之前呼叫它一律失敗，避免有人在骨架階段「假裝」有 API 可用。
 */
export function createApiClient(_options: ApiClientOptions): never {
  throw new Error("API 客戶端尚未產生：請先照 README.md 從 api/openapi.yaml 產生");
}
