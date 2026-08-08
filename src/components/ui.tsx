'use client'

import { AlertCircle, CalendarDays, Check, ChevronDown, ChevronLeft, ChevronRight } from 'lucide-react'
import { createPortal } from 'react-dom'
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
  const [pos, setPos] = useState<{ top?: number; bottom?: number; left: number; width: number } | null>(null)
  const [flipUp, setFlipUp] = useState(false)
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

  // On open, measure the button and wheel the popover into view using getBoundingClientRect.
  // Positioning with fixed coords (instead of `absolute` inside the root) makes the popover a
  // floating layer that is never clipped by the window's scroll containers — the listbox always
  // reads as a panel on top of the glass card, matching the design system's layering.
  useEffect(() => {
    if (!open) return
    setActiveIndex(selectedIndex)
    const btn = buttonRef.current
    if (!btn) return
    const rect = btn.getBoundingClientRect()
    const spaceBelow = window.innerHeight - rect.bottom - 8
    const estHeight = Math.min(288, options.length * 40 + 16)
    const up = spaceBelow < estHeight
    setFlipUp(up)
    setPos(
      up
        ? {
            bottom: window.innerHeight - rect.top + 4,
            left: rect.left,
            width: rect.width,
          }
        : {
            top: rect.bottom + 4,
            left: rect.left,
            width: rect.width,
          }
    )
  }, [open, selectedIndex, options.length])

  // Keep focus inside the listbox for keyboard nav once it has mounted.
  useEffect(() => {
    if (!open) return
    const id = requestAnimationFrame(() => listRef.current?.focus())
    return () => cancelAnimationFrame(id)
  }, [open])

  const close = (returnFocus: boolean) => {
    setOpen(false)
    setPos(null)
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

  const dropdown =
    open && pos ? (
      <ul
        ref={listRef}
        role="listbox"
        tabIndex={-1}
        aria-activedescendant={`${listId}-${activeIndex}`}
        onKeyDown={onListKeyDown}
        style={{
          position: 'fixed',
          top: flipUp ? undefined : (pos.top ?? 0),
          bottom: flipUp ? (pos.bottom ?? 0) : undefined,
          left: pos.left,
          width: pos.width,
          maxWidth: 'min(96vw, 420px)',
        }}
        className={`select-popover fade-up z-[100] max-h-72 overflow-auto rounded-[10px] p-1 outline-none ${
          flipUp ? 'mb-1.5' : 'mt-1.5'
        }`}
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
                isActive ? 'select-option-active' : ''
              }`}
            >
              <span className="truncate">{opt.label}</span>
              {isSelected && <Check className="h-3.5 w-3.5 shrink-0 text-primary" />}
            </li>
          )
        })}
      </ul>
    ) : null

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

      {open && pos && createPortal(dropdown, document.body)}
    </div>
  )
}

const CAL_WEEKDAYS = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa']

/** Local `YYYY-MM-DD` from a Date (matches the store's date keys). */
function toISODate(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

function parseISODate(iso: string): Date {
  const [y, m, d] = iso.split('-').map(Number)
  return new Date(y, m - 1, d)
}

/** `DD/MM/YYYY`, the display the owner asked for — not the OS primitive's M/D/Y. */
function formatDMY(iso: string): string {
  if (!iso) return ''
  const [y, m, d] = iso.split('-')
  return `${d}/${m}/${y}`
}

function sameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  )
}

/**
 * A themed date picker that draws its own calendar, because in WebView2 the native
 * `<input type="date">` shows an unstyled empty control — no calendar opener and no themed
 * segments. Closed it reads exactly like `.input`; open it is a `--popover`/frosted-glass
 * calendar panel (same layering as the custom `Select`), with `DD/MM/YYYY` display and an
 * indigo selection that matches the app's accent.
 */
export function DateField({
  value,
  onChange,
  min,
  max,
  className = '',
  'aria-label': ariaLabel,
}: {
  value: string // '' or `YYYY-MM-DD`
  onChange: (iso: string) => void
  min?: string // `YYYY-MM-DD`
  max?: string
  className?: string
  'aria-label'?: string
}) {
  const [open, setOpen] = useState(false)
  const [pos, setPos] = useState<{ top?: number; bottom?: number; left: number; width: number } | null>(null)
  const [flipUp, setFlipUp] = useState(false)
  // The month visible in the calendar. Starts at the picked date, else the min/max, else today.
  const [view, setView] = useState(() => {
    const seed = parseISODate(value || min || max || toISODate(new Date()))
    return new Date(seed.getFullYear(), seed.getMonth(), 1)
  })
  const rootRef = useRef<HTMLDivElement>(null)
  const buttonRef = useRef<HTMLButtonElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)

  const minD = min ? parseISODate(min) : null
  const maxD = max ? parseISODate(max) : null

  useEffect(() => {
    if (!open) return
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false)
    }
    window.addEventListener('mousedown', onDown)
    return () => window.removeEventListener('mousedown', onDown)
  }, [open])

  useEffect(() => {
    if (!open) return
    const btn = buttonRef.current
    if (!btn) return
    const rect = btn.getBoundingClientRect()
    const estHeight = 348
    const spaceBelow = window.innerHeight - rect.bottom - 8
    const up = spaceBelow < estHeight
    setFlipUp(up)
    setPos(
      up
        ? {
            bottom: window.innerHeight - rect.top + 4,
            left: Math.max(8, Math.min(rect.left, window.innerWidth - 264)),
            width: 256,
          }
        : {
            top: rect.bottom + 4,
            left: Math.max(8, Math.min(rect.left, window.innerWidth - 264)),
            width: 256,
          }
    )
  }, [open])

  useEffect(() => {
    if (!open) return
    const id = requestAnimationFrame(() => panelRef.current?.focus())
    return () => cancelAnimationFrame(id)
  }, [open])

  const pick = (iso: string) => {
    onChange(iso)
    setOpen(false)
    setPos(null)
  }

  const today = new Date()
  const y = view.getFullYear()
  const m = view.getMonth()
  const firstDow = new Date(y, m, 1).getDay()
  const daysInMonth = new Date(y, m + 1, 0).getDate()
  const cells: (Date | null)[] = [
    ...Array.from({ length: firstDow }, () => null),
    ...Array.from({ length: daysInMonth }, (_, i) => new Date(y, m, i + 1)),
  ]
  while (cells.length % 7 !== 0) cells.push(null)

  const isDisabled = (d: Date) =>
    (minD !== null && d < minD) || (maxD !== null && d > maxD)

  const prevMonth = () =>
    setView(new Date(y, m - 1, 1))
  const nextMonth = () =>
    setView(new Date(y, m + 1, 1))
  const monthTitle = view.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })

  const panel = (
    <div
      ref={panelRef}
      tabIndex={-1}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          setOpen(false)
          setPos(null)
          buttonRef.current?.focus()
        }
      }}
      style={{
        position: 'fixed',
        top: flipUp ? undefined : (pos?.top ?? 0),
        bottom: flipUp ? (pos?.bottom ?? 0) : undefined,
        left: pos?.left ?? 0,
        width: pos?.width ?? 256,
      }}
      className={`select-popover fade-up z-[100] rounded-[10px] p-2.5 outline-none ${
        flipUp ? 'mb-1.5' : 'mt-1.5'
      }`}
    >
      <div className="mb-1 flex items-center justify-between">
        <button
          type="button"
          onClick={prevMonth}
          aria-label="Previous month"
          className="flex h-6 w-6 items-center justify-center rounded-md text-muted-foreground hover:bg-accent"
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </button>
        <span className="text-[12.5px] font-medium">{monthTitle}</span>
        <button
          type="button"
          onClick={nextMonth}
          aria-label="Next month"
          className="flex h-6 w-6 items-center justify-center rounded-md text-muted-foreground hover:bg-accent"
        >
          <ChevronRight className="h-3.5 w-3.5" />
        </button>
      </div>
      <div className="mb-1 grid grid-cols-7">
        {CAL_WEEKDAYS.map((d) => (
          <div key={d} className="text-center text-[9px] uppercase tracking-wide text-muted-foreground">
            {d}
          </div>
        ))}
      </div>
      <div className="grid grid-cols-7 gap-y-0.5">
        {cells.map((d, i) => {
          if (!d) return <div key={i} />
          const iso = toISODate(d)
          const selected = iso === value
          const isToday = sameDay(d, today)
          const disabled = isDisabled(d)
          return (
            <button
              key={iso}
              type="button"
              disabled={disabled}
              onClick={() => pick(iso)}
              aria-pressed={selected}
              aria-label={d.toLocaleDateString(undefined, {
                weekday: 'long',
                day: 'numeric',
                month: 'long',
                year: 'numeric',
              })}
              className={[
                'date-cell flex h-7 w-full items-center justify-center rounded-[7px] text-[11.5px]',
                selected
                  ? 'bg-primary font-medium text-primary-foreground'
                  : 'text-foreground',
                disabled ? 'cursor-not-allowed opacity-35' : 'cursor-pointer',
                !selected && isToday ? 'ring-1 ring-inset ring-primary/60' : '',
              ]
                .filter(Boolean)
                .join(' ')}
            >
              {d.getDate()}
            </button>
          )
        })}
      </div>
    </div>
  )

  return (
    <div ref={rootRef} className={`relative w-full ${className}`}>
      <button
        ref={buttonRef}
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={ariaLabel}
        onClick={() => setOpen((o) => !o)}
        className="input flex w-full items-center justify-between gap-2 text-left"
      >
        <span className={`truncate ${value ? '' : 'text-muted-foreground'}`}>
          {value ? formatDMY(value) : 'Select date'}
        </span>
        <CalendarDays className="h-3.5 w-3.5 shrink-0 opacity-50" />
      </button>
      {open && pos && createPortal(panel, document.body)}
    </div>
  )
}
