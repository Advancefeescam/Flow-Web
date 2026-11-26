import { afterEach, describe, expect, it, vi } from "vitest";

describe("isTauriEnv", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it("does not mistake a browser extension shim for the Tauri runtime in web mode", async () => {
    vi.stubEnv("VITE_FLOW_WEB", "true");
    vi.stubGlobal("__TAURI_INTERNALS__", { invoke: () => {} });
    vi.resetModules();
    const { isTauriEnv } = await import("./env");
    await expect(isTauriEnv()).resolves.toBe(false);
  });

  it("answers false when the page goes away while it waits for Tauri", async () => {
    vi.resetModules();
    const { isTauriEnv } = await import("./env");
    const pending = isTauriEnv();
    vi.stubGlobal("window", undefined);
    await expect(pending).resolves.toBe(false);
  });
});
