'use client'

import { useEffect, useState } from 'react'
import { Mic, MicOff, Settings, Shield, Download, Github, ExternalLink, Loader2, CheckCircle, AlertCircle, Info, Sparkles, Zap, Lock, Globe } from 'lucide-react'
import { invoke, listen, emit } from '@tauri-apps/api/core'
import { available } from '@tauri-apps/api/core'

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
  openRouterKey: string
  openRouterModel: string
  aiCleanupEnabled: boolean
  autoPunctuation: boolean
  language: string
  theme: 'light' | 'dark' | 'system'
  startAtLogin: boolean
}

const WHISPER_MODELS = [
  { id: 'tiny.en', size: '39 MB', speed: 'Fastest', quality: 'Good', recommended: false },
  { id: 'base.en', size: '74 MB', speed: 'Fast', quality: 'Better', recommended: true },
  { id: 'small.en', size: '244 MB', speed: 'Balanced', quality: 'Great', recommended: false },
  { id: 'medium', size: '769 MB', speed: 'Slow', quality: 'Excellent', recommended: false },
]

const OPENROUTER_MODELS = [
  { id: 'nvidia/nemotron-3-ultra', name: 'Nemotron 3 Ultra (Free)', free: true },
  { id: 'meta-llama/llama-3.1-8b-instruct:free', name: 'Llama 3.1 8B (Free)', free: true },
  { id: 'google/gemma-2-9b-it:free', name: 'Gemma 2 9B (Free)', free: true },
  { id: 'mistralai/mistral-7b-instruct:free', name: 'Mistral 7B (Free)', free: true },
  { id: 'openai/gpt-4o-mini', name: 'GPT-4o Mini (Paid)', free: false },
]

