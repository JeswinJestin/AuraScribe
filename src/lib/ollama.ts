// AI Cleanup integration with OpenRouter (free models) and Ollama (local)

export interface OpenRouterModel {
  id: string
  name: string
  free: boolean
  contextLength: number
}

export interface CleanupOptions {
  text: string
  style?: 'casual' | 'formal' | 'code' | 'technical' | 'auto'
  language?: string
  autoPunctuation?: boolean
  removeFillers?: boolean
  fixGrammar?: boolean
  customPrompt?: string
}

export interface CleanupResult {
  cleaned: string
  original: string
  modelUsed: string
  processingTimeMs: number
}

// Free OpenRouter models
export const FREE_OPENROUTER_MODELS: OpenRouterModel[] = [
  { id: 'nvidia/nemotron-3-ultra', name: 'Nemotron 3 Ultra', free: true, contextLength: 4096 },
  { id: 'meta-llama/llama-3.1-8b-instruct:free', name: 'Llama 3.1 8B Instruct', free: true, contextLength: 8192 },
  { id: 'google/gemma-2-9b-it:free', name: 'Gemma 2 9B IT', free: true, contextLength: 8192 },
  { id: 'mistralai/mistral-7b-instruct:free', name: 'Mistral 7B Instruct', free: true, contextLength: 8192 },
  { id: 'openchat/openchat-7b:free', name: 'OpenChat 7B', free: true, contextLength: 8192 },
]

// Paid models for reference
export const PAID_OPENROUTER_MODELS: OpenRouterModel[] = [
  { id: 'openai/gpt-4o-mini', name: 'GPT-4o Mini', free: false, contextLength: 128000 },
  { id: 'anthropic/claude-3-haiku', name: 'Claude 3 Haiku', free: false, contextLength: 200000 },
  { id: 'openai/gpt-4o', name: 'GPT-4o', free: false, contextLength: 128000 },
]

// Build system prompt based on style
function buildSystemPrompt(options: CleanupOptions): string {
  const { style = 'auto', language = 'en', autoPunctuation = true, removeFillers = true, fixGrammar = true, customPrompt } = options

  if (customPrompt) {
    return customPrompt
  }

  const stylePrompts: Record<string, string> = {
    casual: 'Write in a casual, conversational tone. Use contractions. Be friendly and natural.',
    formal: 'Write in a professional, formal tone. Avoid contractions. Use complete sentences.',
    code: 'Preserve all code formatting, variable names (camelCase, snake_case), function names, and technical terms exactly. Do not modify code snippets.',
    technical: 'Use precise technical terminology. Preserve acronyms, product names, and technical jargon. Be concise and accurate.',
    auto: 'Auto-detect the appropriate style based on context. Default to clear, natural English.',
  }

  let prompt = `You are a text cleanup assistant. Your job is to take raw speech-to-text output and transform it into clean, well-formatted text.

Rules:
${autoPunctuation ? '- Add proper punctuation (periods, commas, question marks)' : ''}
${removeFillers ? '- Remove filler words (um, uh, like, you know, actually, basically, literally)' : ''}
${fixGrammar ? '- Fix grammar, syntax, and sentence structure' : ''}
- Do NOT change the meaning or add information not in the original
- Preserve the speaker's intent and tone
- Output ONLY the cleaned text, no explanations
- Language: ${language}

Style: ${stylePrompts[style]}

Examples:
Raw: "um so i think we should like maybe schedule the meeting for next tuesday if that works for everyone"
Clean: "I think we should schedule the meeting for next Tuesday if that works for everyone."

Raw: "the function getUserData returns a promise that resolves to the user object"
Clean: "The function \`getUserData\` returns a Promise that resolves to the user object."

Raw: "hey team quick update the deploy failed because of a timeout error in the ci pipeline"
Clean: "Hey team, quick update: the deploy failed because of a timeout error in the CI pipeline."`

  return prompt
}

