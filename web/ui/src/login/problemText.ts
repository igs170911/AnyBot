import { FALLBACK_PROBLEM_CODE } from "@anybot/api-client";
import type { FluentVariable } from "@fluent/bundle";

import { ALL_MESSAGE_CODES, type MessageCode } from "../generated/codes.ts";

export interface Failure {
  readonly code: MessageCode;
  readonly params: Record<string, FluentVariable>;
}

const KNOWN_CODES: ReadonlySet<string> = new Set(ALL_MESSAGE_CODES);

export function isMessageCode(code: string): code is MessageCode {
  return KNOWN_CODES.has(code);
}

function fluentParams(params: Record<string, unknown>): Record<string, FluentVariable> {
  const out: Record<string, FluentVariable> = {};
  for (const [name, value] of Object.entries(params)) {
    if (typeof value === "string" || typeof value === "number") {
      out[name] = value;
    }
  }
  return out;
}

export function failureOf(problem: { code: string; params: Record<string, unknown> }): Failure {
  if (isMessageCode(problem.code)) {
    return { code: problem.code, params: fluentParams(problem.params) };
  }
  return { code: FALLBACK_PROBLEM_CODE, params: {} };
}
