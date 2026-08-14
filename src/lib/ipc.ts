// Tauri IPC wrappers for type-safe communication with the Rust backend

import { invoke } from '@tauri-apps/api/core'
import { listen, emit } from '@tauri-apps/api/event'

export interface Settings {
  hotkey: string
  hotkey_mode: 'press-hold' | 'toggle'
  whisper_model: string
  mic_device: string | null
  ai_cleanup_enabled: boolean
  remove_fillers: boolean
  language: string
  /** 'glass' = light palette on the indigo-glow backdrop (the floating look). */
  theme: 'light' | 'dark' | 'system' | 'glass'
  start_at_login: boolean
  sound_cues: boolean
  /** False until the first-run walkthrough is dismissed; the UI shows onboarding while false. */
  onboarded: boolean
}

export interface Status {
  is_recording: boolean
  is_processing: boolean
  is_model_loaded: boolean
  /** The model actually in memory. Authoritative — trust over the saved setting. */
  loaded_model: string | null
  current_text: string
  last_error: string | null
  hotkey_mode: 'press-hold' | 'toggle'
  ai_cleanup_enabled: boolean
}

export interface ModelInfo {
  id: string
  name: string
  /** Which backend runs this model. 'whisper' is v1; the rest run on sherpa-onnx. */
  engine: 'whisper' | 'moonshine' | 'parakeet' | 'dolphin' | 'nemoctc'
  size_mb: number
  multilingual: boolean
  /** 1 = fastest */
  speed: number
  /** 5 = most accurate */
  accuracy: number
  /** Best choice on THIS machine, computed from measured speed — not a static flag. */
  recommended: boolean
  downloaded: boolean
  path: string | null
  /** Processing time as a multiple of speech length here. Below 1.0 beats real time. */
  realtime_factor: number
  /** Present when this model would be a bad experience on this hardware. */
  warning: string | null
}

export interface UsageStats {
  total_dictations: number
  total_words: number
  words_today: number
  words_per_minute: number
  total_audio_ms: number
  active_days: number
}

export interface StreakInfo {
  /** Current streak, including today if it already crossed the daily word threshold. */
  streak: number
  longest: number
  freezes: number
  max_freezes: number
  /** Counted days still needed to bank the next freeze; 0 when the bank is full. */
  days_to_next_freeze: number
  today_counted: boolean
  words_today: number
  min_words_per_day: number
}

export interface DictionaryEntry {
  id: number
  word: string
  replacement: string
  case_sensitive: number
  whole_word: number
  created_at: number
}

export interface SnippetEntry {
  id: number
  trigger: string
  expansion: string
  description: string | null
  created_at: number
}

export interface TranscriptEntry {
  id: number
  timestamp: number
  raw_text: string
  cleaned_text: string | null
  app_name: string | null
  duration_ms: number
  audio_ms: number
  model_used: string | null
  created_at: number
}

// Settings
export async function getSettings(): Promise<Settings> {
  return invoke('get_settings')
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke('save_settings', { settings })
}

// Status
export async function getStatus(): Promise<Status> {
  return invoke('get_status')
}

// Recording
export async function startRecording(): Promise<void> {
  return invoke('start_recording')
}

export async function stopRecording(): Promise<void> {
  return invoke('stop_recording')
}

// Model management
export async function loadModel(modelId: string): Promise<void> {
  return invoke('load_model', { modelId })
}

export async function downloadModel(modelId: string): Promise<void> {
  return invoke('download_model', { modelId })
}

export async function getDownloadedModels(): Promise<string[]> {
  return invoke('get_downloaded_models')
}

export async function getAvailableModels(): Promise<ModelInfo[]> {
  return invoke('get_available_models')
}

export async function deleteModel(modelId: string): Promise<void> {
  return invoke('delete_model', { modelId })
}

// Dictionary
export async function getDictionary(): Promise<DictionaryEntry[]> {
  return invoke('get_dictionary')
}

export async function addDictionaryEntry(entry: {
  word: string
  replacement: string
  caseSensitive?: boolean
  wholeWord?: boolean
}): Promise<number> {
  return invoke('add_dictionary_entry', { entry })
}

export async function deleteDictionaryEntry(id: number): Promise<void> {
  return invoke('delete_dictionary_entry', { id })
}

// Snippets
export async function getSnippets(): Promise<SnippetEntry[]> {
  return invoke('get_snippets')
}

export async function addSnippet(trigger: string, expansion: string, description?: string): Promise<number> {
  return invoke('add_snippet', { trigger, expansion, description: description ?? null })
}

export async function deleteSnippet(id: number): Promise<void> {
  return invoke('delete_snippet', { id })
}

// Transcripts
export async function getTranscripts(limit = 100, offset = 0): Promise<TranscriptEntry[]> {
  return invoke('get_transcripts', { limit, offset })
}

export async function clearTranscripts(): Promise<void> {
  return invoke('clear_transcripts')
}

export interface DailyCount {
  /** Local date as `YYYY-MM-DD`. */
  day: string
  count: number
}

/** Per-day dictation counts over the last `days` days, for the History usage heatmap. */
export async function transcriptDailyCounts(days = 371): Promise<DailyCount[]> {
  return invoke('transcript_daily_counts', { days })
}

/** Delete transcripts within `[start, end]` (unix seconds, inclusive). Returns rows removed. */
export async function deleteTranscriptsBetween(start: number, end: number): Promise<number> {
  return invoke('delete_transcripts_between', { start, end })
}

export async function getStats(): Promise<UsageStats> {
  return invoke('get_stats')
}

export async function getStreakState(): Promise<StreakInfo> {
  return invoke('get_streak_state')
}

// System
export async function setStartAtLogin(enabled: boolean): Promise<void> {
  return invoke('set_start_at_login', { enabled })
}

export async function openSettingsFolder(): Promise<void> {
  return invoke('open_settings_folder')
}

export async function listAudioDevices(): Promise<string[]> {
  return invoke('list_audio_devices')
}

export async function checkMicrophonePermission(): Promise<boolean> {
  return invoke('check_microphone_permission')
}

export async function requestMicrophonePermission(): Promise<boolean> {
  return invoke('request_microphone_permission')
}

export async function checkAccessibilityPermission(): Promise<boolean> {
  return invoke('check_accessibility_permission')
}

export async function requestAccessibilityPermission(): Promise<void> {
  return invoke('request_accessibility_permission')
}

/** Tells the backend the overlay page mounted. Until this lands the overlay stays hidden. */
export async function overlayReady(): Promise<void> {
  return invoke('overlay_ready')
}

// Events
export async function onStatusChanged(callback: (status: Status) => void) {
  return listen<Status>('status-changed', (e) => callback(e.payload))
}

export async function onSettingsChanged(callback: (settings: Settings) => void) {
  return listen<Settings>('settings-changed', (e) => callback(e.payload))
}

export async function onTranscriptReceived(callback: (text: string) => void) {
  return listen<string>('transcript-received', (e) => callback(e.payload))
}

export async function onModelDownloadProgress(
  callback: (payload: { modelId: string; progress: number }) => void
) {
  return listen<{ modelId: string; progress: number }>('model-download-progress', (e) => callback(e.payload))
}

// Emit to Rust
export async function emitToRust(event: string, payload: unknown) {
  return emit(event, payload)
}
