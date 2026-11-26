import { getSetting, getWatchHistory } from "../api/db";
import { saveJsonFile } from "../saveJsonFile";
import type { ExportResult } from "../api/backup";
import type { WatchHistoryRecord } from "../../types/db";

const PAGE_SIZE = 10_000;
const CHANNEL_ID = /^UC[A-Za-z0-9_-]{22}$/;
const HANDLE = /^@[\p{L}\p{N}_.-]{1,100}$/u;

function channelUrl(id: string): string | null {
  const trimmed = id.trim();
  if (CHANNEL_ID.test(trimmed)) return `https://www.youtube.com/channel/${trimmed}`;
  if (HANDLE.test(trimmed)) return `https://www.youtube.com/${trimmed}`;
  return null;
}

export function historyToTakeout(records: WatchHistoryRecord[]): unknown[] {
  return [...records]
    .filter((record) => record.videoId.trim())
    .sort((a, b) => Date.parse(b.watchDate) - Date.parse(a.watchDate))
    .map((record) => ({
      header: "YouTube",
      title: `Watched ${record.title}`,
      titleUrl: `https://www.youtube.com/watch?v=${record.videoId.trim()}`,
      subtitles: [{
        name: record.channelName?.trim() || "YouTube",
        url: record.channelId ? channelUrl(record.channelId) ?? "https://www.youtube.com/" : "https://www.youtube.com/",
      }],
      time: new Date(Number.isFinite(Date.parse(record.watchDate)) ? Date.parse(record.watchDate) : 0).toISOString(),
      products: ["YouTube"],
      activityControls: ["YouTube watch history"],
    }));
}

export function subscriptionsToNewpipe(raw: unknown): { file: unknown; skipped: number } {
  const channels = Array.isArray(raw) ? raw : [];
  const subscriptions = channels.flatMap((channel) => {
    if (!channel || typeof channel !== "object" || typeof channel.id !== "string") return [];
    const url = channelUrl(channel.id);
    if (!url) return [];
    return [{ service_id: 0, url, name: typeof channel.name === "string" && channel.name.trim() ? channel.name : channel.id }];
  });
  return {
    file: { app_version: "Flow Web", app_version_int: 1, subscriptions },
    skipped: channels.length - subscriptions.length,
  };
}

export async function saveBrowserExport(kind: "WATCH_HISTORY" | "NEWPIPE_SUBSCRIPTIONS", title: string, name: string): Promise<ExportResult> {
  if (kind === "WATCH_HISTORY") {
    const records: WatchHistoryRecord[] = [];
    for (let offset = 0; ; offset += PAGE_SIZE) {
      const page = await getWatchHistory(PAGE_SIZE, offset);
      records.push(...page);
      if (page.length < PAGE_SIZE) break;
    }
    await saveJsonFile(title, name, JSON.stringify(historyToTakeout(records), null, 2));
    return { skipped: 0 };
  }
  const raw = await getSetting("subscriptions");
  const parsed: unknown = raw ? JSON.parse(raw) : [];
  const { file, skipped } = subscriptionsToNewpipe(parsed);
  await saveJsonFile(title, name, JSON.stringify(file, null, 2));
  return { skipped };
}
