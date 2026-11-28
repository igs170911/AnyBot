import type { SseEvent } from "./sse.ts";

export type EventListener = (event: SseEvent) => void;

export type Connect = (onEvent: EventListener, signal: AbortSignal) => Promise<void>;

export interface NotificationHub {
  subscribe(listener: EventListener, signal: AbortSignal): Promise<void>;
}

export function createNotificationHub(connect: Connect): NotificationHub {
  const listeners = new Set<EventListener>();
  const finishers = new Set<() => void>();
  let controller: AbortController | null = null;
  let running = false;

  const endAll = () => {
    running = false;
    controller = null;
    for (const finish of [...finishers]) {
      finish();
    }
  };

  const ensureConnected = () => {
    if (running) {
      return;
    }
    running = true;
    controller = new AbortController();
    const deliver: EventListener = (event) => {
      for (const listener of [...listeners]) {
        listener(event);
      }
    };
    connect(deliver, controller.signal).then(endAll, endAll);
  };

  return {
    subscribe(listener, signal) {
      return new Promise<void>((resolve) => {
        listeners.add(listener);
        const finish = () => {
          listeners.delete(listener);
          finishers.delete(finish);
          if (listeners.size === 0) {
            controller?.abort();
          }
          resolve();
        };
        finishers.add(finish);
        signal.addEventListener("abort", finish, { once: true });
        ensureConnected();
      });
    },
  };
}
