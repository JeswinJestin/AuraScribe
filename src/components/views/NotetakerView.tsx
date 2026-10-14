'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { Circle, Square, Trash2, FileText } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { Meeting, MeetingDetail } from '@/lib/ipc'
import { PageHeader, EmptyState, ErrorNote } from '@/components/ui'

/** mm:ss from milliseconds (machine-reported value → monospace at the call site). */
function fmtDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000))
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${m}:${s.toString().padStart(2, '0')}`
}

function fmtDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  })
}

export function NotetakerView() {
  const [recording, setRecording] = useState(false)
  const [elapsedMs, setElapsedMs] = useState(0)
  const [meetings, setMeetings] = useState<Meeting[]>([])
  const [selected, setSelected] = useState<MeetingDetail | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [live, setLive] = useState<{ offset_ms: number; text: string }[]>([])
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const liveEndRef = useRef<HTMLDivElement | null>(null)

  const loadMeetings = useCallback(async () => {
    try {
      setMeetings(await ipc.listMeetings())
    } catch (e) {
      setError(String(e))
    }
  }, [])

  // Poll authoritative status (events are the fast path, not the only path — the app's rule).
  const refreshStatus = useCallback(async () => {
    try {
      const s = await ipc.meetingStatus()
      setRecording(s.recording)
      setElapsedMs(s.elapsed_ms)
    } catch {
      /* ignore transient poll errors */
    }
  }, [])

  useEffect(() => {
    loadMeetings()
    refreshStatus()
    pollRef.current = setInterval(refreshStatus, 1000)

    // Live transcript: append each segment as it is transcribed during capture.
    const unlistens: Array<() => void> = []
    ipc
      .onMeetingSegment((p) => {
        setLive((prev) => [...prev, { offset_ms: p.offset_ms, text: p.text }])
      })
      .then((u) => unlistens.push(u))
      .catch(() => {}) // event system unavailable (e.g. preview harness) — polling still works
    ipc
      .onMeetingFinished(() => {
        // Title may have been auto-generated; refresh the list to show it.
        loadMeetings()
      })
      .then((u) => unlistens.push(u))
      .catch(() => {})

    return () => {
      if (pollRef.current) clearInterval(pollRef.current)
      unlistens.forEach((u) => u())
    }
  }, [loadMeetings, refreshStatus])

  // Keep the live transcript scrolled to the newest line.
  useEffect(() => {
    liveEndRef.current?.scrollIntoView({ block: 'end' })
  }, [live])

  const start = async () => {
    setError(null)
    setBusy(true)
    setLive([])
    setSelected(null)
    try {
      await ipc.startMeeting()
      await refreshStatus()
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  const stop = async () => {
    setError(null)
    setBusy(true)
    try {
      await ipc.stopMeeting()
      await refreshStatus()
      await loadMeetings()
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  const open = async (id: number) => {
    setError(null)
    try {
      setSelected(await ipc.getMeeting(id))
    } catch (e) {
      setError(String(e))
    }
  }

  const remove = async (id: number) => {
    try {
      await ipc.deleteMeeting(id)
      if (selected?.meeting.id === id) setSelected(null)
      await loadMeetings()
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <div>
      <PageHeader
        title="Notetaker"
        description="Record a meeting and get a transcript — all on your device. Captures what you hear in the call plus your mic. No cloud, no account."
      />

      {/* Record control */}
      <div className="mb-3.5 rounded-[14px] border bg-card p-[18px]">
        <div className="flex items-center gap-4">
          {recording ? (
            <button onClick={stop} disabled={busy} className="btn-ink">
              <Square className="h-4 w-4" />
              Stop
            </button>
          ) : (
            <button onClick={start} disabled={busy} className="btn-ink">
              <Circle className="h-4 w-4" />
              Start recording
            </button>
          )}
          {recording && (
            <div className="flex items-center gap-2">
              {/* Colour = state (DESIGN.md). Solid record hue; no motion here. */}
              <span className="h-2.5 w-2.5 rounded-full bg-[hsl(var(--record))]" aria-hidden />
              <span className="text-sm text-muted-foreground">Recording</span>
              <span className="mono text-sm">{fmtDuration(elapsedMs)}</span>
            </div>
          )}
        </div>
        <p className="mt-3 text-[12px] leading-relaxed text-muted-foreground">
          System-audio capture is Windows-only for now. Long, multi-speaker meetings transcribe less
          accurately than short dictation — this is a local model, not a cloud service.
        </p>
      </div>

      {/* Live transcript — grows as the meeting is transcribed (every few seconds). */}
      {recording && (
        <div className="mb-3.5 rounded-[14px] border bg-card p-[18px]">
          <div className="mb-3 flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-[hsl(var(--record))]" aria-hidden />
            <h3 className="text-sm font-medium">Live transcript</h3>
          </div>
          {live.length === 0 ? (
            <p className="text-[13px] text-muted-foreground">
              Listening… transcribed text will appear here as you speak.
            </p>
          ) : (
            <div className="flex max-h-72 flex-col gap-2 overflow-y-auto">
              {live.map((seg, i) => (
                <div key={i} className="flex gap-3">
                  <span className="mono shrink-0 pt-0.5 text-xs text-muted-foreground/70">
                    {fmtDuration(seg.offset_ms)}
                  </span>
                  <p className="text-[13px] leading-relaxed">{seg.text}</p>
                </div>
              ))}
              <div ref={liveEndRef} />
            </div>
          )}
        </div>
      )}

      {error && (
        <div className="mb-3">
          <ErrorNote>{error}</ErrorNote>
        </div>
      )}

      {/* Past meetings */}
      {meetings.length === 0 ? (
        <EmptyState
          title="No meetings yet"
          hint="Press Start recording before (or during) a call to capture a local transcript."
        />
      ) : (
        <div className="flex flex-col gap-1.5">
          {meetings.map((m) => (
            <div key={m.id} className="panel flex items-center gap-3 px-3 py-2">
              <button onClick={() => open(m.id)} className="flex min-w-0 flex-1 items-center gap-3 text-left">
                <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
                <span className="truncate text-sm">{m.title}</span>
                <span className="mono ml-auto shrink-0 text-xs text-muted-foreground">
                  {fmtDate(m.started_at)} · {fmtDuration(m.duration_ms)} · {m.word_count}w
                </span>
              </button>
              <button
                onClick={() => remove(m.id)}
                className="btn-ghost btn-sm shrink-0"
                aria-label={`Delete ${m.title}`}
              >
                <Trash2 className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Transcript of the selected meeting */}
      {selected && (
        <div className="mt-4 rounded-[14px] border bg-card p-[18px]">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-sm font-medium">{selected.meeting.title}</h3>
            <button onClick={() => setSelected(null)} className="btn-ghost btn-sm">
              Close
            </button>
          </div>
          {selected.segments.length === 0 ? (
            <p className="text-[13px] text-muted-foreground">
              No speech was transcribed for this meeting.
            </p>
          ) : (
            <div className="flex flex-col gap-2">
              {selected.segments.map((seg) => (
                <div key={seg.id} className="flex gap-3">
                  <span className="mono shrink-0 pt-0.5 text-xs text-muted-foreground/70">
                    {fmtDuration(seg.offset_ms)}
                  </span>
                  <p className="text-[13px] leading-relaxed">{seg.text}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
