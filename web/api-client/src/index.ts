export interface ApiClientOptions {
  readonly baseUrl: string;
}
export const apiClientGenerated: boolean = false;
export function createApiClient(_options: ApiClientOptions): never {
  throw new Error("API 客戶端尚未產生：請先照 README.md 從 api/openapi.yaml 產生");
}
