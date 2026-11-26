import { useMediaQuery } from "./useMediaQuery";
import { isWebMode } from "./api/env";

// The existing Flow sidebar breakpoint is 640px. Keep that boundary and
// collapse the tablet rail until the desktop/watch layout has enough space.
export const WEB_COMPACT_QUERY = "(max-width: 639px)";
export const WEB_MEDIUM_QUERY = "(min-width: 640px) and (max-width: 1023px)";
export function useWebCompact(): boolean {
  const compact = useMediaQuery(WEB_COMPACT_QUERY);
  return isWebMode() && compact;
}
export function useWebMedium(): boolean {
  const medium = useMediaQuery(WEB_MEDIUM_QUERY);
  return isWebMode() && medium;
}

/** iPad landscape and smaller desktop web windows keep the rail compact. */
export const WEB_RAIL_QUERY = "(min-width: 640px) and (max-width: 1199px)";
export function useWebRail(): boolean {
  const rail = useMediaQuery(WEB_RAIL_QUERY);
  return isWebMode() && rail;
}

export const WEB_STACKED_WATCH_QUERY = "(max-width: 1023px), (max-width: 1199px) and (orientation: portrait)";
export function useWebStackedWatch(): boolean {
  const stacked = useMediaQuery(WEB_STACKED_WATCH_QUERY);
  return isWebMode() && stacked;
}
