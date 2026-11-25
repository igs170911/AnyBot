import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AppShell } from "./shell.tsx";
import { UI_SHELL_SKELETON } from "./skeleton.ts";

describe("AppShell", () => {
  it("the shell is empty: only a title and a skeleton note, no pages and no features", () => {
    const html = renderToString(<AppShell />);
    expect(html).toContain(`data-message-code="${UI_SHELL_SKELETON}"`);
    expect(html).toContain("AnyBot");
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("<input");
  });
});