export default function Home() {
  const [status, setStatus] = useState<Status>({
    isRecording: false,
    isProcessing: false,
    isModelLoaded: false,
    currentText: '',
    lastError: null,
    hotkeyMode: 'press-hold',
    aiCleanupEnabled: false,
  })
  const [settings, setSettings] = useState<SettingsData>({
    hotkey: 'Ctrl+Space',
    hotkeyMode: 'press-hold',
    whisperModel: 'base.en',
    openRouterKey: '',
    openRouterModel: 'nvidia/nemotron-3-ultra',
    aiCleanupEnabled: false,
    autoPunctuation: true,
    language: 'en',
    theme: 'system',
    startAtLogin: false,
  })
  const [activeTab, setActiveTab] = useState<'dashboard' | 'settings' | 'about'>('dashboard')
  const [isLoading, setIsLoading] = useState(true)
  const [showKey, setShowKey] = useState(false)

  useEffect(() => {
    const init = async () => {
      try {
        const isTauri = await available()
        if (isTauri) {
          const loadedSettings = await invoke<SettingsData>('get_settings')
          setSettings(loadedSettings)
          const loadedStatus = await invoke<Status>('get_status')
          setStatus(loadedStatus)
        }
      } catch (e) {
        console.error('Failed to load initial state:', e)
      } finally {
        setIsLoading(false)
      }
    }
    init()

    const unlistenStatus = await listen<Status>('status-changed', (e) => setStatus(e.payload))
    const unlistenSettings = await listen<SettingsData>('settings-changed', (e) => setSettings(e.payload))

    return () => {
      unlistenStatus()
      unlistenSettings()
    }
  }, [])

  const handleSaveSettings = async (newSettings: Partial<SettingsData>) => {
    try {
      const updated = { ...settings, ...newSettings }
      await invoke('save_settings', { settings: updated })
      setSettings(updated)
    } catch (e) {
      console.error('Failed to save settings:', e)
    }
  }

  const handleStartRecording = async () => {
    try {
      await invoke('start_recording')
    } catch (e) {
      console.error('Failed to start recording:', e)
    }
  }

  const handleStopRecording = async () => {
    try {
      await invoke('stop_recording')
    } catch (e) {
      console.error('Failed to stop recording:', e)
    }
  }

  const handleDownloadModel = async (modelId: string) => {
    try {
      await invoke('download_model', { modelId })
    } catch (e) {
      console.error('Failed to download model:', e)
    }
  }

  const formatHotkey = (hotkey: string) => {
    return hotkey
      .split('+')
      .map((k) => k.charAt(0).toUpperCase() + k.slice(1))
      .join(' + ')
  }

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-12 w-12 animate-spin text-primary" />
          <p className="text-muted-foreground">Loading AuraScribe...</p>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b bg-card/50 backdrop-blur-sm sticky top-0 z-40">
        <div className="container mx-auto px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="relative">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary to-primary/70 flex items-center justify-center">
                <Mic className="w-5 h-5 text-white" />
              </div>
              {status.isRecording && (
                <div className="absolute -top-1 -right-1 w-3 h-3 bg-red-500 rounded-full animate-pulse" />
              )}
            </div>
            <span className="font-bold text-xl">AuraScribe</span>
          </div>

          <nav className="flex items-center gap-1 bg-muted/50 rounded-lg p-1">
            {(['dashboard', 'settings', 'about'] as const).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
                  activeTab === tab
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                {tab.charAt(0).toUpperCase() + tab.slice(1)}
              </button>
            ))}
          </nav>

          <div className="flex items-center gap-2">
            <a
              href="https://github.com/aurascribe/aurascribe"
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 rounded-lg hover:bg-muted transition-colors"
              title="GitHub"
            >
              <Github className="w-5 h-5" />
            </a>
            <button
              onClick={() => handleSaveSettings({ theme: settings.theme === 'dark' ? 'light' : 'dark' })}
              className="p-2 rounded-lg hover:bg-muted transition-colors"
              title="Toggle theme"
            >
              {settings.theme === 'dark' ? <Sun className="w-5 h-5" /> : <Moon className="w-5 h-5" />}
            </button>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-4 py-8">
        {activeTab === 'dashboard' && (
          <DashboardView
            status={status}
            settings={settings}
            onStartRecording={handleStartRecording}
            onStopRecording={handleStopRecording}
            onSaveSettings={handleSaveSettings}
            formatHotkey={formatHotkey}
          />
        )}

        {activeTab === 'settings' && (
          <SettingsView
            settings={settings}
            onSaveSettings={handleSaveSettings}
            onDownloadModel={handleDownloadModel}
            showKey={showKey}
            setShowKey={setShowKey}
          />
        )}

        {activeTab === 'about' && <AboutView />}
      </main>
    </div>
  )
}

