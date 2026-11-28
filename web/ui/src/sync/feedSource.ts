import { type AnyBotClient, type ApiProblem, attempt } from "@anybot/api-client";

import { createNotificationHub } from "./notificationHub.ts";
import type { StoredRecord } from "./records.ts";
import { type SseEvent, SseParser } from "./sse.ts";

export interface AfterPage {
  readonly records: readonly StoredRecord[];
  readonly hasMore: boolean;
}

export type PageResult =
  | { readonly ok: true; readonly page: AfterPage }
  | { readonly ok: false; readonly problem: ApiProblem };

export interface FeedSource {
  fetchAfter(stream: string, after: number, deviceAt: string | null): Promise<PageResult>;
  openNotifications(onEvent: (event: SseEvent) => void, signal: AbortSignal): Promise<void>;
}

async function pumpStream(
  body: ReadableStream<Uint8Array>,
  onEvent: (event: SseEvent) => void,
): Promise<void> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  const parser = new SseParser();
  for (;;) {
    const chunk = await reader.read();
    if (chunk.done) {
      return;
    }
    for (const event of parser.feed(decoder.decode(chunk.value, { stream: true }))) {
      onEvent(event);
    }
  }
}

async function connectNotifications(
  client: AnyBotClient,
  onEvent: (event: SseEvent) => void,
  signal: AbortSignal,
): Promise<void> {
  const reply = await client.GET("/v1/streams/notifications", {
    params: { query: { init_seq: 0 } },
    parseAs: "stream",
    signal,
  });
  if (!reply.response.ok || reply.data === null || reply.data === undefined) {
    throw new Error("notification stream refused");
  }
  await pumpStream(reply.data, onEvent);
}

const sources = new WeakMap<AnyBotClient, FeedSource>();

function buildSource(client: AnyBotClient): FeedSource {
  const hub = createNotificationHub((onEvent, signal) =>
    connectNotifications(client, onEvent, signal),
  );
  return {
    async fetchAfter(stream, after, deviceAt) {
      const query = deviceAt === null ? { stream, after } : { stream, after, device_at: deviceAt };
      const result = await attempt(() => client.GET("/v1/streams/after", { params: { query } }));
      if (!result.ok) {
        return result;
      }
      return {
        ok: true,
        page: { records: result.data.records, hasMore: result.data.has_more },
      };
    },
    openNotifications: (onEvent, signal) => hub.subscribe(onEvent, signal),
  };
}

export function createFeedSource(client: AnyBotClient): FeedSource {
  const known = sources.get(client);
  if (known !== undefined) {
    return known;
  }
  const built = buildSource(client);
  sources.set(client, built);
  return built;
}
