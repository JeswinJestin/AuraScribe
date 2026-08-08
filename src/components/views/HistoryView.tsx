'use client'

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Copy, Check, Trash2, Loader2 } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { TranscriptEntry, DailyCount } from '@/lib/ipc'
import { PageHeader, EmptyState, ErrorNote } from '@/components/ui'

const PAGE_SIZE = 60
const HEATMAP_WEEKS = 53 // a full year of usage, like GitHub's contribution graph
const LEVEL_OPACITY = [0, 0.28, 0.5, 0.72, 1]
const DAY_LABELS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

/** Local `YYYY-MM-DD` — must match the DB's `date(timestamp, 'unixepoch', 'localtime')`. */
function dayKey(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate())
}

/** "Today" / "Yesterday" / "8 August 2026" for a `YYYY-MM-DD` key. */
function dayHeading(key: string): string {
  const today = dayKey(new Date())
  const yesterday = dayKey(new Date(Date.now() - 86_400_000))
  if (key === today) return 'Today'
  if (key === yesterday) return 'Yesterday'
  const [y, m, d] = key.split('-').map(Number)
  return new Date(y, m - 1, d).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  })
}

function timeOfDay(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleTimeString(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  })
}

function level(count: number): number {
  if (count <= 0) return 0
  if (count <= 2) return 1
  if (count <= 5) return 2
  if (count <= 9) return 3
  return 4
}

function cellColor(count: number): string {
  const lvl = level(count)
  return lvl === 0 ? 'hsl(var(--muted))' : `hsl(var(--primary) / ${LEVEL_OPACITY[lvl]})`
}

/**
 * GitHub-contributions-style grid of the last year: one cell per day, intensity = dictation
 * count. Weeks run in columns, Sunday at the top, with month labels on the first week of each
 * month and day-of-week hints on the left. Hovering any cell shows the exact day and count. The
 * grid stretches to fill the whole panel width — no dead space, just like GitHub's own graph.
 */
