import { useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import { isWebMode } from "./api/env";
import { proxiedImage, WEB_IMAGE_BASE } from "./imageProxy";
import { appProxyUrl } from "./settings/appProxy";
import { useAppSettingsStore } from "../store/useAppSettingsStore";
import { useImageProxyStore } from "../store/useImageProxyStore";

/** Loads the image proxy base at startup and when the user's proxy changes; without one, images load directly. */
export function useImageProxyBootstrap(): void {
  const setBase = useImageProxyStore((state) => state.setBase);
  const appProxy = useAppSettingsStore((state) => appProxyUrl(state.values) !== null);
  useEffect(() => {
    if (isWebMode()) { setBase(WEB_IMAGE_BASE); return; }
    invoke<string | null>("image_proxy_base", { appProxy })
      .then((base) => setBase(base ?? null))
      .catch(() => {});
  }, [appProxy, setBase]);
}

/** Maps an image URL to the one the webview should load: through the image proxy where one is needed. */
export function useProxyImageSrc(): (url: string) => string {
  const base = useImageProxyStore((state) => state.base);
  return useCallback((url: string) => proxiedImage(url, isWebMode() ? WEB_IMAGE_BASE : base), [base]);
}
