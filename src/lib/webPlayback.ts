import type { StreamInfo } from "../types/video";

// Web-only extension of the upstream wire contract. Desktop responses do not
// contain it, so the existing source selection remains identical there.
type WebStreamInfo = StreamInfo & { webDirectDashManifestUrl?: string };
const DIRECT_COOLDOWN_MS = 5 * 60_000;

export function createWebDashPolicy() {
  let retryDirectAfter = 0;
  const directUrl = (info: WebStreamInfo): string | null =>
    !info.isLive && !!info.dashManifestUrl && info.webDirectDashManifestUrl
      && info.webDirectDashManifestUrl !== info.dashManifestUrl ? info.webDirectDashManifestUrl : null;

  return {
    choose(info: WebStreamInfo, now = Date.now()): string | null {
      return (now >= retryDirectAfter ? directUrl(info) : null) || info.dashManifestUrl || null;
    },
    fallback(info: WebStreamInfo, active: string | null, now = Date.now()): string | null {
      if (!active || active !== directUrl(info)) return null;
      retryDirectAfter = now + DIRECT_COOLDOWN_MS;
      return info.dashManifestUrl || null;
    },
  };
}

const policy = createWebDashPolicy();
export const chooseWebDashManifest = policy.choose;
export const fallbackWebDashManifest = policy.fallback;

/** Pure local pointer-to-time mapping, independent of media network requests. */
export function webScrubTime(clientX: number, left: number, width: number, duration: number): number | null {
  if (![clientX, left, width, duration].every(Number.isFinite) || width <= 0 || duration <= 0) return null;
  return Math.min(1, Math.max(0, (clientX - left) / width)) * duration;
}
