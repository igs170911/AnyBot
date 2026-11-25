import { UI_SHELL_SKELETON } from "@anybot/ui";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { App } from "./App.tsx";

describe("apps/desktop frontend skeleton", () => {
  it("window is only a view: renders the web/ui shell, no pages and no routing", () => {
    const html = renderToString(<App />);
    expect(html).toContain(`data-message-code="${UI_SHELL_SKELETON}"`);
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("<a href");
  });
});
