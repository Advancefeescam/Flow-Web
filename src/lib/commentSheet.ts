export type CommentSheetSnap = "expanded" | "compact" | "closed";

/** Keep the top under the video, except in very short viewports where the sheet needs room to operate. */
export function commentSheetBounds(viewportBottom: number, videoBottom: number) {
  const bottom = Math.max(0, viewportBottom);
  const top = Math.max(0, Math.min(Math.round(videoBottom), Math.max(0, bottom - 160)));
  const height = bottom - top;
  return { top, height, compactOffset: Math.max(0, Math.min(height * .48, height - 160)) };
}

/** A flick can advance the sheet to the next snap, but a small drag springs back. */
export function commentSheetSnap(height: number, offset: number, velocity = 0): CommentSheetSnap {
  if (height <= 0) return "closed";
  const compact = Math.max(0, Math.min(height * .48, height - 160));
  const projected = offset + Math.max(-180, Math.min(180, velocity * .12));
  if (projected >= (compact + height) / 2) return "closed";
  if (compact > 0 && projected >= compact / 2) return "compact";
  return "expanded";
}
