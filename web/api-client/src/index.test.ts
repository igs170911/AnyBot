import { describe, expect, it } from "vitest";
import type { paths } from "./generated/schema.ts";
import { apiClientGenerated, createApiClient } from "./index.ts";

describe("api-client 已產生", () => {
  it("產生標記是 true，客戶端有健康檢查路徑的型別", () => {
    expect(apiClientGenerated).toBe(true);
    const client = createApiClient({ baseUrl: "http://127.0.0.1:8787" });
    expect(client.GET).toBeTypeOf("function");
    const pathKey: keyof paths = "/v1/health";
    expect(pathKey).toBe("/v1/health");
  });
});
