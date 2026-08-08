'use client'

import { useCallback, useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { PanelLeft, Minus, Square, X } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { Settings, Status } from '@/lib/ipc'
import { Sidebar, type View } from '@/components/Sidebar'
import { WidgetRail } from '@/components/WidgetRail'
import { DictateView } from '@/components/views/DictateView'
import { HistoryView } from '@/components/views/HistoryView'
import { DictionaryView } from '@/components/views/DictionaryView'
import { SnippetsView } from '@/components/views/SnippetsView'
import { InsightsView } from '@/components/views/InsightsView'
import { SettingsView } from '@/components/views/SettingsView'
import { Onboarding } from '@/components/Onboarding'

const DEFAULT_STATUS: Status = {
  is_recording: false,
  is_processing: false,
  is_model_loaded: false,
  loaded_model: null,
  current_text: '',
  last_error: null,
  hotkey_mode: 'toggle',
  ai_cleanup_enabled: true,
}

const DEFAULT_SETTINGS: Settings = {
  hotkey: 'Ctrl+Shift+Space',
  hotkey_mode: 'toggle',
  whisper_model: 'moonshine-base-en',
  mic_device: null,
  ai_cleanup_enabled: true,
  remove_fillers: true,
  language: 'en',
  theme: 'glass',
  start_at_login: false,
  sound_cues: true,
  // Fallback before real settings load: assume onboarded so the browser preview / a failed
  // read never flashes the walkthrough. A genuine fresh install reports onboarded=false.
  onboarded: true,
}

/** Frameless-window controls. No-op outside Tauri (e.g. the browser preview). */
async function windowAction(action: 'minimize' | 'toggleMaximize' | 'close') {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const w = getCurrentWindow()
    if (action === 'minimize') await w.minimize()
    else if (action === 'toggleMaximize') await w.toggleMaximize()
    else await w.close()
  } catch {
    /* not in a Tauri window */
  }
}

