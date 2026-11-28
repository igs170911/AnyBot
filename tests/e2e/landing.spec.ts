import { expect, test } from "@playwright/test";

import { collectConsoleErrors, registerScenarioScreenshots, screenShowsCode } from "./src/act.ts";
import { scenarioLang } from "./src/ftl.ts";
import { appUrl } from "./src/world.ts";

registerScenarioScreenshots();

test("signed out visitor lands on login", async ({ page }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  const consoleErrors = collectConsoleErrors(page);
  await page.goto(appUrl("/"));
  await screenShowsCode(page, "login-submit", lang);
  await expect.poll(() => page.evaluate(() => location.hash)).toBe("#/login");
  expect(consoleErrors, "console must stay clean").toEqual([]);
});
