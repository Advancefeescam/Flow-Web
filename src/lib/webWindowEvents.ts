// Browser-only cross-tab events; BroadcastChannel does not echo to its own sender.
import type { DataChangedPayload, WatchHistoryWrittenPayload } from "./windowEvents";

type WebWindowEventMap = {
  "flow:data-changed": DataChangedPayload;
  "flow:watch-history-written": WatchHistoryWrittenPayload;
  "flow:playback-claimed": { source: string };
};

let channel: BroadcastChannel | null = null;
let listeners = 0;

function getChannel(): BroadcastChannel | null {
  if (channel) return channel;
  if (typeof window === "undefined" || typeof BroadcastChannel === "undefined") return null;
  try {
    channel = new BroadcastChannel("flow-web-window-events-v1");
  } catch {
    // Some browser privacy modes disable cross-tab communication.
  }
  return channel;
}

export function publishWebWindowEvent<K extends keyof WebWindowEventMap>(name: K, payload: WebWindowEventMap[K]): void {
  const target = getChannel();
  if (!target) return;
  try {
    target.postMessage({ name, payload });
  } catch {
    // Local playback and settings must keep working if broadcasting is unavailable.
  } finally {
    if (!listeners) {
      target.close();
      channel = null;
    }
  }
}

export function subscribeWebWindowEvent<K extends keyof WebWindowEventMap>(
  name: K, handler: (payload: WebWindowEventMap[K]) => void,
): () => void {
  const target = getChannel();
  if (!target) return () => {};
  const onMessage = (event: MessageEvent) => {
    if (event.data?.name === name && event.data.payload != null) handler(event.data.payload);
  };
  target.addEventListener("message", onMessage);
  listeners++;
  let active = true;
  return () => {
    if (!active) return;
    active = false;
    target.removeEventListener("message", onMessage);
    if (--listeners === 0) {
      target.close();
      channel = null;
    }
  };
}
