'use client'

import { useEffect, useState } from 'react'
import * as ipc from '@/lib/ipc'
import type { UsageStats } from '@/lib/ipc'
import { PageHeader, Stat, EmptyState, ErrorNote } from '@/components/ui'

/** Typing 40 wpm is a common average; the gap against speaking is the time saved. */
const TYPING_WPM = 40

function formatDuration(ms: number) {
  const minutes = Math.round(ms / 60000)
  if (minutes < 60) return `${minutes} min`
  return `${(minutes / 60).toFixed(1)} hr`
}

export function InsightsView() {
  const [stats, setStats] = useState<UsageStats | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    ipc.getStats().then(setStats).catch((e) => setError(String(e)))
  }, [])

  if (error) return <ErrorNote>{error}</ErrorNote>
  if (!stats) return null

  if (stats.total_dictations === 0) {
    return (
      <div>
        <PageHeader title="Insights" description="How much you actually dictate." />
        <EmptyState
          title="No data yet"
          hint="Dictate a few times and your speaking rate and totals will appear here."
        />
      </div>
    )
  }

  const spokenMinutes = stats.total_audio_ms / 60000
  const typedMinutes = stats.words_per_minute > 0 ? stats.total_words / TYPING_WPM : 0
  const savedMinutes = Math.max(0, typedMinutes - spokenMinutes)

  return (
    <div>
      <PageHeader title="Insights" description="How much you actually dictate." />

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        <Stat value={stats.total_words.toLocaleString()} label="Words dictated" />
        <Stat
          value={stats.words_per_minute || '—'}
          label="Words per minute"
          hint={stats.words_per_minute ? 'while speaking' : 'needs more data'}
        />
        <Stat value={stats.total_dictations.toLocaleString()} label="Dictations" />
        <Stat value={stats.words_today.toLocaleString()} label="Words today" />
        <Stat value={stats.active_days} label="Active days" />
        <Stat
          value={formatDuration(savedMinutes * 60000)}
          label="Time saved"
          hint={`vs typing at ${TYPING_WPM} wpm`}
        />
      </div>

      <p className="mt-3 text-xs text-muted-foreground">
        Counted from your local history. Nothing here is uploaded or shared.
      </p>
    </div>
  )
}
