import { type APIRequestContext, expect, type Page } from "@playwright/test";

import { readEnvPorts } from "./env.ts";

export const CLIENT_VERSION = "0.1.0";
const LOCK_REQUEST_TIMEOUT_MS = 70_000;
const CONTROL_REQUEST_TIMEOUT_MS = 30_000;

export interface SeededBot {
  name: string;
  id: string;
  model: string;
}

export interface SeededWorld {
  account: string;
  password: string;
  bots: SeededBot[];
}

export interface SeedOptions {
  bots: string[];
  admin?: boolean;
}

export function serverOrigin(): string {
  return `http://127.0.0.1:${String(readEnvPorts().server_port)}`;
}

export function controlUrl(path: string): string {
  return `http://127.0.0.1:${String(readEnvPorts().control_port)}${path}`;
}

export function appUrl(path = "/"): string {
  return `${serverOrigin()}${path}`;
}

export async function seedWorld(
  request: APIRequestContext,
  options: SeedOptions,
): Promise<SeededWorld> {
  const reply = await request.post(controlUrl("/control/seed"), {
    data: options,
    timeout: CONTROL_REQUEST_TIMEOUT_MS,
  });
  expect(reply.status(), `seed failed: ${await reply.text()}`).toBe(200);
  return (await reply.json()) as SeededWorld;
}

interface LoginInput {
  account: string;
  password: string;
  version: string;
}

async function openAppOrigin(page: Page): Promise<void> {
  if (!page.url().startsWith(serverOrigin())) {
    await page.goto(appUrl("/"));
  }
}

export async function loginInPage(page: Page, account: string, password: string): Promise<void> {
  await openAppOrigin(page);
  const input: LoginInput = { account, password, version: CLIENT_VERSION };
  const outcome = await page.evaluate(async (given: LoginInput) => {
    const reply = await fetch("/v1/account/login", {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "AnyBot-Client-Version": given.version,
        "AnyBot-CSRF": "1",
        "Idempotency-Key": crypto.randomUUID(),
      },
      body: JSON.stringify({
        account: given.account,
        password: given.password,
        label: "playwright",
        credential: "cookie",
      }),
    });
    return { status: reply.status, text: await reply.text() };
  }, input);
  expect(outcome.status, `login in page failed: ${outcome.text}`).toBe(200);
}

export async function withLock<T>(
  request: APIRequestContext,
  name: string,
  work: () => Promise<T>,
): Promise<T> {
  const taken = await request.post(controlUrl("/control/lock"), {
    data: { name },
    timeout: LOCK_REQUEST_TIMEOUT_MS,
  });
  expect(taken.status(), `lock ${name} not granted`).toBe(200);
  try {
    return await work();
  } finally {
    await request.delete(controlUrl(`/control/lock/${encodeURIComponent(name)}`), {
      timeout: CONTROL_REQUEST_TIMEOUT_MS,
    });
  }
}
