import { describe, expect, it } from "vitest";

import { SseParser } from "./sse.ts";

describe("SseParser", () => {
  it("reads events split across chunks and skips heartbeats", () => {
    const parser = new SseParser();
    const first = parser.feed(': hb\n\nevent: new-records\ndata: {"stream"');
    const second = parser.feed(':"bot/1"}\n\n');
    expect(first).toEqual([]);
    expect(second).toEqual([{ event: "new-records", data: '{"stream":"bot/1"}' }]);
  });

  it("joins multiple data lines and handles CRLF", () => {
    const events = new SseParser().feed("event: x\r\ndata: a\r\ndata: b\r\n\r\n");
    expect(events).toEqual([{ event: "x", data: "a\nb" }]);
  });
});
