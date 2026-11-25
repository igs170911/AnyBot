import { mkdir } from "node:fs/promises";
import nodePath from "node:path";

import { type APIRequestContext, expect, type Locator, type Page, test } from "@playwright/test";
import { readEnvPorts } from "./env.ts";
import type { Lang } from "./ftl.ts";
import { textFor } from "./ftl.ts";

export function byMessageCode(page: Page, code: string, lang: Lang): Locator {
  return page.getByText(textFor(code, lang), { exact: true });
}

export async function pressTextCode(page: Page, code: string, lang: Lang): Promise<void> {
  await page.getByRole("button", { name: textFor(code, lang) }).click();
}

export async function fillFieldCode(
  page: Page,
  labelCode: string,
  value: string,
  lang: Lang,
): Promise<void> {
  await page.getByLabel(textFor(labelCode, lang)).fill(value);
}

export async function screenShowsCode(page: Page, code: string, lang: Lang): Promise<void> {
  await expect(byMessageCode(page, code, lang)).toBeVisible();
}

export async function screenHidesCode(page: Page, code: string, lang: Lang): Promise<void> {
  await expect(byMessageCode(page, code, lang)).toBeHidden();
}

export function collectConsoleErrors(page: Page): string[] {
  const seen: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      seen.push(msg.text());
    }
  });
  return seen;
}

export async function fakeNowMs(request: APIRequestContext): Promise<number> {
  const ports = readEnvPorts();
  const reply = await request.get(`http://127.0.0.1:${String(ports.control_port)}/control/status`);
  const view = (await reply.json()) as { now_ms: number };
  return view.now_ms;
}

export async function installFakeClock(request: APIRequestContext, page: Page): Promise<void> {
  await page.clock.install({ time: new Date(await fakeNowMs(request)) });
}

export async function syncBrowserClock(request: APIRequestContext, page: Page): Promise<void> {
  await page.clock.setFixedTime(new Date(await fakeNowMs(request)));
}

export async function expectNoHorizontalScroll(page: Page): Promise<void> {
  const overflow = await page.evaluate(() => {
    return document.documentElement.scrollWidth - window.innerWidth;
  });
  expect(overflow, "page must not overflow its viewport width").toBeLessThanOrEqual(1);
}

export function registerScenarioScreenshots(): void {
  test.afterEach(async ({ page }, testInfo) => {
    const safeTitle = testInfo.title.replaceAll(/[^A-Za-z0-9-]/g, "_");
    const dir = nodePath.resolve(import.meta.dirname, "../../../target/e2e-screens");
    await mkdir(dir, { recursive: true });
    const target = nodePath.join(dir, `${testInfo.project.name}--${safeTitle}.png`);
    await page.screenshot({ path: target, fullPage: true });
  });
}
