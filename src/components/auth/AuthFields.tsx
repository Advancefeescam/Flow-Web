import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { TextInput } from "../ui/TextInput";

const fieldClass = "w-full !rounded-xl !px-4 !py-3 !text-base focus-visible:!ring-2 focus-visible:!ring-primary";
const labelClass = "flex flex-col gap-2 text-sm font-medium text-chrome-neutral-200";
type Props = { value: string; onChange: (value: string) => void; disabled?: boolean; autoFocus?: boolean };

export function AuthUsernameField({ value, onChange, disabled }: Props) {
  return <label className={labelClass}>
    <span>Username</span>
    <TextInput className={fieldClass} autoComplete="username" autoCapitalize="none" spellCheck={false} required value={value} onChange={onChange} disabled={disabled}/>
  </label>;
}
export function AuthPasswordField({ value, onChange, disabled, autoFocus, label = "Password" }: Props & { label?: string }) {
  const [visible, setVisible] = useState(false);
  return <label className={labelClass}>
    <span>{label}</span>
    <span className="relative block">
      <TextInput className={fieldClass + " !pe-14"} type={visible ? "text" : "password"} value={value}
        onChange={onChange} autoComplete="current-password" required disabled={disabled} autoFocus={autoFocus}/>
      <button type="button" disabled={disabled} onClick={() => setVisible(v => !v)} aria-label={visible ? "Hide password" : "Show password"} aria-pressed={visible}
        className="absolute end-1.5 top-1/2 -translate-y-1/2 grid h-10 w-10 place-items-center rounded-full text-chrome-neutral-400 hover:bg-surface-container-highest focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary disabled:opacity-50">
        {visible ? <EyeOff size={18}/> : <Eye size={18}/>}
      </button>
    </span>
  </label>;
}
export function AuthCodeField({ value, onChange, disabled, autoFocus }: Props) {
  return <label className={labelClass}>
    <span>Verification code</span>
    <TextInput className={fieldClass + " !text-center !font-mono !text-xl tracking-[0.3em] tabular-nums"} dir="ltr"
      value={value} onChange={v => onChange(v.replace(/\D/g, "").slice(0, 6))}
      inputMode="numeric" autoComplete="one-time-code" pattern="[0-9]{6}" required
      placeholder="000000" disabled={disabled} autoFocus={autoFocus}/>
    <span className="block text-xs leading-relaxed font-normal text-chrome-neutral-400">Enter the six-digit code from your authenticator app.</span>
  </label>;
}
export function AuthError({ message }: { message: string }) {
  return message ? <p role="alert" className="rounded-xl border border-outline-soft bg-surface-container-high px-3 py-2.5 text-sm leading-relaxed text-error">{message}</p> : null;
}
