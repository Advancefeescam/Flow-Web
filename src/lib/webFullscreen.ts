/**
 * Browser fullscreen must be requested inside the original click/key gesture.
 * Keep Flow's existing full-viewport player presentation as the fallback when
 * a browser (notably iPhone Safari) doesn't support document fullscreen.
 */
type WebkitRoot = HTMLElement & { webkitRequestFullscreen?: () => void | Promise<void> };
type WebkitDocument = Document & {
  webkitFullscreenElement?: Element | null;
  webkitExitFullscreen?: () => void | Promise<void>;
};

export function createWebFullscreenController(document: Document) {
  const doc = document as WebkitDocument;
  const root = doc.documentElement as WebkitRoot;
  let desired = false;
  const current = () => doc.fullscreenElement ?? doc.webkitFullscreenElement ?? null;

  const exit = async () => {
    if (current() !== root) return;
    try {
      if (typeof doc.exitFullscreen === "function") await doc.exitFullscreen();
      else await doc.webkitExitFullscreen?.();
    } catch {
      // The OS/browser may already have left fullscreen.
    }
  };

  const sync = async (active: boolean): Promise<boolean> => {
    desired = active;
    if (!active) {
      await exit();
      return !desired;
    }
    try {
      if (current() !== root) {
        if (typeof root.requestFullscreen === "function") await root.requestFullscreen();
        else await root.webkitRequestFullscreen?.();
      }
      // A newer exit can arrive while a native request is still in flight.
      if (!desired) await exit();
    } catch {
      // Denied/unsupported native fullscreen: the player still fills the viewport.
    }
    return desired;
  };

  const watch = (onExternalExit: () => void): (() => void) => {
    let wasNative = current() === root;
    const changed = () => {
      const native = current() === root;
      if (native && !desired) {
        // A late browser entry must not undo the user's newer exit.
        void exit();
      } else if (wasNative && !native && desired) {
        desired = false;
        onExternalExit();
      }
      wasNative = native;
    };
    doc.addEventListener("fullscreenchange", changed);
    doc.addEventListener("webkitfullscreenchange", changed);
    return () => {
      doc.removeEventListener("fullscreenchange", changed);
      doc.removeEventListener("webkitfullscreenchange", changed);
    };
  };

  return { sync, watch };
}
