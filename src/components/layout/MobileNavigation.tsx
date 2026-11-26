import { NavLink } from "react-router-dom";
import { SidebarNavIcon, type SidebarNavIconName } from "../ui/SidebarNavIcon";
import { ShortsIcon, ShortsAIcon } from "../ui/ShortsIcon";
import { useAppSettingsStore } from "../../store/useAppSettingsStore";
import { SETTINGS } from "../../lib/settings/schema";
import { useShortsEnabled } from "../../lib/useShortsContent";
import { useMusicEnabled } from "../../lib/useMusicContent";
import { getString, type StringKey } from "../../lib/i18n/index";
import { announceHomeReselected } from "../../lib/homeReselect";

export type MobileDestination = { to: string; label: StringKey; icon: SidebarNavIconName | "shorts" };
export function mobileDestinations(shorts: boolean, music: boolean): MobileDestination[] {
  return [
    { to: "/", label: "home", icon: "home" },
    ...(shorts ? [{ to: "/shorts", label: "sidebar_shorts", icon: "shorts" } as const] : []),
    { to: "/subscriptions", label: "sidebar_subscriptions", icon: "subscriptions" },
    ...(music ? [{ to: "/music", label: "sidebar_music", icon: "music" } as const] : []),
    { to: "/library", label: "sidebar_you", icon: "you" },
  ];
}
export function MobileNavigation() {
  const shortsEnabled = useShortsEnabled();
  const musicEnabled = useMusicEnabled();
  const shortsNav = useAppSettingsStore(s => s.values[SETTINGS.SHORTS_NAVIGATION_ENABLED] !== "false");
  const musicNav = useAppSettingsStore(s => s.values[SETTINGS.MUSIC_NAVIGATION_ENABLED] !== "false");
  const destinations = mobileDestinations(shortsEnabled && shortsNav, musicEnabled && musicNav);
  return <nav data-flow-mobile-nav aria-label={getString("sidebar_menu")} className="flow-web-bottom-nav shrink-0 border-t border-outline-soft bg-surface-container">
    {destinations.map(({ to, label, icon }) => (
      <NavLink key={to} to={to} end={to === "/"} onClick={() => { if (to === "/" && window.location.pathname === "/") announceHomeReselected(); }}
        className={({ isActive }) => "flex min-w-0 flex-1 flex-col items-center justify-center gap-1 py-1.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary " + (isActive ? "text-primary" : "text-chrome-neutral-400")}>
        {({ isActive }) => <>
          <span className={"grid h-8 w-14 max-w-full place-items-center rounded-full transition-colors " + (isActive ? "bg-surface-container-high" : "")}>
            {icon === "shorts" ? (isActive ? <ShortsAIcon className="h-6 w-6"/> : <ShortsIcon className="h-6 w-6"/>)
              : <SidebarNavIcon name={icon} variant={isActive ? "filled" : "outline"} className="h-6 w-6"/>}
          </span>
          <span className="w-full truncate px-0.5 text-center text-[11px] font-medium leading-tight">{getString(label)}</span>
        </>}
      </NavLink>
    ))}
  </nav>;
}
