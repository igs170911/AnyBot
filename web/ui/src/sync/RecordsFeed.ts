import type { ApiProblem } from "@anybot/api-client";

import type { FeedSource } from "./feedSource.ts";
import {
  matchesStream,
  mergeRecords,
  type StoredRecord,
  type StreamRef,
  streamKeyOf,
} from "./records.ts";
import type { SseEvent } from "./sse.ts";

export interface FeedState {
  readonly records: readonly StoredRecord[];
  readonly maxSeq: number;
  readonly problem: ApiProblem | null;
}

export const EMPTY_FEED: FeedState = { records: [], maxSeq: 0, problem: null };

export const RECONNECT_DELAY_MS = 1000;

const HINT_EVENTS: ReadonlySet<string> = new Set(["new-records", "progress"]);

function streamsOfHint(event: SseEvent): string[] {
  try {
    const body: unknown = JSON.parse(event.data);
    if (typeof body !== "object" || body === null) {
      return [];
    }
    const single = (body as { stream?: unknown }).stream;
    if (typeof single === "string") {
      return [single];
    }
    const rows = (body as { streams?: unknown }).streams;
    if (!Array.isArray(rows)) {
      return [];
    }
    return rows.flatMap((row: unknown) => {
      const name = (row as { stream?: unknown } | null)?.stream;
      return typeof name === "string" ? [name] : [];
    });
  } catch {
    return [];
  }
}

export interface FeedOptions {
  readonly source: FeedSource;
  readonly stream: StreamRef;
  readonly onChange: (state: FeedState) => void;
  readonly wait?: (ms: number) => Promise<void>;
}

function realWait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export class RecordsFeed {
  private state: FeedState = EMPTY_FEED;
  private lastDeviceId: string | null = null;
  private pulling = false;
  private again = false;
  private readonly abort = new AbortController();
  private readonly source: FeedSource;
  private readonly stream: StreamRef;
  private readonly onChange: (state: FeedState) => void;
  private readonly wait: (ms: number) => Promise<void>;

  constructor(options: FeedOptions) {
    this.source = options.source;
    this.stream = options.stream;
    this.onChange = options.onChange;
    this.wait = options.wait ?? realWait;
  }

  private isStopped(): boolean {
    return this.abort.signal.aborted;
  }

  private isPulling(): boolean {
    return this.pulling;
  }

  private repeatRequested(): boolean {
    return this.again;
  }

  snapshot(): FeedState {
    return this.state;
  }

  start(): void {
    void this.listen();
  }

  stop(): void {
    this.abort.abort();
  }

  async sync(): Promise<void> {
    if (this.isPulling()) {
      this.again = true;
      return;
    }
    this.pulling = true;
    try {
      do {
        this.again = false;
        await this.pullAll();
      } while (this.repeatRequested() && !this.isStopped());
    } finally {
      this.pulling = false;
    }
  }

  hint(event: SseEvent): void {
    if (!HINT_EVENTS.has(event.event)) {
      return;
    }
    if (streamsOfHint(event).some((name) => matchesStream(this.stream, name))) {
      void this.sync();
    }
  }

  private async listen(): Promise<void> {
    await this.sync();
    while (!this.isStopped()) {
      try {
        await this.source.openNotifications((event) => this.hint(event), this.abort.signal);
      } catch {
        if (this.isStopped()) {
          return;
        }
      }
      if (this.isStopped()) {
        return;
      }
      await this.wait(RECONNECT_DELAY_MS);
      await this.sync();
    }
  }

  private async pullAll(): Promise<void> {
    for (;;) {
      const result = await this.source.fetchAfter(
        streamKeyOf(this.stream),
        this.state.maxSeq,
        this.lastDeviceId,
      );
      if (this.isStopped()) {
        return;
      }
      if (!result.ok) {
        this.publish({ ...this.state, problem: result.problem });
        return;
      }
      this.absorb(result.page.records);
      if (!result.page.hasMore || result.page.records.length === 0) {
        return;
      }
    }
  }

  private absorb(incoming: readonly StoredRecord[]): void {
    const records = mergeRecords(this.state.records, incoming);
    const last = records[records.length - 1];
    this.lastDeviceId = last?.device_id ?? this.lastDeviceId;
    this.publish({ records, maxSeq: last?.seq ?? this.state.maxSeq, problem: null });
  }

  private publish(next: FeedState): void {
    this.state = next;
    this.onChange(next);
  }
}
