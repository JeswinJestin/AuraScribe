'use client'

import { useEffect, useState, useCallback } from 'react'
import {
  Mic, MicOff, Settings, Shield, Download, Github, ExternalLink,
  Loader2, CheckCircle, AlertCircle, Sparkles, Zap, Lock, Globe,
  Sun, Moon, Eye, EyeOff, Keyboard, ChevronRight
} from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { isTauri } from '@tauri-apps/api/core'

interface Status {
  isRecording: boolean
  isProcessing: boolean
  isModelLoaded: boolean
  currentText: string
  lastError: string | null
  hotkeyMode: 'press-hold' | 'toggle'
  aiCleanupEnabled: boolean
}

interface SettingsData {
  hotkey: string
  hotkeyMode: 'press-hold' | 'toggle'
  whisperModel: string
  openrouterKey: string
  openrouterModel: string
  aiCleanupEnabled: boolean
  autoPunctuation: boolean
  language: string
  theme: 'light' | 'dark' | 'system'
  startAtLogin: boolean
}

const OPENROUTER_MODELS = [
  { id: 'nvidia/nemotron-3-ultra', name: 'Nemotron 3 Ultra', free: true },
  { id: 'meta-llama/llama-3.1-8b-instruct:free', name: 'Llama 3.1 8B', free: true },
  { id: 'google/gemma-2-9b-it:free', name: 'Gemma 2 9B', free: true },
  { id: 'mistralai/mistral-7b-instruct:free', name: 'Mistral 7B', free: true },
  { id: 'openai/gpt-4o-mini', name: 'GPT-4o Mini', free: false },
]

