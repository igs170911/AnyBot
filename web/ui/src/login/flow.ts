import { type AnyBotClient, type ApiResult, attempt, type components } from "@anybot/api-client";

import { DEVICE_LABEL, type LoginInput, normalizeAccount } from "./form.ts";
import { type Failure, failureOf } from "./problemText.ts";
import { checkServer, originOf } from "./serverCheck.ts";

type LoginReply = components["schemas"]["LoginSession"] | components["schemas"]["TwoStepChallenge"];

export type LoginOutcome =
  | { readonly kind: "signedIn"; readonly account: string }
  | { readonly kind: "elsewhere" }
  | { readonly kind: "failed"; readonly failure: Failure };

export interface LoginContext {
  readonly origin: string;
  clientAt(url: string): AnyBotClient;
}

const ACCOUNT_EXISTS = "account-exists";

function failed(failure: Failure): LoginOutcome {
  return { kind: "failed", failure };
}

function logIn(client: AnyBotClient, account: string, password: string) {
  return attempt(() =>
    client.POST("/v1/account/login", {
      params: { header: { "Idempotency-Key": globalThis.crypto.randomUUID() } },
      body: { account, password, credential: "cookie", label: DEVICE_LABEL },
    }),
  );
}

function register(client: AnyBotClient, input: LoginInput, account: string) {
  return attempt(() =>
    client.POST("/v1/account/register", {
      params: { header: { "Idempotency-Key": globalThis.crypto.randomUUID() } },
      body: { account, password: input.password, invite: input.invite },
    }),
  );
}

function outcomeOfLogin(result: ApiResult<LoginReply>): LoginOutcome {
  if (!result.ok) {
    return failed(failureOf(result.problem));
  }
  if ("challenge" in result.data) {
    return failed({ code: "login-two-step-later", params: {} });
  }
  return { kind: "signedIn", account: result.data.account };
}

async function createThenLogin(
  client: AnyBotClient,
  input: LoginInput,
  account: string,
): Promise<LoginOutcome> {
  const created = await register(client, input, account);
  if (created.ok) {
    return outcomeOfLogin(await logIn(client, account, input.password));
  }
  if (created.problem.code !== ACCOUNT_EXISTS) {
    return failed(failureOf(created.problem));
  }
  const fallback = await logIn(client, account, input.password);
  return fallback.ok
    ? outcomeOfLogin(fallback)
    : failed({ code: "account-exists-reset", params: {} });
}

export async function runLogin(context: LoginContext, input: LoginInput): Promise<LoginOutcome> {
  const target = originOf(input.url);
  if (target === null) {
    return failed({ code: "login-not-anybot", params: {} });
  }
  const client = context.clientAt(target);
  const check = await checkServer(client, input.url);
  if (!check.ok) {
    return failed(check.failure);
  }
  if (target !== context.origin) {
    return { kind: "elsewhere" };
  }
  const account = normalizeAccount(input.account);
  return input.createAccount
    ? createThenLogin(client, input, account)
    : outcomeOfLogin(await logIn(client, account, input.password));
}
