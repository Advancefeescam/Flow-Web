import { describe, expect, it, vi } from "vitest";
import { createWebFullscreenController } from "./webFullscreen";

function fixture(supported = true) {
  const doc = new EventTarget() as EventTarget & {
    fullscreenElement: unknown;
    documentElement: { requestFullscreen?: () => Promise<void> };
    exitFullscreen: () => Promise<void>;
  };
  doc.fullscreenElement = null;
  doc.documentElement = {};
  const request = vi.fn(async () => {
    doc.fullscreenElement = doc.documentElement;
    doc.dispatchEvent(new Event("fullscreenchange"));
  });
  const exit = vi.fn(async () => {
    doc.fullscreenElement = null;
    doc.dispatchEvent(new Event("fullscreenchange"));
  });
  if (supported) doc.documentElement.requestFullscreen = request;
  doc.exitFullscreen = exit;
  return { doc, request, exit, controller: createWebFullscreenController(doc as unknown as Document) };
}

describe("browser fullscreen", () => {
  it("requests fullscreen immediately in the user gesture and exits normally", async () => {
    const { controller, request, exit } = fixture();
    const enter = controller.sync(true);
    expect(request).toHaveBeenCalledOnce();
    await expect(enter).resolves.toBe(true);
    await expect(controller.sync(false)).resolves.toBe(true);
    expect(exit).toHaveBeenCalledOnce();
  });

  it("keeps viewport presentation when native fullscreen is absent or denied", async () => {
    const unsupported = fixture(false);
    await expect(unsupported.controller.sync(true)).resolves.toBe(true);
    expect(unsupported.request).not.toHaveBeenCalled();
    const denied = fixture();
    denied.request.mockRejectedValueOnce(new Error("not allowed"));
    await expect(denied.controller.sync(true)).resolves.toBe(true);
  });

  it("reconciles an external Escape exit but not an ordinary programmatic exit", async () => {
    const { controller, doc } = fixture();
    const onExternalExit = vi.fn();
    const stop = controller.watch(onExternalExit);
    await controller.sync(true);
    await controller.sync(false);
    expect(onExternalExit).not.toHaveBeenCalled();
    await controller.sync(true);
    doc.fullscreenElement = null;
    doc.dispatchEvent(new Event("fullscreenchange"));
    expect(onExternalExit).toHaveBeenCalledOnce();
    stop();
  });

  it("does not leave a late fullscreen entry active after an exit", async () => {
    const { controller, doc, request, exit } = fixture();
    let release!: () => void;
    request.mockImplementationOnce(() => new Promise<void>((resolve) => {
      release = () => { doc.fullscreenElement = doc.documentElement; resolve(); };
    }));
    const enter = controller.sync(true);
    await controller.sync(false);
    release();
    await expect(enter).resolves.toBe(false);
    expect(exit).toHaveBeenCalledOnce();
    expect(doc.fullscreenElement).toBeNull();
  });
});
