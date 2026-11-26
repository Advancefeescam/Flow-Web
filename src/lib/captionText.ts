import { invokeBackend } from "./api/errors";
import type { CaptionTrack } from "../types/video";

const LOCAL_URL_PREFIX = "local-subtitle:";
const browserCaptions = new Map<string, string>();

export function registerBrowserCaption(text: string): string {
  const id = `web:${crypto.randomUUID()}`;
  browserCaptions.set(id, text);
  return id;
}

export function forgetBrowserCaption(path: string): void {
  browserCaptions.delete(path);
}

export const localCaptionUrl = (path: string) => `${LOCAL_URL_PREFIX}${path}`;

export const isLocalCaption = (caption: Pick<CaptionTrack, "url">) => caption.url.startsWith(LOCAL_URL_PREFIX);

export const localCaptionPath = (caption: Pick<CaptionTrack, "url">) => caption.url.slice(LOCAL_URL_PREFIX.length);

/** A caption track's text, from YouTube or from a file on this computer. */
export function fetchCaptionText(caption: Pick<CaptionTrack, "url">): Promise<string> {
  if (isLocalCaption(caption)) {
    const path = localCaptionPath(caption);
    if (path.startsWith("web:")) {
      const text = browserCaptions.get(path);
      return text === undefined ? Promise.reject(new Error("Subtitle file is no longer available")) : Promise.resolve(text);
    }
    return invokeBackend<string>("read_subtitle_file", { path });
  }
  return invokeBackend<string>("fetch_subtitles", { url: caption.url });
}
