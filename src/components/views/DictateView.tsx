'use client'

import { Copy, Check, Download } from 'lucide-react'
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

  const label = status.is_recording
    ? 'Listening'
    : status.is_processing
    ? 'Processing'
    : 'Ready'

  const copyLast = async () => {
    if (!status.current_text) return
    await navigator.clipboard.writeText(status.current_text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1400)
  }

  if (!status.is_model_loaded) {
    return (
      <div className="mx-auto max-w-md pt-12 text-center">
        <h1 className="font-display text-[28px] font-medium">Add a voice model to begin</h1>
        <p className="mt-3 text-sm text-muted-foreground">
          AuraScribe transcribes on this machine, so it needs a speech model installed first.
          You download it once — after that it works offline, forever.
        </p>
        <button
          onClick={onGoToSettings}
          data-tour="download-model"
          className="btn-primary mx-auto mt-6"
        >
          <Download className="h-4 w-4" />
          Choose a model
        </button>
      </div>
    )
  }

  return (
    <div>
      <h1 className="mb-6 font-display text-[32px] font-medium tracking-tight">
        Ready when you are.
      </h1>

      {/* The dictation stage — frosted indigo glass. */}
      <div className="panel-accent flex flex-col items-center gap-[22px] px-7 py-9">
        <SignalMeter state={meterState} />

        <button
          onClick={onToggleRecording}
          disabled={status.is_processing}
          data-tour="record"
          className={`flex h-[68px] w-[68px] items-center justify-center rounded-full transition-colors disabled:opacity-50 ${
            status.is_recording
              ? 'bg-[hsl(var(--record))] hover:opacity-90'
              : 'bg-primary hover:brightness-95'
          }`}
          aria-label={status.is_recording ? 'Stop dictation' : 'Start dictation'}
        >
          {status.is_recording ? (
            <span className="h-4 w-4 rounded-[3px] bg-white" />
          ) : (
            <span className="h-[21px] w-[14px] rounded-[7px] bg-white" />
          )}
        </button>

        <div className="text-center">
          <p className="mb-2.5 whitespace-nowrap text-[13.5px] text-muted-foreground">
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
          <p
            className="mono text-[10.5px] uppercase tracking-[0.05em]"
            style={{ color: 'hsl(var(--faint))' }}
          >
            {(status.loaded_model ?? settings.whisper_model).toUpperCase()} · {label}
          </p>
        </div>
      </div>

      <div className="mt-3.5 flex gap-2.5">
        {(['toggle', 'press-hold'] as const).map((mode) => {
          const active = settings.hotkey_mode === mode
          return (
            <button
              key={mode}
              onClick={() => onSaveSettings({ hotkey_mode: mode })}
              className={`flex-1 rounded-xl border px-4 py-3 text-left transition-colors ${
                active
                  ? 'border-primary/50 bg-primary/10 text-foreground'
                  : 'hover:bg-primary/5'
              }`}
            >
              <div className="text-[13px] font-semibold">{mode === 'toggle' ? 'Tap' : 'Hold'}</div>
              <div className="mt-0.5 text-[12px] text-muted-foreground">
                {mode === 'toggle' ? 'Press to start and stop' : 'Hold while speaking'}
              </div>
            </button>
          )
        })}
      </div>

      {status.last_error && (
        <div className="mt-3.5">
          <ErrorNote>{status.last_error}</ErrorNote>
        </div>
      )}

      {status.current_text && (
        <div className="fade-up mt-3.5 rounded-[14px] border bg-card p-5">
          <div className="mb-2.5 flex items-center justify-between">
            <span className="eyebrow">Last insert</span>
            <button
              onClick={copyLast}
              className="flex items-center gap-1 text-[12.5px] text-primary hover:brightness-95"
            >
              {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
              {copied ? 'Copied' : 'Copy'}
            </button>
          </div>
          <p className="whitespace-pre-wrap text-[14.5px] leading-relaxed">
            {status.current_text}
          </p>
        </div>
      )}
    </div>
  )
}
