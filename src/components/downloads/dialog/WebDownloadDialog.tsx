import { useEffect, useState } from "react";
import { AudioLines, FileVideo2 } from "lucide-react";

import { getBackendErrorMessage } from "../../../lib/api/errors";
import { getMusicStream } from "../../../lib/api/music";
import { getStreamInfo } from "../../../lib/api/youtube";
import { getString } from "../../../lib/i18n/index";
import {
  browserAudioChoices,
  browserVideoChoices,
  isLocalStream,
  startBrowserDownload,
  type BrowserDownloadChoice,
} from "../../../lib/webDownload";
import type { SongItem } from "../../../types/music";
import type { VideoSummary } from "../../../types/video";
import { Button } from "../../ui/Button";
import { DialogFrame, DialogHeader } from "./DownloadDialogShell";

type DownloadItem = { kind: "video"; video: VideoSummary } | { kind: "music"; track: SongItem };

export function WebDownloadDialog({ item, onClose }: { item: DownloadItem; onClose: () => void }) {
  const [mode, setMode] = useState<"video" | "audio">(item.kind === "music" ? "audio" : "video");
  const [videoChoices, setVideoChoices] = useState<BrowserDownloadChoice[]>([]);
  const [audioChoices, setAudioChoices] = useState<BrowserDownloadChoice[]>([]);
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const title = item.kind === "video" ? item.video.title : item.track.title;
  const itemId = item.kind === "video" ? item.video.id : item.track.videoId ?? item.track.id;
  const [fileName, setFileName] = useState(title);
  const choices = mode === "video" ? videoChoices : audioChoices;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    if (item.kind === "video") {
      void getStreamInfo(itemId).then((info) => {
        if (cancelled) return;
        setVideoChoices(browserVideoChoices(info));
        setAudioChoices(browserAudioChoices(info));
      }).catch((reason) => {
        if (!cancelled) setError(getBackendErrorMessage(reason));
      }).finally(() => {
        if (!cancelled) setLoading(false);
      });
    } else {
      void getMusicStream(itemId, "High", true).then((info) => {
        if (cancelled) return;
        if (!isLocalStream(info.audioUrl) || !/^audio\/(mp4|webm)(?:;|$)/.test(info.mimeType)) {
          throw new Error("The audio download is unavailable.");
        }
        setAudioChoices([{
          label: getString("download_quality_high"),
          url: info.audioUrl,
          extension: info.mimeType.startsWith("audio/webm") ? "webm" : "m4a",
        }]);
      }).catch((reason) => {
        if (!cancelled) setError(getBackendErrorMessage(reason));
      }).finally(() => {
        if (!cancelled) setLoading(false);
      });
    }
    return () => { cancelled = true; };
  }, [item.kind, itemId]);

  const submit = () => {
    const choice = choices[selected];
    if (!choice) return;
    try {
      startBrowserDownload(choice, fileName);
      onClose();
    } catch (reason) {
      setError(getBackendErrorMessage(reason));
    }
  };

  return (
    <DialogFrame onClose={onClose}>
      <DialogHeader
        artwork={item.kind === "video" ? item.video.thumbnailUrl : item.track.thumbnail}
        shape={item.kind === "video" ? "landscape" : "square"}
        fallbackIcon={item.kind === "video" ? <FileVideo2 size={22} /> : <AudioLines size={22} />}
        title={title}
        subtitle={item.kind === "video" ? item.video.channelName : item.track.artists.map((artist) => artist.name).join(", ")}
        onClose={onClose}
      />
      <div className="flex flex-col gap-3 px-5 pb-5">
        {item.kind === "video" ? (
          <div className="flex gap-2" role="group" aria-label={getString("download_mode")}>
            {(["video", "audio"] as const).map((value) => (
              <Button key={value} variant={mode === value ? "primary" : "ghost"} onClick={() => { setMode(value); setSelected(0); }}>
                {getString(value === "video" ? "download_mode_video" : "download_mode_audio")}
              </Button>
            ))}
          </div>
        ) : null}
        {loading ? <p className="text-sm text-chrome-neutral-400">{getString("download_preparing")}</p> : null}
        {!loading && choices.length > 0 ? (
          <label className="flex flex-col gap-1 text-sm text-chrome-neutral-300">
            {getString("download_quality")}
            <select value={selected} onChange={(event) => setSelected(Number(event.target.value))}
              className="rounded-lg border border-outline-soft bg-surface-container-high px-3 py-2 text-chrome-neutral-100">
              {choices.map((choice, index) => <option key={`${choice.url}:${index}`} value={index}>{choice.label} · {choice.extension.toUpperCase()}</option>)}
            </select>
          </label>
        ) : null}
        <label className="flex flex-col gap-1 text-sm text-chrome-neutral-300">
          {getString("download_file_name")}
          <input value={fileName} onChange={(event) => setFileName(event.target.value)} maxLength={120}
            className="rounded-lg border border-outline-soft bg-surface-container-high px-3 py-2 text-chrome-neutral-100" />
        </label>
        {!loading && choices.length === 0 && !error ? <p className="text-sm text-chrome-neutral-400">{getString("download_browser_unavailable")}</p> : null}
        {error ? <p role="alert" className="text-sm text-red-400">{error}</p> : null}
        <p className="text-xs text-chrome-neutral-400">{getString("download_browser_note")}</p>
      </div>
      <div className="flex justify-end gap-2 border-t border-outline-soft px-5 py-3">
        <Button variant="ghost" onClick={onClose}>{getString("cancel")}</Button>
        <Button onClick={submit} disabled={loading || choices.length === 0}>{getString("download")}</Button>
      </div>
    </DialogFrame>
  );
}
