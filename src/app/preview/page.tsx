'use client'

/**
 * DEV-ONLY visual preview harness. NOT part of the shipped app.
 *
 * It renders the REAL Onboarding / Insights / Recap components against a stubbed Tauri
 * `invoke`, so their look can be reviewed in a plain browser (`next dev` → /preview) without
 * installing the desktop app or touching the owner's real database. Delete this route (or leave
 * it — it only loads its mock when opened) before cutting a release.
 *
 * How the mock works: `@tauri-apps/api/core`'s `invoke(cmd, args)` calls
 * `window.__TAURI_INTERNALS__.invoke`. We install a stub of exactly that, returning
 * representative-but-fake numbers for the three read commands these screens use. Nothing here
 * runs in production — the shipping UI at `/` goes through the real backend.
 */

import { useEffect, useState } from 'react'
import { SpotlightTour } from '@/components/SpotlightTour'
import { InsightsView } from '@/components/views/InsightsView'
import { RecapView } from '@/components/views/RecapView'
import { HistoryView } from '@/components/views/HistoryView'
import { recapYear } from '@/components/views/RecapView'

const YEAR = recapYear()

// Representative sample data — a believable few months of daily use, not the owner's real data.
const MOCK_TRANSCRIPTS = [
  {
    id: 1,
    timestamp: Math.floor(Date.now() / 1000) - 120,
    raw_text: 'we deployed the new kubernetes cluster today and all pods are healthy',
    cleaned_text: 'We deployed the new Kubernetes cluster today and all pods are healthy.',
    app_name: 'Visual Studio Code',
    duration_ms: 2400,
    audio_ms: 3200,
    model_used: 'moonshine-base-en',
    created_at: Math.floor(Date.now() / 1000) - 120,
  },
  {
    id: 2,
    timestamp: Math.floor(Date.now() / 1000) - 3600,
    raw_text: 'git status',
    cleaned_text: 'Git status',
    app_name: 'Windows Terminal',
    duration_ms: 800,
    audio_ms: 1100,
    model_used: 'moonshine-base-en',
    created_at: Math.floor(Date.now() / 1000) - 3600,
  },
  {
    id: 3,
    timestamp: Math.floor(Date.now() / 1000) - 86400,
    raw_text: 'react hooks are awesome for managing local component state',
    cleaned_text: 'React hooks are awesome for managing local component state.',
    app_name: 'Slack',
    duration_ms: 2100,
    audio_ms: 2800,
    model_used: 'moonshine-base-en',
    created_at: Math.floor(Date.now() / 1000) - 86400,
  },
]

const MOCK: Record<string, unknown> = {
  get_transcripts: MOCK_TRANSCRIPTS,
  transcript_daily_counts: [
    { day: new Date().toISOString().slice(0, 10), count: 5 },
    { day: new Date(Date.now() - 86400000).toISOString().slice(0, 10), count: 3 },
  ],
  delete_transcript: null,
  get_streak_state: {
    streak: 12,
    longest: 21,
    freezes: 3,
    max_freezes: 5,
    days_to_next_freeze: 4,
    today_counted: true,
    words_today: 340,
    min_words_per_day: 25,
  },
  get_stats: {
    total_dictations: 271,
    total_words: 48210,
    words_today: 340,
    words_per_minute: 132,
    total_audio_ms: 21_900_000, // ~6.1 hr spoken → ~14 hr saved vs typing at 40 wpm
    active_days: 46,
  },
  get_year_recap: {
    year: YEAR,
    total_words: 48210,
    total_dictations: 271,
    active_days: 46,
    hours_spoken: 6.1,
    hours_saved: 14.0,
    words_per_minute: 132,
    busiest_day: `${YEAR}-03-14`,
    busiest_day_words: 1820,
    top_app: 'Visual Studio Code',
    top_app_dictations: 96,
  },
  // The share button would call this; return a plausible path so the success state renders.
  save_share_image: `C:\\Users\\you\\Pictures\\aurascribe-${YEAR}-recap.png`,
}

// Install the stub at module load (client only), before any component effect runs.
if (typeof window !== 'undefined') {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const w = window as any
  w.__TAURI_INTERNALS__ = w.__TAURI_INTERNALS__ ?? {}
  w.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: any) => {
    if (cmd === 'search_transcripts') {
      const q = (args?.query ?? '').toLowerCase()
      return MOCK_TRANSCRIPTS.filter(
        (t) =>
          t.raw_text.toLowerCase().includes(q) ||
          (t.cleaned_text && t.cleaned_text.toLowerCase().includes(q))
      )
    }
    if (cmd in MOCK) return MOCK[cmd]
    // Anything else these screens might touch: don't throw, just return empty.
    return null
  }
}

type Mode = 'history' | 'insights' | 'recap' | 'onboarding'

export default function PreviewPage() {
  const [mode, setMode] = useState<Mode>('history')

  // Match the real app's default appearance (Glass): light text on dark frosted panels over the
  // bluish backdrop. Same class toggles as src/app/page.tsx.
  useEffect(() => {
    const root = document.documentElement
    root.classList.add('glass-bg', 'dark')
    return () => root.classList.remove('glass-bg', 'dark')
  }, [])

  const tabs: { id: Mode; label: string }[] = [
    { id: 'history', label: 'History · Search & Delete' },
    { id: 'insights', label: 'Insights · Streak Share' },
    { id: 'recap', label: `Recap · ${YEAR}` },
    { id: 'onboarding', label: 'Onboarding' },
  ]

  return (
    <div className="min-h-screen w-full bg-background">
      {/* Preview-only toolbar. Not part of the app. */}
      <div className="sticky top-0 z-[60] flex items-center gap-2 border-b bg-background/80 px-5 py-3 backdrop-blur-md">
        <span className="mr-2 text-[12px] font-medium text-muted-foreground">Preview</span>
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setMode(t.id)}
            className={`rounded-full border px-3 py-1 text-[12px] transition-colors ${
              mode === t.id
                ? 'border-[hsl(var(--primary))] font-medium text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            {t.label}
          </button>
        ))}
        <span className="ml-auto text-[11px] text-muted-foreground">
          sample data · local testing
        </span>
      </div>

      {mode === 'history' && (
        <div className="mx-auto max-w-3xl px-8 py-10">
          <HistoryView />
        </div>
      )}

      {mode === 'insights' && (
        <div className="mx-auto max-w-3xl px-8 py-10">
          <InsightsView onOpenRecap={() => setMode('recap')} />
        </div>
      )}

      {mode === 'recap' && (
        <div className="mx-auto max-w-3xl px-8 py-10">
          <RecapView onBack={() => setMode('insights')} />
        </div>
      )}

      {mode === 'onboarding' && (
        <div className="relative h-[calc(100vh-49px)] w-full overflow-hidden">
          <div className="mx-auto max-w-md pt-16 text-center">
            <h1 className="font-display text-[28px] font-medium">Add a voice model to begin</h1>
            <p className="mt-3 text-sm text-muted-foreground">
              AuraScribe transcribes on this machine, so it needs a speech model installed first. You
              download it once — after that it works offline, forever.
            </p>
            <button data-tour="download-model" className="btn-primary mx-auto mt-6">
              Choose a model
            </button>
          </div>
          <SpotlightTour hotkey="Ctrl+Shift+Space" onFinish={() => setMode('history')} />
        </div>
      )}
    </div>
  )
}
