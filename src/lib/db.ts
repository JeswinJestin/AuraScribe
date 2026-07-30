// Database wrapper using Tauri SQL plugin (SQLite with SQLCipher encryption)

import { invoke } from '@tauri-apps/api/core'

export interface DictionaryEntry {
  id?: number
  word: string
  replacement: string
  case_sensitive: boolean
  whole_word: boolean
  created_at: number
  updated_at: number
}

export interface SnippetEntry {
  id?: number
  trigger: string
  expansion: string
  description?: string
  created_at: number
  updated_at: number
}

export interface AppProfile {
  id?: number
  app_name: string
  app_identifier?: string
  style: 'casual' | 'formal' | 'code' | 'technical' | 'custom'
  custom_prompt?: string
  ai_cleanup: boolean
  auto_punctuation: boolean
  created_at: number
  updated_at: number
}

export interface TranscriptEntry {
  id?: number
  timestamp: number
  raw_text: string
  cleaned_text?: string
  app_name?: string
  duration_ms: number
  model_used: string
  created_at: number
}

export interface SettingsRow {
  key: string
  value: string
  encrypted: boolean
  updated_at: number
}

// Initialize database (called from Rust backend)
export async function initDatabase(masterKey: string): Promise<void> {
  await invoke('init_database', { masterKey })
}

// Settings
export async function getSetting(key: string): Promise<string | null> {
  return invoke('get_setting', { key })
}

export async function setSetting(key: string, value: string, encrypted = false): Promise<void> {
  await invoke('set_setting', { key, value, encrypted })
}

export async function getAllSettings(): Promise<Record<string, string>> {
  return invoke('get_all_settings')
}

// Dictionary
export async function getDictionary(): Promise<DictionaryEntry[]> {
  return invoke('get_dictionary')
}

export async function addDictionaryEntry(entry: Omit<DictionaryEntry, 'id' | 'created_at' | 'updated_at'>): Promise<number> {
  return invoke('add_dictionary_entry', { entry })
}

export async function updateDictionaryEntry(id: number, entry: Partial<DictionaryEntry>): Promise<void> {
  await invoke('update_dictionary_entry', { id, entry })
}

export async function deleteDictionaryEntry(id: number): Promise<void> {
  await invoke('delete_dictionary_entry', { id })
}

// Snippets
export async function getSnippets(): Promise<SnippetEntry[]> {
  return invoke('get_snippets')
}

export async function addSnippet(snippet: Omit<SnippetEntry, 'id' | 'created_at' | 'updated_at'>): Promise<number> {
  return invoke('add_snippet', { snippet })
}

export async function updateSnippet(id: number, snippet: Partial<SnippetEntry>): Promise<void> {
  await invoke('update_snippet', { id, snippet })
}

export async function deleteSnippet(id: number): Promise<void> {
  await invoke('delete_snippet', { id })
}

// App Profiles
export async function getAppProfiles(): Promise<AppProfile[]> {
  return invoke('get_app_profiles')
}

export async function addAppProfile(profile: Omit<AppProfile, 'id' | 'created_at' | 'updated_at'>): Promise<number> {
  return invoke('add_app_profile', { profile })
}

export async function updateAppProfile(id: number, profile: Partial<AppProfile>): Promise<void> {
  await invoke('update_app_profile', { id, profile })
}

export async function deleteAppProfile(id: number): Promise<void> {
  await invoke('delete_app_profile', { id })
}

// Transcripts (history)
export async function getTranscripts(limit = 100, offset = 0): Promise<TranscriptEntry[]> {
  return invoke('get_transcripts', { limit, offset })
}

export async function addTranscript(transcript: Omit<TranscriptEntry, 'id' | 'created_at'>): Promise<number> {
  return invoke('add_transcript', { transcript })
}

export async function clearTranscripts(): Promise<void> {
  await invoke('clear_transcripts')
}

// Models
export async function getDownloadedModels(): Promise<string[]> {
  return invoke('get_downloaded_models')
}

export async function deleteModel(modelId: string): Promise<void> {
  await invoke('delete_model', { modelId })
}

// Backup/Export
export async function exportData(): Promise<string> {
  return invoke('export_data')
}

export async function importData(data: string): Promise<void> {
  await invoke('import_data', { data })
}

// Database maintenance
export async function vacuumDatabase(): Promise<void> {
  await invoke('vacuum_database')
}

export async function getDatabaseSize(): Promise<number> {
  return invoke('get_database_size')
}