export default function Home() {
  const [status, setStatus] = useState<Status>({
    isRecording: false,
    isProcessing: false,
    isModelLoaded: false,
    currentText: '',
    lastError: null,
    hotkeyMode: 'toggle',
    aiCleanupEnabled: false,
  })
  const [settings, setSettings] = useState<SettingsData>({
    hotkey: 'Ctrl+Alt',
    hotkeyMode: 'toggle',
    whisperModel: 'base.en',
    openrouterKey: '',
    openrouterModel: 'nvidia/nemotron-3-ultra',
    aiCleanupEnabled: false,
    autoPunctuation: true,
    language: 'en',
    theme: 'dark',
    startAtLogin: false,
  })
  const [activeTab, setActiveTab] = useState<'home' | 'settings'>('home')
  const [isLoading, setIsLoading] = useState(true)
  const [showKey, setShowKey] = useState(false)
  const [tauriAvailable, setTauriAvailable] = useState(false)

  useEffect(() => {
    let unlistenStatus: (() => void) | null = null
    let unlistenSettings: (() => void) | null = null

    const init = async () => {
      try {
        const available = await isTauri()
        setTauriAvailable(available)
        if (available) {
          const loadedSettings = await invoke<SettingsData>('get_settings')
          setSettings(loadedSettings)
          const loadedStatus = await invoke<Status>('get_status')
          setStatus(loadedStatus)

          unlistenStatus = await listen<Status>('status-changed', (e) => setStatus(e.payload))
          unlistenSettings = await listen<SettingsData>('settings-changed', (e) => setSettings(e.payload))
        }
      } catch (e) {
        console.error('Init error:', e)
      } finally {
        setIsLoading(false)
      }
    }
    init()

    return () => {
      unlistenStatus?.()
      unlistenSettings?.()
    }
  }, [])

  const saveSettings = useCallback(async (patch: Partial<SettingsData>) => {
    const updated = { ...settings, ...patch }
    setSettings(updated)
    if (tauriAvailable) {
      try {
        await invoke('save_settings', { settings: updated })
      } catch (e) {
        console.error('Save settings error:', e)
      }
    }
  }, [settings, tauriAvailable])

  const handleToggleRecording = async () => {
    if (!tauriAvailable) return
    try {
      if (status.isRecording) {
        await invoke('stop_recording')
      } else {
        await invoke('start_recording')
      }
    } catch (e) {
      console.error('Recording error:', e)
    }
  }

  const handleLoadModel = async () => {
    if (!tauriAvailable) return
    try {
      await invoke('load_model', { modelId: settings.whisperModel })
    } catch (e) {
      console.error('Load model error:', e)
    }
  }

  useEffect(() => {
    if (settings.theme === 'dark') {
      document.documentElement.classList.add('dark')
    } else if (settings.theme === 'light') {
      document.documentElement.classList.remove('dark')
    } else {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
      document.documentElement.classList.toggle('dark', prefersDark)
    }
  }, [settings.theme])

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-10 w-10 animate-spin text-primary" />
          <p className="text-sm text-muted-foreground">Loading AuraScribe...</p>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background flex flex-col">
      <header className="sticky top-0 z-50 glass-strong border-b border-border/50">
        <div className="max-w-4xl mx-auto px-4 h-14 flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="relative w-8 h-8 rounded-lg bg-gradient-to-br from-primary to-purple-600 flex items-center justify-center shadow-lg shadow-primary/20">
              <Mic className="w-4 h-4 text-white" />
              {status.isRecording && (
                <span className="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 bg-red-500 rounded-full animate-pulse" />
              )}
            </div>
            <span className="font-bold text-lg tracking-tight">AuraScribe</span>
          </div>

          <nav className="flex items-center gap-0.5 bg-muted/50 rounded-lg p-0.5">
            {(['home', 'settings'] as const).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-4 py-1.5 rounded-md text-sm font-medium transition-all duration-200 ${
                  activeTab === tab
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                {tab === 'home' ? 'Home' : 'Settings'}
              </button>
            ))}
          </nav>

          <div className="flex items-center gap-1">
            <a
              href="https://github.com/JeswinJestin/AuraScribe"
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 rounded-lg hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
              title="GitHub"
            >
              <Github className="w-4 h-4" />
            </a>
            <button
              onClick={() => saveSettings({ theme: settings.theme === 'dark' ? 'light' : 'dark' })}
              className="p-2 rounded-lg hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
              title="Toggle theme"
            >
              {settings.theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>
          </div>
        </div>
      </header>

      <main className="flex-1 max-w-4xl mx-auto w-full px-4 py-6">
        {activeTab === 'home' && (
          <HomeView
            status={status}
            settings={settings}
            onToggleRecording={handleToggleRecording}
            onLoadModel={handleLoadModel}
            onSaveSettings={saveSettings}
          />
        )}
        {activeTab === 'settings' && (
          <SettingsView
            settings={settings}
            onSaveSettings={saveSettings}
            showKey={showKey}
            setShowKey={setShowKey}
          />
        )}
      </main>

      <footer className="border-t border-border/50 py-3">
        <div className="max-w-4xl mx-auto px-4 flex items-center justify-between text-xs text-muted-foreground">
          <span>AuraScribe v1.0.0</span>
          <span>MIT License</span>
        </div>
      </footer>
    </div>
  )
}

