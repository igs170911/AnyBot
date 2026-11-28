import { expect, test } from "@playwright/test";

import { registerScenarioScreenshots } from "./src/act.ts";
import { scenarioLang } from "./src/ftl.ts";
import {
  fillLogin,
  labelled,
  pressLogin,
  shownText,
  useFreshAddress,
  waitWhileClockSlides,
} from "./src/login-steps.ts";
import {
  accountSignsIn,
  appUrl,
  deleteAccount,
  disableAccount,
  readInvite,
  seedWorld,
  withLock,
} from "./src/world.ts";

const BOB_PASSWORD = "bob-password-1";
const WRONG_PASSWORD = "not-the-password";

registerScenarioScreenshots();

test("T-account-login-1 registering Bob creates bob and signs in", async ({ page, request }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  await withLock(request, "fixed-account-bob", async () => {
    await deleteAccount(request, "bob");
    try {
      await useFreshAddress(page);
      const invite = await readInvite(request);
      await page.goto(appUrl("/"));
      await labelled(page, "login-create-account", lang).check();
      await expect(labelled(page, "login-privacy-ack", lang)).toBeVisible();
      await fillLogin(page, lang, {
        account: "Bob",
        password: BOB_PASSWORD,
        create: true,
        invite,
        acknowledge: true,
      });
      await pressLogin(page, lang);
      await expect(page.locator('[data-route="bots"]')).toBeAttached();
      await expect(page.locator('[data-route="login"]')).toHaveCount(0);
      expect(await accountSignsIn(request, "bob", BOB_PASSWORD)).toBe(200);
    } finally {
      await deleteAccount(request, "bob");
    }
  });
});

test("T-account-login-5 existing account with wrong password", async ({ page, request }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  await useFreshAddress(page);
  const invite = await readInvite(request);
  await page.goto(appUrl("/"));
  await fillLogin(page, lang, {
    account: "alice",
    password: WRONG_PASSWORD,
    create: true,
    invite,
    acknowledge: true,
  });
  await pressLogin(page, lang);
  await waitWhileClockSlides(request, shownText(page, "account-exists-reset", lang));
});

test("T-account-login-7 disabled account is told so", async ({ page, request }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  const world = await seedWorld(request, { bots: [] });
  await disableAccount(request, world.account);
  await useFreshAddress(page);
  await page.goto(appUrl("/"));
  await fillLogin(page, lang, { account: world.account, password: world.password, create: false });
  await pressLogin(page, lang);
  await waitWhileClockSlides(request, shownText(page, "account-login-disabled", lang));
});

test("T-account-login-8 register page names the admin", async ({ page }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  await page.goto(appUrl("/"));
  await expect(shownText(page, "login-privacy-note", lang, { admin: "alice" })).toHaveCount(0);
  await labelled(page, "login-create-account", lang).check();
  await expect(shownText(page, "login-privacy-note", lang, { admin: "alice" })).toBeVisible();
});

test("wrong password on plain login shows the credentials message", async ({ page, request }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  const world = await seedWorld(request, { bots: [] });
  await useFreshAddress(page);
  await page.goto(appUrl("/"));
  await fillLogin(page, lang, { account: world.account, password: WRONG_PASSWORD, create: false });
  await pressLogin(page, lang);
  await waitWhileClockSlides(request, shownText(page, "account-login-credentials", lang));
});
