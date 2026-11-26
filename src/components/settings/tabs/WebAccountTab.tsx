import { useEffect, useRef, useState, type FormEvent } from "react";
import { ShieldCheck, LockKeyhole, LogOut, Monitor, LoaderCircle, CheckCircle2 } from "lucide-react";
import { Button } from "../../ui/Button";
import { SettingsGroup } from "../SettingsGroup";
import { SettingItem } from "../SettingItem";
import { AuthPasswordField, AuthCodeField, AuthError } from "../../auth/AuthFields";
import { TotpEnrollment } from "../../auth/TotpEnrollment";
import { ProfileAvatar } from "../../ui/ProfileAvatar";
import { getWebSession, signOut, beginTotp, confirmTotp, disableTotp, cancelTotp, revokeOtherSessions, type WebSession, type Enrollment } from "../../../lib/api/webAuth";
import { getString } from "../../../lib/i18n/index";

type Action = "setup" | "disable" | "revoke" | null;
export function WebAccountTab() {
  const [session, setSession] = useState<WebSession | null>(null);
  const [action, setAction] = useState<Action>(null);
  const [enrollment, setEnrollment] = useState<Enrollment | null>(null);
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const dialogRef = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (!action) return;
    const dialog = dialogRef.current;
    if (dialog && !dialog.open) dialog.showModal();
    return () => { if (dialog?.open) dialog.close(); };
  }, [action]);
  useEffect(() => {
    let alive = true;
    getWebSession().then(s => { if (alive) setSession(s); }).catch(e => { if (alive) setError(e.message); });
    return () => { alive = false; };
  }, []);
  const clear = () => { setPassword(""); setCode(""); setEnrollment(null); setAction(null); };
  const open = (next: Action) => { clear(); setError(""); setNotice(""); setAction(next); };
  async function run(event: FormEvent) {
    event.preventDefault();
    if (busy || !action || !session) return;
    setBusy(true); setError(""); setNotice("");
    try {
      if (action === "setup" && !enrollment) {
        setEnrollment(await beginTotp(password));
        setPassword("");
        return;
      }
      const updated = action === "setup" ? await confirmTotp(password, code)
        : action === "disable" ? await disableTotp(password, code)
        : await revokeOtherSessions(password, code);
      setSession(updated);
      setNotice(action === "setup" ? "Two-step verification is enabled. Other sessions were signed out."
        : action === "disable" ? "Two-step verification is disabled." : "All other sessions were signed out.");
      clear();
    } catch (e) { setCode(""); setError(e instanceof Error ? e.message : "Security settings could not be updated."); }
    finally { setBusy(false); }
  }
  async function cancel() {
    if (busy) return;
    setBusy(true);
    try { if (enrollment) await cancelTotp(); clear(); setError(""); }
    catch (e) { setError(e instanceof Error ? e.message : "Could not cancel setup."); }
    finally { setBusy(false); }
  }
  async function logout() {
    if (busy) return;
    setBusy(true);
    try { await signOut(); window.location.reload(); }
    catch (e) { setError(e instanceof Error ? e.message : "Could not sign out."); setBusy(false); }
  }
  const needsCode = Boolean(enrollment || (session?.totpEnabled && action !== "setup"));
  const heading = action === "setup" ? "Set up two-step verification" : action === "disable" ? "Disable two-step verification?" : "Sign out other sessions?";
  return <div className="space-y-6 pb-8 max-w-3xl min-w-0">
    <div className="space-y-1">
      <h2 className="text-2xl font-bold text-on-surface">{getString("settings_web_account")}</h2>
      <p className="text-sm text-on-surface-variant">Manage access to this Flow Web instance.</p>
    </div>
    <section className="rounded-2xl bg-surface-row p-5 flex flex-wrap items-center gap-4">
      <ProfileAvatar name={session?.username ?? "flow"} colorKey="" kind="initial" size="xl"/>
      <div className="flex-1 min-w-[100px]">
        <h3 className="font-semibold text-lg text-chrome-neutral-100 truncate">{session?.username ?? "flow"}</h3>
        <p className="text-sm text-chrome-neutral-400">Administrator</p>
      </div>
      <span className="inline-flex items-center gap-1.5 rounded-full bg-surface-container-high px-3 py-1.5 text-xs font-medium text-chrome-neutral-200">
        {session?.totpEnabled ? <ShieldCheck size={16}/> : <LockKeyhole size={16}/>}
        {session ? session.totpEnabled ? "2FA enabled" : "Password protected" : "Loading…"}
      </span>
    </section>
    <SettingsGroup title="Security">
      <SettingItem title={getString("settings_web_two_factor")} description={session ? (session.totpEnabled ? "Enabled · Authenticator app" : "Off · Add an authenticator app") : "Loading…"} icon={<ShieldCheck size={20}/>} disabled={!session || busy} onSelect={() => open(session?.totpEnabled ? "disable" : "setup")}/>
      <SettingItem title={getString("settings_web_sessions")} description={session ? session.sessionCount + (session.sessionCount === 1 ? " active session" : " active sessions") + ", including this browser" : "Loading…"} icon={<Monitor size={20}/>} disabled={!session || busy} onSelect={() => open("revoke")}/>
    </SettingsGroup>
    <SettingsGroup title="Your account">
      <SettingItem title={getString("settings_web_sign_out")} description="End this browser session." icon={<LogOut size={20}/>} disabled={!session || busy} onSelect={() => void logout()}/>
    </SettingsGroup>
    {action && <dialog ref={dialogRef} onCancel={event => { event.preventDefault(); void cancel(); }} aria-labelledby="security-action"
      className="fixed inset-0 m-auto w-[calc(100%-24px)] max-w-[460px] max-h-[calc(100dvh-24px)] overflow-y-auto rounded-[28px] border border-outline-soft bg-surface-container text-on-surface p-0 backdrop:bg-black/60">
      <form onSubmit={run}>
        <header className="px-6 pt-6 pb-4 flex items-start gap-3">
          <span className="grid h-11 w-11 shrink-0 place-items-center rounded-full bg-surface-container-high text-primary"><LockKeyhole size={21}/></span>
          <div className="min-w-0 flex-1">
            <h2 id="security-action" className="text-lg font-bold leading-tight tracking-tight text-chrome-neutral-100">{heading}</h2>
            <p className="mt-1 text-xs text-chrome-neutral-400">{action === "setup" ? enrollment ? "Step 2 of 2 · Add authenticator" : "Step 1 of 2 · Confirm password" : "Confirm your identity"}</p>
          </div>
        </header>
        <div className="space-y-4 px-5 sm:px-6 pb-5">
          {action === "setup" && !enrollment && <p className="text-sm leading-relaxed text-chrome-neutral-400">Confirm your password to add an authenticator app to your account.</p>}
          {action === "disable" && <p className="text-sm leading-relaxed text-chrome-neutral-400">Your account will no longer require a verification code. Other sessions will be signed out.</p>}
          {action === "revoke" && <p className="text-sm leading-relaxed text-chrome-neutral-400">All other browsers will need to sign in again. This browser stays signed in.</p>}
          {enrollment && <TotpEnrollment enrollment={enrollment}/>}
          <AuthPasswordField key={enrollment ? "verify" : "password"} label={enrollment ? "Confirm administrator password" : "Administrator password"} value={password} onChange={setPassword} disabled={busy}/>
          {needsCode && <AuthCodeField value={code} onChange={setCode} disabled={busy}/>}
          <AuthError message={error}/>
          {enrollment && <p className="text-xs leading-relaxed text-chrome-neutral-400">If you lose your authenticator, server-owner recovery is required.</p>}
        </div>
        <footer className="sticky bottom-0 flex flex-wrap justify-end gap-2 border-t border-outline-soft bg-surface-container-low px-5 sm:px-6 py-4">
          <Button type="button" variant="ghost" onClick={() => void cancel()} disabled={busy}>{getString("cancel")}</Button>
          <Button type="submit" variant={action === "disable" ? "destructive" : "primary"} disabled={busy || !password || (needsCode && code.length !== 6)}>
            {busy && <LoaderCircle size={16} className="animate-spin"/>}
            {busy ? "Please wait…" : action === "setup" && !enrollment ? "Continue" : action === "setup" ? "Verify and enable" : action === "disable" ? "Disable" : "Sign out other sessions"}
          </Button>
        </footer>
      </form>
    </dialog>}
    {!action && <AuthError message={error}/>}
    {notice && <p role="status" className="flex items-start gap-2 rounded-xl bg-surface-container-high p-3 text-sm text-chrome-neutral-200"><CheckCircle2 size={18} className="shrink-0 text-primary"/>{notice}</p>}
    <p className="text-xs leading-relaxed text-chrome-neutral-400">The server owner manages the administrator password. This instance currently has one administrator account.</p>
  </div>;
}
