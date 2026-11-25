import { formatIn, UI_SHELL_SKELETON } from "@anybot/ui";
import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App.tsx";

describe("apps/desktop frontend skeleton", () => {
  it("window is only a view: renders the web/ui shell, no pages and no routing", () => {
    vi.stubGlobal("navigator", { languages: ["zh-TW"] });
    const html = renderToString(<App />);
    vi.unstubAllGlobals();
    expect(html).toContain(formatIn("zh-TW", UI_SHELL_SKELETON));
    expect(html).not.toContain("data-message-code");
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("<a href");
  });
});
