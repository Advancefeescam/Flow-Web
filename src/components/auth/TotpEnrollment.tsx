import { useState } from "react";
import { QRCodeSVG } from "qrcode.react";
import { Check, Copy, ChevronDown, ExternalLink } from "lucide-react";
import { copyText } from "../../lib/clipboard";
import type { Enrollment } from "../../lib/api/webAuth";

export function TotpEnrollment({ enrollment }: { enrollment: Enrollment }) {
  const [manual, setManual] = useState(false);
  const [copied, setCopied] = useState(false);
  const [copyFailed, setCopyFailed] = useState(false);
  async function copy() {
    const success = await copyText(enrollment.secret);
    setCopied(success); setCopyFailed(!success);
  }
  return <div className="space-y-4">
    <div className="space-y-1 text-center">
      <h3 className="font-semibold text-chrome-neutral-100">Scan the QR code</h3>
      <p className="text-sm leading-relaxed text-chrome-neutral-400">Add an account in your authenticator app, then enter its verification code.</p>
    </div>
    <div className="mx-auto w-fit max-w-full rounded-2xl bg-white p-2">
      {/* QR codes deliberately retain high contrast and a four-module quiet zone in every theme. */}
      <QRCodeSVG value={enrollment.uri} size={208} level="M" marginSize={4}
        fgColor="#000000" bgColor="#ffffff" title="Authenticator setup QR code" style={{ maxWidth: "100%", height: "auto", display: "block" }}/>
    </div>
    <div className="rounded-2xl bg-surface-container-high overflow-hidden">
      <button type="button" onClick={() => setManual(v => !v)} aria-expanded={manual} aria-controls="totp-manual"
        className="w-full flex items-center justify-between gap-2 px-4 py-3 text-start text-sm font-medium text-chrome-neutral-200 hover:bg-surface-container-highest focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary">
        Enter a setup key instead <ChevronDown size={18} className={manual ? "rotate-180 transition-transform" : "transition-transform"}/>
      </button>
      <div id="totp-manual" hidden={!manual} className="space-y-3 px-4 pb-4">{manual && <>
        <p className="text-xs leading-relaxed text-chrome-neutral-400">Account: flow · Time-based · 6 digits · 30 seconds</p>
        <div className="flex items-start gap-2 rounded-xl border border-outline-soft bg-surface-container p-3">
          <code dir="ltr" className="min-w-0 flex-1 select-all break-all font-mono text-sm leading-relaxed text-chrome-neutral-100">{enrollment.secret.match(/.{1,4}/g)?.join(" ")}</code>
          <button type="button" aria-label={copied ? "Copied" : "Copy setup key"} title={copied ? "Copied" : "Copy setup key"} onClick={() => void copy()}
            className="shrink-0 rounded-full p-2 text-chrome-neutral-300 hover:bg-surface-container-high focus-visible:ring-2 focus-visible:ring-primary">
            {copied ? <Check size={18}/> : <Copy size={18}/>}
          </button>
        </div>
        {copied && <p role="status" className="text-xs text-chrome-neutral-400">Setup key copied.</p>}
        {copyFailed && <p role="alert" className="text-xs text-error">Copy the key manually, without spaces.</p>}
        <a href={enrollment.uri} className="inline-flex items-center justify-center gap-2 rounded-full bg-surface-container px-3 py-2 text-xs font-medium text-chrome-neutral-200 hover:bg-surface-container-highest focus-visible:ring-2 focus-visible:ring-primary"><ExternalLink size={14}/> Open authenticator</a>
      </>}
      </div>
    </div>
    <p className="text-xs text-center leading-relaxed text-chrome-neutral-400">Setup expires in five minutes. Keep this key private.</p>
  </div>;
}
