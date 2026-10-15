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
 * The floating Notetaker panel — a full-height, right-docked companion window shown while a meeting
 * records (the main app hides while it's up, like Wispr). Tabs: My thoughts / Transcript / Summary,
 * with a persistent bottom Stop bar. AuraScribe glass theme, 100% local.
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
    ipc.notetakerReady().catch(() => {})
    return () => {
      clearInterval(poll)
      unlistens.forEach((u) => u())
    }
  }, [refreshStatus])

  useEffect(() => {
    if (tab === 'transcript') liveEndRef.current?.scrollIntoView({ block: 'end' })
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
    ipc.renameMeeting(meetingId, t).catch(() => {})
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

  const stop = () => {
    ipc.stopMeeting().catch(() => {})
  }
  const minimize = () => {
    getCurrentWindow().minimize().catch(() => {})
  }

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
    <div className="flex h-screen w-screen flex-col overflow-hidden rounded-[16px] border border-white/10 bg-card/90 text-foreground shadow-2xl backdrop-blur-2xl">
      {/* Header (draggable) */}
      <div
        data-tauri-drag-region
        className="flex shrink-0 items-center gap-2 px-4 pb-2 pt-3.5"
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
            <span className="min-w-0 truncate text-sm font-semibold">{title}</span>
            <Pencil className="h-3 w-3 shrink-0 text-muted-foreground/0 transition-colors group-hover:text-muted-foreground" />
          </button>
        )}
        <span className="mono ml-auto shrink-0 text-xs tabular-nums text-muted-foreground">
          {fmtClock(elapsed)}
        </span>
        <button
          onClick={minimize}
          className="btn-ghost shrink-0 rounded-md px-1.5 py-1"
          aria-label="Minimize"
        >
          <Minus className="h-4 w-4" />
        </button>
      </div>

      {/* Tabs */}
      <div className="flex shrink-0 gap-5 border-b border-white/10 px-4">
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`-mb-px border-b-2 pb-2.5 pt-1 text-[13px] font-medium transition-colors ${
              tab === t.id
                ? 'border-[hsl(var(--primary))] text-foreground'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Content (fills) */}
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        {tab === 'notes' && (
          <textarea
            className="input min-h-full w-full resize-none border-0 bg-transparent px-0 py-0 text-[14px] leading-relaxed focus-visible:ring-0"
            placeholder="Write your notes here. A summary is generated on the Summary tab."
            value={notes}
            onChange={(e) => onNotesChange(e.target.value)}
          />
        )}

        {tab === 'transcript' && (
          <div className="flex flex-col gap-2.5">
            {allLines.length === 0 ? (
              <p className="text-[13px] text-muted-foreground">
                Listening… transcribed text will appear here as the meeting goes on.
              </p>
            ) : (
              allLines.map((seg, i) => (
                <div key={i} className="flex gap-2.5">
                  <span className="mono shrink-0 pt-0.5 text-[11px] tabular-nums text-muted-foreground/70">
                    {fmtClock(seg.offset_ms)}
                  </span>
                  <p className="text-[13.5px] leading-relaxed">{seg.text}</p>
                </div>
              ))
            )}
            <div ref={liveEndRef} />
          </div>
        )}

        {tab === 'summary' && (
          <div>
            {summary ? (
              <div className="rounded-[12px] border border-white/10 bg-secondary/30 p-4">
                <div className="mb-2.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                  On-device summary
                </div>
                {summary.key_points.length === 0 ? (
                  <p className="text-[13px] text-muted-foreground">Not enough speech yet.</p>
                ) : (
                  <ul className="flex list-disc flex-col gap-2 pl-5">
                    {summary.key_points.map((p, i) => (
                      <li key={i} className="text-[13.5px] leading-relaxed">
                        {p}
                      </li>
                    ))}
                  </ul>
                )}
                {summary.keywords.length > 0 && (
                  <div className="mt-3.5 flex flex-wrap gap-1.5">
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
              <div className="flex h-full flex-col items-center justify-center gap-2 py-10 text-center">
                <Sparkles className="h-5 w-5 text-muted-foreground/60" />
                <p className="max-w-[240px] text-[13px] text-muted-foreground">
                  Generate a local summary of what's been said so far — no cloud, no AI upload.
                </p>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Bottom action bar */}
      <div className="flex shrink-0 items-center gap-2 border-t border-white/10 px-4 py-3">
        {tab === 'summary' && (
          <button onClick={summarize} disabled={summarizing} className="btn-primary flex-1 gap-1.5">
            <Sparkles className="h-4 w-4" />
            {summarizing ? 'Summarizing…' : summary ? 'Regenerate summary' : 'Generate summary'}
          </button>
        )}
        <button
          onClick={stop}
          className={`btn gap-2 text-white hover:opacity-90 ${tab === 'summary' ? '' : 'flex-1'}`}
          style={{ background: 'hsl(var(--record))' }}
        >
          <Square className="h-3.5 w-3.5" fill="currentColor" />
          Stop & save
        </button>
      </div>
    </div>
  )
}
