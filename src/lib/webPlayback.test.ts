import { describe, expect, it } from "vitest";
import type { StreamInfo } from "../types/video";
import { createWebDashPolicy, webScrubTime } from "./webPlayback";

const info = {streamId: "id", localUrl: "/media/stream/video", expiresAt: "", variants: [], captions: [], audioTracks: [],
  dashManifestUrl: "/media/stream/proxy", webDirectDashManifestUrl: "/media/stream/direct"} satisfies StreamInfo & {webDirectDashManifestUrl: string};

describe("web direct DASH policy", () => {
  it("uses direct media only with a proxy fallback", () => {
    const policy = createWebDashPolicy();
    expect(policy.choose(info, 0)).toBe(info.webDirectDashManifestUrl);
    expect(policy.choose({...info, webDirectDashManifestUrl: undefined}, 0)).toBe(info.dashManifestUrl);
    expect(policy.choose({...info, dashManifestUrl: null}, 0)).toBeNull();
    expect(policy.choose({...info, isLive: true}, 0)).toBe(info.dashManifestUrl);
  });
  it("falls back to the same manifest quality and temporarily avoids direct retries", () => {
    const policy = createWebDashPolicy();
    expect(policy.fallback(info, info.webDirectDashManifestUrl, 10)).toBe(info.dashManifestUrl);
    expect(policy.fallback(info, info.dashManifestUrl, 11)).toBeNull();
    expect(policy.choose(info, 12)).toBe(info.dashManifestUrl);
    expect(policy.choose(info, 300011)).toBe(info.webDirectDashManifestUrl);
  });
});
describe("web scrub preview", () => {
  it("maps and clamps the pointer locally", () => {
    expect(webScrubTime(150, 100, 100, 200)).toBe(100);
    expect(webScrubTime(20, 100, 100, 200)).toBe(0);
    expect(webScrubTime(300, 100, 100, 200)).toBe(200);
    expect(webScrubTime(150, 100, 0, 200)).toBeNull();
    expect(webScrubTime(150, 100, 100, Infinity)).toBeNull();
  });
});
