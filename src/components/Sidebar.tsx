'use client'

import {
  Mic,
  History,
  BookMarked,
  Scissors,
  BarChart3,
  Settings2,
  AudioLines,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-react'
import type { Status } from '@/lib/ipc'

export type View = 'dictate' | 'history' | 'dictionary' | 'snippets' | 'insights' | 'settings'

const NAV: { id: View; label: string; icon: typeof Mic }[] = [
  { id: 'dictate', label: 'Dictate', icon: Mic },
  { id: 'history', label: 'History', icon: History },
  { id: 'dictionary', label: 'Words', icon: BookMarked },
  { id: 'snippets', label: 'Snippets', icon: Scissors },
  { id: 'insights', label: 'Insights', icon: BarChart3 },
  { id: 'settings', label: 'Settings', icon: Settings2 },
]

export function Sidebar({
  view,
  onNavigate,
  status,
  collapsed,
  onToggleCollapsed,
}: {
  view: View
  onNavigate: (v: View) => void
  status: Status
  collapsed: boolean
  onToggleCollapsed: () => void
}) {
  const state = status.is_recording
    ? { label: 'Listening', color: 'bg-[hsl(var(--record))]' }
    : status.is_processing
    ? { label: 'Working', color: 'bg-[hsl(var(--standby))]' }
    : status.is_model_loaded
    ? { label: 'Ready', color: 'bg-primary' }
    : { label: 'Needs a model', color: 'bg-muted-foreground/40' }

  return (
    <aside
      className={`flex shrink-0 flex-col border-r bg-card transition-[width] duration-200 ${
        collapsed ? 'w-[60px]' : 'w-[212px]'
      }`}
    >
      <div
        className={`flex items-center gap-2 py-4 ${collapsed ? 'justify-center px-0' : 'px-4'}`}
      >
        <AudioLines className="h-[18px] w-[18px] shrink-0 text-primary" strokeWidth={2} />
        {!collapsed && (
          <div className="min-w-0 leading-none">
            <div className="truncate text-[13px] font-semibold tracking-tight">AuraScribe</div>
            <div className="mono mt-0.5 text-[10px] uppercase tracking-widest text-muted-foreground">
              on-device
            </div>
          </div>
        )}
      </div>

      <nav className={`flex flex-col gap-0.5 ${collapsed ? 'px-1.5' : 'px-2'}`}>
        {NAV.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => onNavigate(id)}
            aria-current={view === id ? 'page' : undefined}
            title={collapsed ? label : undefined}
            className={`nav-item ${view === id ? 'nav-item-active' : ''} ${
              collapsed ? 'justify-center px-0' : ''
            }`}
          >
            <Icon className="h-4 w-4 shrink-0" strokeWidth={2} />
            {!collapsed && label}
          </button>
        ))}
      </nav>

      <div className="mt-auto">
        <button
          onClick={onToggleCollapsed}
          className={`nav-item mb-1 ${collapsed ? 'justify-center px-0' : 'mx-2'}`}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {collapsed ? (
            <PanelLeftOpen className="h-4 w-4 shrink-0" />
          ) : (
            <>
              <PanelLeftClose className="h-4 w-4 shrink-0" />
              Collapse
            </>
          )}
        </button>

        {/* Status rail — the app says what it is doing without being asked. */}
        <div className={`border-t py-3 ${collapsed ? 'flex justify-center px-0' : 'px-4'}`}>
          {collapsed ? (
            <span className={`dot ${state.color}`} title={state.label} />
          ) : (
            <>
              <div className="flex items-center gap-2">
                <span className={`dot ${state.color}`} />
                <span className="text-xs font-medium">{state.label}</span>
              </div>
              <div
                className="mono mt-1.5 truncate text-[10px] text-muted-foreground"
                title={status.loaded_model ?? undefined}
              >
                {status.loaded_model ?? 'no model loaded'}
              </div>
            </>
          )}
        </div>
      </div>
    </aside>
  )
}