// Dashboard View Component
function DashboardView({
  status,
  settings,
  onStartRecording,
  onStopRecording,
  onSaveSettings,
  formatHotkey,
}: {
  status: Status
  settings: SettingsData
  onStartRecording: () => void
  onStopRecording: () => void
  onSaveSettings: (s: Partial<SettingsData>) => void
  formatHotkey: (h: string) => string
}) {
  const [recentTranscripts, setRecentTranscripts] = useState<string[]>([])

  return (
    <div className="space-y-6 max-w-4xl mx-auto">
      {/* Status Card */}
      <div className="card-base overflow-hidden">
        <div className={`p-6 flex items-center justify-between gap-4 ${status.isRecording ? 'bg-primary/10 border-l-4 border-primary' : ''}`}>
          <div className="flex items-center gap-4">
            <div className={`relative w-16 h-16 rounded-2xl flex items-center justify-center ${
              status.isRecording
                ? 'bg-primary/20 animate-pulse'
                : status.isProcessing
                ? 'bg-yellow-500/20'
                : status.isModelLoaded
                ? 'bg-green-500/20'
                : 'bg-muted'
            }`}>
              {status.isRecording ? (
                <Mic className="w-8 h-8 text-primary" />
              ) : status.isProcessing ? (
                <Loader2 className="w-8 h-8 text-yellow-500 animate-spin" />
              ) : status.isModelLoaded ? (
                <CheckCircle className="w-8 h-8 text-green-500" />
              ) : (
                <MicOff className="w-8 h-8 text-muted-foreground" />
              )}
            </div>
            <div>
              <h2 className="text-xl font-semibold">
                {status.isRecording
                  ? 'Listening...'
                  : status.isProcessing
                  ? 'Processing...'
                  : status.isModelLoaded
                  ? 'Ready to Dictate'
                  : 'Model Not Loaded'}
              </h2>
              <p className="text-sm text-muted-foreground">
                {status.isRecording
                  ? `Speak now — release ${formatHotkey(settings.hotkey)} to insert`
                  : status.isProcessing
                  ? 'Transcribing and cleaning up...'
                  : status.isModelLoaded
                  ? `Press and hold ${formatHotkey(settings.hotkey)} to start (${settings.hotkeyMode === 'press-hold' ? 'hold' : 'tap twice'})`
                  : 'Click "Load Model" to download Whisper model'}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {!status.isModelLoaded && (
              <button
                onClick={() => invoke('load_model', { modelId: settings.whisperModel })}
                className="btn-primary"
                disabled={status.isProcessing}
              >
                <Download className="w-4 h-4 mr-2" />
                Load Model
              </button>
            )}
            {status.isModelLoaded && !status.isRecording && !status.isProcessing && (
              <button
                onClick={onStartRecording}
                className="btn-primary bg-green-600 hover:bg-green-700"
              >
                <Mic className="w-4 h-4 mr-2" />
                Start Dictation
              </button>
            )}
            {status.isRecording && (
              <button
                onClick={onStopRecording}
                className="btn-primary bg-red-600 hover:bg-red-700"
              >
                <MicOff className="w-4 h-4 mr-2" />
                Stop & Insert
              </button>
            )}
            {status.isProcessing && (
              <button className="btn-primary" disabled>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Processing...
              </button>
            )}
          </div>
        </div>

        {/* Live Transcript Preview */}
        {status.currentText && (
          <div className="card-base p-6 animate-slide-up">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-medium">Live Transcript</h3>
              {status.aiCleanupEnabled && (
                <span className="flex items-center gap-1 text-xs text-primary bg-primary/10 px-2 py-1 rounded-full">
                  <Sparkles className="w-3 h-3" />
                  AI Enhanced
                </span>
              )}
            </div>
            <div className="p-4 bg-muted/50 rounded-lg min-h-[80px] font-mono text-sm whitespace-pre-wrap">
              {status.currentText}
            </div>
          </div>
        )}

        {/* Quick Stats */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <StatCard
            icon={<Shield className="w-5 h-5" />}
            title="Privacy First"
            value="100% Local"
            desc="Audio never leaves your device"
          />
          <StatCard
            icon={<Zap className="w-5 h-5" />}
            title="Latency"
            value="~500ms"
            desc="Speak → Text appears"
          />
          <StatCard
            icon={<Lock className="w-5 h-5" />}
            title="Encrypted"
            value="AES-256"
            desc="Settings & dictionary encrypted"
          />
        </div>

        {/* Hotkey Guide */}
        <div className="card-base p-6">
          <h3 className="font-semibold mb-4">Hotkey Guide</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <HotkeyCard
              mode="press-hold"
              active={settings.hotkeyMode === 'press-hold'}
              hotkey={settings.hotkey}
              onClick={() => onSaveSettings({ hotkeyMode: 'press-hold' })}
            />
            <HotkeyCard
              mode="toggle"
              active={settings.hotkeyMode === 'toggle'}
              hotkey={settings.hotkey}
              onClick={() => onSaveSettings({ hotkeyMode: 'toggle' })}
            />
          </div>
        </div>

        {/* Error Display */}
        {status.lastError && (
          <div className="card-base border-destructive/50 p-4 bg-destructive/10">
            <div className="flex items-start gap-3">
              <AlertCircle className="w-5 h-5 text-destructive mt-0.5" />
              <div>
                <p className="font-medium text-destructive">Error</p>
                <p className="text-sm text-muted-foreground mt-1">{status.lastError}</p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

function StatCard({ icon, title, value, desc }: { icon: React.ReactNode; title: string; value: string; desc: string }) {
  return (
    <div className="card-base p-5">
      <div className="flex items-center gap-3 mb-2">
        <div className="p-2 bg-primary/10 rounded-lg text-primary">{icon}</div>
        <h4 className="font-medium">{title}</h4>
      </div>
      <p className="text-2xl font-bold">{value}</p>
      <p className="text-sm text-muted-foreground mt-1">{desc}</p>
    </div>
  )
}

function HotkeyCard({ mode, active, hotkey, onClick }: { mode: 'press-hold' | 'toggle'; active: boolean; hotkey: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={`card-base p-4 text-left transition-all ${
        active ? 'border-primary bg-primary/5' : 'hover:border-primary/50'
      }`}
    >
      <div className="flex items-center justify-between mb-2">
        <span className="font-medium capitalize">{mode.replace('-', ' ')}</span>
        {active && <CheckCircle className="w-5 h-5 text-primary" />}
      </div>
      <p className="text-sm text-muted-foreground mb-3">
        {mode === 'press-hold' ? 'Hold to speak, release to insert' : 'Tap to start, tap again to stop'}
      </p>
      <div className="flex items-center gap-2 text-sm font-mono bg-muted px-2 py-1 rounded">
        {hotkey.split('+').map((k, i) => (
          <React.Fragment key={i}>
            <kbd className="px-1.5 py-0.5 bg-background rounded text-xs">{k.charAt(0).toUpperCase() + k.slice(1)}</kbd>
            {i < hotkey.split('+').length - 1 && <span>+</span>}
          </React.Fragment>
        ))}
      </div>
    </button>
  )
}

// Settings View Component
function SettingsView({
  settings,
  onSaveSettings,
  onDownloadModel,
  showKey,
  setShowKey,
}: {
  settings: SettingsData
  onSaveSettings: (s: Partial<SettingsData>) => void
  onDownloadModel: (modelId: string) => void
  showKey: boolean
  setShowKey: (show: boolean) => void
}) {
  return (
    <div className="max-w-3xl mx-auto space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Settings</h1>
        <p className="text-muted-foreground">Configure AuraScribe to match your workflow</p>
      </div>

      {/* Hotkey Settings */}
      <section className="card-base p-6 space-y-4">
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <Zap className="w-5 h-5" />
          Hotkey
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium mb-1">Hotkey Combination</label>
            <div className="input-field bg-muted font-mono text-center" style={{ userSelect: 'all' }}>
              {settings.hotkey}
            </div>
            <p className="text-xs text-muted-foreground mt-1">Click to record new hotkey (coming soon)</p>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Mode</label>
            <select
              value={settings.hotkeyMode}
              onChange={(e) => onSaveSettings({ hotkeyMode: e.target.value as 'press-hold' | 'toggle' })}
              className="input-field"
            >
              <option value="press-hold">Press & Hold — Hold to speak, release to insert</option>
              <option value="toggle">Toggle — Tap to start, tap again to stop</option>
            </select>
          </div>
        </div>
      </section>

      {/* Model Settings */}
      <section className="card-base p-6 space-y-4">
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <Download className="w-5 h-5" />
          Whisper Model
        </h2>
        <p className="text-sm text-muted-foreground">Models download once and run entirely offline</p>
        <div className="space-y-2">
          {WHISPER_MODELS.map((model) => (
            <ModelCard
              key={model.id}
              model={model}
              selected={settings.whisperModel === model.id}
              onSelect={() => onSaveSettings({ whisperModel: model.id })}
              onDownload={() => onDownloadModel(model.id)}
            />
          ))}
        </div>
      </section>

      {/* AI Cleanup Settings */}
      <section className="card-base p-6 space-y-4">
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <Sparkles className="w-5 h-5" />
          AI Cleanup (Optional)
        </h2>
        <p className="text-sm text-muted-foreground">
          Uses OpenRouter free models. Your API key is encrypted locally. Audio never leaves your device.
        </p>

        <div className="flex items-center gap-4">
          <input
            type="checkbox"
            id="ai-cleanup"
            checked={settings.aiCleanupEnabled}
            onChange={(e) => onSaveSettings({ aiCleanupEnabled: e.target.checked })}
            className="w-4 h-4 rounded border-input bg-background text-primary focus:ring-primary"
          />
          <label htmlFor="ai-cleanup" className="font-medium cursor-pointer">
            Enable AI-powered grammar, punctuation & filler removal
          </label>
        </div>

        {settings.aiCleanupEnabled && (
          <div className="space-y-4 pl-10 border-l-2 border-primary/20">
            <div>
              <label className="block text-sm font-medium mb-1">OpenRouter API Key</label>
              <div className="relative">
                <input
                  type={showKey ? 'text' : 'password'}
                  value={settings.openRouterKey}
                  onChange={(e) => onSaveSettings({ openRouterKey: e.target.value })}
                  placeholder="sk-or-v1-..."
                  className="input-field pr-10"
                />
                <button
                  onClick={() => setShowKey(!showKey)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                >
                  {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <p className="text-xs text-muted-foreground mt-1">
                Get a free key at <a href="https://openrouter.ai/keys" target="_blank" rel="noopener" className="underline">openrouter.ai/keys</a>
              </p>
            </div>

            <div>
              <label className="block text-sm font-medium mb-1">Model</label>
              <select
                value={settings.openRouterModel}
                onChange={(e) => onSaveSettings({ openRouterModel: e.target.value })}
                className="input-field"
              >
                {OPENROUTER_MODELS.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name} {m.free ? '✓ Free' : '💰 Paid'}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.autoPunctuation}
                  onChange={(e) => onSaveSettings({ autoPunctuation: e.target.checked })}
                  className="w-4 h-4 rounded border-input bg-background text-primary focus:ring-primary"
                />
                <span>Auto punctuation & capitalization</span>
              </label>
            </div>
          </div>
        )}
      </section>

      {/* Appearance */}
      <section className="card-base p-6 space-y-4">
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <Globe className="w-5 h-5" />
          Appearance & Behavior
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium mb-1">Theme</label>
            <select
              value={settings.theme}
              onChange={(e) => onSaveSettings({ theme: e.target.value as 'light' | 'dark' | 'system' })}
              className="input-field"
            >
              <option value="system">System</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </div>
          <div>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.startAtLogin}
                onChange={(e) => onSaveSettings({ startAtLogin: e.target.checked })}
                className="w-4 h-4 rounded border-input bg-background text-primary focus:ring-primary"
              />
              <span>Start at login</span>
            </label>
          </div>
        </div>
      </section>

      {/* Privacy Notice */}
      <section className="card-base p-6 border-green p-6 border-green-200 dark:border-green-800">
        <div className="flex items-start gap-3">
          <Shield className="w-6 h-6 text-green-600 dark:text-green-400 mt-0.5" />
          <div>
            <h3 className="font-semibold text-green-800 dark:text-green-200">Privacy Guarantee</h3>
            <ul className="text-sm text-green-700 dark:text-green-300 mt-2 space-y-1">
              <li>• Audio processed 100% locally via Whisper.cpp</li>
              <li>• AI cleanup only sends text (not audio) to OpenRouter</li>
              <li>• API key encrypted with AES-256 in local SQLite</li>
              <li>• Zero telemetry, zero analytics, zero tracking</li>
              <li>• Open source — audit the code yourself</li>
            </ul>
          </div>
        </div>
      </section>
    </div>
  )
}

function ModelCard({ model, selected, onSelect, onDownload }: { model: typeof WHISPER_MODELS[0]; selected: boolean; onSelect: () => void; onDownload: () => void }) {
  return (
    <button
      onClick={onSelect}
      className={`card-base p-4 flex items-center justify-between transition-all ${
        selected ? 'border-primary bg-primary/5' : 'hover:border-primary/50'
      }`}
    >
      <div className="flex items-center gap-4">
        <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${selected ? 'bg-primary/20 text-primary' : 'bg-muted text-muted-foreground'}`}>
          <Download className="w-5 h-5" />
        </div>
        <div>
          <div className="flex items-center gap-2">
            <span className="font-medium">{model.id}</span>
            {model.recommended && <span className="text-xs bg-primary/10 text-primary px-1.5 py-0.5 rounded">Recommended</span>}
          </div>
          <p className="text-sm text-muted-foreground">{model.size} • {model.speed} • {model.quality} quality</p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        {selected ? (
          <CheckCircle className="w-5 h-5 text-primary" />
        ) : (
          <button onClick={(e) => { e.stopPropagation(); onDownload(); }} className="btn-ghost text-sm">
            Download
          </button>
        )}
      </div>
    </button>
  )
}

// About View Component
function AboutView() {
  return (
    <div className="max-w-3xl mx-auto space-y-6">
      <div className="text-center">
        <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-primary to-primary/70 flex items-center justify-center mx-auto mb-4">
          <Mic className="w-10 h-10 text-white" />
        </div>
        <h1 className="text-3xl font-bold">AuraScribe</h1>
        <p className="text-muted-foreground mt-2">Your voice. Everywhere. Free forever.</p>
      </div>

      <div className="card-base p-6 space-y-4">
        <h2 className="text-lg font-semibold">What is AuraScribe?</h2>
        <p className="text-muted-foreground">
          AuraScribe is a free, open-source voice input layer that sits on top of your operating system.
          Speak naturally anywhere you'd type — VS Code, Notion, Slack, Email, Terminal, Browser — and watch
          your words appear instantly with AI-powered cleanup.
        </p>
      </div>

      <div className="card-base p-6 space-y-4">
        <h2 className="text-lg font-semibold">Core Principles</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <PrincipleCard icon={<Lock />} title="Privacy First" desc="100% local processing. Your audio never leaves your device. AI cleanup is opt-in only." />
          <PrincipleCard icon={<Github />} title="Open Source" desc="MIT licensed. Community-driven. No vendor lock-in. Fork it, modify it, self-host it." />
          <PrincipleCard icon={<Zap />} title="Zero Latency" desc="Streaming Whisper.cpp + Silero VAD. Text appears as you speak, not after." />
          <PrincipleCard icon={<Shield />} title="Secure by Default" desc="Encrypted settings, no telemetry, minimal permissions, code-signed releases." />
        </div>
      </div>

      <div className="card-base p-6 space-y-4">
        <h2 className="text-lg font-semibold">Tech Stack</h2>
        <div className="flex flex-wrap gap-2">
          {['Tauri', 'Rust', 'Next.js', 'React', 'Whisper.cpp', 'Silero VAD', 'OpenRouter', 'SQLCipher', 'Tailwind CSS'].map((tech) => (
            <span key={tech} className="px-3 py-1 bg-muted rounded-full text-sm font-medium">{tech}</span>
          ))}
        </div>
      </div>

      <div className="card-base p-6 text-center">
        <h2 className="text-lg font-semibold mb-2">Built with the community</h2>
        <p className="text-muted-foreground mb-4">Join us on GitHub — contribute, report issues, request features</p>
        <a
          href="https://github.com/aurascribe/aurascribe"
          target="_blank"
          rel="noopener noreferrer"
          className="btn-primary inline-flex items-center gap-2"
        >
          <Github className="w-4 h-4" />
          View on GitHub
        </a>
      </div>

      <div className="text-center text-sm text-muted-foreground">
        <p>AuraScribe v1.0.0 • MIT License • Made with ❤️ by contributors worldwide</p>
      </div>
    </div>
  )
}

function PrincipleCard({ icon, title, desc }: { icon: React.ReactNode; title: string; desc: string }) {
  return (
    <div className="card-base p-4">
      <div className="p-2 bg-primary/10 rounded-lg text-primary w-fit mb-3">{icon}</div>
      <h3 className="font-semibold mb-1">{title}</h3>
      <p className="text-sm text-muted-foreground">{desc}</p>
    </div>
  )
}

// Missing icons
import { Sun, Moon, Eye, EyeOff } from 'lucide-react'