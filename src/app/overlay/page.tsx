'use client'

import { useEffect, useState } from 'react'
import { Mic, Loader2 } from 'lucide-react'
import { overlayReady } from '@/lib/ipc'
import { listen } from '@tauri-apps/api/event'

interface Status {
  isRecording: boolean
  isProcessing: boolean
}

export default function OverlayPage() {
  const [status, setStatus] = useState<Status>({ isRecording: false, isProcessing: false })

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

  const listening = status.isRecording
  const label = listening ? 'Listening…' : status.isProcessing ? 'Processing…' : ''

  if (!listening && !status.isProcessing) {
    return null
  }

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-transparent">
      <div className="flex items-center gap-2.5 rounded-full bg-black/85 px-4 py-2.5 shadow-2xl">
        <span
          className={`flex h-6 w-6 items-center justify-center rounded-full ${
            listening ? 'bg-red-500' : 'bg-yellow-500'
          }`}
        >
          {listening ? (
            <Mic className="h-3.5 w-3.5 text-white" />
          ) : (
            <Loader2 className="h-3.5 w-3.5 animate-spin text-white" />
          )}
        </span>
        <span className="text-sm font-medium text-white">{label}</span>
      </div>
    </div>
  )
}
