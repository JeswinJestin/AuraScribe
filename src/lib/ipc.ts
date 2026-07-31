// Tauri IPC wrappers for type-safe communication with Rust backend

import { invoke } from '@tauri-apps/api/core'
import { listen, emit } from '@tauri-apps/api/event'

// Types matching Rust backend
export interface Settings {
  hotkey: string
  hotkey_mode: 'press-hold' | 'toggle'
  whisper_model: string
  openrouter_key: string
  openrouter_model: string
  ai_cleanup_enabled: boolean
  auto_punctuation: boolean
  language: string
  theme: 'light' | 'dark' | 'system'
  start_at_login: boolean
}

export interface Status {
  is_recording: boolean
  is_processing: boolean
  is_model_loaded: boolean
  current_text: string
  last_error: string | null
  hotkey_mode: 'press-hold' | 'toggle'
  ai_cleanup_enabled: boolean
}

export interface AppProfile {
  id: string
  name: string
  executable: string
  window_title_pattern?: string
  style: 'casual' | 'formal' | 'code' | 'technical' | 'auto'
  custom_prompt?: string
  dictionary_words: string[]
  snippets: Record<string, string>
  ai_cleanup_enabled: boolean
  openrouter_model?: string
}

export interface DictionaryEntry {
  id: number
  word: string
  replacement: string
  case_sensitive: boolean
  created_at: string
}

export interface SnippetEntry {
  id: number
  trigger: string
  expansion: string
  description?: string
  created_at: string
}

export interface TranscriptEntry {
  id: number
  raw_text: string
  cleaned_text: string
  app_profile?: string
  model_used: string
  processing_time_ms: number
  created_at: string
}

// Settings
export async function getSettings(): Promise<Settings> {
  return invoke('get_settings')
}

export async function saveSettings(settings: Partial<Settings>): Promise<void> {
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

export async function deleteModel(modelId: string): Promise<void> {
  return invoke('delete_model', { modelId })
}

// Dictionary
export async function getDictionary(): Promise<DictionaryEntry[]> {
  return invoke('get_dictionary')
}

export async function addDictionaryEntry(entry: Omit<DictionaryEntry, 'id' | 'created_at'>): Promise<number> {
  return invoke('add_dictionary_entry', { entry })
}

export async function updateDictionaryEntry(id: number, entry: Partial<DictionaryEntry>): Promise<void> {
  return invoke('update_dictionary_entry', { id, entry })
}

export async function deleteDictionaryEntry(id: number): Promise<void> {
  return invoke('delete_dictionary_entry', { id })
}

// Snippets
export async function getSnippets(): Promise<SnippetEntry[]> {
  return invoke('get_snippets')
}

export async function addSnippet(snippet: Omit<SnippetEntry, 'id' | 'created_at'>): Promise<number> {
  return invoke('add_snippet', { snippet })
}

export async function updateSnippet(id: number, snippet: Partial<SnippetEntry>): Promise<void> {
  return invoke('update_snippet', { id, snippet })
}

export async function deleteSnippet(id: number): Promise<void> {
  return invoke('delete_snippet', { id })
}

// App Profiles
export async function getAppProfiles(): Promise<AppProfile[]> {
  return invoke('get_app_profiles')
}

export async function addAppProfile(profile: Omit<AppProfile, 'id'>): Promise<string> {
  return invoke('add_app_profile', { profile })
}

export async function updateAppProfile(id: string, profile: Partial<AppProfile>): Promise<void> {
  return invoke('update_app_profile', { id, profile })
}

export async function deleteAppProfile(id: string): Promise<void> {
  return invoke('delete_app_profile', { id })
}

// Transcripts
export async function getTranscripts(limit = 100, offset = 0): Promise<TranscriptEntry[]> {
  return invoke('get_transcripts', { limit, offset })
}

export async function clearTranscripts(): Promise<void> {
  return invoke('clear_transcripts')
}

// System
export async function setStartAtLogin(enabled: boolean): Promise<void> {
  return invoke('set_start_at_login', { enabled })
}

export async function openSettingsFolder(): Promise<void> {
  return invoke('open_settings_folder')
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

export async function onError(callback: (error: string) => void) {
  return listen<string>('error', (e) => callback(e.payload))
}

// Emit to Rust
export async function emitToRust(event: string, payload: unknown) {
  return emit(event, payload)
}

// Cleanup
export async function cleanupWithAI(
  text: string,
  style: string,
  openRouterKey: string,
  openRouterModel: string
): Promise<string> {
  return invoke('cleanup_with_ai', { text, style, openRouterKey, openRouterModel })
}