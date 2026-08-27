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
import { SettingsView } from '@/components/views/SettingsView'
import type { Settings, Status } from '@/lib/ipc'

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
  // Settings screen reads these on mount.
  list_audio_devices: ['Default microphone', 'USB Microphone'],
  get_available_models: [
    {
      id: 'moonshine-base-en',
      name: 'AuraScribe English',
      engine: 'moonshine',
      size_mb: 286,
      multilingual: false,
      speed: 1,
      accuracy: 4,
      recommended: true,
      downloaded: true,
      path: null,
      realtime_factor: 0.15,
      warning: null,
    },
    {
      id: 'parakeet-v3-multilingual',
      name: 'AuraScribe European',
      engine: 'parakeet',
      size_mb: 671,
      multilingual: true,
      speed: 3,
      accuracy: 5,
      recommended: false,
      downloaded: true,
      path: null,
      realtime_factor: 0.5,
      warning: null,
    },
  ],
  // Storage report mirroring a real machine: legitimate models + dead Whisper leftovers + a
  // stranded partial download — so the "Reclaim" affordance is visible with a meaningful number.
  get_storage_report: {
    entries: [
      { name: 'ggml-large-v3.bin', size_bytes: 3_095_033_483, kind: 'orphan' },
      { name: 'ggml-large-v3-turbo.bin.part', size_bytes: 1_571_170_194, kind: 'partial' },
      { name: 'parakeet-v3-multilingual', size_bytes: 671_000_000, kind: 'model' },
      { name: 'ggml-small.bin', size_bytes: 487_601_967, kind: 'orphan' },
      { name: 'moonshine-base-en', size_bytes: 286_000_000, kind: 'model' },
    ],
    models_bytes: 957_000_000,
    reclaimable_bytes: 3_095_033_483 + 1_571_170_194 + 487_601_967,
    db_bytes: 901_120,
    total_bytes: 957_000_000 + 3_095_033_483 + 1_571_170_194 + 487_601_967 + 901_120,
  },
  reclaim_storage: 3_095_033_483 + 1_571_170_194 + 487_601_967,
}

const MOCK_SETTINGS: Settings = {
  hotkey: 'Ctrl+Shift+Space',
  hotkey_mode: 'toggle',
  whisper_model: 'moonshine-base-en',
  mic_device: null,
  ai_cleanup_enabled: true,
  remove_fillers: true,
  language: 'en',
  theme: 'glass',
  start_at_login: false,
  sound_cues: true,
  onboarded: true,
  hotkey_enabled: true,
  noise_suppression: false,
  prompt_optimize_enabled: true,
  prompt_optimize_hotkey: 'Ctrl+Shift+O',
}

const MOCK_STATUS: Status = {
  is_recording: false,
  is_processing: false,
  is_model_loaded: true,
  loaded_model: 'moonshine-base-en',
  current_text: '',
  last_error: null,
  hotkey_mode: 'toggle',
  ai_cleanup_enabled: true,
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

type Mode = 'history' | 'insights' | 'recap' | 'onboarding' | 'settings'

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
    { id: 'settings', label: 'Settings · Storage' },
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

      {mode === 'settings' && (
        <div className="mx-auto max-w-3xl px-8 py-10">
          <SettingsView
            settings={MOCK_SETTINGS}
            status={MOCK_STATUS}
            onSaveSettings={() => {}}
          />
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
