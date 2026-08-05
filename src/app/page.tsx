'use client'

import { useCallback, useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import * as ipc from '@/lib/ipc'
import type { Settings, Status } from '@/lib/ipc'
import { Sidebar, type View } from '@/components/Sidebar'
import { DictateView } from '@/components/views/DictateView'
import { HistoryView } from '@/components/views/HistoryView'
import { DictionaryView } from '@/components/views/DictionaryView'
import { SnippetsView } from '@/components/views/SnippetsView'
import { InsightsView } from '@/components/views/InsightsView'
import { SettingsView } from '@/components/views/SettingsView'

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
  whisper_model: 'base.en',
  mic_device: null,
  ai_cleanup_enabled: true,
  remove_fillers: true,
  language: 'en',
  theme: 'dark',
  start_at_login: false,
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
  // interval (and on navigation) makes that desync self-healing. get_status is a cheap
  // in-memory read, so the cost is negligible.
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
        // Put the stored values back so the UI never claims a change that didn't stick.
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
    if (settings.theme === 'dark') root.classList.add('dark')
    else if (settings.theme === 'light') root.classList.remove('dark')
    else
      root.classList.toggle('dark', window.matchMedia('(prefers-color-scheme: dark)').matches)
  }, [settings.theme])

  if (!ready) return <div className="h-screen bg-background" />

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <Sidebar
        view={view}
        onNavigate={setView}
        status={status}
        collapsed={collapsed}
        onToggleCollapsed={() => setCollapsed((c) => !c)}
      />

      <main className="flex-1 overflow-y-auto px-6 py-6">
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
      </main>
    </div>
  )
}
