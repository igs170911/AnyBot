import { describe, expect, it } from "vitest";
import type { paths } from "./generated/schema.ts";
import { apiClientGenerated, createApiClient } from "./index.ts";

describe("api-client is generated", () => {
  it("the generated marker is true and the client has types for the health path", () => {
    expect(apiClientGenerated).toBe(true);
    const client = createApiClient({ baseUrl: "http://127.0.0.1:32411" });
    expect(client.GET).toBeTypeOf("function");
    const pathKey: keyof paths = "/v1/health";
    expect(pathKey).toBe("/v1/health");
  });
});
