'use client'

import { useCallback, useEffect, useState } from 'react'
import { Download, Loader2, Check, Trash2, Globe, AlertTriangle } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { ModelInfo, Settings, Status } from '@/lib/ipc'
import { PageHeader, Section, ErrorNote, Toggle } from '@/components/ui'

function HotkeyCapture({
  value,
  onChange,
}: {
  value: string
  onChange: (combo: string) => void
}) {
  const [capturing, setCapturing] = useState(false)

  useEffect(() => {
    if (!capturing) return
    const mods = new Set<string>()

    const handler = (e: KeyboardEvent) => {
      e.preventDefault()
      if (e.code === 'Escape') {
        setCapturing(false)
        return
      }
      if (/^(Control|Alt|Shift|Meta)/.test(e.code)) {
        if (e.code.startsWith('Control')) mods.add('Ctrl')
        if (e.code.startsWith('Alt')) mods.add('Alt')
        if (e.code.startsWith('Shift')) mods.add('Shift')
        if (e.code.startsWith('Meta')) mods.add('Super')
        return
      }
      onChange([...mods, e.code].join('+'))
      setCapturing(false)
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [capturing, onChange])

  return (
    <button
      onClick={() => setCapturing(true)}
      className={`input mono text-center ${capturing ? 'border-primary' : ''}`}
    >
      {capturing ? 'Press your keys…' : value}
    </button>
  )
}

function speedLabel(speed: number) {
  return ['', 'Fastest', 'Fast', 'Moderate', 'Slow', 'Slowest'][speed] ?? ''
}

export function SettingsView({
  settings,
  status,
  onSaveSettings,
}: {
  settings: Settings
  status: Status
  onSaveSettings: (patch: Partial<Settings>) => void
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
                      <span className="mono text-sm">{m.id}</span>
                      {m.recommended && (
                        <span className="rounded bg-primary/12 px-1.5 py-0.5 text-[10px] font-medium text-primary">
                          Recommended
                        </span>
                      )}
                      {m.multilingual && (
                        <span
                          className="inline-flex items-center gap-1 text-[10px] text-muted-foreground"
                          title="Handles languages beyond English"
                        >
                          <Globe className="h-3 w-3" />
                          multilingual
                        </span>
                      )}
                    </div>
                    <div className="mono mt-1 text-[11px] text-muted-foreground">
                      {m.size_mb >= 1000
                        ? `${(m.size_mb / 1000).toFixed(1)} GB`
                        : `${m.size_mb} MB`}
                      {' · '}
                      {m.realtime_factor <= 1
                        ? `${m.realtime_factor.toFixed(1)}x — faster than you speak`
                        : `${m.realtime_factor.toFixed(1)}x the length of what you say`}
                      {' · '}
                      accuracy {m.accuracy}/5
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
            <select
              className="input"
              value={settings.hotkey_mode}
              onChange={(e) =>
                onSaveSettings({ hotkey_mode: e.target.value as Settings['hotkey_mode'] })
              }
            >
              <option value="toggle">Tap to start and stop</option>
              <option value="press-hold">Hold while speaking</option>
            </select>
          </label>
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
            <select
              className="input"
              value={settings.mic_device ?? ''}
              onChange={(e) => onSaveSettings({ mic_device: e.target.value || null })}
            >
              <option value="">System default</option>
              {micDevices.map((d) => (
                <option key={d} value={d}>
                  {d}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span className="mb-1 block text-xs font-medium">Language</span>
            <select
              className="input"
              value={settings.language}
              onChange={(e) => onSaveSettings({ language: e.target.value })}
            >
              <option value="en">English</option>
              <option value="auto">Detect automatically</option>
              <option value="hi">Hindi</option>
              <option value="ml">Malayalam</option>
              <option value="es">Spanish</option>
              <option value="fr">French</option>
              <option value="de">German</option>
              <option value="ja">Japanese</option>
              <option value="zh">Chinese</option>
            </select>
          </label>
        </div>
        <p className="mt-2 text-[11px] text-muted-foreground">
          Languages other than English need a multilingual model.
        </p>
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
            <select
              className="input w-36"
              value={settings.theme}
              onChange={(e) => onSaveSettings({ theme: e.target.value as Settings['theme'] })}
            >
              <option value="system">Match system</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
              <option value="glass">Glass</option>
            </select>
          </label>
        </div>
      </Section>

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