function HomeView({
  status,
  settings,
  onToggleRecording,
  onLoadModel,
  onSaveSettings,
}: {
  status: Status
  settings: SettingsData
  onToggleRecording: () => void
  onLoadModel: () => void
  onSaveSettings: (s: Partial<SettingsData>) => void
}) {
  const getMicButtonState = () => {
    if (status.isRecording) return { color: 'bg-red-500 hover:bg-red-600', ring: 'ring-red-500/30', label: 'Stop & Insert' }
    if (status.isProcessing) return { color: 'bg-yellow-500 hover:bg-yellow-600', ring: 'ring-yellow-500/30', label: 'Processing...' }
    return { color: 'bg-primary hover:bg-primary/90', ring: 'ring-primary/30', label: 'Start Dictation' }
  }

  const micState = getMicButtonState()

  return (
    <div className="space-y-5">
      <div className="text-center space-y-1">
        <h1 className="text-2xl font-bold tracking-tight">Voice Dictation</h1>
        <p className="text-sm text-muted-foreground">
          Press <kbd className="px-1.5 py-0.5 bg-muted rounded text-xs font-mono font-medium">{settings.hotkey.replace('+', ' + ')}</kbd> or click below to dictate
        </p>
      </div>

      <div className="card p-6">
        <div className="flex flex-col items-center gap-5">
          <div className="relative">
            {status.isRecording && (
              <>
                <div className="absolute inset-0 rounded-full bg-red-500/20 pulse-ring" />
                <div className="absolute inset-0 rounded-full bg-red-500/10 pulse-ring" style={{ animationDelay: '0.5s' }} />
              </>
            )}
            <button
              onClick={onToggleRecording}
              disabled={status.isProcessing || (!status.isModelLoaded && !status.isRecording)}
              className={`relative w-24 h-24 rounded-full ${micState.color} text-white shadow-xl ${micState.ring} ring-4 transition-all duration-200 flex items-center justify-center disabled:opacity-40 disabled:cursor-not-allowed hover:scale-105 active:scale-95`}
            >
              {status.isProcessing ? (
                <Loader2 className="w-10 h-10 animate-spin" />
              ) : status.isRecording ? (
                <MicOff className="w-10 h-10" />
              ) : (
                <Mic className="w-10 h-10" />
              )}
            </button>
          </div>

          <div className="text-center">
            <p className="font-semibold text-lg">
              {status.isRecording
                ? 'Listening...'
                : status.isProcessing
                ? 'Processing audio...'
                : status.isModelLoaded
                ? 'Ready'
                : 'Setup Required'}
            </p>
            <p className="text-sm text-muted-foreground mt-1">
              {status.isRecording
                ? 'Speak clearly, then click to stop and insert text'
                : status.isProcessing
                ? 'Transcribing your speech...'
                : status.isModelLoaded
                ? `Press the mic or ${settings.hotkey.replace('+', ' + ')} to start`
                : 'Load a model below to get started'}
            </p>
          </div>

          {status.lastError && (
            <div className="w-full p-3 bg-destructive/10 border border-destructive/20 rounded-xl flex items-start gap-2.5">
              <AlertCircle className="w-4 h-4 text-destructive mt-0.5 shrink-0" />
              <p className="text-sm text-destructive">{status.lastError}</p>
            </div>
          )}

          {status.currentText && (
            <div className="w-full animate-slide-up">
              <div className="flex items-center gap-2 mb-2">
                <h3 className="text-sm font-medium text-muted-foreground">Last Transcript</h3>
                {status.aiCleanupEnabled && (
                  <span className="flex items-center gap-1 text-xs text-primary bg-primary/10 px-2 py-0.5 rounded-full">
                    <Sparkles className="w-3 h-3" /> AI Enhanced
                  </span>
                )}
              </div>
              <div className="p-4 bg-muted/50 rounded-xl min-h-[60px] font-mono text-sm whitespace-pre-wrap leading-relaxed">
                {status.currentText}
              </div>
            </div>
          )}
        </div>
      </div>

      {!status.isModelLoaded && (
        <div className="card p-5">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="font-semibold">Load Whisper Model</h3>
              <p className="text-sm text-muted-foreground mt-0.5">
                Choose a model size for transcription
              </p>
            </div>
            <button onClick={onLoadModel} className="btn-primary">
              <Download className="w-4 h-4" />
              Load
            </button>
          </div>
        </div>
      )}

      <div className="grid grid-cols-3 gap-3">
        <div className="card p-4 text-center">
          <Shield className="w-5 h-5 text-primary mx-auto mb-2" />
          <p className="text-sm font-semibold">100% Local</p>
          <p className="text-xs text-muted-foreground mt-0.5">Audio on-device</p>
        </div>
        <div className="card p-4 text-center">
          <Zap className="w-5 h-5 text-yellow-500 mx-auto mb-2" />
          <p className="text-sm font-semibold">Fast</p>
          <p className="text-xs text-muted-foreground mt-0.5">API transcription</p>
        </div>
        <div className="card p-4 text-center">
          <Lock className="w-5 h-5 text-green-500 mx-auto mb-2" />
          <p className="text-sm font-semibold">Encrypted</p>
          <p className="text-xs text-muted-foreground mt-0.5">AES-256 keys</p>
        </div>
      </div>

      <div className="card p-5">
        <h3 className="font-semibold mb-3">Hotkey Mode</h3>
        <div className="grid grid-cols-2 gap-3">
          <button
            onClick={() => onSaveSettings({ hotkeyMode: 'toggle' })}
            className={`p-3 rounded-xl border-2 text-left transition-all ${
              settings.hotkeyMode === 'toggle'
                ? 'border-primary bg-primary/5'
                : 'border-border hover:border-primary/50'
            }`}
          >
            <div className="flex items-center justify-between mb-1">
              <span className="font-medium text-sm">Toggle</span>
              {settings.hotkeyMode === 'toggle' && <CheckCircle className="w-4 h-4 text-primary" />}
            </div>
            <p className="text-xs text-muted-foreground">Press to start, press again to stop</p>
          </button>
          <button
            onClick={() => onSaveSettings({ hotkeyMode: 'press-hold' })}
            className={`p-3 rounded-xl border-2 text-left transition-all ${
              settings.hotkeyMode === 'press-hold'
                ? 'border-primary bg-primary/5'
                : 'border-border hover:border-primary/50'
            }`}
          >
            <div className="flex items-center justify-between mb-1">
              <span className="font-medium text-sm">Hold</span>
              {settings.hotkeyMode === 'press-hold' && <CheckCircle className="w-4 h-4 text-primary" />}
            </div>
            <p className="text-xs text-muted-foreground">Hold to speak, release to insert</p>
          </button>
        </div>
      </div>
    </div>
  )
}

