/** Flow's badge, in a 24-unit box: the rounded play-button shape and the slanted F inside it. */
const DESKTOP_SHAPE_PATH =
  "M21.58 7.16C21.33 6.22 20.59 5.48 19.65 5.23C17.96 4.77 12 4.77 12 4.77C12 4.77 6.04 4.77 4.35 5.23C3.41 5.48 2.67 6.22 2.42 7.16C1.96 8.85 1.96 12.38 1.96 12.38C1.96 12.38 1.96 15.91 2.42 17.6C2.67 18.54 3.41 19.28 4.35 19.53C6.04 19.99 12 19.99 12 19.99C12 19.99 17.96 19.99 19.65 19.53C20.59 19.28 21.33 18.54 21.58 17.6C22.04 15.91 22.04 12.38 22.04 12.38C22.04 12.38 22.04 8.85 21.58 7.16Z";

const DESKTOP_GLYPH_PATH = "M10 7L18 7L17.2 9.5H12.8L12.2 11.5H16L15.2 14H11.5L10.5 17H7.5L10 7Z";

/** Where the shape sits inside the 24-unit box, for drawing the badge flush with its edges. */
const DESKTOP_SHAPE_BOUNDS = { x: 1.96, y: 4.77, width: 20.08, height: 15.22 } as const;

/** Browser builds use the Flow Web mark; native builds retain the upstream logo. */
const WEB_SHAPE_PATH = "M6.5 2.5H17.5A5 5 0 0 1 22.5 7.5V16.5A5 5 0 0 1 17.5 21.5H6.5A5 5 0 0 1 1.5 16.5V7.5A5 5 0 0 1 6.5 2.5Z";
const WEB_GLYPH_PATH = "M2 7.7H22V8.7H2Z M6.45 5.5A.85 .85 0 1 1 4.75 5.5A.85 .85 0 1 1 6.45 5.5Z M9.15 5.5A.85 .85 0 1 1 7.45 5.5A.85 .85 0 1 1 9.15 5.5Z M10.2 10.7H16.6L15.96 12.7H12.44L11.96 14.3H15L14.36 16.3H11.4L10.6 18.7H8.2Z";
const WEB_SHAPE_BOUNDS = { x: 1.5, y: 2.5, width: 21, height: 19 } as const;

const isWeb = import.meta.env.VITE_FLOW_WEB === "true";
export const LOGO_SHAPE_PATH = isWeb ? WEB_SHAPE_PATH : DESKTOP_SHAPE_PATH;
export const LOGO_GLYPH_PATH = isWeb ? WEB_GLYPH_PATH : DESKTOP_GLYPH_PATH;
export const LOGO_SHAPE_BOUNDS = isWeb ? WEB_SHAPE_BOUNDS : DESKTOP_SHAPE_BOUNDS;
