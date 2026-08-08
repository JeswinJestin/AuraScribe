'use client'

import { useEffect, useState } from 'react'
import { Mic, Loader2, Square } from 'lucide-react'
import { overlayReady, stopRecording } from '@/lib/ipc'
import { listen } from '@tauri-apps/api/event'

// Field names must match the Rust `Status` wire format exactly. They are snake_case;
// this file previously used camelCase, which happened to match an older serde rename.
// When that rename was dropped everywhere else, this was the only place still on it.
interface Status {
  is_recording: boolean
  is_processing: boolean
}

export default function OverlayPage() {
  const [status, setStatus] = useState<Status>({ is_recording: false, is_processing: false })
  const [hover, setHover] = useState(false)
  const [stopping, setStopping] = useState(false)

  useEffect(() => {
    document.documentElement.style.background = 'transparent'
    document.body.style.background = 'transparent'

    let unlisten: (() => void) | null = null
    listen<Status>('status-changed', (e) => setStatus(e.payload)).then((fn) => {
      unlisten = fn
      // Only announce readiness once we can actually receive status, so the backend never
      // shows a window that would render blank.
      overlayReady().catch(() => {})
    })
    return () => {
      unlisten?.()
    }
  }, [])

  const listening = status.is_recording
  // Reset the transient "stopping" state once the backend confirms recording ended.
  useEffect(() => {
    if (!listening) setStopping(false)
  }, [listening])

  if (!listening && !status.is_processing) {
    return null
  }

  // Clicking the pill stops dictation — the same effect as pressing the hotkey again, but with
  // the mouse. Only meaningful while listening; during processing there's nothing to stop.
  const canStop = listening && !stopping
  const onStop = () => {
    if (!canStop) return
    setStopping(true)
    stopRecording().catch(() => setStopping(false))
  }

  const label = stopping
    ? 'Stopping…'
    : listening
      ? hover
        ? 'Stop'
        : 'Listening…'
      : 'Processing…'

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-transparent">
      <button
        type="button"
        onClick={onStop}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        disabled={!canStop}
        title={canStop ? 'Click to stop dictation' : undefined}
        // Fixed pill width so the label changing ("Listening…" → "Stop") never resizes the
        // pill and shifts it under the cursor — that width change was the hover "jitter".
        className={`flex w-[168px] items-center gap-2.5 rounded-full px-4 py-2.5 shadow-2xl transition-colors duration-150 ${
          canStop ? 'cursor-pointer' : 'cursor-default'
        } ${hover && canStop ? 'bg-black/95' : 'bg-black/85'}`}
      >
        <span
          className={`flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full transition-colors ${
            listening ? 'bg-red-500' : 'bg-yellow-500'
          }`}
        >
          {stopping || status.is_processing ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin text-white" />
          ) : hover && canStop ? (
            <Square className="h-3 w-3 fill-white text-white" />
          ) : (
            <Mic className="h-3.5 w-3.5 text-white" />
          )}
        </span>
        {/* Fixed-width, left-aligned label: its content changes but its box does not. */}
        <span className="w-[104px] text-left text-sm font-medium text-white">{label}</span>
      </button>
    </div>
  )
}
