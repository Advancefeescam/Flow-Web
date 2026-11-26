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
