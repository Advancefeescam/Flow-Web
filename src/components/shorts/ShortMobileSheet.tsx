import { useEffect, useRef, type PointerEvent, type ReactNode } from "react";
import { getString } from "../../lib/i18n/index";
import { SHORT_SHEET_DEFAULT, SHORT_SHEET_STOPS, shortSheetShare, shortSheetTarget } from "../../lib/shortSheet";

interface Props {
  share: number;
  onShare: (share: number) => void;
  onDragging: (dragging: boolean) => void;
  onClose: () => void;
  children: ReactNode;
}

/** A grip-only drag keeps vertical comment scrolling independent from the reel. */
export function ShortMobileSheet({ share, onShare, onDragging, onClose, children }: Props) {
  const grip = useRef<HTMLButtonElement>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  const drag = useRef<{ id: number; y: number; at: number; start: number; height: number; moved: boolean } | null>(null);
  const suppressClick = useRef(false);

  useEffect(() => {
    const previous = document.activeElement;
    grip.current?.focus({ preventScroll: true });
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        event.preventDefault();
        closeRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus({ preventScroll: true });
    };
  }, []);

  const onDown = (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0 || !event.isPrimary) return;
    const stage = event.currentTarget.closest(".short-stage");
    const height = stage?.getBoundingClientRect().height ?? 0;
    if (height <= 0) return;
    const sheet = event.currentTarget.closest(".flow-short-mobile-sheet");
    const actualShare = (sheet?.getBoundingClientRect().height ?? share * height) / height;
    suppressClick.current = false;
    drag.current = { id: event.pointerId, y: event.clientY, at: performance.now(), start: actualShare, height, moved: false };
    event.currentTarget.setPointerCapture?.(event.pointerId);
  };
  const onMove = (event: PointerEvent<HTMLButtonElement>) => {
    const d = drag.current;
    if (!d || d.id !== event.pointerId) return;
    const delta = event.clientY - d.y;
    if (!d.moved && Math.abs(delta) > 5) { d.moved = true; onDragging(true); }
    if (d.moved) onShare(shortSheetShare(d.start, delta, d.height));
  };
  const finish = (event: PointerEvent<HTMLButtonElement>) => {
    const d = drag.current;
    if (!d || d.id !== event.pointerId) return;
    drag.current = null;
    onDragging(false);
    if (!d.moved) return;
    suppressClick.current = true;
    const delta = event.clientY - d.y;
    const target = shortSheetTarget(shortSheetShare(d.start, delta, d.height), delta / Math.max(1, performance.now() - d.at) * 1000);
    if (target === null) onClose();
    else onShare(target);
  };
  const cancel = () => {
    if (!drag.current) return;
    const start = drag.current.start;
    drag.current = null;
    onDragging(false);
    onShare(shortSheetTarget(start) ?? SHORT_SHEET_DEFAULT);
  };
  return <section role="dialog" aria-label={getString("shorts_comments_title")}
    className="flow-short-mobile-sheet flex flex-col rounded-t-[24px] border-t border-outline-soft bg-background text-on-surface">
    <button ref={grip} type="button" aria-label={share >= SHORT_SHEET_DEFAULT ? "Resize or collapse panel" : "Expand panel"}
      onPointerDown={onDown} onPointerMove={onMove} onPointerUp={finish} onPointerCancel={cancel} onLostPointerCapture={cancel}
      onClick={() => {
        if (suppressClick.current) { suppressClick.current = false; return; }
        onShare(share >= SHORT_SHEET_DEFAULT ? SHORT_SHEET_STOPS[0] : SHORT_SHEET_DEFAULT);
      }}
      className="grid h-8 w-full shrink-0 cursor-grab touch-none select-none place-items-center active:cursor-grabbing focus-visible:outline-2 focus-visible:outline-primary">
      <span aria-hidden="true" className="h-1 w-9 rounded-full bg-on-surface-variant/45"/>
    </button>
    <div className="min-h-0 flex-1 overflow-hidden">{children}</div>
  </section>;
}