export async function cleanupWithOpenRouter(
  apiKey: string,
  model: string,
  options: CleanupOptions
): Promise<CleanupResult> {
  const startTime = Date.now()

  const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${apiKey}`,
      'Content-Type': 'application/json',
      'HTTP-Referer': 'https://github.com/aurascribe/aurascribe',
      'X-Title': 'AuraScribe',
    },
    body: JSON.stringify({
      model,
      messages: [
        { role: 'system', content: buildSystemPrompt(options) },
        { role: 'user', content: options.text },
      ],
      temperature: 0.1,
      max_tokens: Math.min(options.text.length * 2, 4096),
    }),
  })

  if (!response.ok) {
    const error = await response.json().catch(() => ({}))
    throw new Error(`OpenRouter API error: ${response.status} - ${error.error?.message || response.statusText}`)
  }

  const data = await response.json()
  const cleaned = data.choices[0]?.message?.content?.trim() || options.text

  return {
    cleaned,
    original: options.text,
    modelUsed: model,
    processingTimeMs: Date.now() - startTime,
  }
}

// Local Ollama integration (optional)
export async function cleanupWithOllama(
  baseUrl: string,
  model: string,
  options: CleanupOptions
): Promise<CleanupResult> {
  const startTime = Date.now()

  const response = await fetch(`${baseUrl}/api/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model,
      messages: [
        { role: 'system', content: buildSystemPrompt(options) },
        { role: 'user', content: options.text },
      ],
      stream: false,
      options: { temperature: 0.1 },
    }),
  })

  if (!response.ok) {
    throw new Error(`Ollama API error: ${response.status}`)
  }

  const data = await response.json()
  const cleaned = data.message?.content?.trim() || options.text

  return {
    cleaned,
    original: options.text,
    modelUsed: `ollama:${model}`,
    processingTimeMs: Date.now() - startTime,
  }
}

// Auto-detect best cleanup method
export async function cleanupText(
  text: string,
  options: CleanupOptions & {
    openRouterKey?: string
    openRouterModel?: string
    ollamaUrl?: string
    ollamaModel?: string
    preferLocal?: boolean
  }
): Promise<CleanupResult> {
  // Try local Ollama first if preferred and available
  if (options.preferLocal && options.ollamaUrl && options.ollamaModel) {
    try {
      return await cleanupWithOllama(options.ollamaUrl, options.ollamaModel, options)
    } catch (e) {
      console.warn('Ollama cleanup failed, falling back to OpenRouter:', e)
    }
  }

  // Try OpenRouter
  if (options.openRouterKey && options.openRouterModel) {
    return cleanupWithOpenRouter(options.openRouterKey, options.openRouterModel, options)
  }

  // No AI cleanup available, return basic cleanup
  return basicCleanup(text, options)
}

// Basic cleanup without AI (always available)
export function basicCleanup(text: string, options: CleanupOptions): CleanupResult {
  let cleaned = text
  const startTime = Date.now()

  if (options.removeFillers !== false) {
    // Remove common fillers
    cleaned = cleaned.replace(/\b(um|uh|like|you know|actually|basically|literally|so|well|i mean|sort of|kind of)\b/gi, '')
  }

  if (options.autoPunctuation !== false) {
    // Basic punctuation
    cleaned = cleaned
      .replace(/\s+/g, ' ')
      .replace(/\.+/g, '.')
      .replace(/\?+/g, '?')
      .replace(/!+/g, '!')
      .replace(/([.!?])\s*([a-z])/g, (_, p, c) => `${p} ${c.toUpperCase()}`)
  }

  if (options.fixGrammar !== false) {
    // Basic capitalization
    cleaned = cleaned.charAt(0).toUpperCase() + cleaned.slice(1)
    // Fix "i " -> "I "
    cleaned = cleaned.replace(/\bi\b/g, 'I')
  }

  // Clean up extra spaces
  cleaned = cleaned.replace(/\s+/g, ' ').trim()

  return {
    cleaned,
    original: text,
    modelUsed: 'basic',
    processingTimeMs: Date.now() - startTime,
  }
}