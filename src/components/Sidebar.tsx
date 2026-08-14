'use client'

import { useEffect, useState } from 'react'
import { Mic, History, BookMarked, Scissors, BarChart3, Settings2, Flame } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { Status, StreakInfo } from '@/lib/ipc'

export type View = 'dictate' | 'history' | 'dictionary' | 'snippets' | 'insights' | 'settings'

const NAV: { id: View; label: string; icon: typeof Mic }[] = [
  { id: 'dictate', label: 'Dictate', icon: Mic },
  { id: 'history', label: 'History', icon: History },
  { id: 'dictionary', label: 'Words', icon: BookMarked },
  { id: 'snippets', label: 'Snippets', icon: Scissors },
  { id: 'insights', label: 'Insights', icon: BarChart3 },
  { id: 'settings', label: 'Settings', icon: Settings2 },
]

/** Three rising bars — the AuraScribe mark. */
function Wordmark() {
  return (
    <span className="flex h-[22px] w-[22px] shrink-0 items-end justify-center gap-[2px]">
      <span className="w-[3px] rounded-[1px] bg-foreground" style={{ height: 8 }} />
      <span className="w-[3px] rounded-[1px] bg-foreground" style={{ height: 15 }} />
      <span className="w-[3px] rounded-[1px] bg-foreground" style={{ height: 5 }} />
    </span>
  )
}

export function Sidebar({
  view,
  onNavigate,
  status,
  collapsed,
}: {
  view: View
  onNavigate: (v: View) => void
  status: Status
  collapsed: boolean
}) {
  const state = status.is_recording
    ? { label: 'Listening', color: 'bg-[hsl(var(--record))]' }
    : status.is_processing
    ? { label: 'Working', color: 'bg-primary' }
    : status.is_model_loaded
    ? { label: 'Ready', color: 'bg-primary' }
    : { label: 'Needs a model', color: 'bg-muted-foreground/40' }

  // Glanceable streak in the status rail. Re-read on navigation and when a dictation ends
  // (is_recording flips false) so it reflects the latest without a manual refresh. Silently
  // absent outside Tauri (browser preview) — it is an overlay, never load-bearing.
  const [streak, setStreak] = useState<StreakInfo | null>(null)
  useEffect(() => {
    let cancelled = false
    const load = () =>
      ipc
        .getStreakState()
        .then((s) => {
          if (!cancelled) setStreak(s)
        })
        .catch(() => {})
    load()
    const id = setInterval(load, 10_000)
    return () => {
      cancelled = true
      clearInterval(id)
    }
  }, [view, status.is_recording])

  // Fixed icon slot so collapsing changes width and hides labels only — icons never shift.
  const slot = 'flex w-[22px] shrink-0 items-center justify-center'

  return (
    <aside
      className={`flex shrink-0 flex-col border-r px-3.5 py-[18px] transition-[width] duration-200 ${
        collapsed ? 'w-[68px]' : 'w-[216px]'
      }`}
    >
      <div className="flex items-center gap-2.5 px-2 pb-6">
        <Wordmark />
        {!collapsed && (
          <span className="font-display text-[17px] font-semibold tracking-tight">AuraScribe</span>
        )}
      </div>

      <nav className="flex flex-1 flex-col gap-1.5">
        {NAV.map(({ id, label, icon: Icon }) => {
          const active = view === id
          return (
            <button
              key={id}
              onClick={() => onNavigate(id)}
              aria-current={active ? 'page' : undefined}
              title={collapsed ? label : undefined}
              className={`nav-item ${active ? 'nav-item-active' : ''}`}
            >
              <span className={slot}>
                <Icon
                  className="h-[21px] w-[21px]"
                  strokeWidth={active ? 2.4 : 2}
                  color={active ? 'hsl(var(--foreground))' : 'hsl(var(--muted-foreground))'}
                />
              </span>
              {!collapsed && label}
            </button>
          )
        })}
      </nav>

      <div className="mt-2 border-t pt-3">
        <div className="flex items-center gap-2 pl-1">
          <span className={slot}>
            <span className={`h-1.5 w-1.5 rounded-full ${state.color}`} title={state.label} />
          </span>
          {!collapsed && (
            <span className="truncate text-[12px] text-muted-foreground">
              {state.label}
              {status.loaded_model && (
                <>
                  {' '}
                  <span className="mono" style={{ color: 'hsl(var(--faint))' }}>
                    · {status.loaded_model}
                  </span>
                </>
              )}
            </span>
          )}
        </div>

        {streak && streak.streak > 0 && (
          <div
            className="mt-2 flex items-center gap-2 pl-1"
            title={`${streak.streak} day streak${streak.today_counted ? '' : ' — dictate 25 words to keep it today'}`}
          >
            <span className={slot}>
              <Flame
                className="h-[15px] w-[15px]"
                strokeWidth={2}
                color={streak.today_counted ? 'hsl(var(--primary))' : 'hsl(var(--muted-foreground))'}
              />
            </span>
            {!collapsed ? (
              <span className="text-[12px] text-muted-foreground">
                <span className="mono tnum" style={{ color: 'hsl(var(--foreground))' }}>
                  {streak.streak}
                </span>{' '}
                day streak
              </span>
            ) : (
              <span className="mono tnum text-[11px]" style={{ color: 'hsl(var(--foreground))' }}>
                {streak.streak}
              </span>
            )}
          </div>
        )}
      </div>
    </aside>
  )
}
