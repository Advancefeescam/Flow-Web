export const SHORT_SHEET_DEFAULT = .62;
export const SHORT_SHEET_STOPS = [.42, SHORT_SHEET_DEFAULT, .78] as const;

export function shortSheetShare(start: number, deltaY: number, stageHeight: number): number {
  if (stageHeight <= 0) return start;
  return Math.max(.15, Math.min(.85, start - deltaY / stageHeight));
}

export function shortSheetTarget(share: number, velocityY = 0): number | null {
  const projected = share - Math.max(-.22, Math.min(.22, velocityY / 6000));
  if (projected < .27) return null;
  return SHORT_SHEET_STOPS.reduce<number>((best, stop) =>
    Math.abs(stop - projected) < Math.abs(best - projected) ? stop : best, SHORT_SHEET_DEFAULT);
}
