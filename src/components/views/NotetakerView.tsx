'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { Circle, Square, Trash2, FileText, Sparkles, Pencil } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { Meeting, MeetingDetail, Summary } from '@/lib/ipc'
import { PageHeader, EmptyState, ErrorNote } from '@/components/ui'

/** mm:ss from milliseconds (machine-reported value → monospace at the call site). */
function fmtDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000))
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${m}:${s.toString().padStart(2, '0')}`
}

/** Time of day, e.g. "4:15 PM" — the per-row timestamp (Wispr-style list). */
function fmtTime(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleTimeString(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  })
}

/** Section label for a meeting's day: "Today" / "Yesterday" / "Tue, Oct 13". */
function dayLabel(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000)
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime()
  const diffDays = Math.round((startOf(new Date()) - startOf(d)) / 86_400_000)
  if (diffDays === 0) return 'Today'
  if (diffDays === 1) return 'Yesterday'
  return d.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })
}

/** Group an already-sorted (newest-first) meeting list into consecutive day sections. */
function groupByDay(meetings: Meeting[]): { label: string; items: Meeting[] }[] {
  const groups: { label: string; items: Meeting[] }[] = []
  for (const m of meetings) {
    const label = dayLabel(m.started_at)
    const last = groups[groups.length - 1]
    if (last && last.label === label) last.items.push(m)
    else groups.push({ label, items: [m] })
  }
  return groups
}

export function NotetakerView() {
  const [recording, setRecording] = useState(false)
  const [elapsedMs, setElapsedMs] = useState(0)
  const [meetings, setMeetings] = useState<Meeting[]>([])
  const [selected, setSelected] = useState<MeetingDetail | null>(null)
  const [summary, setSummary] = useState<Summary | null>(null)
  const [summarizing, setSummarizing] = useState(false)
  const [editingTitle, setEditingTitle] = useState(false)
  const [titleDraft, setTitleDraft] = useState('')
  const [notesDraft, setNotesDraft] = useState('')
  const [editingSeg, setEditingSeg] = useState<number | null>(null)
  const [segDraft, setSegDraft] = useState('')
  const notesTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
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
    setSummary(null)
    setEditingTitle(false)
    setEditingSeg(null)
    try {
      const detail = await ipc.getMeeting(id)
      setSelected(detail)
      setNotesDraft(detail.meeting.notes ?? '')
    } catch (e) {
      setError(String(e))
    }
  }

  const saveTitle = async () => {
    if (!selected) return
    const title = titleDraft.trim()
    setEditingTitle(false)
    if (!title || title === selected.meeting.title) return
    try {
      await ipc.renameMeeting(selected.meeting.id, title)
      setSelected({ ...selected, meeting: { ...selected.meeting, title } })
      await loadMeetings()
    } catch (e) {
      setError(String(e))
    }
  }

  // Debounced autosave for the notes field, so typing doesn't hit the DB on every keystroke.
  const onNotesChange = (value: string) => {
    setNotesDraft(value)
    if (!selected) return
    const id = selected.meeting.id
    if (notesTimer.current) clearTimeout(notesTimer.current)
    notesTimer.current = setTimeout(() => {
      ipc.saveMeetingNotes(id, value).catch((e) => setError(String(e)))
    }, 600)
  }

  const saveSegment = async (segmentId: number) => {
    if (!selected) return
    const text = segDraft.trim()
    setEditingSeg(null)
    const seg = selected.segments.find((s) => s.id === segmentId)
    if (!seg || text === seg.text) return
    try {
      if (text === '') {
        await ipc.deleteMeetingSegment(segmentId)
        setSelected({ ...selected, segments: selected.segments.filter((s) => s.id !== segmentId) })
      } else {
        await ipc.updateMeetingSegment(segmentId, text)
        setSelected({
          ...selected,
          segments: selected.segments.map((s) => (s.id === segmentId ? { ...s, text } : s)),
        })
      }
    } catch (e) {
      setError(String(e))
    }
  }

  const summarize = async () => {
    if (!selected) return
    setError(null)
    setSummarizing(true)
    try {
      setSummary(await ipc.summarizeMeeting(selected.meeting.id))
    } catch (e) {
      setError(String(e))
    } finally {
      setSummarizing(false)
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
            <button
              onClick={stop}
              disabled={busy}
              className="btn gap-2 text-white hover:opacity-90 disabled:opacity-50"
              style={{ background: 'hsl(var(--record))' }}
            >
              <Square className="h-3.5 w-3.5" fill="currentColor" />
              Stop recording
            </button>
          ) : (
            <button onClick={start} disabled={busy} className="btn-primary gap-2 disabled:opacity-50">
              {/* Red dot = the record affordance; colour means state (DESIGN.md). */}
              <span className="h-2.5 w-2.5 rounded-full bg-[hsl(var(--record))]" aria-hidden />
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

      {/* Past meetings — grouped by day, newest first (the Notetaker library). */}
      {meetings.length === 0 ? (
        <EmptyState
          title="No meetings yet"
          hint="Press Start recording before (or during) a call to capture a local transcript."
        />
      ) : (
        <div className="flex flex-col gap-6">
          {groupByDay(meetings).map((group) => (
            <section key={group.label}>
              <h3 className="mb-2 px-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground/70">
                {group.label}
              </h3>
              <div className="flex flex-col">
                {group.items.map((m) => {
                  const isOpen = selected?.meeting.id === m.id
                  return (
                    <div
                      key={m.id}
                      className={`group/row flex items-center gap-3 rounded-xl px-2.5 py-2 transition-colors ${
                        isOpen
                          ? 'bg-muted ring-1 ring-inset ring-[hsl(var(--primary)/0.35)]'
                          : 'hover:bg-muted/70'
                      }`}
                    >
                      <button
                        onClick={() => open(m.id)}
                        className="flex min-w-0 flex-1 items-center gap-3 text-left"
                      >
                        <span className="grid h-9 w-9 shrink-0 place-items-center rounded-lg border bg-card text-muted-foreground">
                          <FileText className="h-[18px] w-[18px]" />
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-[14px] font-medium leading-tight text-foreground">
                            {m.title}
                          </span>
                          <span className="mt-0.5 block truncate text-[12px] text-muted-foreground">
                            <span className="mono tabular-nums">{fmtTime(m.started_at)}</span>
                            <span className="text-muted-foreground/40"> · </span>
                            <span className="mono tabular-nums">{fmtDuration(m.duration_ms)}</span>
                            <span className="text-muted-foreground/40"> · </span>
                            <span className="mono tabular-nums">{m.word_count}</span> words
                          </span>
                        </span>
                      </button>
                      <button
                        onClick={() => remove(m.id)}
                        className="btn-ghost btn-sm shrink-0 opacity-0 transition-opacity focus-visible:opacity-100 group-hover/row:opacity-100"
                        aria-label={`Delete ${m.title}`}
                        title="Delete meeting"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  )
                })}
              </div>
            </section>
          ))}
        </div>
      )}

      {/* Transcript of the selected meeting */}
      {selected && (
        <div className="mt-4 rounded-[14px] border bg-card p-[18px]">
          <div className="mb-3 flex items-center justify-between gap-3">
            {editingTitle ? (
              <input
                autoFocus
                className="input min-w-0 flex-1 text-sm"
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
                  setTitleDraft(selected.meeting.title)
                  setEditingTitle(true)
                }}
                className="group flex min-w-0 items-center gap-1.5 text-left"
                title="Rename meeting"
              >
                <h3 className="min-w-0 truncate text-sm font-medium">{selected.meeting.title}</h3>
                <Pencil className="h-3 w-3 shrink-0 text-muted-foreground/0 transition-colors group-hover:text-muted-foreground" />
              </button>
            )}
            <div className="flex shrink-0 items-center gap-2">
              {selected.segments.length > 0 && (
                <button
                  onClick={summarize}
                  disabled={summarizing}
                  className="btn-secondary btn-sm gap-1.5"
                >
                  <Sparkles className="h-3 w-3" />
                  {summarizing ? 'Summarizing…' : 'Summarize'}
                </button>
              )}
              <button onClick={() => setSelected(null)} className="btn-ghost btn-sm">
                Close
              </button>
            </div>
          </div>

          {/* My notes — free text, saved locally as you type. */}
          <div className="mb-4">
            <label className="mb-1.5 block text-xs font-medium text-muted-foreground">My notes</label>
            <textarea
              className="input min-h-[72px] resize-y text-[13px] leading-relaxed"
              placeholder="Type your own notes, action items, or follow-ups here…"
              value={notesDraft}
              onChange={(e) => onNotesChange(e.target.value)}
            />
          </div>

          {/* Local extractive summary (no cloud, no model) — shown once generated. */}
          {summary && (
            <div className="mb-4 rounded-[12px] border bg-secondary/40 p-4">
              <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                <Sparkles className="h-3 w-3" />
                Summary
                <span className="font-normal text-muted-foreground/60">· on-device, extractive</span>
              </div>
              {summary.key_points.length === 0 ? (
                <p className="text-[13px] text-muted-foreground">Not enough speech to summarize.</p>
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
                      className="rounded-full border bg-card px-2 py-0.5 text-[11px] text-muted-foreground"
                    >
                      {k}
                    </span>
                  ))}
                </div>
              )}
            </div>
          )}

          <label className="mb-1.5 block text-xs font-medium text-muted-foreground">
            Transcript
            <span className="ml-1.5 font-normal text-muted-foreground/60">· click a line to edit</span>
          </label>
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
                  {editingSeg === seg.id ? (
                    <textarea
                      autoFocus
                      className="input min-h-[52px] flex-1 resize-y text-[13px] leading-relaxed"
                      value={segDraft}
                      onChange={(e) => setSegDraft(e.target.value)}
                      onBlur={() => saveSegment(seg.id)}
                      onKeyDown={(e) => {
                        if (e.key === 'Escape') setEditingSeg(null)
                      }}
                    />
                  ) : (
                    <p
                      onClick={() => {
                        setSegDraft(seg.text)
                        setEditingSeg(seg.id)
                      }}
                      className="flex-1 cursor-text rounded px-1 py-0.5 text-[13px] leading-relaxed hover:bg-secondary/50"
                      title="Click to edit (clear the text to delete this line)"
                    >
                      {seg.text}
                    </p>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
