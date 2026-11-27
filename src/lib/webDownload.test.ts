import { describe, expect, it } from "vitest";
import { browserDownloadUrl, browserVideoChoices, isLocalStream } from "./webDownload";
import type { StreamInfo } from "../types/video";

const token = "019c6fbd-8014-4b7b-8aa7-ac7bb084ca8f";

describe("browser downloads", () => {
  it("offers only muxed streams served by this Flow instance", () => {
    const info = {
      localUrl: "",
      variants: [
        { localUrl: `/media/stream/${token}`, qualityLabel: "720p", mimeType: "video/mp4", isPlayable: true, hasAudio: true },
        { localUrl: "/media/stream/other", qualityLabel: "1080p", isPlayable: true, hasAudio: false },
        { localUrl: "https://evil.example/video", qualityLabel: "Fake", isPlayable: true, hasAudio: true },
      ],
    } as StreamInfo;
    expect(browserVideoChoices(info)).toEqual([{ label: "720p", url: `/media/stream/${token}`, extension: "mp4" }]);
  });

  it("rejects foreign origins and unsafe filenames", () => {
    expect(isLocalStream(`https://evil.example/media/stream/${token}`)).toBe(false);
    expect(isLocalStream(`/media/stream/${token}/../other`)).toBe(false);
    const url = new URL(browserDownloadUrl({ label: "Music", url: `/media/stream/${token}`, extension: "m4a" }, "A/B\r\n<script>"));
    expect(url.searchParams.get("download")).toBe("ABscript.m4a");
  });
});
