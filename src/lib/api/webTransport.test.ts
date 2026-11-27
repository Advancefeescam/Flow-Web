import { describe, expect, it, vi } from "vitest";
import { invokeWeb } from "./webTransport";

describe("Flow Web transport", () => {
  it("posts the original command and camelCase arguments to a same-origin endpoint", async () => {
    const request = vi.fn<typeof fetch>().mockResolvedValue(Response.json({ items: [] }));
    await expect(invokeWeb("search_videos", { request: { query: "Flow" } }, request))
      .resolves.toEqual({ items: [] });
    expect(request).toHaveBeenCalledWith("/api/v1/rpc/search_videos", expect.objectContaining({
      method: "POST", credentials: "same-origin", redirect: "error",
      body: '{"request":{"query":"Flow"}}',
    }));
  });

  it("preserves backend error kinds and timing metadata", async () => {
    const error = { kind: "liveUpcoming", message: "Upcoming", startsAtMs: 123 };
    const request = vi.fn<typeof fetch>().mockResolvedValue(Response.json(error, { status: 422 }));
    await expect(invokeWeb("get_stream_info", {}, request)).rejects.toEqual(error);
  });

  it("rejects a command containing a path before sending it", async () => {
    const request = vi.fn<typeof fetch>();
    await expect(invokeWeb("../admin", {}, request)).rejects.toMatchObject({ kind: "validation" });
    expect(request).not.toHaveBeenCalled();
  });

  it("reports unreachable servers without mock content", async () => {
    const request = vi.fn<typeof fetch>().mockRejectedValue(new Error("offline"));
    await expect(invokeWeb("get_video_details", {}, request)).rejects.toMatchObject({ kind: "network" });
  });

  it("reports non-JSON unauthorized responses", async () => {
    const request = vi.fn<typeof fetch>().mockResolvedValue(new Response("login", { status: 401 }));
    await expect(invokeWeb("get_video_details", {}, request)).rejects.toMatchObject({ kind: "unauthorized" });
  });
});
