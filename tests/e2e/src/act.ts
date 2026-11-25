import { expect, type Locator, type Page } from "@playwright/test";
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

export async function expectNoHorizontalScroll(page: Page): Promise<void> {
  const overflow = await page.evaluate(() => {
    return document.documentElement.scrollWidth - window.innerWidth;
  });
  expect(overflow, "page must not overflow its viewport width").toBeLessThanOrEqual(1);
}
