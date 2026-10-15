'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { Square, Pencil, Sparkles, Minus } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { MeetingSegment, Summary } from '@/lib/ipc'
import { getCurrentWindow } from '@tauri-apps/api/window'

type Tab = 'notes' | 'transcript' | 'summary'

function fmtClock(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000))
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${m}:${s.toString().padStart(2, '0')}`
}

/**
 * The floating Notetaker panel — a separate always-on-top window shown while a meeting records.
 * Tabs: My thoughts (notes) · Transcript (live) · Summary. Mirrors the in-app Notetaker but as a
 * compact companion window, in AuraScribe's glass theme. All local — no cloud.
 */
export default function PanelPage() {
  const [tab, setTab] = useState<Tab>('transcript')
  const [meetingId, setMeetingId] = useState<number | null>(null)
  const [elapsed, setElapsed] = useState(0)
  const [recording, setRecording] = useState(false)
  const [title, setTitle] = useState('Untitled meeting')
  const [editingTitle, setEditingTitle] = useState(false)
  const [titleDraft, setTitleDraft] = useState('')
  const [segments, setSegments] = useState<MeetingSegment[]>([])
  const [live, setLive] = useState<{ offset_ms: number; text: string }[]>([])
  const [notes, setNotes] = useState('')
  const [summary, setSummary] = useState<Summary | null>(null)
  const [summarizing, setSummarizing] = useState(false)
  const notesTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const liveEndRef = useRef<HTMLDivElement | null>(null)
  const loadedFor = useRef<number | null>(null)

  // Glass theme, transparent window background.
  useEffect(() => {
    const root = document.documentElement
    root.classList.add('glass-bg', 'dark')
    root.style.background = 'transparent'
    document.body.style.background = 'transparent'
    return () => root.classList.remove('glass-bg', 'dark')
  }, [])

  const refreshStatus = useCallback(async () => {
    try {
      const s = await ipc.meetingStatus()
      setRecording(s.recording)
      setElapsed(s.elapsed_ms)
      setMeetingId(s.meeting_id)
    } catch {
      /* ignore */
    }
  }, [])

  // Load the meeting's existing title/notes/segments once we know its id.
  useEffect(() => {
    if (meetingId == null || loadedFor.current === meetingId) return
    loadedFor.current = meetingId
    ipc
      .getMeeting(meetingId)
      .then((d) => {
        setTitle(d.meeting.title)
        setNotes(d.meeting.notes ?? '')
        setSegments(d.segments)
      })
      .catch(() => {})
  }, [meetingId])

  useEffect(() => {
    refreshStatus()
    const poll = setInterval(refreshStatus, 1000)
    const unlistens: Array<() => void> = []
    ipc
      .onMeetingSegment((p) => setLive((prev) => [...prev, { offset_ms: p.offset_ms, text: p.text }]))
      .then((u) => unlistens.push(u))
      .catch(() => {})
    // Tell the backend the panel is ready (shows it if a meeting is already recording).
    ipc.notetakerReady().catch(() => {})
    return () => {
      clearInterval(poll)
      unlistens.forEach((u) => u())
    }
  }, [refreshStatus])

  useEffect(() => {
    liveEndRef.current?.scrollIntoView({ block: 'end' })
  }, [live, tab])

  const onNotesChange = (value: string) => {
    setNotes(value)
    if (meetingId == null) return
    const id = meetingId
    if (notesTimer.current) clearTimeout(notesTimer.current)
    notesTimer.current = setTimeout(() => {
      ipc.saveMeetingNotes(id, value).catch(() => {})
    }, 500)
  }

  const saveTitle = async () => {
    const t = titleDraft.trim()
    setEditingTitle(false)
    if (!t || t === title || meetingId == null) return
    setTitle(t)
    try {
      await ipc.renameMeeting(meetingId, t)
    } catch {
      /* ignore */
    }
  }

  const summarize = async () => {
    if (meetingId == null) return
    setSummarizing(true)
    try {
      setSummary(await ipc.summarizeMeeting(meetingId))
    } catch {
      /* ignore */
    } finally {
      setSummarizing(false)
    }
  }

  const stop = async () => {
    try {
      await ipc.stopMeeting()
    } catch {
      /* ignore */
    }
  }

  const minimize = () => {
    getCurrentWindow().minimize().catch(() => {})
  }

  // Merge persisted segments + live ones for the transcript tab.
  const allLines = [
    ...segments.map((s) => ({ offset_ms: s.offset_ms, text: s.text })),
    ...live,
  ]

  const tabs: { id: Tab; label: string }[] = [
    { id: 'notes', label: 'My thoughts' },
    { id: 'transcript', label: 'Transcript' },
    { id: 'summary', label: 'Summary' },
  ]

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden rounded-[14px] border border-white/10 bg-card/95 text-foreground backdrop-blur-xl">
      {/* Draggable header */}
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 border-b border-white/10 px-3.5 py-2.5"
      >
        <span
          className={`h-2 w-2 shrink-0 rounded-full ${recording ? 'bg-[hsl(var(--record))]' : 'bg-muted-foreground/40'}`}
          aria-hidden
        />
        {editingTitle ? (
          <input
            autoFocus
            className="input h-7 min-w-0 flex-1 py-1 text-[13px]"
            value={titleDraft}
            onChange={(e) => setTitleDraft(e.target.value)}
            onBlur={saveTitle}
            onKeyDown={(e) => {
              if (e.key === 'Enter') saveTitle()
              if (e.key === 'Escape') setEditingTitle(false)
            }}
          />
        ) : (
          <button
            onClick={() => {
              setTitleDraft(title)
              setEditingTitle(true)
            }}
            className="group flex min-w-0 items-center gap-1.5 text-left"
            title="Rename meeting"
          >
            <span className="min-w-0 truncate text-[13px] font-medium">{title}</span>
            <Pencil className="h-3 w-3 shrink-0 text-muted-foreground/0 transition-colors group-hover:text-muted-foreground" />
          </button>
        )}
        <span className="mono ml-auto shrink-0 text-xs text-muted-foreground">{fmtClock(elapsed)}</span>
        <button onClick={minimize} className="btn-ghost btn-sm shrink-0 px-1.5" aria-label="Minimize">
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={stop}
          className="btn btn-sm shrink-0 gap-1.5 text-white hover:opacity-90"
          style={{ background: 'hsl(var(--record))' }}
        >
          <Square className="h-3 w-3" fill="currentColor" />
          Stop
        </button>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 border-b border-white/10 px-2.5 pt-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`rounded-t-md px-3 py-1.5 text-[12px] font-medium transition-colors ${
              tab === t.id
                ? 'border-b-2 border-[hsl(var(--primary))] text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="min-h-0 flex-1 overflow-y-auto p-3.5">
        {tab === 'notes' && (
          <textarea
            className="input min-h-full w-full resize-none text-[13px] leading-relaxed"
            placeholder="Write your notes here. A summary is generated on the Summary tab."
            value={notes}
            onChange={(e) => onNotesChange(e.target.value)}
          />
        )}

        {tab === 'transcript' && (
          <div className="flex flex-col gap-2">
            {allLines.length === 0 ? (
              <p className="text-[13px] text-muted-foreground">
                Listening… transcribed text will appear here as the meeting goes on.
              </p>
            ) : (
              allLines.map((seg, i) => (
                <div key={i} className="flex gap-2.5">
                  <span className="mono shrink-0 pt-0.5 text-[11px] text-muted-foreground/70">
                    {fmtClock(seg.offset_ms)}
                  </span>
                  <p className="text-[13px] leading-relaxed">{seg.text}</p>
                </div>
              ))
            )}
            <div ref={liveEndRef} />
          </div>
        )}

        {tab === 'summary' && (
          <div>
            <button onClick={summarize} disabled={summarizing} className="btn-secondary btn-sm mb-3 gap-1.5">
              <Sparkles className="h-3 w-3" />
              {summarizing ? 'Summarizing…' : 'Summarize so far'}
            </button>
            {summary ? (
              <div className="rounded-[12px] border border-white/10 bg-secondary/40 p-3.5">
                <div className="mb-2 text-[11px] font-medium text-muted-foreground">
                  On-device summary
                </div>
                {summary.key_points.length === 0 ? (
                  <p className="text-[13px] text-muted-foreground">Not enough speech yet.</p>
                ) : (
                  <ul className="flex list-disc flex-col gap-1.5 pl-5">
                    {summary.key_points.map((p, i) => (
                      <li key={i} className="text-[13px] leading-relaxed">
                        {p}
                      </li>
                    ))}
                  </ul>
                )}
                {summary.keywords.length > 0 && (
                  <div className="mt-3 flex flex-wrap gap-1.5">
                    {summary.keywords.map((k) => (
                      <span
                        key={k}
                        className="rounded-full border border-white/10 bg-card px-2 py-0.5 text-[11px] text-muted-foreground"
                      >
                        {k}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <p className="text-[13px] text-muted-foreground">
                Generate a local summary of what's been said so far — no cloud, no AI upload.
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
