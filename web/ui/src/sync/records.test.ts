import { describe, expect, it } from "vitest";

import { botsFromRecords } from "./accountBots.ts";
import { mergeRecords, type StoredRecord } from "./records.ts";

function record(seq: number, kind = "message", content: unknown = {}): StoredRecord {
  return { seq, kind, content, device_id: "d", created_at_ms: 0, offline_sent: false };
}

describe("records", () => {
  it("mergeRecords keeps one copy per seq", () => {
    const merged = mergeRecords(
      [record(1), record(2), record(3)],
      [record(2), record(3), record(4)],
    );
    expect(merged.map((item) => item.seq)).toEqual([1, 2, 3, 4]);
  });

  it("mergeRecords orders late arrivals by seq", () => {
    const merged = mergeRecords([record(3)], [record(1), record(2)]);
    expect(merged.map((item) => item.seq)).toEqual([1, 2, 3]);
  });

  it("bots list follows create and delete records", () => {
    const bots = botsFromRecords([
      record(1, "bots_create", { bot_id: "a", name: "First" }),
      record(2, "bots_create", { bot_id: "b", name: "Second" }),
      record(3, "bots_delete", { bot_id: "a" }),
    ]);
    expect(bots).toEqual([{ id: "b", name: "Second" }]);
  });

  it("restored bots come back and unknown records are ignored", () => {
    const bots = botsFromRecords([
      record(1, "bots_create", { bot_id: "a", name: "First" }),
      record(2, "bots_delete", { bot_id: "a" }),
      record(3, "bots_restore", { bot_id: "a" }),
      record(4, "message", "plain"),
    ]);
    expect(bots).toEqual([{ id: "a", name: "First" }]);
  });
});
