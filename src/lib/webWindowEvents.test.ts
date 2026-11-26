// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { publishWebWindowEvent, subscribeWebWindowEvent } from "./webWindowEvents";

class FakeChannel extends EventTarget {
  static instances: FakeChannel[] = [];
  closed = false;
  constructor(readonly name: string) {
    super();
    FakeChannel.instances.push(this);
  }
  postMessage(data: unknown) {
    for (const peer of FakeChannel.instances) {
      if (peer !== this && !peer.closed && peer.name === this.name) {
        peer.dispatchEvent(new MessageEvent("message", { data }));
      }
    }
  }
  close() { this.closed = true; }
}

afterEach(() => {
  FakeChannel.instances = [];
  vi.unstubAllGlobals();
});

describe("web window events", () => {
  it("routes messages across tabs without echo and closes when unsubscribed", () => {
    vi.stubGlobal("BroadcastChannel", FakeChannel);
    const peer = new FakeChannel("flow-web-window-events-v1");
    const inbound = vi.fn();
    const outbound = vi.fn();
    peer.addEventListener("message", outbound);
    const stop = subscribeWebWindowEvent("flow:data-changed", inbound);
    try {
      const payload = { source: "main", keys: ["liked_items"] };
      publishWebWindowEvent("flow:data-changed", payload);
      expect(outbound).toHaveBeenCalledTimes(1);
      expect(inbound).not.toHaveBeenCalled();
      peer.postMessage({ name: "flow:playback-claimed", payload: { source: "main" } });
      expect(inbound).not.toHaveBeenCalled();
      peer.postMessage({ name: "flow:data-changed", payload });
      expect(inbound).toHaveBeenCalledWith(payload);
      stop();
      stop();
      expect(FakeChannel.instances[1]?.closed).toBe(true);
      peer.postMessage({ name: "flow:data-changed", payload });
      expect(inbound).toHaveBeenCalledTimes(1);
    } finally { stop(); peer.close(); }
  });

  it("keeps local behavior available when BroadcastChannel is unsupported", () => {
    vi.stubGlobal("BroadcastChannel", undefined);
    expect(() => publishWebWindowEvent("flow:playback-claimed", { source: "main" })).not.toThrow();
    expect(() => subscribeWebWindowEvent("flow:playback-claimed", vi.fn())()).not.toThrow();
  });
});
