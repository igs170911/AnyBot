export interface SseEvent {
  readonly event: string;
  readonly data: string;
}

const DEFAULT_EVENT = "message";

function parseBlock(block: string): SseEvent | null {
  let event = DEFAULT_EVENT;
  const data: string[] = [];
  for (const line of block.split("\n")) {
    if (line.startsWith(":")) {
      continue;
    }
    if (line.startsWith("event:")) {
      event = line.slice("event:".length).trim();
    } else if (line.startsWith("data:")) {
      data.push(line.slice("data:".length).replace(/^ /, ""));
    }
  }
  return data.length === 0 ? null : { event, data: data.join("\n") };
}

export class SseParser {
  private buffer = "";

  feed(chunk: string): SseEvent[] {
    this.buffer += chunk.replaceAll("\r\n", "\n");
    const events: SseEvent[] = [];
    let cut = this.buffer.indexOf("\n\n");
    while (cut >= 0) {
      const parsed = parseBlock(this.buffer.slice(0, cut));
      if (parsed !== null) {
        events.push(parsed);
      }
      this.buffer = this.buffer.slice(cut + 2);
      cut = this.buffer.indexOf("\n\n");
    }
    return events;
  }
}
