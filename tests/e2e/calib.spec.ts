import { expect, test } from "@playwright/test";

import { EDGE_OFFENDERS_JS, installWideFont } from "./src/act.ts";

const CALIB_ENV = "ANYBOT_CALIB_OVERFLOW";
const CALIB_CSS =
  ".calib-clip { width: 200px; overflow: hidden; } .calib-clip span { display: block; width: 420px; }";

test("calib: strengthened edge check catches clipped overflow", async ({ page }) => {
  test.skip(process.env[CALIB_ENV] !== "1", "calibration only");
  test.skip(page.viewportSize() === null, "needs a known viewport");
  const size = page.viewportSize();
  test.skip(size !== null && size.width < 500, "calibration targets wide projects");
  await page.goto("/");
  await installWideFont(page);
  await page.addStyleTag({ content: CALIB_CSS });
  await page.evaluate(() => {
    const box = document.createElement("div");
    box.className = "calib-clip";
    const inner = document.createElement("span");
    inner.textContent = "calibration clipped content";
    box.append(inner);
    document.body.append(box);
  });
  const offenders = await page.evaluate(EDGE_OFFENDERS_JS);
  expect(offenders, "strengthened check must flag clipped and wide elements").not.toEqual([]);
});
