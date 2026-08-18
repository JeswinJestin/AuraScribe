'use client'

import { useEffect, useState } from 'react'
import * as ipc from '@/lib/ipc'
import type { YearRecap } from '@/lib/ipc'
import { PageHeader, Stat, EmptyState, ErrorNote } from '@/components/ui'
import { renderAndSaveCard } from '@/lib/shareCard'

function fmtHours(h: number) {
  if (h < 1) return `${Math.round(h * 60)} min`
  return `${h.toFixed(1)} hr`
}

function fmtDay(d: string | null) {
  if (!d) return '—'
  const dt = new Date(d + 'T00:00:00')
  return dt.toLocaleDateString(undefined, { day: 'numeric', month: 'long' })
}

/** Which year's recap to show: the current year, except in January, when the year that just
 *  finished is the interesting one (like Spotify Wrapped landing in December/January). */
export function recapYear(now = new Date()): number {
  return now.getMonth() === 0 ? now.getFullYear() - 1 : now.getFullYear()
}

export function RecapView({ onBack }: { onBack?: () => void }) {
  const [recap, setRecap] = useState<YearRecap | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [shareMsg, setShareMsg] = useState<string | null>(null)
  const [sharing, setSharing] = useState(false)

  useEffect(() => {
    ipc.getYearRecap(recapYear()).then(setRecap).catch((e) => setError(String(e)))
  }, [])

  async function share() {
    if (!recap) return
    setSharing(true)
    setShareMsg(null)
    try {
      await renderAndSaveCard({
        filename: `aurascribe-${recap.year}-recap`,
        kicker: `AuraScribe · Your ${recap.year}`,
        headline: fmtHours(recap.hours_saved),
        headlineSub: 'saved vs typing this year, by speaking',
        stats: [
          { value: recap.total_words.toLocaleString(), label: 'Words dictated' },
          { value: recap.total_dictations.toLocaleString(), label: 'Dictations' },
          { value: String(recap.active_days), label: 'Active days' },
          { value: String(recap.words_per_minute || '—'), label: 'Words / min' },
        ],
      })
      setShareMsg('Saved to your Pictures folder.')
    } catch (e) {
      setShareMsg(`Could not save: ${e}`)
    } finally {
      setSharing(false)
    }
  }

  if (error) return <ErrorNote>{error}</ErrorNote>
  if (!recap) return null

  const back = onBack && (
    <button
      onClick={onBack}
      className="mb-3 text-[13px] text-muted-foreground transition-colors hover:text-foreground"
    >
      ← Back to Insights
    </button>
  )

  if (recap.total_dictations === 0) {
    return (
      <div>
        {back}
        <PageHeader title={`Your ${recap.year}`} description="Your dictation year in review." />
        <EmptyState
          title="Nothing yet for this year"
          hint="Once you dictate, your yearly recap fills in here — hours saved, words, your busiest day."
        />
      </div>
    )
  }

  return (
    <div>
      {back}
      <PageHeader
        title={`Your ${recap.year}`}
        description="Your dictation year in review. Counted from your local history — nothing is uploaded or shared."
      />

      {/* Headline: the number that makes the case for the whole app. */}
      <div className="rounded-[14px] border bg-card p-6">
        <div className="text-[13px] font-medium" style={{ color: 'hsl(var(--faint))' }}>
          You saved
        </div>
        <div className="mt-1 font-display tnum text-[44px] font-medium leading-none">
          {fmtHours(recap.hours_saved)}
        </div>
        <div className="mt-2 text-[13px]">of typing this year, by speaking instead.</div>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-3">
        <Stat value={recap.total_words.toLocaleString()} label="Words dictated" />
        <Stat value={recap.total_dictations.toLocaleString()} label="Dictations" />
        <Stat value={recap.active_days} label="Active days" />
        <Stat
          value={recap.words_per_minute || '—'}
          label="Words per minute"
          hint={recap.words_per_minute ? 'while speaking' : 'needs more data'}
        />
        <Stat value={fmtHours(recap.hours_spoken)} label="Time spoken" />
        <Stat
          value={fmtDay(recap.busiest_day)}
          label="Busiest day"
          hint={recap.busiest_day ? `${recap.busiest_day_words.toLocaleString()} words` : undefined}
        />
      </div>

      {recap.top_app && (
        <div className="mt-3 rounded-[14px] border bg-card p-5">
          <div className="text-[13px] font-medium">Where you dictated most</div>
          <div className="mt-1 font-display text-[20px]">{recap.top_app}</div>
          <div className="mt-0.5 text-[12px]" style={{ color: 'hsl(var(--faint))' }}>
            {recap.top_app_dictations.toLocaleString()} dictations
          </div>
        </div>
      )}

      <div className="mt-4 flex items-center gap-3">
        <button
          onClick={share}
          disabled={sharing}
          className="rounded-[10px] border px-4 py-2 text-[13px] font-medium transition-colors hover:border-[hsl(var(--primary))] disabled:opacity-60"
        >
          {sharing ? 'Saving…' : 'Share as image'}
        </button>
        {shareMsg && (
          <span className="text-[12px]" style={{ color: 'hsl(var(--faint))' }}>
            {shareMsg}
          </span>
        )}
      </div>
      <p className="mt-2 text-xs text-muted-foreground">
        The image is saved to your device. Nothing is uploaded — you choose where to share it.
      </p>
    </div>
  )
}
