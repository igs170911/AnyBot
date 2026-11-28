import { expect, test } from "@playwright/test";

import { collectConsoleErrors, registerScenarioScreenshots } from "./src/act.ts";
import { readEnvPorts } from "./src/env.ts";
import { appUrl, controlUrl, seedWorld, serverOrigin } from "./src/world.ts";

const SHELL_BRAND = "AnyBot";
const MODEL_REQUEST_TIMEOUT_MS = 20_000;

registerScenarioScreenshots();

test("testenv serves the web app on the server origin", async ({ page }) => {
  const consoleErrors = collectConsoleErrors(page);
  await page.goto(appUrl("/"));
  await expect(page.getByRole("heading", { name: SHELL_BRAND })).toBeVisible();
  const origin = await page.evaluate(() => location.origin);
  expect(origin).toBe(`http://127.0.0.1:${String(readEnvPorts().server_port)}`);
  expect(origin).toBe(serverOrigin());
  expect(consoleErrors, "console must stay clean").toEqual([]);
});

test("seeded worlds keep separate model scripts", async ({ request }) => {
  const first = await seedWorld(request, { bots: ["A"] });
  const second = await seedWorld(request, { bots: ["B"] });
  const models = [first.bots[0]?.model, second.bots[0]?.model];
  expect(models[0]).toBeDefined();
  expect(models[1]).toBeDefined();
  expect(models[0]).not.toBe(models[1]);
  for (const model of models) {
    const script = {
      model,
      steps: [{ ReplyText: { text: `reply for ${String(model)}`, stream: false } }],
    };
    const accepted = await request.post(controlUrl("/control/model/script"), { data: script });
    expect(accepted.status()).toBe(202);
  }
  const modelUrl = `http://127.0.0.1:${String(readEnvPorts().model_port)}/v1/chat/completions`;
  for (const model of [...models].reverse()) {
    const reply = await request.post(modelUrl, {
      data: { model, stream: false, messages: [] },
      timeout: MODEL_REQUEST_TIMEOUT_MS,
    });
    const body = (await reply.json()) as { choices?: { message?: { content?: string } }[] };
    expect(body.choices?.[0]?.message?.content).toBe(`reply for ${String(model)}`);
  }
  for (const model of models) {
    const seen = await request.get(
      controlUrl(`/control/model/requests?model=${encodeURIComponent(String(model))}`),
    );
    expect(((await seen.json()) as unknown[]).length).toBe(1);
  }
});
