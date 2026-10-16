'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import {
  Square,
  Pencil,
  Sparkles,
  Minus,
  Pause,
  Play,
  Clock,
  Lightbulb,
  Maximize2,
  Minimize2,
  X,
} from 'lucide-react'
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

function wordsIn(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length
}

/**
 * The floating Notetaker panel — a full-height, right-docked companion window shown while a meeting
 * records (the main app hides while it's up). It **matches the theme chosen in Settings** (returned
 * by `notetaker_ready`) with a solid, legible surface in every theme. Tabs: My thoughts / Transcript
 * / Summary, with a persistent bottom action bar. All local — no cloud.
 */
export default function PanelPage() {
  const [tab, setTab] = useState<Tab>('transcript')
  const [isGlass, setIsGlass] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const [meetingId, setMeetingId] = useState<number | null>(null)
  const [elapsed, setElapsed] = useState(0)
  const [recording, setRecording] = useState(false)
  const [paused, setPaused] = useState(false)
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

  const applyTheme = useCallback((theme: string) => {
    const root = document.documentElement
    root.classList.toggle('glass-bg', theme === 'glass')
    if (theme === 'dark' || theme === 'glass') root.classList.add('dark')
    else if (theme === 'light') root.classList.remove('dark')
    else root.classList.toggle('dark', window.matchMedia('(prefers-color-scheme: dark)').matches)
    setIsGlass(theme === 'glass')
  }, [])

  const refreshStatus = useCallback(async () => {
    try {
      const s = await ipc.meetingStatus()
      setRecording(s.recording)
      setPaused(s.paused)
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
    // notetaker_ready returns the user's theme — apply it reliably (the in-panel settings fetch
    // was racing/failing, which left the panel stuck on the light surface).
    ipc
      .notetakerReady()
      .then((theme) => applyTheme(theme))
      .catch(() => applyTheme('glass'))
    return () => {
      clearInterval(poll)
      unlistens.forEach((u) => u())
    }
  }, [refreshStatus, applyTheme])

  useEffect(() => {
    if (tab === 'transcript') liveEndRef.current?.scrollIntoView({ block: 'end' })
  }, [live, tab])

  // Each fresh meeting re-docks the window on the Rust side (panel::show → position); keep the
  // expand/restore toggle in sync so its icon doesn't claim "expanded" after a restart.
  const wasRecording = useRef(false)
  useEffect(() => {
    if (recording && !wasRecording.current) setExpanded(false)
    wasRecording.current = recording
  }, [recording])

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

  const togglePause = () => {
    if (paused) {
      setPaused(false)
      ipc.resumeMeeting().catch(() => {})
    } else {
      setPaused(true)
      ipc.pauseMeeting().catch(() => {})
    }
  }
  const stop = () => ipc.stopMeeting().catch(() => {})
  const minimize = () => getCurrentWindow().minimize().catch(() => {})
  const toggleExpand = () => {
    if (expanded) {
      setExpanded(false)
      ipc.panelDock().catch(() => {})
    } else {
      setExpanded(true)
      ipc.panelExpand().catch(() => {})
    }
  }
  const close = () => ipc.panelClose().catch(() => {})

  const allLines = [...segments.map((s) => ({ offset_ms: s.offset_ms, text: s.text })), ...live]
  const wordCount = allLines.reduce((n, l) => n + wordsIn(l.text), 0)
  const readMin = Math.max(1, Math.round(wordCount / 200))

  const tabs: { id: Tab; label: string }[] = [
    { id: 'notes', label: 'My thoughts' },
    { id: 'transcript', label: 'Transcript' },
    { id: 'summary', label: 'Summary' },
  ]

  const surface: React.CSSProperties = isGlass
    ? {
        backgroundImage:
          'linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0) 22%), linear-gradient(165deg, #1a2550 0%, #0c1226 55%, #080c1c 100%)',
      }
    : { background: 'hsl(var(--card))' }

  // Expanded is a large centered window — keep text at a readable measure instead of letting lines
  // run the full width. Docked stays edge-to-edge in its narrow column.
  const col = expanded ? 'mx-auto w-full max-w-[720px]' : ''

  return (
    <div
      className="flex h-screen w-screen flex-col overflow-hidden rounded-[16px] border text-foreground shadow-2xl"
      style={surface}
    >
      {/* Header (draggable) */}
      <div data-tauri-drag-region className={`shrink-0 px-5 pb-3 pt-4 ${col}`}>
        <div className="mb-2 flex items-center gap-2">
          <span
            className={`h-2.5 w-2.5 shrink-0 rounded-full ${
              recording && !paused ? 'animate-pulse bg-[hsl(var(--record))]' : 'bg-muted-foreground/40'
            }`}
            aria-hidden
          />
          <span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
            {paused ? 'Paused' : recording ? 'Recording' : 'Notetaker'}
          </span>
          <span className="mono ml-auto text-[12px] tabular-nums text-muted-foreground">
            {fmtClock(elapsed)}
          </span>
          {/* Window chrome: expand ⇄ restore-to-side, minimize, close. */}
          <div className="-mr-1.5 flex shrink-0 items-center gap-0.5">
            <button
              onClick={toggleExpand}
              className="btn-ghost rounded-md p-1.5 text-muted-foreground hover:text-foreground"
              aria-label={expanded ? 'Dock to side' : 'Expand'}
              title={expanded ? 'Dock to the side' : 'Expand'}
            >
              {expanded ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
            </button>
            <button
              onClick={minimize}
              className="btn-ghost rounded-md p-1.5 text-muted-foreground hover:text-foreground"
              aria-label="Minimize"
              title="Minimize"
            >
              <Minus className="h-4 w-4" />
            </button>
            <button
              onClick={close}
              className="btn-ghost rounded-md p-1.5 text-muted-foreground hover:bg-[hsl(var(--record)/0.14)] hover:text-[hsl(var(--record))]"
              aria-label="Close panel"
              title="Close (keeps recording in the app)"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>
        {editingTitle ? (
          <input
            autoFocus
            className="input h-9 w-full font-display text-[21px] font-medium"
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
            className="group flex w-full items-center gap-2 text-left"
            title="Rename meeting"
          >
            <h1 className="min-w-0 truncate font-display text-[22px] font-medium leading-tight tracking-tight">
              {title}
            </h1>
            <Pencil className="h-3.5 w-3.5 shrink-0 text-muted-foreground/0 transition-colors group-hover:text-muted-foreground" />
          </button>
        )}
      </div>

      {/* Tabs */}
      <div className={`flex shrink-0 gap-6 border-b px-5 ${col}`}>
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`-mb-px border-b-2 pb-2.5 pt-1 text-[13.5px] font-medium transition-colors ${
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
      <div className={`min-h-0 flex-1 overflow-y-auto px-5 py-4 ${expanded ? 'w-full' : ''}`}>
        <div className={col}>
        {tab === 'notes' && (
          <div className="flex h-full flex-col">
            <textarea
              className="min-h-0 flex-1 w-full resize-none border-0 bg-transparent px-0 py-0 text-[14.5px] leading-relaxed text-foreground outline-none placeholder:text-muted-foreground focus:outline-none"
              placeholder="Jot your own notes, questions, or action items while you listen…"
              value={notes}
              onChange={(e) => onNotesChange(e.target.value)}
            />
            <p className="mt-2 shrink-0 text-[11px] text-muted-foreground/70">
              Saved to this device only.
            </p>
          </div>
        )}

        {tab === 'transcript' && (
          <div>
            {allLines.length > 0 && (
              <div className="mb-3 inline-flex items-center gap-1.5 rounded-full border bg-secondary/60 px-2.5 py-1 text-[11px] text-muted-foreground">
                <Clock className="h-3 w-3" />
                <span className="mono tabular-nums">{fmtClock(elapsed)}</span>
                <span className="text-muted-foreground/50">·</span>
                <span className="mono tabular-nums">{wordCount}</span> words
              </div>
            )}
            <div className="flex flex-col gap-3">
              {allLines.length === 0 ? (
                <p className="text-sm leading-relaxed text-muted-foreground">
                  {paused
                    ? 'Paused — transcription stopped. Press Resume to continue.'
                    : 'Listening… transcribed text will appear here as the meeting goes on.'}
                </p>
              ) : (
                allLines.map((seg, i) => (
                  <div key={i} className="flex gap-3">
                    <span className="mono shrink-0 select-none pt-[3px] text-[11px] tabular-nums text-muted-foreground/60">
                      {fmtClock(seg.offset_ms)}
                    </span>
                    <p className="text-[14px] leading-relaxed">{seg.text}</p>
                  </div>
                ))
              )}
              <div ref={liveEndRef} />
            </div>
          </div>
        )}

        {tab === 'summary' && (
          <div>
            <div className="mb-3 flex items-center justify-between gap-3">
              {summary && summary.key_points.length > 0 ? (
                <span className="inline-flex items-center gap-1.5 rounded-full border bg-secondary/60 px-2.5 py-1 text-[11px] text-muted-foreground">
                  <Lightbulb className="h-3 w-3" />~{readMin} min read
                </span>
              ) : (
                <span />
              )}
              <button
                onClick={summarize}
                disabled={summarizing || wordCount === 0}
                className="btn-secondary btn-sm gap-1.5"
              >
                <Sparkles className="h-3.5 w-3.5" />
                {summarizing ? 'Summarizing…' : summary ? 'Regenerate' : 'Generate'}
              </button>
            </div>
            {summary ? (
              summary.key_points.length === 0 ? (
                <p className="text-sm text-muted-foreground">Not enough speech yet to summarize.</p>
              ) : (
                <div>
                  <ul className="flex list-disc flex-col gap-2.5 pl-5 marker:text-muted-foreground/50">
                    {summary.key_points.map((p, i) => (
                      <li key={i} className="pl-1 text-[14px] leading-relaxed">
                        {p}
                      </li>
                    ))}
                  </ul>
                  {summary.keywords.length > 0 && (
                    <div className="mt-4 flex flex-wrap gap-1.5">
                      {summary.keywords.map((k) => (
                        <span
                          key={k}
                          className="rounded-full border px-2.5 py-0.5 text-[11px] text-muted-foreground"
                        >
                          {k}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              )
            ) : (
              <div className="flex flex-col items-center justify-center gap-2.5 py-14 text-center">
                <Lightbulb className="h-5 w-5 text-muted-foreground/50" />
                <p className="max-w-[260px] text-sm leading-relaxed text-muted-foreground">
                  Generate a local summary of what's been said so far — key points and topics, on your
                  device, no cloud.
                </p>
              </div>
            )}
          </div>
        )}
        </div>
      </div>

      {/* Bottom action bar */}
      <div className={`flex shrink-0 items-center gap-2.5 border-t px-5 py-3.5 ${col}`}>
        <button
          onClick={togglePause}
          disabled={!recording}
          className="btn-secondary gap-1.5"
          title={paused ? 'Resume recording' : 'Pause recording'}
        >
          {paused ? <Play className="h-4 w-4" /> : <Pause className="h-4 w-4" />}
          {paused ? 'Resume' : 'Pause'}
        </button>
        <button
          onClick={stop}
          className="btn flex-1 gap-2 text-white hover:brightness-110"
          style={{ background: 'hsl(var(--record))' }}
        >
          <Square className="h-3.5 w-3.5" fill="currentColor" />
          Stop &amp; save
        </button>
      </div>
    </div>
  )
}
