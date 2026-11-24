import { describe, expect, it } from "vitest";
import { apiClientGenerated, createApiClient } from "./index.ts";

describe("api-client 骨架", () => {
  it("還沒有產生：標記是 false，建立客戶端一律失敗", () => {
    expect(apiClientGenerated).toBe(false);
    expect(() => createApiClient({ baseUrl: "http://127.0.0.1:1" })).toThrow(/api\/openapi\.yaml/);
  });
});
