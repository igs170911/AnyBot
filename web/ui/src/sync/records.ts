import type { components } from "@anybot/api-client";

export type StoredRecord = components["schemas"]["StoredRecord"];

export type StreamRef =
  | { readonly kind: "account" }
  | { readonly kind: "bot"; readonly id: string };

export const ACCOUNT_STREAM_KEY = "account/self";

export function streamKeyOf(stream: StreamRef): string {
  return stream.kind === "account" ? ACCOUNT_STREAM_KEY : `bot/${stream.id}`;
}

export function streamIdentity(stream: StreamRef): string {
  return stream.kind === "account" ? "account" : `bot:${stream.id}`;
}

export function refOfIdentity(identity: string): StreamRef {
  return identity.startsWith("bot:")
    ? { kind: "bot", id: identity.slice("bot:".length) }
    : { kind: "account" };
}

export function matchesStream(stream: StreamRef, eventStream: string): boolean {
  if (stream.kind === "account") {
    return eventStream.startsWith("account/");
  }
  return eventStream === `bot/${stream.id}`;
}

export function mergeRecords(
  existing: readonly StoredRecord[],
  incoming: readonly StoredRecord[],
): StoredRecord[] {
  const bySeq = new Map<number, StoredRecord>();
  for (const record of existing) {
    bySeq.set(record.seq, record);
  }
  for (const record of incoming) {
    if (!bySeq.has(record.seq)) {
      bySeq.set(record.seq, record);
    }
  }
  return [...bySeq.values()].sort((left, right) => left.seq - right.seq);
}
