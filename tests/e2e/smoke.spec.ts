import { expect, test } from "@playwright/test";

import { collectConsoleErrors, expectNoHorizontalScroll, screenShowsCode } from "./src/act.ts";
import { scenarioLang } from "./src/ftl.ts";

const SHELL_BRAND = "AnyBot";

test("smoke: shell renders, no horizontal overflow, console clean", async ({ page }) => {
  const lang = scenarioLang(test.info().project.use.locale);
  const consoleErrors = collectConsoleErrors(page);
  const zoom = test.info().project.use.maxTextZoom;
  await page.goto("/");
  if (zoom !== undefined) {
    await page.addStyleTag({ content: `html { font-size: ${(Number(zoom) * 100).toFixed(0)}%; }` });
  }
  await expect(page.getByRole("heading", { name: SHELL_BRAND })).toBeVisible();
  await screenShowsCode(page, "ui-shell-skeleton", lang);
  const isPhone = test.info().project.name === "phone-max-font";
  if (isPhone) {
    await expectNoHorizontalScroll(page);
  }
  expect(consoleErrors, "console must stay clean").toEqual([]);
});
