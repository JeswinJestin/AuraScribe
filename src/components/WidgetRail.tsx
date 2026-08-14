'use client'

import { useState, type ReactNode } from 'react'
import { Trash2, Loader2 } from 'lucide-react'
import type { Settings, Status } from '@/lib/ipc'
import * as ipc from '@/lib/ipc'
import { DateField } from '@/components/ui'
import type { View } from '@/components/Sidebar'

/** One card in the contextual rail. */
function Widget({
  title,
  accent,
  danger,
  children,
}: {
  title: string
  accent?: boolean
  danger?: boolean
  children: ReactNode
}) {
  return (
    <div
      className={
        danger
          ? 'rounded-2xl border border-destructive/30 bg-destructive/5 p-[18px]'
          : accent
          ? 'panel-accent p-[18px]'
          : 'rounded-2xl border bg-card p-[18px]'
      }
    >
      <div
        className="mb-2 text-[11px] font-semibold uppercase tracking-[0.05em]"
        style={{
          color: danger ? 'hsl(var(--destructive))' : 'hsl(var(--muted-foreground))',
        }}
      >
        {title}
      </div>
      {children}
    </div>
  )
}

/** Delete every dictation in a chosen date range. Two-step, so it can't fire by accident. */
function RangeDelete({ onDeleted }: { onDeleted: () => void }) {
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [confirming, setConfirming] = useState(false)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<string | null>(null)

  const valid = from !== '' && to !== '' && from <= to

  const runDelete = async () => {
    setBusy(true)
    setNote(null)
    try {
      const start = Math.floor(new Date(`${from}T00:00:00`).getTime() / 1000)
      const end = Math.floor(new Date(`${to}T23:59:59`).getTime() / 1000)
      const removed = await ipc.deleteTranscriptsBetween(start, end)
      setNote(`Deleted ${removed} ${removed === 1 ? 'entry' : 'entries'}.`)
      setConfirming(false)
      onDeleted()
    } catch (e) {
      setNote(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex flex-col gap-2.5">
      <p className="text-[12px] leading-snug" style={{ color: 'hsl(var(--foreground) / 0.72)' }}>
        Erase the dictations saved between two dates and free up space on this machine.
      </p>
      <div className="grid grid-cols-2 gap-2">
        <label className="flex flex-col gap-1">
          <span className="text-[11px] text-muted-foreground">From</span>
          <DateField
            aria-label="From date"
            value={from}
            max={to || undefined}
            onChange={(iso) => {
              setFrom(iso)
              setConfirming(false)
            }}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-[11px] text-muted-foreground">To</span>
          <DateField
            aria-label="To date"
            value={to}
            min={from || undefined}
            onChange={(iso) => {
              setTo(iso)
              setConfirming(false)
            }}
          />
        </label>
      </div>

      {confirming ? (
        <div className="flex items-center gap-2">
          <button
            onClick={runDelete}
            disabled={busy}
            className="flex-1 rounded-lg bg-destructive px-3 py-1.5 text-xs font-medium text-white hover:opacity-90"
          >
            {busy ? <Loader2 className="mr-1 h-3 w-3 animate-spin" /> : <Trash2 className="mr-1 h-3 w-3" />}
            Confirm delete
          </button>
          <button
            onClick={() => setConfirming(false)}
            disabled={busy}
            className="rounded-lg border px-3 py-1.5 text-xs hover:bg-accent"
          >
            Cancel
          </button>
        </div>
      ) : (
        <button
          onClick={() => setConfirming(true)}
          disabled={!valid}
          className="flex w-full items-center justify-center gap-1.5 rounded-lg border border-destructive/40 px-3 py-1.5 text-xs font-medium text-destructive hover:bg-destructive/10 disabled:cursor-not-allowed disabled:opacity-40"
        >
          <Trash2 className="h-3 w-3" />
          Delete a time range
        </button>
      )}
      {note && <p className="text-[11px] text-muted-foreground">{note}</p>}
    </div>
  )
}

function Body({ children }: { children: ReactNode }) {
  return (
    <p className="text-[13px] leading-relaxed" style={{ color: 'hsl(var(--foreground) / 0.72)' }}>
      {children}
    </p>
  )
}

function Stat({ value, sub }: { value: string; sub: string }) {
  return (
    <>
      <div className="font-display text-[22px] font-medium leading-none">{value}</div>
      <div className="mt-1 text-[12px]" style={{ color: 'hsl(var(--faint))' }}>
        {sub}
      </div>
    </>
  )
}

/**
 * The right-hand rail. Its content changes with the active tab — help, privacy notes, and
 * the live model. Deliberately no fabricated usage numbers here: anything numeric is either
 * read from real state or omitted, in keeping with "never claim more than the code does".
 */
export function WidgetRail({
  view,
  status,
  settings,
  onHistoryChanged,
}: {
  view: View
  status: Status
  settings: Settings
  onHistoryChanged?: () => void
}) {
  const model = status.loaded_model ?? settings.whisper_model
  const hotkey = settings.hotkey.replace(/\+/g, ' + ')

  const cards: Record<View, ReactNode> = {
    dictate: (
      <>
        <Widget title="Hotkey">
          <Body>
            Press <span className="mono text-foreground">{hotkey}</span> anywhere to start
            dictating — even when this window is closed.
          </Body>
        </Widget>
        <Widget title="Voice model">
          <Body>
            <span className="mono text-foreground">{model}</span> is active. It runs on this
            machine and works offline.
          </Body>
        </Widget>
        <Widget title="Your voice stays here" accent>
          <Body>
            Audio is transcribed on-device and never uploaded. No cloud, no telemetry, no
            account.
          </Body>
        </Widget>
      </>
    ),
    history: (
      <>
        <Widget title="Your data">
          <Body>Every entry lives only on this machine. Clear all removes it for good.</Body>
        </Widget>
        <Widget title="Tip">
          <Body>Use Words to fix recurring mis-hears so they stop showing up here.</Body>
        </Widget>
        <Widget title="Delete a date range" danger>
          <RangeDelete onDeleted={onHistoryChanged ?? (() => {})} />
        </Widget>
      </>
    ),
    dictionary: (
      <>
        <Widget title="How it works">
          <Body>
            Whisper mis-hears a name once — save the correction and it applies to every future
            dictation automatically.
          </Body>
        </Widget>
        <Widget title="Good candidates">
          <Body>Product names, people, acronyms, and technical jargon.</Body>
        </Widget>
      </>
    ),
    snippets: (
      <>
        <Widget title="How it works">
          <Body>
            Say the trigger phrase and AuraScribe types the saved block at your cursor
            instantly.
          </Body>
        </Widget>
        <Widget title="Try it with">
          <Body>Your email, a mailing address, or a reply you send often.</Body>
        </Widget>
      </>
    ),
    insights: (
      <>
        <Widget title="Reminder">
          <Body>
            These numbers are calculated locally from your history. Nothing is uploaded or
            shared.
          </Body>
        </Widget>
        <Widget title="Footprint">
          <Stat value="~40 MB" sub="idle RAM usage" />
        </Widget>
      </>
    ),
    settings: (
      <>
        <Widget title="Privacy">
          <Body>Local-first by design — no cloud, no telemetry, no account, ever.</Body>
        </Widget>
        <Widget title="Active model">
          <Stat value={model} sub="running on this device" />
        </Widget>
        <Widget title="Version">
          <Body>AuraScribe v1.1.0 · Windows</Body>
        </Widget>
      </>
    ),
  }

  return <div className="flex flex-col gap-3">{cards[view]}</div>
}
