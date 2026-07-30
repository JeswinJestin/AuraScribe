// Encryption utilities using Web Crypto API (AES-GCM 256)

const ALGORITHM = 'AES-GCM'
const KEY_LENGTH = 256
const IV_LENGTH = 12
const SALT_LENGTH = 16
const ITERATIONS = 100000

export interface EncryptedData {
  data: string // base64
  iv: string // base64
  salt: string // base64
}

async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
  const encoder = new TextEncoder()
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  )

  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations: ITERATIONS,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: ALGORITHM, length: KEY_LENGTH },
    false,
    ['encrypt', 'decrypt']
  )
}

export async function encrypt(plaintext: string, password: string): Promise<EncryptedData> {
  const encoder = new TextEncoder()
  const data = encoder.encode(plaintext)

  // Generate random salt and IV
  const salt = crypto.getRandomValues(new Uint8Array(SALT_LENGTH))
  const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH))

  const key = await deriveKey(password, salt)
  const encrypted = await crypto.subtle.encrypt({ name: ALGORITHM, iv }, key, data)

  return {
    data: btoa(String.fromCharCode(...new Uint8Array(encrypted))),
    iv: btoa(String.fromCharCode(...iv)),
    salt: btoa(String.fromCharCode(...salt)),
  }
}

export async function decrypt(encrypted: EncryptedData, password: string): Promise<string> {
  const decoder = new TextDecoder()

  const iv = Uint8Array.from(atob(encrypted.iv), (c) => c.charCodeAt(0))
  const salt = Uint8Array.from(atob(encrypted.salt), (c) => c.charCodeAt(0))
  const data = Uint8Array.from(atob(encrypted.data), (c) => c.charCodeAt(0))

  const key = await deriveKey(password, salt)
  const decrypted = await crypto.subtle.decrypt({ name: ALGORITHM, iv }, key, data)

  return decoder.decode(decrypted)
}

// Generate a secure random password for database encryption
export function generateSecurePassword(): string {
  const array = new Uint8Array(32)
  crypto.getRandomValues(array)
  return btoa(String.fromCharCode(...array))
}

// Store password in secure platform-specific storage (Keychain, Credential Manager, Secret Service)
// For Tauri, we use the plugin-store with encryption
export const STORAGE_KEY = 'aurascribe_master_key'

export async function getOrCreateMasterKey(): Promise<string> {
  // In Tauri, this would use the secure store
  // For now, generate and store in localStorage (dev only)
  if (typeof window !== 'undefined') {
    let key = localStorage.getItem(STORAGE_KEY)
    if (!key) {
      key = generateSecurePassword()
      localStorage.setItem(STORAGE_KEY, key)
    }
    return key
  }
  // Server-side (Rust) will handle this differently
  return generateSecurePassword()
}