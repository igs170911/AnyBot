import { createAnyBotClient } from "@anybot/api-client";
import { formatIn, UI_SHELL_SKELETON } from "@anybot/ui";
import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App.tsx";

describe("apps/web PWA skeleton", () => {
  it("the root component renders the web/ui shell, no pages and no routing", () => {
    vi.stubGlobal("navigator", { languages: ["en-US", "en"] });
    const html = renderToString(
      <App
        client={createAnyBotClient({ clientVersion: "0.0.0-test", baseUrl: "http://localhost" })}
      />,
    );
    vi.unstubAllGlobals();
    expect(html).toContain(formatIn("en", UI_SHELL_SKELETON));
    expect(html).not.toContain("data-message-code");
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("<a href");
  });
});
