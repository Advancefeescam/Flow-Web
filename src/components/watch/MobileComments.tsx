import { lazy, Suspense, useCallback, useEffect, useLayoutEffect, useRef, useState, type PointerEvent } from "react";
import { createPortal } from "react-dom";
import { ChevronRight, MessageSquareText, Loader2 } from "lucide-react";
import { getString } from "../../lib/i18n/index";
import { useVideoComments } from "../../lib/useVideoComments";
import { usePlayerStore } from "../../store/usePlayerStore";
import { commentSheetBounds, commentSheetSnap, type CommentSheetSnap } from "../../lib/commentSheet";
const CommentsSection = lazy(() => import("./CommentsSection"));

const SLOT = "[data-flow-watch-player-slot]";

/** The sheet is non-modal above its top edge so playback controls stay usable. */
export function MobileComments({ videoId }: { videoId: string }) {
  const thread = useVideoComments(videoId);
  const first = thread.comments[0];
  const [open, setOpen] = useState(false);
  const [bounds, setBounds] = useState(() => commentSheetBounds(window.innerHeight, window.innerHeight * .3));
  const [snap, setSnap] = useState<CommentSheetSnap>("expanded");
  const [dragOffset, setDragOffset] = useState<number | null>(null);
  const drag = useRef<{ id: number; y: number; at: number; offset: number; moved: boolean } | null>(null);
  const swallowClick = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const handle = useRef<HTMLButtonElement>(null);
  const fullscreen = usePlayerStore(s => s.isVideoFullscreen);

  const close = useCallback(() => {
    drag.current = null;
    setDragOffset(null);
    setOpen(false);
    trigger.current?.focus({ preventScroll: true });
  }, []);

  useEffect(() => {
    if (fullscreen && open) close();
  }, [fullscreen, open, close]);

  useLayoutEffect(() => {
    if (!open) return;
    const slot = document.querySelector<HTMLElement>(SLOT);
    const topbar = document.querySelector<HTMLElement>(".flow-adaptive-topbar");
    const viewport = window.visualViewport;
    const getBottom = () => viewport ? viewport.offsetTop + viewport.height : window.innerHeight;
    // Opening from lower in the watch page brings the video back into view.
    if (slot) {
      const rect = slot.getBoundingClientRect();
      if (rect.top < (topbar?.getBoundingClientRect().bottom ?? 0) || rect.bottom > getBottom() - 150) {
        slot.scrollIntoView({ block: "start", behavior: "auto" });
      }
    }
    const measure = () => {
      const bottom = getBottom();
      setBounds(commentSheetBounds(bottom, slot?.getBoundingClientRect().bottom ?? bottom * .3));
    };
    measure();
    handle.current?.focus({ preventScroll: true });
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    if (slot) observer?.observe(slot);
    if (topbar) observer?.observe(topbar);
    window.addEventListener("resize", measure);
    window.addEventListener("scroll", measure, true);
    viewport?.addEventListener("resize", measure);
    viewport?.addEventListener("scroll", measure);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        event.preventDefault();
        close();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", measure);
      window.removeEventListener("scroll", measure, true);
      viewport?.removeEventListener("resize", measure);
      viewport?.removeEventListener("scroll", measure);
      window.removeEventListener("keydown", onKey);
    };
  }, [open, close]);

  const settledOffset = snap === "compact" ? bounds.compactOffset : 0;
  const offset = Math.min(bounds.height, Math.max(0, dragOffset ?? settledOffset));
  const onDown = (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0 || !event.isPrimary) return;
    swallowClick.current = false;
    drag.current = { id: event.pointerId, y: event.clientY, at: performance.now(), offset, moved: false };
    event.currentTarget.setPointerCapture?.(event.pointerId);
  };
  const onMove = (event: PointerEvent<HTMLButtonElement>) => {
    const d = drag.current;
    if (!d || d.id !== event.pointerId) return;
    const delta = event.clientY - d.y;
    if (Math.abs(delta) > 5) d.moved = true;
    if (d.moved) setDragOffset(d.offset + delta);
  };
  const onUp = (event: PointerEvent<HTMLButtonElement>) => {
    const d = drag.current;
    if (!d || d.id !== event.pointerId) return;
    drag.current = null;
    setDragOffset(null);
    if (!d.moved) return;
    swallowClick.current = true;
    const delta = event.clientY - d.y;
    const velocity = delta / Math.max(1, performance.now() - d.at) * 1000;
    const target = commentSheetSnap(bounds.height, d.offset + delta, velocity);
    if (target === "closed") close();
    else setSnap(target);
  };
  const cancelDrag = () => { drag.current = null; setDragOffset(null); };

  const count = thread.totalCount?.toLocaleString() ?? thread.countText;
  return <>
    <button ref={trigger} type="button" onClick={() => { setSnap("expanded"); setOpen(true); }} aria-haspopup="dialog" aria-expanded={open}
      className="w-full rounded-2xl bg-surface-container p-3.5 text-start focus-visible:outline-2 focus-visible:outline-primary">
      <span className="flex items-center justify-between gap-2">
        <span className="font-semibold text-sm text-chrome-neutral-100">{getString("watch_comments")} {count && <span className="font-normal text-chrome-neutral-400">{count}</span>}</span>
        <ChevronRight className="h-5 w-5 shrink-0 text-chrome-neutral-400 rtl:rotate-180"/>
      </span>
      <CommentPreview text={first?.text} loading={thread.loading} disabled={thread.disabled}/>
    </button>
    {open && createPortal(
      <div className="fixed inset-x-0 z-[100] overflow-hidden" style={{ top: bounds.top, height: bounds.height }}>
        <div aria-hidden="true" className="absolute inset-0 bg-black/35" onClick={close}/>
        <section role="dialog" aria-label={getString("watch_comments")}
          className="flow-comments-sheet absolute inset-x-0 mx-auto flex max-w-[860px] flex-col overflow-hidden rounded-t-[24px] border-t border-outline-soft bg-background text-on-surface shadow-xl"
          style={{ top: offset, bottom: 0, transition: dragOffset === null ? "top 180ms cubic-bezier(.2,.8,.2,1)" : "none" }}>
          <button ref={handle} type="button" aria-label={snap === "expanded" ? "Collapse comments" : "Expand comments"} aria-expanded={snap === "expanded"}
            onPointerDown={onDown} onPointerMove={onMove} onPointerUp={onUp} onPointerCancel={cancelDrag} onLostPointerCapture={cancelDrag}
            onClick={() => { if (swallowClick.current) { swallowClick.current = false; return; } setSnap(s => s === "expanded" ? "compact" : "expanded"); }}
            className="grid h-8 w-full shrink-0 cursor-grab touch-none select-none place-items-center active:cursor-grabbing focus-visible:outline-2 focus-visible:outline-primary">
            <span aria-hidden="true" className="h-1 w-9 rounded-full bg-on-surface-variant/45"/>
          </button>
          <div className="min-h-0 flex-1 overflow-hidden px-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
            <Suspense fallback={<Loader2 className="mx-auto animate-spin text-primary"/>}>
              <CommentsSection videoId={videoId} fill onClose={close}/>
            </Suspense>
          </div>
        </section>
      </div>, document.body)}
  </>;
}

function CommentPreview({ text, loading, disabled }: { text?: string | null; loading: boolean; disabled: boolean }) {
  return <span className="mt-2 flex items-center gap-2 text-sm text-chrome-neutral-300">
    <MessageSquareText className="h-4 w-4 shrink-0 text-chrome-neutral-400"/>
    <span dir="auto" className="line-clamp-2 min-w-0">
      {disabled ? getString("watch_comments_disabled") : loading ? getString("watch_loading_more_comments") : text || getString("watch_no_comments")}
    </span>
  </span>;
}
