import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AppShell } from "./shell.tsx";

describe("AppShell", () => {
  it("是空的外殼：只有標題和骨架說明，沒有頁面或功能", () => {
    const html = renderToString(<AppShell />);
    expect(html).toContain("anybot-shell");
    expect(html).toContain("AnyBot");
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("<input");
  });
});
