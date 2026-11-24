import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { App } from "./App.tsx";

describe("apps/web PWA 骨架", () => {
  it("根元件只顯示 web/ui 的外殼，沒有頁面和路由", () => {
    const html = renderToString(<App />);
    expect(html).toContain("anybot-shell");
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("<a href");
  });
});
