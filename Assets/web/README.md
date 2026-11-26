# Flow Web branding

The web mark adapts the original Flow slanted F into a browser-window silhouette. The original logo is retained in its existing locations. The web-specific SVGs use Flow red and Material-inspired tonal surfaces, with no gradients or external fonts.

| Asset | Use |
| --- | --- |
| `logo.svg` | Standalone web mark; suitable as a source for favicons and app icons |
| `logo-monochrome.svg` | Single-colour-style alternative on light backgrounds |
| `banner-light.svg`, `banner-dark.svg` | Repository header |
| `install-*.svg`, `server-*.svg`, `license-*.svg` | Repository navigation badges |

Use a GitHub `picture` element to select the dark variant with `prefers-color-scheme: dark`. The assets are editable SVG, and the original Flow logo and attribution are preserved. Flow Web is an unofficial fork, not an official Flow release.

The web build uses `public/flow-web-logo.svg` for the favicon and PWA manifest, and matching paths in `src/lib/brand/logo.ts` for theme-aware in-app logos. Native Flow Desktop icons are unchanged.
