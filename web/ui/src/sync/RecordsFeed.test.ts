import { describe, expect, it } from "vitest";

import type { FeedSource, PageResult } from "./feedSource.ts";
import { RecordsFeed } from "./RecordsFeed.ts";
import type { StoredRecord } from "./records.ts";
import type { SseEvent } from "./sse.ts";

function record(seq: number): StoredRecord {
  return {
    seq,
    kind: "message",
    content: {},
    device_id: `d${seq}`,
    created_at_ms: 0,
    offline_sent: false,
  };
}

interface Call {
  stream: string;
  after: number;
  deviceAt: string | null;
}

function fakeSource(all: StoredRecord[], calls: Call[], pageSize = 100): FeedSource {
  return {
    fetchAfter(stream, after, deviceAt): Promise<PageResult> {
      calls.push({ stream, after, deviceAt });
      const rest = all.filter((item) => item.seq > after);
      const page = rest.slice(0, pageSize);
      return Promise.resolve({
        ok: true,
        page: { records: page, hasMore: rest.length > page.length },
      });
    },
    openNotifications: () => new Promise<void>(() => undefined),
  };
}

const NEW_RECORDS: SseEvent = {
  event: "new-records",
  data: '{"stream":"bot/1","seq":2,"device_id":"d"}',
};

function makeFeed(source: FeedSource) {
  return new RecordsFeed({ source, stream: { kind: "bot", id: "1" }, onChange: () => undefined });
}

describe("RecordsFeed", () => {
  it("a missed or duplicated hint gives the same records", async () => {
    const all = [record(1), record(2), record(3)];
    const plain = makeFeed(fakeSource(all, []));
    await plain.sync();
    const noisy = makeFeed(fakeSource(all, []));
    noisy.hint(NEW_RECORDS);
    noisy.hint(NEW_RECORDS);
    await noisy.sync();
    await noisy.sync();
    expect(noisy.snapshot().records).toEqual(plain.snapshot().records);
    expect(noisy.snapshot().maxSeq).toBe(3);
  });

  it("pages continue with the device id of the last record seen", async () => {
    const calls: Call[] = [];
    const feed = makeFeed(fakeSource([record(1), record(2), record(3)], calls, 2));
    await feed.sync();
    expect(calls.map((call) => [call.after, call.deviceAt])).toEqual([
      [0, null],
      [2, "d2"],
    ]);
    expect(feed.snapshot().maxSeq).toBe(3);
  });

  it("hints for other streams do not trigger a pull", async () => {
    const calls: Call[] = [];
    const feed = makeFeed(fakeSource([record(1)], calls));
    feed.hint({ event: "new-records", data: '{"stream":"bot/2","seq":1,"device_id":"d"}' });
    await feed.sync();
    expect(calls).toHaveLength(1);
  });
});
