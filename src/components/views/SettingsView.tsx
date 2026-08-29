'use client'

import { useCallback, useEffect, useState } from 'react'
import { Download, Loader2, Check, Trash2, Globe, AlertTriangle } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { ModelInfo, Settings, Status, StorageReport, OptimizeModelStatus } from '@/lib/ipc'
import { modelDisplay, EUROPEAN_LANGS } from '@/lib/models'
import { PageHeader, Section, ErrorNote, Toggle, Select } from '@/components/ui'

function HotkeyCapture({
  value,
  onChange,
}: {
  value: string
  onChange: (combo: string) => void
}) {
  const [capturing, setCapturing] = useState(false)
  const [hint, setHint] = useState<string | null>(null)

  useEffect(() => {
    if (!capturing) return
    const mods = new Set<string>()

    const handler = (e: KeyboardEvent) => {
      e.preventDefault()
      if (e.code === 'Escape') {
        setCapturing(false)
        setHint(null)
        return
      }
      if (/^(Control|Alt|Shift|Meta)/.test(e.code)) {
        if (e.code.startsWith('Control')) mods.add('Ctrl')
        if (e.code.startsWith('Alt')) mods.add('Alt')
        if (e.code.startsWith('Shift')) mods.add('Shift')
        if (e.code.startsWith('Meta')) mods.add('Super')
        return
      }
      // A lone key (a letter, a number, Space, F-key…) would hijack dictation every time you
      // type that key anywhere. A shortcut must combine a modifier with a key — that is what
      // makes a global hotkey usable while you keep typing in other apps.
      if (mods.size === 0) {
        setHint('Hold a modifier (Ctrl/Alt/Shift) and press a key, e.g. Ctrl+Shift+Space.')
        return
      }
      onChange([...mods, e.code].join('+'))
      setCapturing(false)
      setHint(null)
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [capturing, onChange])

  return (
    <div className="flex flex-col gap-1">
      <button
        onClick={() => {
          setCapturing((c) => !c)
          setHint(null)
        }}
        className={`input mono text-center ${capturing ? 'border-primary' : ''}`}
      >
        {capturing ? 'Press your keys…' : value}
      </button>
      {(hint || capturing) && (
        <span className="text-[11px] leading-snug text-muted-foreground">
          {hint ??
            'Hold a modifier (Ctrl / Alt / Shift) and press a key — a single key won’t be accepted.'}
        </span>
      )}
    </div>
  )
}

function speedLabel(speed: number) {
  return ['', 'Fastest', 'Fast', 'Moderate', 'Slow', 'Slowest'][speed] ?? ''
}

/** Human-readable byte size (binary units). */
function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let v = n / 1024
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i += 1
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`
}

/** Settings → Storage: shows the on-device disk footprint (models + data), lets the user delete a
 *  downloaded model, and reclaims orphaned/partial model files the app can no longer use. */
function StorageSection({ onChanged }: { onChanged?: () => void }) {
  const [report, setReport] = useState<StorageReport | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setReport(await ipc.getStorageReport())
    } catch (e) {
      setErr(String(e))
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const reclaim = async () => {
    setBusy('reclaim')
    setErr(null)
    try {
      await ipc.reclaimStorage()
      await load()
    } catch (e) {
      setErr(String(e))
    } finally {
      setBusy(null)
    }
  }

  const removeModel = async (name: string) => {
    setBusy(name)
    setErr(null)
    try {
      await ipc.deleteModel(name)
      await load()
      onChanged?.()
    } catch (e) {
      setErr(String(e))
    } finally {
      setBusy(null)
    }
  }

  return (
    <Section
      title="Storage"
      description="What AuraScribe uses on this device. Your transcripts are text, so they take almost no space — the weight is downloaded models."
    >
      <div className="flex flex-col gap-3">
        {err && <ErrorNote>{err}</ErrorNote>}

        {report && (
          <>
            <div className="grid grid-cols-3 gap-2">
              {[
                ['Models', report.models_bytes],
                ['Your data', report.db_bytes],
                ['Total', report.total_bytes],
              ].map(([label, bytes]) => (
                <div key={label as string} className="panel p-3">
                  <div className="text-[11px] text-muted-foreground">{label}</div>
                  <div className="mono mt-0.5 text-sm font-semibold">{formatBytes(bytes as number)}</div>
                </div>
              ))}
            </div>

            {report.reclaimable_bytes > 0 && (
              <div className="panel border-primary/40 p-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="text-sm font-semibold">
                      Reclaim {formatBytes(report.reclaimable_bytes)}
                    </div>
                    <p className="mt-0.5 text-[11px] text-muted-foreground">
                      Leftover and partly-downloaded model files the app can no longer use. Safe to
                      remove — your active models are untouched.
                    </p>
                  </div>
                  <button
                    onClick={reclaim}
                    disabled={busy !== null}
                    className="btn-primary btn-sm shrink-0"
                  >
                    {busy === 'reclaim' ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      'Reclaim'
                    )}
                  </button>
                </div>
              </div>
            )}

            <ul className="flex flex-col divide-y divide-border">
              {report.entries.map((e) => (
                <li key={e.name} className="flex items-center justify-between gap-3 py-2">
                  <div className="min-w-0">
                    <div className="truncate text-sm">{e.name}</div>
                    <div className="text-[11px] text-muted-foreground">
                      {e.kind === 'model'
                        ? 'Active model'
                        : e.kind === 'partial'
                          ? 'Partial download'
                          : 'Leftover'}
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="mono text-xs text-muted-foreground">
                      {formatBytes(e.size_bytes)}
                    </span>
                    {e.kind === 'model' && (
                      <button
                        onClick={() => removeModel(e.name)}
                        disabled={busy !== null}
                        aria-label={`Delete ${e.name}`}
                        className="btn-secondary btn-sm"
                      >
                        {busy === e.name ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Trash2 className="h-4 w-4" />
                        )}
                      </button>
                    )}
                  </div>
                </li>
              ))}
            </ul>

            {report.entries.length === 0 && (
              <p className="text-[11px] text-muted-foreground">
                No models downloaded yet.
              </p>
            )}
          </>
        )}
      </div>
    </Section>
  )
}

/** The optimizer-model status + one-click download, shown inside Settings → Prompt optimization.
 *  Replaces the old "coming soon" note so users can install the model from the UI. */
function OptimizeModelRow() {
  const [status, setStatus] = useState<OptimizeModelStatus | null>(null)
  const [downloading, setDownloading] = useState(false)
  const [progress, setProgress] = useState(0)
  const [err, setErr] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      setStatus(await ipc.optimizeModelStatus())
    } catch (e) {
      setErr(String(e))
    }
  }, [])

  useEffect(() => {
    refresh()
    const un = ipc.onOptimizeModelDownloadProgress((p) => setProgress(p))
    return () => {
      un.then((f) => f())
    }
  }, [refresh])

  const download = async () => {
    setDownloading(true)
    setErr(null)
    setProgress(0)
    try {
      await ipc.downloadOptimizeModel()
      await refresh()
    } catch (e) {
      setErr(String(e))
    } finally {
      setDownloading(false)
    }
  }

  if (status?.installed) {
    return (
      <div className="flex items-center gap-2 text-[12px] text-muted-foreground">
        <Check className="h-4 w-4 text-[hsl(var(--primary))]" />
        <span>Optimizer model installed{status.name ? ` (${status.name})` : ''}.</span>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-2">
      {err && <ErrorNote>{err}</ErrorNote>}
      <div className="flex items-center justify-between gap-3">
        <p className="text-[11px] text-muted-foreground">
          The optimizer needs a small local model (Qwen2.5-0.5B, ~{status?.size_mb ?? 491} MB, one-time).
          Nothing runs until it&rsquo;s installed.
        </p>
        <button onClick={download} disabled={downloading} className="btn-primary btn-sm shrink-0">
          {downloading ? (
            <span className="mono">{Math.round(progress * 100)}%</span>
          ) : (
            <span className="inline-flex items-center gap-1">
              <Download className="h-4 w-4" /> Download model
            </span>
          )}
        </button>
      </div>
      {downloading && (
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-[hsl(var(--muted))]">
          <div
            className="h-full bg-[hsl(var(--primary))] transition-[width] duration-200"
            style={{ width: `${Math.round(progress * 100)}%` }}
          />
        </div>
      )}
    </div>
  )
}

export function SettingsView({
  settings,
  status,
  onSaveSettings,
  onReplayTour,
}: {
  settings: Settings
  status: Status
  onSaveSettings: (patch: Partial<Settings>) => void
  /** Re-open the first-run walkthrough. Absent in contexts without the tour (e.g. previews). */
  onReplayTour?: () => void
}) {
  const [models, setModels] = useState<ModelInfo[]>([])
  const [micDevices, setMicDevices] = useState<string[]>([])
  const [progress, setProgress] = useState<Record<string, number>>({})
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      setModels(await ipc.getAvailableModels())
    } catch (e) {
      setError(String(e))
    }
  }, [])

  useEffect(() => {
    refresh()
    ipc.listAudioDevices().then(setMicDevices).catch(() => {})
    const un = ipc.onModelDownloadProgress(({ modelId, progress }) =>
      setProgress((p) => ({ ...p, [modelId]: progress }))
    )
    return () => {
      un.then((f) => f())
    }
  }, [refresh])

  // Downloading is pointless on its own, so install and activate in one action.
  const install = async (id: string) => {
    setBusy(id)
    setError(null)
    try {
      await ipc.downloadModel(id)
      await refresh()
      await ipc.loadModel(id)
      onSaveSettings({ whisper_model: id })
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(null)
      setProgress((p) => ({ ...p, [id]: 0 }))
    }
  }

  const activate = async (id: string) => {
    setBusy(id)
    setError(null)
    try {
      await ipc.loadModel(id)
      onSaveSettings({ whisper_model: id })
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(null)
    }
  }

  const remove = async (id: string) => {
    setError(null)
    try {
      await ipc.deleteModel(id)
      await refresh()
    } catch (e) {
      setError(String(e))
    }
  }

  // How the Language control should behave for the model that's actually loaded. Parakeet (and
  // any custom transducer bundle) auto-detects; Moonshine / tiny.en are English-only; a
  // multilingual Whisper model uses the manual selector.
  const activeModel = models.find((m) => m.id === status.loaded_model)
  const langMode: 'auto' | 'english' | 'manual' = !activeModel
    ? 'manual'
    : activeModel.engine === 'parakeet' || activeModel.engine === 'dolphin' || activeModel.engine === 'nemoctc'
    ? 'auto'
    : activeModel.multilingual
    ? 'manual'
    : 'english'

  return (
    <div className="flex flex-col gap-3.5 pb-6">
      <PageHeader title="Settings" />

      <Section
        title="Voice model"
        description="Runs on this machine. Downloaded once, then works offline forever."
      >
        {error && (
          <div className="mb-2.5">
            <ErrorNote>{error}</ErrorNote>
          </div>
        )}

        <div className="flex flex-col gap-1.5">
          {models.map((m) => {
            // Trust what is actually loaded, not what was last saved — those diverge
            // whenever a load fails, and the saved value alone once made every model
            // look inactive.
            const active = status.loaded_model === m.id
            const pct = progress[m.id]
            const downloading = busy === m.id && pct !== undefined && pct > 0 && pct < 1

            return (
              <div key={m.id} className="panel p-3">
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <span className="text-[14px] font-semibold">{modelDisplay(m).title}</span>
                      {m.recommended && (
                        <span className="rounded bg-primary/12 px-1.5 py-0.5 text-[10px] font-medium text-primary">
                          Recommended
                        </span>
                      )}
                    </div>
                    <div className="mono mt-0.5 text-[11px] text-muted-foreground">
                      Powered by {modelDisplay(m).poweredBy}
                      {' · '}
                      {m.size_mb >= 1000
                        ? `${(m.size_mb / 1000).toFixed(1)} GB`
                        : `${m.size_mb} MB`}
                      {' · '}
                      {m.realtime_factor <= 1
                        ? `${m.realtime_factor.toFixed(1)}x — faster than you speak`
                        : `${m.realtime_factor.toFixed(1)}x`}
                    </div>

                    {/* Full language coverage — so you know exactly what a model offers. */}
                    <div className="mt-1.5 flex items-start gap-1.5 text-[11px] leading-relaxed text-muted-foreground">
                      <Globe className="mt-0.5 h-3 w-3 shrink-0 opacity-70" />
                      <span>{modelDisplay(m).languages}</span>
                    </div>

                    {/* Shown before download, not after. A user picked the model badged most
                        accurate and waited 7.9 minutes for 17 seconds of speech. */}
                    {m.warning && (
                      <p
                        className={`mt-1.5 flex items-start gap-1.5 text-[11px] leading-snug ${
                          m.realtime_factor > 2 ? 'text-record' : 'text-standby'
                        }`}
                      >
                        <AlertTriangle className="mt-px h-3 w-3 shrink-0" />
                        <span>{m.warning}</span>
                      </p>
                    )}
                  </div>

                  <div className="flex shrink-0 items-center gap-1.5">
                    {active ? (
                      <span className="inline-flex items-center gap-1 text-xs text-primary">
                        <Check className="h-3.5 w-3.5" />
                        In use
                      </span>
                    ) : m.downloaded ? (
                      <>
                        <button
                          onClick={() => activate(m.id)}
                          disabled={busy !== null}
                          className="btn-secondary btn-sm"
                        >
                          {busy === m.id ? (
                            <Loader2 className="h-3 w-3 animate-spin" />
                          ) : (
                            'Use'
                          )}
                        </button>
                        <button
                          onClick={() => remove(m.id)}
                          className="btn-ghost btn-sm"
                          aria-label={`Delete ${m.id}`}
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      </>
                    ) : (
                      <button
                        onClick={() => install(m.id)}
                        disabled={busy !== null}
                        className="btn-primary btn-sm"
                      >
                        {busy === m.id ? (
                          <Loader2 className="h-3 w-3 animate-spin" />
                        ) : (
                          <Download className="h-3 w-3" />
                        )}
                        {busy === m.id ? 'Getting…' : 'Install'}
                      </button>
                    )}
                  </div>
                </div>

                {downloading && (
                  <div className="mt-2 h-1 overflow-hidden rounded-full bg-secondary">
                    <div
                      className="h-full bg-primary transition-[width] duration-200"
                      style={{ width: `${Math.round(pct * 100)}%` }}
                    />
                  </div>
                )}
              </div>
            )
          })}
        </div>
      </Section>

      <Section title="Hotkey" description="Works in any application, even when this window is closed.">
        <div className="grid grid-cols-2 gap-3">
          <label>
            <span className="mb-1 block text-xs font-medium">Shortcut</span>
            <HotkeyCapture
              value={settings.hotkey}
              onChange={(combo) => onSaveSettings({ hotkey: combo })}
            />
          </label>
          <label>
            <span className="mb-1 block text-xs font-medium">Mode</span>
            <Select
              aria-label="Hotkey mode"
              value={settings.hotkey_mode}
              onChange={(v) => onSaveSettings({ hotkey_mode: v as Settings['hotkey_mode'] })}
              options={[
                { value: 'toggle', label: 'Tap to start and stop' },
                { value: 'press-hold', label: 'Hold while speaking' },
              ]}
            />
          </label>
        </div>
        <div className="mt-3">
          <Toggle
            checked={settings.hotkey_enabled}
            onChange={(v) => onSaveSettings({ hotkey_enabled: v })}
            label="Enable the dictation hotkey"
          />
          {!settings.hotkey_enabled && (
            <p className="mt-1.5 text-[11px] text-muted-foreground">
              The hotkey is off — AuraScribe won’t start dictation from a keypress until you turn
              this back on.
            </p>
          )}
        </div>
      </Section>

      <Section
        title="Cleanup"
        description="Runs on this machine straight after transcription — no network, no delay."
      >
        <div className="flex flex-col gap-3">
          <Toggle
            checked={settings.ai_cleanup_enabled}
            onChange={(v) => onSaveSettings({ ai_cleanup_enabled: v })}
            label="Tidy up my dictation"
            hint="Fixes punctuation and capitalisation, drops background-noise artefacts"
          />
          {settings.ai_cleanup_enabled && (
            <Toggle
              checked={settings.remove_fillers}
              onChange={(v) => onSaveSettings({ remove_fillers: v })}
              label="Remove filler words"
              hint="um, uh, like, you know"
            />
          )}
        </div>
      </Section>

      <Section title="Audio and language">
        <div className="grid grid-cols-2 gap-3">
          <label>
            <span className="mb-1 block text-xs font-medium">Microphone</span>
            <Select
              aria-label="Microphone"
              value={settings.mic_device ?? ''}
              onChange={(v) => onSaveSettings({ mic_device: v || null })}
              options={[
                { value: '', label: 'System default' },
                ...micDevices.map((d) => ({ value: d, label: d })),
              ]}
            />
          </label>
          <label>
            <span className="mb-1 block text-xs font-medium">Language</span>
            {/* The language control depends on the active engine: Parakeet auto-detects and
                ignores a manual choice; Moonshine / tiny.en are English-only; only a
                multilingual Whisper model actually uses the manual selector. Showing an
                editable dropdown for a model that ignores it was misleading. */}
            {langMode === 'auto' ? (
              <div className="input flex items-center text-muted-foreground" title={EUROPEAN_LANGS}>
                Detected automatically
              </div>
            ) : langMode === 'english' ? (
              <div className="input flex items-center text-muted-foreground">English</div>
            ) : (
              <Select
                aria-label="Language"
                value={settings.language}
                onChange={(v) => onSaveSettings({ language: v })}
                options={[
                  { value: 'auto', label: 'Detect automatically' },
                  { value: 'en', label: 'English' },
                  { value: 'hi', label: 'Hindi' },
                  { value: 'ml', label: 'Malayalam' },
                  { value: 'es', label: 'Spanish' },
                  { value: 'fr', label: 'French' },
                  { value: 'de', label: 'German' },
                  { value: 'ja', label: 'Japanese' },
                  { value: 'zh', label: 'Chinese' },
                ]}
              />
            )}
          </label>
        </div>
        <p className="mt-2 text-[11px] text-muted-foreground">
          {langMode === 'auto'
            ? activeModel?.engine === 'nemoctc'
              ? `${modelDisplay(activeModel).languages} No language selection needed.`
              : activeModel?.engine === 'dolphin'
              ? 'Detects the language automatically across ~40 Asian languages, including Hindi, Tamil, Telugu, Bengali, Urdu, Marathi, Gujarati and more. It does not cover Malayalam or Kannada.'
              : 'Detects the language automatically across 25 European languages (English, German, French, Spanish, Italian, Portuguese, Dutch, Polish, Russian, Ukrainian and more). It does not cover Hindi, Malayalam, or other non-European languages.'
            : langMode === 'english'
            ? 'This model transcribes English only. Multilingual dictation needs a multilingual model.'
            : 'This model uses the language you pick above.'}
        </p>
        <div className="mt-4">
          <Toggle
            checked={settings.noise_suppression}
            onChange={(v) => onSaveSettings({ noise_suppression: v })}
            label="Reduce background noise"
            hint="Cuts steady noise (fans, AC, traffic hum) before transcription, on your device. Best in a consistently noisy room; leave off in a quiet one."
          />
        </div>
      </Section>

      <Section
        title="Prompt optimization"
        description="Select any text and press a shortcut to rewrite it into a better AI prompt, in place. Runs on your device."
      >
        <div className="flex flex-col gap-3">
          <Toggle
            checked={settings.prompt_optimize_enabled}
            onChange={(v) => onSaveSettings({ prompt_optimize_enabled: v })}
            label="Enable the prompt-optimization hotkey"
            hint="It infers what you want (a structured prompt, a cleanup, or both) and never loses your original context."
          />
          {settings.prompt_optimize_enabled && (
            <label>
              <span className="mb-1 block text-xs font-medium">Shortcut</span>
              <HotkeyCapture
                value={settings.prompt_optimize_hotkey}
                onChange={(combo) => onSaveSettings({ prompt_optimize_hotkey: combo })}
              />
            </label>
          )}
          <OptimizeModelRow />
        </div>
      </Section>

      <Section title="Application">
        <div className="flex flex-col gap-3">
          <Toggle
            checked={settings.start_at_login}
            onChange={(v) => onSaveSettings({ start_at_login: v })}
            label="Start when I sign in"
          />
          <Toggle
            checked={settings.sound_cues}
            onChange={(v) => onSaveSettings({ sound_cues: v })}
            label="Play a sound when dictation starts and stops"
          />
          <label className="flex items-center justify-between gap-4">
            <span className="text-sm">Appearance</span>
            <Select
              aria-label="Appearance"
              className="w-36"
              value={settings.theme}
              onChange={(v) => onSaveSettings({ theme: v as Settings['theme'] })}
              options={[
                { value: 'system', label: 'Match system' },
                { value: 'light', label: 'Light' },
                { value: 'dark', label: 'Dark' },
                { value: 'glass', label: 'Glass' },
              ]}
            />
          </label>
          {onReplayTour && (
            <label className="flex items-center justify-between gap-4">
              <span className="text-sm">Walkthrough</span>
              <button onClick={onReplayTour} className="btn-secondary btn-sm">
                Replay walkthrough
              </button>
            </label>
          )}
        </div>
      </Section>

      <StorageSection onChanged={refresh} />

      <div className="panel p-4">
        <h2 className="text-sm font-semibold">Your voice stays here</h2>
        <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
          <li>Audio is transcribed on this device and never uploaded.</li>
          <li>Cleanup is plain local text processing, not a cloud service.</li>
          <li>The only network request AuraScribe makes is downloading a model.</li>
          <li>No telemetry, no analytics, no account.</li>
        </ul>
      </div>
    </div>
  )
}
