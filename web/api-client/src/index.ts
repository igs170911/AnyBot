import createClient from "openapi-fetch";

import type { components, paths } from "./generated/schema.ts";

export interface ApiClientOptions {
  readonly baseUrl: string;
}

export const apiClientGenerated: boolean = true;

export function createApiClient(options: ApiClientOptions) {
  return createClient<paths>({ baseUrl: options.baseUrl });
}

export {
  type AnyBotClient,
  type AnyBotClientOptions,
  CLIENT_VERSION_HEADER,
  CSRF_HEADER,
  CSRF_VALUE,
  createAnyBotClient,
  headerMiddleware,
} from "./client.ts";
export {
  type ApiProblem,
  type ApiResult,
  attempt,
  FALLBACK_PROBLEM_CODE,
  problemOf,
  unwrap,
} from "./problem.ts";
export type { components, paths };