function SettingsView({
  settings,
  onSaveSettings,
  showKey,
  setShowKey,
}: {
  settings: SettingsData
  onSaveSettings: (s: Partial<SettingsData>) => void
  showKey: boolean
  setShowKey: (show: boolean) => void
}) {
  return (
    <div className="space-y-5">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Settings</h1>
        <p className="text-sm text-muted-foreground mt-1">Configure AuraScribe to your workflow</p>
      </div>

      <section className="card p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Keyboard className="w-4 h-4 text-primary" />
          <h2 className="font-semibold">Hotkey</h2>
        </div>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium mb-1.5">Combination</label>
            <div className="input bg-muted font-mono text-center text-sm" style={{ userSelect: 'all' }}>
              {settings.hotkey}
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">Mode</label>
            <select
              value={settings.hotkeyMode}
              onChange={(e) => onSaveSettings({ hotkeyMode: e.target.value as 'press-hold' | 'toggle' })}
              className="input text-sm"
            >
              <option value="press-hold">Press & Hold</option>
              <option value="toggle">Toggle (tap on/off)</option>
            </select>
          </div>
        </div>
      </section>

      <section className="card p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Sparkles className="w-4 h-4 text-primary" />
          <h2 className="font-semibold">AI Cleanup</h2>
        </div>
        <p className="text-sm text-muted-foreground">
          Polish your transcribed text with AI — fix grammar, punctuation, and remove filler words.
        </p>

        <label className="flex items-center gap-3 cursor-pointer">
          <div
            className={`relative w-10 h-6 rounded-full transition-colors ${
              settings.aiCleanupEnabled ? 'bg-primary' : 'bg-muted'
            }`}
            onClick={() => onSaveSettings({ aiCleanupEnabled: !settings.aiCleanupEnabled })}
          >
            <div
              className={`absolute top-1 left-1 w-4 h-4 bg-white rounded-full transition-transform ${
                settings.aiCleanupEnabled ? 'translate-x-4' : ''
              }`}
            />
          </div>
          <span className="text-sm font-medium">Enable AI cleanup</span>
        </label>

        {settings.aiCleanupEnabled && (
          <div className="space-y-4 pl-0 border-l-2 border-primary/20 pl-4">
            <div>
              <label className="block text-sm font-medium mb-1.5">OpenRouter API Key</label>
              <div className="relative">
                <input
                  type={showKey ? 'text' : 'password'}
                  value={settings.openrouterKey}
                  onChange={(e) => onSaveSettings({ openrouterKey: e.target.value })}
                  placeholder="sk-or-v1-..."
                  className="input pr-10 text-sm"
                />
                <button
                  onClick={() => setShowKey(!showKey)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                >
                  {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <p className="text-xs text-muted-foreground mt-1">
                Free key at{' '}
                <a href="https://openrouter.ai/keys" target="_blank" rel="noopener" className="underline text-primary">
                  openrouter.ai/keys
                </a>
              </p>
            </div>

            <div>
              <label className="block text-sm font-medium mb-1.5">Model</label>
              <select
                value={settings.openrouterModel}
                onChange={(e) => onSaveSettings({ openrouterModel: e.target.value })}
                className="input text-sm"
              >
                {OPENROUTER_MODELS.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name} {m.free ? '(Free)' : '(Paid)'}
                  </option>
                ))}
              </select>
            </div>

            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.autoPunctuation}
                onChange={(e) => onSaveSettings({ autoPunctuation: e.target.checked })}
                className="w-4 h-4 rounded border-input bg-background text-primary focus:ring-primary"
              />
              <span className="text-sm">Auto punctuation & capitalization</span>
            </label>
          </div>
        )}
      </section>

      <section className="card p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Globe className="w-4 h-4 text-primary" />
          <h2 className="font-semibold">General</h2>
        </div>
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1.5">Language</label>
              <select
                value={settings.language}
                onChange={(e) => onSaveSettings({ language: e.target.value })}
                className="input text-sm"
              >
                <option value="en">English</option>
                <option value="auto">Auto-detect</option>
                <option value="es">Spanish</option>
                <option value="fr">French</option>
                <option value="de">German</option>
                <option value="ja">Japanese</option>
                <option value="zh">Chinese</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1.5">Theme</label>
              <select
                value={settings.theme}
                onChange={(e) => onSaveSettings({ theme: e.target.value as 'light' | 'dark' | 'system' })}
                className="input text-sm"
              >
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
            </div>
          </div>
          <label className="flex items-center gap-3 cursor-pointer">
            <div
              className={`relative w-10 h-6 rounded-full transition-colors ${
                settings.startAtLogin ? 'bg-primary' : 'bg-muted'
              }`}
              onClick={() => onSaveSettings({ startAtLogin: !settings.startAtLogin })}
            >
              <div
                className={`absolute top-1 left-1 w-4 h-4 bg-white rounded-full transition-transform ${
                  settings.startAtLogin ? 'translate-x-4' : ''
                }`}
              />
            </div>
            <span className="text-sm font-medium">Start at login</span>
          </label>
        </div>
      </section>

      <section className="card p-5 border-green-200 dark:border-green-800/50 bg-green-50/50 dark:bg-green-950/20">
        <div className="flex items-start gap-3">
          <Shield className="w-5 h-5 text-green-600 dark:text-green-400 mt-0.5 shrink-0" />
          <div>
            <h3 className="font-semibold text-green-800 dark:text-green-200 text-sm">Privacy Guarantee</h3>
            <ul className="text-xs text-green-700 dark:text-green-300/80 mt-2 space-y-1">
              <li>- Audio processed via Whisper API (not stored)</li>
              <li>- AI cleanup only sends text, never audio</li>
              <li>- API key encrypted with AES-256 locally</li>
              <li>- Zero telemetry, zero analytics, zero tracking</li>
              <li>- Fully open source on GitHub</li>
            </ul>
          </div>
        </div>
      </section>
    </div>
  )
}
