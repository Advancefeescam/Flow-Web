// The Linux AppImage's webview can't verify HTTPS certificates on non-Debian
// distros, so there remote images load through the local proxy's image route
// (src-tauri/src/streaming/images.rs). Keep this host list in step with the
// backend's, which is the one that enforces it.
const PROXIED_HOST_RE =
  /^(?:(?:[\w-]+\.)*(?:ytimg\.com|ggpht\.com|googleusercontent\.com)|img\.youtube\.com|dearrow-thumb\.ajay\.app)$/i;
export const WEB_IMAGE_BASE = "/media/image";
const IMAGE_ROUTE_RE = /^http:\/\/127\.0\.0\.1:\d+\/img\/[^/?#]+$/;

/** `url` routed through the image proxy at `base`, or unchanged when no proxy is needed or allowed. */
export function proxiedImage(url: string, base: string | null): string {
  if (!base) return url;
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "https:" || !PROXIED_HOST_RE.test(parsed.hostname)) return url;
    if (base === WEB_IMAGE_BASE && (parsed.username || parsed.password || parsed.port || parsed.hash)) return url;
  } catch {
    return url;
  }
  return `${base}?u=${encodeURIComponent(url)}`;
}

/** The remote URL behind a proxied image URL; any other URL unchanged. */
export function originalImageUrl(url: string): string {
  try {
    const parsed = new URL(url, typeof location === "undefined" ? "http://localhost" : location.origin);
    const webRoute = parsed.pathname === WEB_IMAGE_BASE && (url.startsWith("/") || (typeof location !== "undefined" && parsed.origin === location.origin));
    if (!webRoute && !IMAGE_ROUTE_RE.test(`${parsed.origin}${parsed.pathname}`)) return url;
    return parsed.searchParams.get("u") ?? url;
  } catch {
    return url;
  }
}
