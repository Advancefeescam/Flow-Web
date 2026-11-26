import { lazy, Suspense, useEffect, useRef, useState } from "react";
import { ChevronRight, MessageSquareText, Loader2 } from "lucide-react";
import { getString } from "../../lib/i18n/index";
import { useVideoComments } from "../../lib/useVideoComments";
const CommentsSection = lazy(() => import("./CommentsSection"));

/** Keep the watch feed short; the existing shared thread powers both preview and sheet. */
export function MobileComments({ videoId, countText }: { videoId: string; countText?: string | null }) {
  const thread = useVideoComments(videoId);
  const first = thread.comments[0];
  const [open, setOpen] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    const element = dialog.current;
    if (element && !element.open) element.showModal();
    return () => { if (element?.open) element.close(); };
  }, [open]);
  function close() { setOpen(false); trigger.current?.focus({ preventScroll: true }); }
  const count = thread.totalCount?.toLocaleString() ?? thread.countText ?? countText;
  return <>
    <button ref={trigger} type="button" onClick={() => setOpen(true)} aria-haspopup="dialog"
      className="w-full rounded-2xl bg-surface-container p-3.5 text-start focus-visible:outline-2 focus-visible:outline-primary">
      <span className="flex items-center justify-between gap-2">
        <span className="font-semibold text-sm text-chrome-neutral-100">{getString("watch_comments")} {count && <span className="font-normal text-chrome-neutral-400">{count}</span>}</span>
        <ChevronRight className="h-5 w-5 shrink-0 text-chrome-neutral-400 rtl:rotate-180"/>
      </span>
      <CommentPreview text={first?.text} loading={thread.loading} disabled={thread.disabled}/>
    </button>
    {open && <dialog ref={dialog} onCancel={event => { event.preventDefault(); close(); }}
      onClick={event => { if (event.target === dialog.current) close(); }}
      aria-label={getString("watch_comments")}
      className="flow-comments-sheet fixed inset-x-0 bottom-0 top-auto mx-auto mb-0 h-[min(74dvh,720px)] w-full max-w-[720px] overflow-hidden rounded-t-[28px] border border-outline-soft bg-surface-container p-0 text-chrome-neutral-100 backdrop:bg-black/40">
      <div className="flex h-full min-h-0 flex-col px-4 pt-3 pb-[max(1rem,env(safe-area-inset-bottom))]" onClick={event => event.stopPropagation()}>
        <span aria-hidden="true" className="mx-auto mb-3 h-1 w-8 shrink-0 rounded-full bg-chrome-neutral-500"/>
        <Suspense fallback={<Loader2 className="mx-auto animate-spin"/>}>
          <CommentsSection videoId={videoId} fill onClose={close}/>
        </Suspense>
      </div>
    </dialog>}
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
