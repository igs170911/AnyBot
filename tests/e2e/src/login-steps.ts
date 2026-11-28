import { type APIRequestContext, expect, type Locator, type Page } from "@playwright/test";

import { type Lang, textFor } from "./ftl.ts";
import { advanceClock, serverOrigin, withLock } from "./world.ts";

const SLOW_FAIL_STEP_MS = 1000;
const SLOW_FAIL_MAX_STEPS = 30;
const POLL_VISIBLE_MS = 300;

let addressCounter = 1;

export async function useFreshAddress(page: Page): Promise<void> {
  addressCounter += 1;
  const address = `10.${String(200 + (addressCounter % 50))}.${String(Math.floor(Math.random() * 250))}.${String(1 + Math.floor(Math.random() * 250))}`;
  await page.route("**/v1/account/*", async (route) => {
    await route.continue({
      headers: { ...route.request().headers(), "x-forwarded-for": address },
    });
  });
}

export function labelled(page: Page, code: string, lang: Lang): Locator {
  return page.getByLabel(textFor(code, lang), { exact: true });
}

export function shownText(
  page: Page,
  code: string,
  lang: Lang,
  args?: Readonly<Record<string, string>>,
): Locator {
  return page.getByText(textFor(code, lang, args), { exact: true });
}

export interface LoginForm {
  account: string;
  password: string;
  create: boolean;
  invite?: string;
  acknowledge?: boolean;
}

export async function fillLogin(page: Page, lang: Lang, form: LoginForm): Promise<void> {
  await labelled(page, "login-url-label", lang).fill(serverOrigin());
  await labelled(page, "login-account-label", lang).fill(form.account);
  await labelled(page, "login-password-label", lang).fill(form.password);
  if (form.create) {
    await labelled(page, "login-create-account", lang).check();
    await labelled(page, "login-invite-label", lang).fill(form.invite ?? "");
  }
  if (form.acknowledge === true) {
    await labelled(page, "login-privacy-ack", lang).check();
  }
}

export async function pressLogin(page: Page, lang: Lang): Promise<void> {
  await page.getByRole("button", { name: textFor("login-submit", lang), exact: true }).click();
}

export async function waitWhileClockSlides(
  request: APIRequestContext,
  target: Locator,
): Promise<void> {
  await withLock(request, "clock", async () => {
    for (let step = 0; step < SLOW_FAIL_MAX_STEPS; step += 1) {
      const shown = await target.waitFor({ state: "visible", timeout: POLL_VISIBLE_MS }).then(
        () => true,
        () => false,
      );
      if (shown) {
        return;
      }
      await advanceClock(request, SLOW_FAIL_STEP_MS);
    }
  });
  await expect(target).toBeVisible();
}
