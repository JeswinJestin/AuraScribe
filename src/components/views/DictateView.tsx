'use client'

import { Mic, Square, Copy, Check, Download } from 'lucide-react'
import { useState } from 'react'
import { SignalMeter } from '@/components/SignalMeter'
import { ErrorNote } from '@/components/ui'
import type { Settings, Status } from '@/lib/ipc'

function Hotkey({ combo }: { combo: string }) {
  return (
    <span className="inline-flex items-center gap-1">
      {combo.split('+').map((key) => (
        <kbd key={key} className="kbd">
          {key}
        </kbd>
      ))}
    </span>
  )
}

export function DictateView({
  status,
  settings,
  onToggleRecording,
  onSaveSettings,
  onGoToSettings,
}: {
  status: Status
  settings: Settings
  onToggleRecording: () => void
  onSaveSettings: (patch: Partial<Settings>) => void
  onGoToSettings: () => void
}) {
  const [copied, setCopied] = useState(false)

  const meterState = status.is_recording
    ? 'listening'
    : status.is_processing
    ? 'processing'
    : 'idle'

  const copyLast = async () => {
    if (!status.current_text) return
    await navigator.clipboard.writeText(status.current_text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1400)
  }

  if (!status.is_model_loaded) {
    return (
      <div className="mx-auto max-w-md pt-10 text-center">
        <h1 className="text-lg font-semibold tracking-tight">Add a voice model to begin</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          AuraScribe transcribes on this machine, so it needs a speech model installed first.
          You download it once — after that it works offline, forever.
        </p>
        <button onClick={onGoToSettings} className="btn-primary mt-5">
          <Download className="h-4 w-4" />
          Choose a model
        </button>
      </div>
    )
  }

  return (
    <div className="mx-auto max-w-xl">
      <div className="panel p-5">
        <SignalMeter state={meterState} />

        <div className="mt-5 flex flex-col items-center gap-3">
          <button
            onClick={onToggleRecording}
            disabled={status.is_processing}
            className={`flex h-14 w-14 items-center justify-center rounded-full transition-colors ${
              status.is_recording
                ? 'bg-[hsl(var(--record))] text-white hover:opacity-90'
                : 'bg-primary text-primary-foreground hover:bg-primary/90'
            } disabled:opacity-40`}
            aria-label={status.is_recording ? 'Stop dictation' : 'Start dictation'}
          >
            {status.is_recording ? (
              <Square className="h-5 w-5" fill="currentColor" />
            ) : (
              <Mic className="h-5 w-5" />
            )}
          </button>

          <p className="text-sm text-muted-foreground">
            {status.is_recording ? (
              'Speak, then stop to insert'
            ) : status.is_processing ? (
              'Turning your voice into text'
            ) : (
              <>
                Press <Hotkey combo={settings.hotkey} /> anywhere
              </>
            )}
          </p>

          {status.loaded_model && !status.is_recording && !status.is_processing && (
            <p className="mono text-[10px] uppercase tracking-widest text-muted-foreground">
              {status.loaded_model}
            </p>
          )}
        </div>
      </div>

      <div className="mt-3 flex gap-2">
        {(['toggle', 'press-hold'] as const).map((mode) => (
          <button
            key={mode}
            onClick={() => onSaveSettings({ hotkey_mode: mode })}
            className={`panel flex-1 px-3 py-2 text-left transition-colors ${
              settings.hotkey_mode === mode ? 'border-primary' : 'hover:bg-accent'
            }`}
          >
            <div className="text-xs font-medium">{mode === 'toggle' ? 'Tap' : 'Hold'}</div>
            <div className="mt-0.5 text-[11px] text-muted-foreground">
              {mode === 'toggle' ? 'Press to start and stop' : 'Hold while speaking'}
            </div>
          </button>
        ))}
      </div>

      {status.last_error && (
        <div className="mt-3">
          <ErrorNote>{status.last_error}</ErrorNote>
        </div>
      )}

      {status.current_text && (
        <div className="panel mt-3 p-4 fade-up">
          <div className="flex items-center justify-between">
            <span className="mono text-[10px] uppercase tracking-widest text-muted-foreground">
              last insert
            </span>
            <button onClick={copyLast} className="btn-ghost btn-sm">
              {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
              {copied ? 'Copied' : 'Copy'}
            </button>
          </div>
          <p className="mt-2 whitespace-pre-wrap text-sm leading-relaxed">{status.current_text}</p>
        </div>
      )}
    </div>
  )
}
