'use client'

import { useEffect, useState } from 'react'
import { Flame, Snowflake, Share2 } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { UsageStats, StreakInfo } from '@/lib/ipc'
import { PageHeader, Stat, EmptyState, ErrorNote } from '@/components/ui'
import { recapYear } from '@/components/views/RecapView'
import { renderAndSaveCard } from '@/lib/shareCard'

/** Typing 40 wpm is a common average; the gap against speaking is the time saved. */
const TYPING_WPM = 40

function formatDuration(ms: number) {
  const minutes = Math.round(ms / 60000)
  if (minutes < 60) return `${minutes} min`
  return `${(minutes / 60).toFixed(1)} hr`
}

// Milestones are quiet achievements, not a game. Values are round and honest; a badge lights up
// once the underlying number (from real local history) crosses it.
const STREAK_MILESTONES = [7, 30, 100, 365]
const WORD_MILESTONES = [10_000, 100_000, 1_000_000]
const HOUR_MILESTONES = [10, 100]
const ACK_KEY = 'aurascribe.milestones.ack'

type Milestone = { key: string; label: string; reached: boolean }

function compactWords(n: number) {
  if (n >= 1_000_000) return `${n / 1_000_000}M words`
  if (n >= 1_000) return `${n / 1_000}k words`
  return `${n} words`
}

