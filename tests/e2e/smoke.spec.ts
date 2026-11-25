import { expect, type Page, test } from "@playwright/test";

import {
  collectConsoleErrors,
  expectNoHorizontalScroll,
  installWideFont,
  registerScenarioScreenshots,
  screenShowsCode,
} from "./src/act.ts";
import { scenarioLang } from "./src/ftl.ts";

const SHELL_BRAND = "AnyBot";

registerScenarioScreenshots();

async function injectMaxTextZoom(page: Page, zoom: string): Promise<void> {
  await page.addStyleTag({ content: `html { font-size: ${(Number(zoom) * 100).toFixed(0)}%; }` });
}

test("smoke: shell renders, no horizontal overflow, console clean", async ({ page }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  const consoleErrors = collectConsoleErrors(page);
  const zoom = test.info().project.use.maxTextZoom;
  await page.goto("/");
  if (zoom !== undefined) {
    await injectMaxTextZoom(page, zoom);
  }
  await expect(page.getByRole("heading", { name: SHELL_BRAND })).toBeVisible();
  await screenShowsCode(page, "ui-shell-skeleton", lang);
  const isPhone = test.info().project.name === "phone-max-font";
  if (isPhone) {
    await expectNoHorizontalScroll(page);
  }
  expect(consoleErrors, "console must stay clean").toEqual([]);
});

test("t-phonefit-width-1: max text zoom with a very wide font stays inside the phone viewport", async ({
  page,
}) => {
  const lang = scenarioLang(test.info().project.use.locale);
  const consoleErrors = collectConsoleErrors(page);
  const zoom = test.info().project.use.maxTextZoom;
  await page.goto("/");
  if (zoom !== undefined) {
    await injectMaxTextZoom(page, zoom);
  }
  await installWideFont(page);
  await expect(page.getByRole("heading", { name: SHELL_BRAND })).toBeVisible();
  await screenShowsCode(page, "ui-shell-skeleton", lang);
  await expectNoHorizontalScroll(page);
  expect(consoleErrors, "console must stay clean").toEqual([]);
});
