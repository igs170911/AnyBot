import { createAnyBotClient } from "@anybot/api-client";
import { describe, expect, it } from "vitest";

import { type LoginContext, runLogin } from "./flow.ts";
import type { LoginInput } from "./form.ts";

const ORIGIN = "http://localhost:8787";

interface Seen {
  readonly path: string;
  readonly body: unknown;
}

interface Script {
  readonly status?: Response;
  readonly register?: Response;
  readonly login?: Response;
  readonly down?: boolean;
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const STATUS_OK = json(200, { version: "0.1.0", protocol_version: 1, admin_account: "alice" });
const SESSION = { account: "bob", device_id: "d1" };

function problem(status: number, code: string): Response {
  return json(status, { type: "x", title: "t", status, code, params: {} });
}

function contextFor(script: Script, seen: Seen[]): LoginContext {
  const route = (path: string): Response => {
    if (path === "/v1/server/status") {
      return script.status ?? STATUS_OK;
    }
    if (path === "/v1/account/register") {
      return script.register ?? json(201, { created: true, admin: false });
    }
    return script.login ?? json(200, SESSION);
  };
  const serve = async (request: Request): Promise<Response> => {
    if (script.down === true) {
      throw new TypeError("network down");
    }
    const path = new URL(request.url).pathname;
    const body: unknown = request.method === "POST" ? await request.clone().json() : null;
    seen.push({ path, body });
    return route(path).clone();
  };
  return {
    origin: ORIGIN,
    clientAt: (url) => createAnyBotClient({ clientVersion: "0.1.0", baseUrl: url, fetch: serve }),
  };
}

function input(overrides: Partial<LoginInput>): LoginInput {
  return {
    url: ORIGIN,
    account: "Bob",
    password: "longenough",
    createAccount: false,
    invite: "",
    ...overrides,
  };
}

function bodiesAt(seen: Seen[], path: string): unknown[] {
  return seen.filter((entry) => entry.path === path).map((entry) => entry.body);
}

describe("runLogin signing in", () => {
  it("login lowercases the account name", async () => {
    const seen: Seen[] = [];
    const outcome = await runLogin(
      contextFor({}, seen),
      input({ createAccount: true, invite: "k" }),
    );
    expect(outcome).toEqual({ kind: "signedIn", account: "bob" });
    expect(bodiesAt(seen, "/v1/account/register")).toEqual([
      { account: "bob", password: "longenough", invite: "k" },
    ]);
    expect(bodiesAt(seen, "/v1/account/login")).toEqual([
      { account: "bob", password: "longenough", credential: "cookie", label: "web browser" },
    ]);
  });

  it("an existing account falls back to login and then to the reset hint", async () => {
    const exists = problem(409, "account-exists");
    const wrong = problem(401, "account-login-credentials");
    const seen: Seen[] = [];
    const signedIn = await runLogin(
      contextFor({ register: exists }, seen),
      input({ createAccount: true }),
    );
    expect(signedIn.kind).toBe("signedIn");
    const refused = await runLogin(
      contextFor({ register: exists, login: wrong }, seen),
      input({ createAccount: true }),
    );
    expect(refused).toEqual({
      kind: "failed",
      failure: { code: "account-exists-reset", params: {} },
    });
  });
});

describe("runLogin refusals", () => {
  it("login shows the server's own code for a wrong password", async () => {
    const wrong = problem(401, "account-login-credentials");
    const outcome = await runLogin(contextFor({ login: wrong }, []), input({}));
    expect(outcome).toEqual({
      kind: "failed",
      failure: { code: "account-login-credentials", params: {} },
    });
  });

  it("a disabled account is reported by code", async () => {
    const disabled = problem(403, "account-login-disabled");
    const outcome = await runLogin(contextFor({ login: disabled }, []), input({}));
    expect(outcome).toMatchObject({ failure: { code: "account-login-disabled" } });
  });

  it("an address that is not an AnyBot server is refused before any login", async () => {
    const seen: Seen[] = [];
    const outcome = await runLogin(
      contextFor({ status: json(200, { hello: "world" }) }, seen),
      input({}),
    );
    expect(outcome).toEqual({ kind: "failed", failure: { code: "login-not-anybot", params: {} } });
    expect(bodiesAt(seen, "/v1/account/login")).toEqual([]);
  });

  it("an unreachable address names the address", async () => {
    const outcome = await runLogin(contextFor({ down: true }, []), input({}));
    expect(outcome).toEqual({
      kind: "failed",
      failure: { code: "login-unreachable", params: { url: ORIGIN } },
    });
  });

  it("another origin only gets the status check", async () => {
    const seen: Seen[] = [];
    const outcome = await runLogin(contextFor({}, seen), input({ url: "http://other.test" }));
    expect(outcome).toEqual({ kind: "elsewhere" });
    expect(seen.map((entry) => entry.path)).toEqual(["/v1/server/status"]);
  });

  it("a two-step challenge is not treated as signed in", async () => {
    const challenge = json(200, { account: "bob", challenge: "c" });
    const outcome = await runLogin(contextFor({ login: challenge }, []), input({}));
    expect(outcome).toMatchObject({ failure: { code: "login-two-step-later" } });
  });
});
