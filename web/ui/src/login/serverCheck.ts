import { type AnyBotClient, attempt, type components } from "@anybot/api-client";

import type { Failure } from "./problemText.ts";

export const SERVER_CHECK_TIMEOUT_MS = 15_000;

type StatusBody = components["schemas"]["StatusResponse"] & { admin_account?: string | null };

export interface ServerInfo {
  readonly adminAccount: string | null;
}

export type ServerCheck =
  | { readonly ok: true; readonly info: ServerInfo }
  | { readonly ok: false; readonly failure: Failure };

const NOT_ANYBOT: Failure = { code: "login-not-anybot", params: {} };

export async function checkServer(client: AnyBotClient, url: string): Promise<ServerCheck> {
  const result = await attempt(() =>
    client.GET("/v1/server/status", { signal: AbortSignal.timeout(SERVER_CHECK_TIMEOUT_MS) }),
  );
  if (!result.ok) {
    if (result.problem.status === 0) {
      return { ok: false, failure: { code: "login-unreachable", params: { url } } };
    }
    return { ok: false, failure: NOT_ANYBOT };
  }
  const body: StatusBody = result.data;
  if (typeof body.protocol_version !== "number") {
    return { ok: false, failure: NOT_ANYBOT };
  }
  return { ok: true, info: { adminAccount: body.admin_account ?? null } };
}

export function originOf(url: string): string | null {
  try {
    return new URL(url).origin;
  } catch {
    return null;
  }
}
