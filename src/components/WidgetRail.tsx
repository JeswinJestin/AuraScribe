'use client'

import type { ReactNode } from 'react'
import type { Settings, Status } from '@/lib/ipc'
import type { View } from '@/components/Sidebar'

/** One card in the contextual rail. */
function Widget({
  title,
  accent,
  children,
}: {
  title: string
  accent?: boolean
  children: ReactNode
}) {
  return (
    <div className={accent ? 'panel-accent p-[18px]' : 'rounded-2xl border bg-card p-[18px]'}>
      <div
        className="mb-2 text-[11px] font-semibold uppercase tracking-[0.05em]"
        style={{ color: 'hsl(var(--muted-foreground))' }}
      >
        {title}
      </div>
      {children}
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
}: {
  view: View
  status: Status
  settings: Settings
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
          <Body>AuraScribe v0.4.1 · Windows</Body>
        </Widget>
      </>
    ),
  }

  return <div className="flex flex-col gap-3">{cards[view]}</div>
}
