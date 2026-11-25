import { FluentBundle, FluentResource } from "@fluent/bundle";
import { renderToString } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EN_FTL_SOURCE, ZH_TW_FTL_SOURCE } from "./generated/locales.ts";
import { AppShell } from "./shell.tsx";
import { UI_SHELL_SKELETON } from "./skeleton.ts";

function fixtureText(source: string, lang: string): string {
  const bundle = new FluentBundle(lang, { useIsolating: false });
  bundle.addResource(new FluentResource(source));
  const message = bundle.getMessage(UI_SHELL_SKELETON);
  if (message === undefined || message.value === null) {
    throw new Error(`fixture bundle ${lang} is missing the skeleton message`);
  }
  const errors: Array<Error> = [];
  const text = bundle.formatPattern(message.value, null, errors);
  if (errors.length > 0) {
    throw new Error(`fixture bundle ${lang} hit format errors`);
  }
  return text;
}

function stubBrowserLanguages(languages: readonly string[]): void {
  vi.stubGlobal("navigator", { languages });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("AppShell", () => {
  it("renders the skeleton message formatted in English for English browsers", () => {
    stubBrowserLanguages(["en"]);
    const html = renderToString(<AppShell />);
    expect(html).toContain(fixtureText(EN_FTL_SOURCE, "en"));
    expect(html).not.toContain("data-message-code");
  });

  it("renders the skeleton message formatted in zh-TW for zh-TW browsers", () => {
    stubBrowserLanguages(["zh-TW"]);
    const html = renderToString(<AppShell />);
    expect(html).toContain(fixtureText(ZH_TW_FTL_SOURCE, "zh-TW"));
    expect(html).not.toContain("data-message-code");
  });

  it("falls back to the English bundle for unsupported browser languages", () => {
    stubBrowserLanguages(["fr", "de"]);
    const html = renderToString(<AppShell />);
    expect(html).toContain(fixtureText(EN_FTL_SOURCE, "en"));
    expect(html).not.toContain(fixtureText(ZH_TW_FTL_SOURCE, "zh-TW"));
  });

  it("keeps the skeleton empty of nav, input, and table", () => {
    stubBrowserLanguages(["en"]);
    const html = renderToString(<AppShell />);
    for (const tag of ["<nav", "<input", "<table"]) {
      expect(html).not.toContain(tag);
    }
  });
});
