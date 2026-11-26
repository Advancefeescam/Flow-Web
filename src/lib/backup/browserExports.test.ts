import { describe, expect, it } from "vitest";
import { historyToTakeout, subscriptionsToNewpipe } from "./browserExports";

describe("browser data exports", () => {
  it("writes watch history in Takeout's newest-first shape", () => {
    const file = historyToTakeout([
      { videoId: "aaaaaaaaaaa", title: "Old", watchDate: "2024-01-01T00:00:00Z", watchDurationSeconds: 5 },
      { videoId: "bbbbbbbbbbb", title: "New", channelName: "Channel", channelId: "UCaaaaaaaaaaaaaaaaaaaaaa", watchDate: "2025-01-01T00:00:00Z", watchDurationSeconds: 5 },
    ]);
    expect(file).toHaveLength(2);
    expect(file[0]).toMatchObject({ title: "Watched New", titleUrl: "https://www.youtube.com/watch?v=bbbbbbbbbbb", subtitles: [{ name: "Channel", url: "https://www.youtube.com/channel/UCaaaaaaaaaaaaaaaaaaaaaa" }] });
  });

  it("skips subscription identifiers NewPipe cannot open", () => {
    const { file, skipped } = subscriptionsToNewpipe([
      { id: "UCaaaaaaaaaaaaaaaaaaaaaa", name: "Channel" },
      { id: "bad/path", name: "Invalid" },
    ]);
    expect(skipped).toBe(1);
    expect(file).toMatchObject({ subscriptions: [{ service_id: 0, url: "https://www.youtube.com/channel/UCaaaaaaaaaaaaaaaaaaaaaa", name: "Channel" }] });
  });
});