/** Current streak, longest, freeze slots, and today's progress — the habit readout. */
function StreakCard({ s, stats }: { s: StreakInfo; stats?: UsageStats | null }) {
  const alive = s.streak > 0
  const [sharing, setSharing] = useState(false)
  const [shareMsg, setShareMsg] = useState<string | null>(null)

  // Colour means state: the flame reads live (signal-cyan) only when today is already safe;
  // otherwise it is muted so an at-risk day looks at-risk.
  const flameColor = s.today_counted ? 'hsl(var(--primary))' : 'hsl(var(--muted-foreground))'
  const remaining = Math.max(0, s.min_words_per_day - s.words_today)

  const share = async () => {
    if (!s || s.streak <= 0) return
    setSharing(true)
    setShareMsg(null)
    try {
      await renderAndSaveCard({
        filename: `aurascribe-streak-${s.streak}-days`,
        kicker: 'AuraScribe · Habit',
        headline: `${s.streak}`,
        headlineSub: s.streak === 1 ? 'day active streak' : 'days in a row',
        stats: [
          {
            value: stats ? stats.total_words.toLocaleString() : String(s.words_today),
            label: 'Words dictated',
          },
          { value: `${s.longest} days`, label: 'Longest streak' },
          { value: `${s.freezes} / ${s.max_freezes}`, label: 'Freezes banked' },
          { value: String(stats?.active_days ?? '—'), label: 'Active days' },
        ],
      })
      setShareMsg('Saved to Pictures')
      setTimeout(() => setShareMsg(null), 3000)
    } catch (e) {
      setShareMsg(`Could not save: ${e}`)
    } finally {
      setSharing(false)
    }
  }

  return (
    <div className="rounded-[14px] border bg-card p-5">
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3">
          <Flame className="h-8 w-8 shrink-0" strokeWidth={1.8} color={flameColor} />
          <div>
            <div className="font-display tnum text-[34px] font-medium leading-none">{s.streak}</div>
            <div className="mt-1.5 text-[13px] font-medium">
              {alive ? `day${s.streak === 1 ? '' : 's'} in a row` : 'no streak yet'}
            </div>
          </div>
        </div>

        <div className="text-right">
          <div className="mono text-[12px]" style={{ color: 'hsl(var(--faint))' }}>
            longest {s.longest}
          </div>
          <div className="mt-2 flex justify-end gap-1" title={`${s.freezes} of ${s.max_freezes} streak freezes`}>
            {Array.from({ length: s.max_freezes }).map((_, i) => (
              <Snowflake
                key={i}
                className="h-[15px] w-[15px]"
                strokeWidth={2}
                color={i < s.freezes ? 'hsl(var(--primary))' : 'hsl(var(--border))'}
              />
            ))}
          </div>
          <div className="mt-1 text-[11px]" style={{ color: 'hsl(var(--faint))' }}>
            {s.freezes >= s.max_freezes
              ? 'freezes full'
              : `${s.days_to_next_freeze} day${s.days_to_next_freeze === 1 ? '' : 's'} to next freeze`}
          </div>
        </div>
      </div>

      <div className="mt-4 flex items-center justify-between border-t pt-3 text-[12px]">
        {s.today_counted ? (
          <span>
            <span style={{ color: 'hsl(var(--primary))' }}>Today counts</span>
            <span style={{ color: 'hsl(var(--faint))' }}> · {s.words_today} words so far</span>
          </span>
        ) : (
          <span style={{ color: 'hsl(var(--faint))' }}>
            <span className="mono" style={{ color: 'hsl(var(--muted-foreground))' }}>
              {s.words_today}/{s.min_words_per_day}
            </span>{' '}
            words today — {remaining} more keeps your streak
          </span>
        )}

        {alive && (
          <div className="flex items-center gap-2">
            {shareMsg && <span className="text-[11px] text-muted-foreground">{shareMsg}</span>}
            <button
              onClick={share}
              disabled={sharing}
              className="btn-ghost btn-sm text-[11px]"
              title="Save shareable streak card to Pictures"
            >
              <Share2 className="h-3 w-3" />
              {sharing ? 'Saving…' : 'Share card'}
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

/** A wrapped strip of earned/unearned milestone chips. No motion (instrument aesthetic); a newly
 *  crossed milestone gets a plain "new" tag, acknowledged in localStorage so it shows only once. */
function Milestones({ items }: { items: Milestone[] }) {
  const [fresh, setFresh] = useState<Set<string>>(new Set())

  useEffect(() => {
    const reached = items.filter((m) => m.reached).map((m) => m.key)
    let ack: string[] = []
    try {
      ack = JSON.parse(localStorage.getItem(ACK_KEY) || '[]')
    } catch {
      /* corrupt/absent — treat as none acknowledged */
    }
    const newly = reached.filter((k) => !ack.includes(k))
    if (newly.length) setFresh(new Set(newly))
    // Acknowledge everything currently reached so the "new" tag shows only on this visit.
    try {
      localStorage.setItem(ACK_KEY, JSON.stringify(reached))
    } catch {
      /* private mode / storage full — the tag just won't persist, which is fine */
    }
  }, [items])

  return (
    <div>
      <div className="mb-2 text-[13px] font-medium">Milestones</div>
      <div className="flex flex-wrap gap-2">
        {items.map((m) => (
          <span
            key={m.key}
            className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[12px] ${
              m.reached ? 'font-medium' : ''
            }`}
            style={
              m.reached
                ? undefined
                : { color: 'hsl(var(--faint))', borderColor: 'hsl(var(--border))', opacity: 0.7 }
            }
          >
            <span
              className="h-1.5 w-1.5 rounded-full"
              style={{ background: m.reached ? 'hsl(var(--primary))' : 'hsl(var(--border))' }}
            />
            {m.label}
            {fresh.has(m.key) && (
              <span className="mono text-[10px]" style={{ color: 'hsl(var(--primary))' }}>
                new
              </span>
            )}
          </span>
        ))}
      </div>
    </div>
  )
}

export function InsightsView({ onOpenRecap }: { onOpenRecap?: () => void }) {
  const [stats, setStats] = useState<UsageStats | null>(null)
  const [streak, setStreak] = useState<StreakInfo | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    ipc.getStats().then(setStats).catch((e) => setError(String(e)))
    // The streak is a nice-to-have overlay; if it fails (e.g. browser preview) the page still works.
    ipc.getStreakState().then(setStreak).catch(() => {})
  }, [])

  if (error) return <ErrorNote>{error}</ErrorNote>
  if (!stats) return null

  if (stats.total_dictations === 0) {
    return (
      <div>
        <PageHeader title="Insights" description="How much you actually dictate." />
        <EmptyState
          title="No data yet"
          hint="Dictate a few times and your streak, speaking rate and totals will appear here."
        />
      </div>
    )
  }

  const spokenMinutes = stats.total_audio_ms / 60000
  const typedMinutes = stats.words_per_minute > 0 ? stats.total_words / TYPING_WPM : 0
  const savedMinutes = Math.max(0, typedMinutes - spokenMinutes)
  const savedHours = (savedMinutes * 60000) / 3_600_000
  const bestStreak = streak ? Math.max(streak.streak, streak.longest) : 0

  const milestones: Milestone[] = [
    ...STREAK_MILESTONES.map((d) => ({
      key: `streak-${d}`,
      label: `${d}-day streak`,
      reached: bestStreak >= d,
    })),
    ...WORD_MILESTONES.map((w) => ({
      key: `words-${w}`,
      label: compactWords(w),
      reached: stats.total_words >= w,
    })),
    ...HOUR_MILESTONES.map((h) => ({
      key: `saved-${h}`,
      label: `${h}h saved`,
      reached: savedHours >= h,
    })),
  ]

  return (
    <div>
      <PageHeader title="Insights" description="How much you actually dictate." />

      {streak && (
        <div className="mb-3">
          <StreakCard s={streak} stats={stats} />
        </div>
      )}

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

      <div className="mt-4">
        <Milestones items={milestones} />
      </div>

      {onOpenRecap && (
        <button
          onClick={onOpenRecap}
          className="mt-4 flex w-full items-center justify-between rounded-[14px] border bg-card p-5 text-left transition-colors hover:border-[hsl(var(--primary))]"
        >
          <div>
            <div className="text-[13px] font-medium">Your {recapYear()} in review</div>
            <div className="mt-0.5 text-[12px]" style={{ color: 'hsl(var(--faint))' }}>
              Hours saved, words, your busiest day — a yearly recap.
            </div>
          </div>
          <span aria-hidden className="text-muted-foreground">
            →
          </span>
        </button>
      )}

      <p className="mt-3 text-xs text-muted-foreground">
        Counted from your local history. Nothing here is uploaded or shared.
      </p>
    </div>
  )
}
