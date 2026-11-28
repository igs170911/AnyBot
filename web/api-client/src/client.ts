import createClient, { type Middleware } from "openapi-fetch";

import type { paths } from "./generated/schema.ts";

export const CLIENT_VERSION_HEADER = "AnyBot-Client-Version";
export const CSRF_HEADER = "AnyBot-CSRF";
export const CSRF_VALUE = "1";

const READ_ONLY_METHODS: ReadonlySet<string> = new Set(["GET", "HEAD", "OPTIONS"]);

export interface AnyBotClientOptions {
  readonly clientVersion: string;
  readonly baseUrl?: string;
  readonly fetch?: (request: Request) => Promise<Response>;
}

export function headerMiddleware(clientVersion: string): Middleware {
  return {
    onRequest({ request }) {
      request.headers.set(CLIENT_VERSION_HEADER, clientVersion);
      if (!READ_ONLY_METHODS.has(request.method.toUpperCase())) {
        request.headers.set(CSRF_HEADER, CSRF_VALUE);
      }
      return request;
    },
  };
}

function sameOrigin(): string {
  return globalThis.location.origin;
}

export function createAnyBotClient(options: AnyBotClientOptions) {
  const client = createClient<paths>({
    baseUrl: options.baseUrl ?? sameOrigin(),
    ...(options.fetch === undefined ? {} : { fetch: options.fetch }),
  });
  client.use(headerMiddleware(options.clientVersion));
  return client;
}

export type AnyBotClient = ReturnType<typeof createAnyBotClient>;