export default function App() {
  const [status, setStatus] = useState<Status>(DEFAULT_STATUS)
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS)
  const [view, setView] = useState<View>('dictate')
  const [ready, setReady] = useState(false)
  const [tauri, setTauri] = useState(false)
  const [collapsed, setCollapsed] = useState(false)

  useEffect(() => {
    let offStatus: (() => void) | null = null
    let offSettings: (() => void) | null = null

    ;(async () => {
      try {
        const available = await isTauri()
        setTauri(available)
        if (available) {
          setSettings(await ipc.getSettings())
          setStatus(await ipc.getStatus())
          offStatus = await ipc.onStatusChanged(setStatus)
          offSettings = await ipc.onSettingsChanged(setSettings)
        }
      } catch (e) {
        console.error('Startup failed:', e)
      } finally {
        setReady(true)
      }
    })()

    return () => {
      offStatus?.()
      offSettings?.()
    }
  }, [])

  // Events are the fast path, not the only path. A missed `status-changed` repeatedly
  // stranded the UI insisting no model was loaded while the backend had one in memory,
  // with no way for the user to recover. Re-reading the authoritative state on a short
  // interval (and on navigation) makes that desync self-healing.
  useEffect(() => {
    if (!tauri) return
    let cancelled = false
    const sync = () => {
      ipc
        .getStatus()
        .then((s) => {
          if (!cancelled) setStatus(s)
        })
        .catch(() => {})
    }
    sync()
    const id = setInterval(sync, 1500)
    return () => {
      cancelled = true
      clearInterval(id)
    }
  }, [view, tauri])

  const saveSettings = useCallback(
    async (patch: Partial<Settings>) => {
      const next = { ...settings, ...patch }
      setSettings(next)
      if (!tauri) return
      try {
        await ipc.saveSettings(next)
      } catch (e) {
        console.error('Could not save settings:', e)
        try {
          setSettings(await ipc.getSettings())
        } catch {
          /* keep the optimistic value if even reading fails */
        }
      } finally {
        try {
          setStatus(await ipc.getStatus())
        } catch {
          /* status refresh is best-effort */
        }
      }
    },
    [settings, tauri]
  )

  const toggleRecording = useCallback(async () => {
    if (!tauri) return
    try {
      if (status.is_recording) await ipc.stopRecording()
      else await ipc.startRecording()
    } catch (e) {
      console.error('Recording failed:', e)
      setStatus(await ipc.getStatus())
    }
  }, [status.is_recording, tauri])

  useEffect(() => {
    const root = document.documentElement
    // "glass" is dark vibrancy: light text on dark frosted panels over the image backdrop,
    // so it rides on the *dark* token set (see `.glass-bg` in globals.css). The others are solid.
    root.classList.toggle('glass-bg', settings.theme === 'glass')
    if (settings.theme === 'dark' || settings.theme === 'glass') root.classList.add('dark')
    else if (settings.theme === 'light') root.classList.remove('dark')
    else
      root.classList.toggle('dark', window.matchMedia('(prefers-color-scheme: dark)').matches)
  }, [settings.theme])

  if (!ready) return <div className="h-screen w-screen bg-background" />

  // First run only. `onboarded` is false solely on a freshly created database (see the Rust
  // `Database::new` fresh-install path); returning users and the browser preview never see it.
  const showOnboarding = tauri && !settings.onboarded

  return (
    // Full-bleed in every appearance — the app fills the frameless window edge to edge.
    // "Glass" is not a smaller box: it is the same full size, but the shell and its cards
    // become frosted glass over a bluish backdrop (see `.glass-bg` in globals.css).
    <div className="h-screen w-screen overflow-hidden">
      {showOnboarding && (
        <Onboarding
          hotkey={settings.hotkey}
          onFinish={() => saveSettings({ onboarded: true })}
          onOpenModels={() => {
            setView('settings')
            saveSettings({ onboarded: true })
          }}
        />
      )}
      <div className="app-shell flex h-full flex-col overflow-hidden bg-background">
        {/* Custom titlebar — the window is frameless, so this is the drag handle and the
            min / maximize / close controls. */}
        <div
          data-tauri-drag-region
          className="flex flex-shrink-0 items-center justify-between border-b px-4 py-2.5"
        >
          <button
            onClick={() => setCollapsed((c) => !c)}
            className="flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-accent"
            aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            <PanelLeft className="h-[17px] w-[17px]" strokeWidth={1.8} />
          </button>

          <div className="flex items-center gap-1">
            <button
              onClick={() => windowAction('minimize')}
              className="flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-accent"
              aria-label="Minimize"
            >
              <Minus className="h-4 w-4" strokeWidth={1.8} />
            </button>
            <button
              onClick={() => windowAction('toggleMaximize')}
              className="flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-accent"
              aria-label="Maximize"
            >
              <Square className="h-3 w-3" strokeWidth={1.9} />
            </button>
            <button
              onClick={() => windowAction('close')}
              className="flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-destructive hover:text-white"
              aria-label="Close"
            >
              <X className="h-4 w-4" strokeWidth={1.8} />
            </button>
          </div>
        </div>

        <div className="flex flex-1 overflow-hidden">
          <Sidebar view={view} onNavigate={setView} status={status} collapsed={collapsed} />

          <div className="flex flex-1 gap-5 overflow-hidden px-7 py-7 lg:px-8">
            <div key={view} className="fade-up flex-[1.7] overflow-y-auto overflow-x-hidden pr-1">
              {view === 'dictate' && (
                <DictateView
                  status={status}
                  settings={settings}
                  onToggleRecording={toggleRecording}
                  onSaveSettings={saveSettings}
                  onGoToSettings={() => setView('settings')}
                />
              )}
              {view === 'history' && <HistoryView />}
              {view === 'dictionary' && <DictionaryView />}
              {view === 'snippets' && <SnippetsView />}
              {view === 'insights' && <InsightsView />}
              {view === 'settings' && (
                <SettingsView settings={settings} status={status} onSaveSettings={saveSettings} />
              )}
            </div>

            {/* Contextual right rail — changes with the active tab. Hidden on narrow widths. */}
            <div className="hidden w-[250px] flex-shrink-0 overflow-y-auto overflow-x-hidden xl:block">
              <WidgetRail view={view} status={status} settings={settings} />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
