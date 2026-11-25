import createClient from "openapi-fetch";

import type { paths } from "./generated/schema.ts";

export interface ApiClientOptions {
  readonly baseUrl: string;
}

export const apiClientGenerated: boolean = true;

export function createApiClient(options: ApiClientOptions) {
  return createClient<paths>({ baseUrl: options.baseUrl });
}

export type { paths };
