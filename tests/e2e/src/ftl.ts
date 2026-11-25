import { readFileSync } from "node:fs";
import path from "node:path";

const LANGS = ["en", "zh-TW"] as const;
export type Lang = (typeof LANGS)[number];

const cache = new Map<Lang, Map<string, string>>();

function isLang(tag: string): tag is Lang {
  return tag === "en" || tag === "zh-TW";
}

function parseFtl(text: string): Map<string, string> {
  const entries = new Map<string, string>();
  let currentKey: string | undefined;
  for (const line of text.split("\n")) {
    const sep = line.indexOf(" = ");
    if (sep > 0 && !line.startsWith(" ")) {
      currentKey = line.slice(0, sep).trim();
      entries.set(currentKey, line.slice(sep + 3).trim());
      continue;
    }
    if (currentKey !== undefined && line.startsWith(" ")) {
      const prev = entries.get(currentKey) ?? "";
      entries.set(currentKey, `${prev} ${line.trim()}`);
    }
  }
  return entries;
}

function entriesFor(lang: Lang): Map<string, string> {
  const cached = cache.get(lang);
  if (cached !== undefined) {
    return cached;
  }
  const repoRoot = path.resolve(import.meta.dirname, "../../..");
  const parsed = parseFtl(readFileSync(path.join(repoRoot, "locales", `${lang}.ftl`), "utf8"));
  cache.set(lang, parsed);
  return parsed;
}

export function scenarioLang(locale: string | undefined): Lang {
  if (locale !== undefined && isLang(locale)) {
    return locale;
  }
  return "en";
}

export function textFor(code: string, lang: Lang): string {
  const entry = entriesFor(lang).get(code);
  if (entry === undefined) {
    throw new Error(`locale ${lang} has no entry for message code ${code}`);
  }
  return entry;
}
