export interface ApiProblem {
  readonly code: string;
  readonly params: Record<string, unknown>;
  readonly status: number;
}

export type ApiResult<T> =
  | { readonly ok: true; readonly data: T }
  | { readonly ok: false; readonly problem: ApiProblem };

export const FALLBACK_PROBLEM_CODE = "problem-internal";

interface RawResult<T> {
  readonly data?: T | undefined;
  readonly error?: unknown;
  readonly response: Response;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function problemOf(error: unknown, status: number): ApiProblem {
  if (isRecord(error) && typeof error["code"] === "string") {
    const params = error["params"];
    return { code: error["code"], params: isRecord(params) ? params : {}, status };
  }
  return { code: FALLBACK_PROBLEM_CODE, params: {}, status };
}

export function unwrap<T>(result: RawResult<T>): ApiResult<T> {
  if (result.response.ok && result.data !== undefined) {
    return { ok: true, data: result.data };
  }
  return { ok: false, problem: problemOf(result.error, result.response.status) };
}

export async function attempt<T>(run: () => Promise<RawResult<T>>): Promise<ApiResult<T>> {
  try {
    return unwrap(await run());
  } catch {
    return { ok: false, problem: { code: FALLBACK_PROBLEM_CODE, params: {}, status: 0 } };
  }
}
