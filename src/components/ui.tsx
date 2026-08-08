'use client'

import { AlertCircle, Check, ChevronDown } from 'lucide-react'
import { useEffect, useId, useRef, useState, type KeyboardEvent, type ReactNode } from 'react'

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
        className={`relative h-[22px] w-[38px] shrink-0 rounded-full border transition-colors ${
          checked
            ? 'border-transparent bg-primary'
            : 'border-white/20 bg-foreground/20'
        }`}
      >
        <span
          className={`absolute top-[3px] h-4 w-4 rounded-full bg-white shadow-sm transition-all ${
            checked ? 'left-[18px]' : 'left-[3px]'
          }`}
        />
      </span>
    </button>
  )
}

export interface SelectOption {
  value: string
  label: string
}

/**
 * A themed dropdown that matches the design system, unlike a native `<select>` whose open option
 * list is drawn by the OS and can't be styled. Closed, it reads exactly like an `.input`; open, it
 * is a `--popover`-token listbox with indigo selection. Fully keyboard-navigable (Up/Down/Home/End,
 * Enter/Space to pick, Escape to close, type-ahead) and screen-reader labelled.
 */
export function Select({
  value,
  onChange,
  options,
  className = '',
  'aria-label': ariaLabel,
}: {
  value: string
  onChange: (value: string) => void
  options: SelectOption[]
  className?: string
  'aria-label'?: string
}) {
  const [open, setOpen] = useState(false)
  const [activeIndex, setActiveIndex] = useState(0)
  const rootRef = useRef<HTMLDivElement>(null)
  const buttonRef = useRef<HTMLButtonElement>(null)
  const listRef = useRef<HTMLUListElement>(null)
  const listId = useId()

  const selectedIndex = Math.max(
    0,
    options.findIndex((o) => o.value === value)
  )
  const selected = options[selectedIndex]

  // Close when clicking anywhere outside the control.
  useEffect(() => {
    if (!open) return
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false)
    }
    window.addEventListener('mousedown', onDown)
    return () => window.removeEventListener('mousedown', onDown)
  }, [open])

  // On open, highlight the current value and move focus into the listbox for keyboard nav.
  useEffect(() => {
    if (!open) return
    setActiveIndex(selectedIndex)
    const id = requestAnimationFrame(() => listRef.current?.focus())
    return () => cancelAnimationFrame(id)
  }, [open, selectedIndex])

  const close = (returnFocus: boolean) => {
    setOpen(false)
    if (returnFocus) buttonRef.current?.focus()
  }

  const commit = (index: number) => {
    const opt = options[index]
    if (opt) onChange(opt.value)
    close(true)
  }

  const onButtonKeyDown = (e: KeyboardEvent) => {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
      e.preventDefault()
      setOpen(true)
    }
  }

  const onListKeyDown = (e: KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        setActiveIndex((i) => Math.min(options.length - 1, i + 1))
        break
      case 'ArrowUp':
        e.preventDefault()
        setActiveIndex((i) => Math.max(0, i - 1))
        break
      case 'Home':
        e.preventDefault()
        setActiveIndex(0)
        break
      case 'End':
        e.preventDefault()
        setActiveIndex(options.length - 1)
        break
      case 'Enter':
      case ' ':
        e.preventDefault()
        commit(activeIndex)
        break
      case 'Escape':
        e.preventDefault()
        close(true)
        break
      case 'Tab':
        close(false)
        break
      default:
        // Type-ahead: jump to the next option whose label starts with the typed letter.
        if (e.key.length === 1) {
          const ch = e.key.toLowerCase()
          const after = options.findIndex(
            (o, idx) => idx > activeIndex && o.label.toLowerCase().startsWith(ch)
          )
          const match =
            after !== -1
              ? after
              : options.findIndex((o) => o.label.toLowerCase().startsWith(ch))
          if (match !== -1) setActiveIndex(match)
        }
    }
  }

  return (
    <div ref={rootRef} className={`relative ${className}`}>
      <button
        ref={buttonRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={ariaLabel}
        onClick={() => setOpen((o) => !o)}
        onKeyDown={onButtonKeyDown}
        className="input flex items-center justify-between gap-2 text-left"
      >
        <span className="truncate">{selected?.label ?? ''}</span>
        <ChevronDown
          className={`h-3.5 w-3.5 shrink-0 opacity-60 transition-transform ${open ? 'rotate-180' : ''}`}
        />
      </button>

      {open && (
        <ul
          ref={listRef}
          role="listbox"
          tabIndex={-1}
          aria-activedescendant={`${listId}-${activeIndex}`}
          onKeyDown={onListKeyDown}
          className="fade-up absolute z-50 mt-1.5 max-h-60 w-full overflow-auto rounded-[10px] border bg-popover p-1 text-popover-foreground shadow-lg outline-none"
        >
          {options.map((opt, idx) => {
            const isSelected = opt.value === value
            const isActive = idx === activeIndex
            return (
              <li
                key={opt.value}
                id={`${listId}-${idx}`}
                role="option"
                aria-selected={isSelected}
                onMouseEnter={() => setActiveIndex(idx)}
                onClick={() => commit(idx)}
                className={`flex cursor-pointer items-center justify-between gap-2 rounded-[7px] px-2.5 py-2 text-sm ${
                  isActive ? 'bg-accent' : ''
                }`}
              >
                <span className="truncate">{opt.label}</span>
                {isSelected && <Check className="h-3.5 w-3.5 shrink-0 text-primary" />}
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
