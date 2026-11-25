import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";
import type { MessageCode } from "./generated/codes.ts";
import { EN_FTL_SOURCE, ZH_TW_FTL_SOURCE } from "./generated/locales.ts";

export const UI_LANG_TAGS = ["en", "zh-TW"] as const;
export type UiLang = (typeof UI_LANG_TAGS)[number];

const bundleCache = new Map<UiLang, FluentBundle>();

function bundleFor(lang: UiLang): FluentBundle {
  const cached = bundleCache.get(lang);
  if (cached !== undefined) {
    return cached;
  }
  const source = lang === "en" ? EN_FTL_SOURCE : ZH_TW_FTL_SOURCE;
  const bundle = new FluentBundle(lang, { useIsolating: false });
  const errors = bundle.addResource(new FluentResource(source));
  if (errors.length > 0) {
    throw new Error(`bundle ${lang} rejected ${errors.length} resource entries`);
  }
  bundleCache.set(lang, bundle);
  return bundle;
}

export function pickLang(languages: readonly string[]): UiLang {
  for (const tag of languages) {
    if (tag === "en" || tag === "zh-TW") {
      return tag;
    }
  }
  return "en";
}

export function browserLanguages(): readonly string[] {
  return globalThis.navigator?.languages ?? [];
}

export function currentLang(): UiLang {
  return pickLang(browserLanguages());
}

export function formatIn(
  lang: UiLang,
  code: MessageCode,
  args?: Record<string, FluentVariable>,
): string {
  const bundle = bundleFor(lang);
  const message = bundle.getMessage(code);
  if (message === undefined || message.value === null) {
    throw new Error(`message ${code} is missing from the ${lang} bundle`);
  }
  const errors: Array<Error> = [];
  const text = bundle.formatPattern(message.value, args ?? null, errors);
  if (errors.length > 0) {
    throw new Error(`message ${code} hit ${errors.length} format errors in ${lang}`);
  }
  return text;
}

export function formatMessage(code: MessageCode, args?: Record<string, FluentVariable>): string {
  return formatIn(currentLang(), code, args);
}
