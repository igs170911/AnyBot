import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const SHELL_CSS = path.resolve(import.meta.dirname, "shell.css");

function cssText(): string {
  return readFileSync(SHELL_CSS, "utf8");
}

describe("apps/web shell layout gates", () => {
  it("t-phonefit-width-1: long words break inside the viewport on any font", () => {
    const css = cssText();
    expect(css).toContain("overflow-wrap: break-word;");
    expect(css).toContain("word-break: break-word;");
    expect(css).toContain("max-width: 100%;");
  });

  it("t-phonefit-width-2: the page never grows a horizontal scrollbar", () => {
    const css = cssText();
    expect(css).toContain("overflow-x: hidden;");
    expect(css).toContain("box-sizing: border-box;");
    expect(css).toContain("margin: 0;");
  });
});
