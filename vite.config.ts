import { defineConfig } from "vite";
import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: [
    react(),
    // A pilot: only components that start with "use memo" are compiled. Everything else is
    // built exactly as before.
    babel({ presets: [reactCompilerPreset({ compilationMode: "annotation" })] }),
    tailwindcss(),
    ...(mode === "web" ? [{ name: "flow-web-html", transformIndexHtml: (html: string) => html.replace('href="/logo.svg"', 'href="/flow-web-logo.svg"').replace("<title>Flow Desktop</title>", '<title>Flow Web</title><link rel="manifest" href="/manifest.webmanifest" /><meta name="theme-color" content="#141414" /><script>if ("serviceWorker" in navigator) window.addEventListener("load", () => navigator.serviceWorker.register("/sw.js").catch(() => {}));</script>').replace("Flow Desktop is the privacy-first YouTube and YouTube Music companion to Flow for Android, powered by a native local recommendation engine.", "Unofficial self-hosted web port of Flow for YouTube and YouTube Music.") }] : []),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    ...(mode === "web" ? { proxy: { "/api": "http://127.0.0.1:8780", "/media": "http://127.0.0.1:8780" } } : {}),
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
