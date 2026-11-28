import { expect, test } from "@playwright/test";

import { installFakeClock, registerScenarioScreenshots, syncBrowserClock } from "./src/act.ts";
import { readEnvPorts } from "./src/env.ts";

const SCRIPT_TEXT = "hello from the control script";

registerScenarioScreenshots();

function nowMs(iso: string): number {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) {
    throw new Error(`health now is not RFC 3339: ${iso}`);
  }
  return parsed.getTime();
}

test("testenv control api drives fake model, records, and clock end to end", async ({
  request,
  page,
}) => {
  test.skip(test.info().project.name !== "desktop-large", "one cross-side run is enough");
  const ports = readEnvPorts();
  const control = `http://127.0.0.1:${String(ports.control_port)}`;
  const script = [{ ReplyText: { text: SCRIPT_TEXT, stream: false } }];
  const scriptReply = await request.post(`${control}/control/model/script`, { data: script });
  expect(scriptReply.status()).toBe(202);

  const modelReply = await request.post(
    `http://127.0.0.1:${String(ports.model_port)}/v1/chat/completions`,
    {
      data: { model: "gpt-test", stream: false, messages: [] },
    },
  );
  expect(modelReply.status()).toBe(200);
  const body = (await modelReply.json()) as { choices?: { message?: { content?: string } }[] };
  const content = body.choices?.[0]?.message?.content;
  expect(content).toBe(SCRIPT_TEXT);

  const recorded = await request.get(`${control}/control/model/requests?model=gpt-test`);
  const recordList = (await recorded.json()) as { path: string; body: string }[];
  expect(recordList.length).toBe(1);
  expect(recordList[0]?.path).toBe("/v1/chat/completions");
  expect(recordList[0]?.body).toContain("gpt-test");

  await installFakeClock(request, page);

  const healthBefore = await request.get(`http://127.0.0.1:${String(ports.server_port)}/v1/health`);
  const nowBefore = nowMs(((await healthBefore.json()) as { now: string }).now);
  const advance = await request.post(`${control}/control/clock/advance`, {
    data: { ms: 3_600_000 },
  });
  expect(advance.status()).toBe(200);
  const healthAfter = await request.get(`http://127.0.0.1:${String(ports.server_port)}/v1/health`);
  const nowAfter = nowMs(((await healthAfter.json()) as { now: string }).now);
  expect(nowAfter - nowBefore).toBe(3_600_000);

  await syncBrowserClock(request, page);
  const browserNow = await page.evaluate(() => Date.now());
  expect(browserNow).toBe(nowAfter);
});
