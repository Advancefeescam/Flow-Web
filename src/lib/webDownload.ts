import type { StreamInfo } from "../types/video";

export interface BrowserDownloadChoice {
  label: string;
  url: string;
  extension: "mp4" | "m4a" | "webm" | "ogg";
}

export function browserVideoChoices(info: StreamInfo): BrowserDownloadChoice[] {
  const choices = info.variants
    .filter((variant) => variant.isPlayable && variant.hasAudio && isLocalStream(variant.localUrl) &&
      (!variant.mimeType || /^video\/(mp4|webm)(?:;|$)/.test(variant.mimeType)))
    .map((variant) => ({
      label: variant.qualityLabel,
      url: variant.localUrl,
      extension: variant.mimeType?.startsWith("video/webm") ? "webm" as const : "mp4" as const,
    }));
  if (isLocalStream(info.localUrl) && !choices.some((choice) => choice.url === info.localUrl)) {
    choices.push({ label: "Auto", url: info.localUrl, extension: "mp4" });
  }
  return choices;
}

export function browserAudioChoices(info: StreamInfo): BrowserDownloadChoice[] {
  return info.audioTracks
    .filter((track) => track.available && isLocalStream(track.localUrl) &&
      (!track.mimeType || /^audio\/(mp4|webm|ogg)(?:;|$)/.test(track.mimeType)))
    .map((track) => ({
      label: track.label,
      url: track.localUrl,
      extension: track.mimeType?.startsWith("audio/webm") ? "webm" as const
        : track.mimeType?.startsWith("audio/ogg") ? "ogg" as const : "m4a" as const,
    }));
}

export function isLocalStream(raw: string): boolean {
  try {
    const url = new URL(raw, location.origin);
    return url.origin === location.origin && /^\/media\/stream\/[0-9a-f-]{36}$/.test(url.pathname);
  } catch {
    return false;
  }
}

export function browserDownloadUrl(choice: BrowserDownloadChoice, title: string): string {
  if (!isLocalStream(choice.url)) throw new Error("The download link is unavailable.");
  const name = title.normalize("NFKD").replace(/[^a-zA-Z0-9 _().-]/g, "").trim().slice(0, 120) || "Flow download";
  const url = new URL(choice.url, location.origin);
  url.searchParams.set("download", `${name}.${choice.extension}`);
  return url.href;
}

export function startBrowserDownload(choice: BrowserDownloadChoice, title: string): void {
  const anchor = document.createElement("a");
  anchor.href = browserDownloadUrl(choice, title);
  anchor.download = "";
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
}
