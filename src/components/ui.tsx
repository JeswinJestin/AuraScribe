'use client'

import { AlertCircle } from 'lucide-react'
import type { ReactNode } from 'react'

export function PageHeader({
  title,
  description,
  action,
}: {
  title: string
  description?: string
  action?: ReactNode
}) {
  return (
    <div className="mb-6 flex items-start justify-between gap-4">
      <div>
        <h1 className="font-display text-[32px] font-medium leading-tight tracking-tight">
          {title}
        </h1>
        {description && <p className="mt-1 text-sm text-muted-foreground">{description}</p>}
      </div>
      {action}
    </div>
  )
}

export function Section({
  title,
  description,
  children,
}: {
  title: string
  description?: string
  children: ReactNode
}) {
  return (
    <section className="rounded-2xl border bg-card p-[22px]">
      <h2 className="text-[14.5px] font-semibold">{title}</h2>
      {description && <p className="mt-0.5 text-[13px] text-muted-foreground">{description}</p>}
      <div className="mt-4">{children}</div>
    </section>
  )
}

export function ErrorNote({ children }: { children: ReactNode }) {
  return (
    <div className="flex items-start gap-2 rounded-xl border border-destructive/30 bg-destructive/5 p-3">
      <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-destructive" />
      <p className="text-xs text-destructive">{children}</p>
    </div>
  )
}

/** Empty states are an invitation to act, not a shrug. */
export function EmptyState({
  title,
  hint,
  action,
}: {
  title: string
  hint: string
  action?: ReactNode
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-1.5 rounded-2xl border bg-card px-6 py-14 text-center">
      <p className="font-display text-lg">{title}</p>
      <p className="max-w-xs text-[13px] text-muted-foreground">{hint}</p>
      {action && <div className="mt-3">{action}</div>}
    </div>
  )
}

export function Stat({
  value,
  label,
  hint,
}: {
  value: string | number
  label: string
  hint?: string
}) {
  return (
    <div className="rounded-[14px] border bg-card p-5">
      <div className="font-display tnum text-[26px] font-medium leading-none">{value}</div>
      <div className="mt-1.5 text-[13px] font-medium">{label}</div>
      {hint && (
        <div className="mt-0.5 text-[12px]" style={{ color: 'hsl(var(--faint))' }}>
          {hint}
        </div>
      )}
    </div>
  )
}

export function Toggle({
  checked,
  onChange,
  label,
  hint,
}: {
  checked: boolean
  onChange: (v: boolean) => void
  label: string
  hint?: string
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className="flex w-full items-center justify-between gap-4 py-1.5 text-left"
    >
      <span>
        <span className="block text-[13.5px] font-medium">{label}</span>
        {hint && (
          <span className="mt-0.5 block text-[12.5px] text-muted-foreground">{hint}</span>
        )}
      </span>
      <span
        className={`relative h-[22px] w-[38px] shrink-0 rounded-full transition-colors ${
          checked ? 'bg-foreground' : 'bg-border'
        }`}
      >
        <span
          className={`absolute top-[3px] h-4 w-4 rounded-full bg-white shadow-sm transition-all ${
            checked ? 'left-[19px]' : 'left-[3px]'
          }`}
        />
      </span>
    </button>
  )
}