function UsageHeatmap({ counts }: { counts: DailyCount[] }) {
  const wrapRef = useRef<HTMLDivElement>(null)
  const [cellSize, setCellSize] = useState(10)

  // Measure the container so the cells fill the full panel width (no empty slab), recomputing
  // whenever the panel resizes.
  useEffect(() => {
    const el = wrapRef.current
    if (!el) return
    const update = () => {
      const w = el.clientWidth
      const size = Math.max(8, Math.floor((w - HEATMAP_WEEKS * 3 - 12) / HEATMAP_WEEKS))
      setCellSize(size)
    }
    update()
    const ro = new ResizeObserver(update)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const { weeks, monthMarks } = useMemo(() => {
    const byDay = new Map<string, number>()
    for (const c of counts) byDay.set(c.day, c.count)

    // End on the Saturday of this week so today sits in the last column.
    const end = startOfDay(new Date())
    end.setDate(end.getDate() + (6 - end.getDay()))
    const start = new Date(end)
    start.setDate(start.getDate() - (HEATMAP_WEEKS * 7 - 1))

    const cols: { key: string; date: Date; count: number }[][] = []
    const marks: (string | null)[] = []
    const cur = new Date(start)
    for (let w = 0; w < HEATMAP_WEEKS; w++) {
      const col: { key: string; date: Date; count: number }[] = []
      let label: string | null = null
      for (let d = 0; d < 7; d++) {
        const key = dayKey(cur)
        col.push({ key, date: new Date(cur), count: byDay.get(key) ?? 0 })
        // Label the week if the day before this one was the first day of a month, or if this is
        // the very first week shown.
        if (cur.getDate() <= 7 && label === null) {
          label = cur.toLocaleDateString(undefined, { month: 'short' })
        }
        cur.setDate(cur.getDate() + 1)
      }
      cols.push(col)
      marks.push(label)
    }
    return { weeks: cols, monthMarks: marks }
  }, [counts])

  const today = startOfDay(new Date())
  const gap = 3

  return (
    <div className="panel p-4">
      <div className="mb-2.5 flex items-center justify-between">
        <span className="eyebrow">Activity</span>
        <div className="flex items-center gap-1 text-[10px] text-muted-foreground">
          <span className="mr-0.5">Less</span>
          {LEVEL_OPACITY.map((_, l) => (
            <span
              key={l}
              className="rounded-[2px]"
              style={{
                width: cellSize - 1,
                height: cellSize - 1,
                backgroundColor:
                  l === 0 ? 'hsl(var(--muted))' : `hsl(var(--primary) / ${LEVEL_OPACITY[l]})`,
              }}
            />
          ))}
          <span className="ml-0.5">More</span>
        </div>
      </div>

      <div ref={wrapRef} className="w-full">
        {/* Month labels across the top. */}
        <div className="mb-1 flex" style={{ gap, marginLeft: 15 }}>
          {monthMarks.map((label, wi) => (
            <span
              key={wi}
              className="truncate text-[9px] leading-[10px] text-muted-foreground"
              style={{ width: cellSize, textAlign: 'left' }}
            >
              {label ?? ''}
            </span>
          ))}
        </div>

        <div className="mt-1 flex w-full" style={{ gap }}>
          {/* Day-of-week column on the left, like GitHub. */}
          <div
            className="flex flex-col text-[8px] leading-[9px] text-muted-foreground"
            style={{ gap }}
          >
            <span style={{ width: 12, height: cellSize, lineHeight: `${cellSize}px` }}>Su</span>
            <span style={{ width: 12, height: cellSize }} />
            <span style={{ width: 12, height: cellSize, lineHeight: `${cellSize}px` }}>Tu</span>
            <span style={{ width: 12, height: cellSize }} />
            <span style={{ width: 12, height: cellSize, lineHeight: `${cellSize}px` }}>Th</span>
            <span style={{ width: 12, height: cellSize }} />
            <span style={{ width: 12, height: cellSize, lineHeight: `${cellSize}px` }}>Sa</span>
          </div>

          <div className="flex min-w-0 flex-1" style={{ gap }}>
            {weeks.map((col, wi) => (
              <div key={wi} className="flex min-w-0 flex-1 flex-col" style={{ gap }}>
                {col.map((cell) => {
                  const isFuture = cell.date > today
                  return (
                    <div
                      key={cell.key}
                      title={
                        isFuture
                          ? undefined
                          : `${
                              cell.count > 0 ? cell.count : 'No'
                            } dictation${cell.count === 1 ? '' : 's'} on ${cell.date.toLocaleDateString(
                              undefined,
                              { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' }
                            )}`
                      }
                      className="rounded-[2px]"
                      style={{
                        width: cellSize,
                        height: cellSize,
                        backgroundColor: isFuture ? 'transparent' : cellColor(cell.count),
                      }}
                    />
                  )
                })}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

/** Delete every dictation in a chosen date range. Two-step, so it can't fire by accident. */
function RangeDelete({ onDeleted }: { onDeleted: () => void }) {
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [confirming, setConfirming] = useState(false)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<string | null>(null)

  const valid = from !== '' && to !== '' && from <= to

  const runDelete = async () => {
    setBusy(true)
    setNote(null)
    try {
      const start = Math.floor(new Date(`${from}T00:00:00`).getTime() / 1000)
      const end = Math.floor(new Date(`${to}T23:59:59`).getTime() / 1000)
      const removed = await ipc.deleteTranscriptsBetween(start, end)
      setNote(`Deleted ${removed} ${removed === 1 ? 'entry' : 'entries'}.`)
      setConfirming(false)
      onDeleted()
    } catch (e) {
      setNote(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="panel p-4">
      <span className="eyebrow">Delete a date range</span>
      <div className="mt-2.5 flex flex-wrap items-end gap-3">
        <label className="flex flex-col gap-1">
          <span className="text-[11px] text-muted-foreground">From</span>
          <input
            type="date"
            value={from}
            max={to || undefined}
            onChange={(e) => {
              setFrom(e.target.value)
              setConfirming(false)
            }}
            className="input w-40"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-[11px] text-muted-foreground">To</span>
          <input
            type="date"
            value={to}
            min={from || undefined}
            onChange={(e) => {
              setTo(e.target.value)
              setConfirming(false)
            }}
            className="input w-40"
          />
        </label>

        {confirming ? (
          <div className="flex items-center gap-2">
            <button onClick={runDelete} disabled={busy} className="btn-danger btn-sm">
              {busy ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <Trash2 className="h-3 w-3" />
              )}
              Confirm delete
            </button>
            <button
              onClick={() => setConfirming(false)}
              disabled={busy}
              className="btn-ghost btn-sm"
            >
              Cancel
            </button>
          </div>
        ) : (
          <button
            onClick={() => setConfirming(true)}
            disabled={!valid}
            className="btn-secondary btn-sm"
          >
            <Trash2 className="h-3 w-3" />
            Delete range
          </button>
        )}
      </div>
      {note && <p className="mt-2 text-[11px] text-muted-foreground">{note}</p>}
    </div>
  )
}

export function HistoryView() {
  const [items, setItems] = useState<TranscriptEntry[]>([])
  const [counts, setCounts] = useState<DailyCount[]>([])
  const [offset, setOffset] = useState(0)
  const [hasMore, setHasMore] = useState(false)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [copiedId, setCopiedId] = useState<number | null>(null)

  const reload = useCallback(async () => {
    try {
      const [first, daily] = await Promise.all([
        ipc.getTranscripts(PAGE_SIZE, 0),
        ipc.transcriptDailyCounts(),
      ])
      setItems(first)
      setOffset(first.length)
      setHasMore(first.length === PAGE_SIZE)
      setCounts(daily)
      setError(null)
    } catch (e) {
      setError(String(e))
    }
  }, [])

  useEffect(() => {
    reload()
  }, [reload])

  const loadMore = async () => {
    setLoadingMore(true)
    try {
      const next = await ipc.getTranscripts(PAGE_SIZE, offset)
      setItems((prev) => [...prev, ...next])
      setOffset((o) => o + next.length)
      setHasMore(next.length === PAGE_SIZE)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoadingMore(false)
    }
  }

  const copy = async (item: TranscriptEntry) => {
    await navigator.clipboard.writeText(item.cleaned_text || item.raw_text)
    setCopiedId(item.id)
    setTimeout(() => setCopiedId(null), 1400)
  }

  const clearAll = async () => {
    try {
      await ipc.clearTranscripts()
      await reload()
    } catch (e) {
      setError(String(e))
    }
  }

  // Group the loaded items under day headings (they arrive newest-first).
  const groups = useMemo(() => {
    const out: { key: string; heading: string; items: TranscriptEntry[] }[] = []
    for (const item of items) {
      const key = dayKey(new Date(item.timestamp * 1000))
      const last = out[out.length - 1]
      if (last && last.key === key) {
        last.items.push(item)
      } else {
        out.push({ key, heading: dayHeading(key), items: [item] })
      }
    }
    return out
  }, [items])

  const hasAnything = items.length > 0 || counts.length > 0

  return (
    <div className="flex flex-col gap-4">
      <PageHeader
        title="History"
        description="Everything you have dictated, stored only on this machine."
        action={
          items.length > 0 ? (
            <button onClick={clearAll} className="btn-danger btn-sm">
              <Trash2 className="h-3 w-3" />
              Clear all
            </button>
          ) : undefined
        }
      />

      {error && (
        <div className="mb-3">
          <ErrorNote>{error}</ErrorNote>
        </div>
      )}

      {!hasAnything ? (
        <EmptyState
          title="Nothing dictated yet"
          hint="Your dictations will collect here so you can copy anything that landed in the wrong window."
        />
      ) : (
        <div className="flex items-start gap-4">
          {/* Left column — the heatmap on top, transcript groups below. */}
          <div className="flex min-w-0 flex-1 flex-col gap-4">
            <UsageHeatmap counts={counts} />

            {groups.map((group) => (
              <div key={group.key} className="flex flex-col gap-2">
                <h2 className="mono px-0.5 pt-1 text-[10px] uppercase tracking-widest text-muted-foreground">
                  {group.heading}
                </h2>
                {group.items.map((item) => (
                  <div key={item.id} className="panel p-3">
                    <div className="flex items-center justify-between gap-3">
                      <span className="mono text-[10px] uppercase tracking-widest text-muted-foreground">
                        {timeOfDay(item.timestamp)}
                        {item.audio_ms > 0 && ` · ${(item.audio_ms / 1000).toFixed(1)}s`}
                      </span>
                      <button onClick={() => copy(item)} className="btn-ghost btn-sm">
                        {copiedId === item.id ? (
                          <Check className="h-3 w-3" />
                        ) : (
                          <Copy className="h-3 w-3" />
                        )}
                        {copiedId === item.id ? 'Copied' : 'Copy'}
                      </button>
                    </div>
                    <p className="mt-1.5 whitespace-pre-wrap text-sm leading-relaxed">
                      {item.cleaned_text || item.raw_text}
                    </p>
                  </div>
                ))}
              </div>
            ))}

            {hasMore && (
              <div className="flex justify-center pt-1">
                <button onClick={loadMore} disabled={loadingMore} className="btn-secondary btn-sm">
                  {loadingMore ? <Loader2 className="h-3 w-3 animate-spin" /> : null}
                  {loadingMore ? 'Loading…' : 'Show more'}
                </button>
              </div>
            )}
          </div>

          {/* Right rail — the date-range delete control sits here, not under the heatmap. */}
          <div className="w-[300px] flex-shrink-0">
            <RangeDelete onDeleted={reload} />
          </div>
        </div>
      )}
    </div>
  )
}
