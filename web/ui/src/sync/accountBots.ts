import type { StoredRecord } from "./records.ts";

export interface BotEntry {
  readonly id: string;
  readonly name: string;
}

function botIdOf(record: StoredRecord): string | null {
  const content = record.content;
  if (typeof content !== "object" || content === null) {
    return null;
  }
  const id = (content as { bot_id?: unknown }).bot_id;
  return typeof id === "string" ? id : null;
}

function nameOf(record: StoredRecord): string {
  const name = (record.content as { name?: unknown }).name;
  return typeof name === "string" ? name : "";
}

export function botsFromRecords(records: readonly StoredRecord[]): BotEntry[] {
  const live = new Map<string, BotEntry>();
  const removed = new Map<string, BotEntry>();
  for (const record of records) {
    const id = botIdOf(record);
    if (id === null) {
      continue;
    }
    if (record.kind === "bots_create") {
      live.set(id, { id, name: nameOf(record) });
    } else if (record.kind === "bots_delete") {
      const entry = live.get(id);
      if (entry !== undefined) {
        live.delete(id);
        removed.set(id, entry);
      }
    } else if (record.kind === "bots_restore") {
      const entry = removed.get(id);
      if (entry !== undefined) {
        removed.delete(id);
        live.set(id, entry);
      }
    }
  }
  return [...live.values()];
}
