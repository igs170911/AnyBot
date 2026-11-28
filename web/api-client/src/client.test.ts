import { describe, expect, it } from "vitest";

import { createAnyBotClient } from "./client.ts";
import { attempt, problemOf } from "./problem.ts";

function recordingFetch(seen: Request[], reply: () => Response) {
  return async (request: Request): Promise<Response> => {
    seen.push(request);
    return reply();
  };
}

function jsonReply(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("createAnyBotClient", () => {
  it("client adds version and csrf headers", async () => {
    const seen: Request[] = [];
    const client = createAnyBotClient({
      clientVersion: "0.1.0",
      baseUrl: "http://127.0.0.1:1",
      fetch: recordingFetch(seen, () => jsonReply(200, { ok: true })),
    });
    await client.POST("/v1/account/logout", {
      params: { header: { "Idempotency-Key": "k1" } },
      body: { ok: true },
    });
    await client.GET("/v1/health");
    const post = seen[0];
    const get = seen[1];
    expect(post?.headers.get("AnyBot-Client-Version")).toBe("0.1.0");
    expect(post?.headers.get("AnyBot-CSRF")).toBe("1");
    expect(get?.headers.get("AnyBot-Client-Version")).toBe("0.1.0");
    expect(get?.headers.get("AnyBot-CSRF")).toBeNull();
  });

  it("errors become code and params from the problem body", async () => {
    const problem = {
      code: "account-login-credentials",
      params: { minutes: 5 },
      status: 401,
      title: "t",
      type: "x",
    };
    const client = createAnyBotClient({
      clientVersion: "0.1.0",
      baseUrl: "http://127.0.0.1:1",
      fetch: recordingFetch([], () => jsonReply(401, problem)),
    });
    const result = await attempt(() => client.GET("/v1/settings"));
    expect(result).toEqual({
      ok: false,
      problem: { code: "account-login-credentials", params: { minutes: 5 }, status: 401 },
    });
  });

  it("a body without a code falls back to the internal problem code", () => {
    expect(problemOf("nope", 502)).toEqual({ code: "problem-internal", params: {}, status: 502 });
  });

  it("a network failure becomes the internal problem code with status 0", async () => {
    const client = createAnyBotClient({
      clientVersion: "0.1.0",
      baseUrl: "http://127.0.0.1:1",
      fetch: () => Promise.reject(new Error("offline")),
    });
    const result = await attempt(() => client.GET("/v1/settings"));
    expect(result).toEqual({
      ok: false,
      problem: { code: "problem-internal", params: {}, status: 0 },
    });
  });
});
