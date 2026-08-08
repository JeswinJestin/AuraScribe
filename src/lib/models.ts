// Presentation layer for models: AuraScribe-branded names, the open engine that powers each one
// (credited, as the licenses ask), and the full list of languages a model covers — so a user can
// tell exactly what a model offers just by reading its card.

import type { ModelInfo } from './ipc'

/** Parakeet v3 — 25 European languages, auto-detected. */
export const EUROPEAN_LANGS =
  'English, German, French, Spanish, Italian, Portuguese, Dutch, Polish, Russian, Ukrainian, ' +
  'Greek, Czech, Swedish, Danish, Finnish, Hungarian, Romanian, Bulgarian, Croatian, Slovak, ' +
  'Slovenian, Estonian, Latvian, Lithuanian, Maltese'

/** Dolphin — ~40 Asian/Eastern languages, auto-detected (Malayalam & Kannada are NOT included). */
export const ASIAN_LANGS =
  'Hindi, Tamil, Telugu, Bengali, Urdu, Marathi, Gujarati, Punjabi, Odia, Nepali, Sinhala, ' +
  'Chinese, Japanese, Korean, Thai, Vietnamese, Indonesian, Malay, Filipino, Khmer, Lao, Burmese, ' +
  'Arabic, Persian, Kazakh, Uzbek, Mongolian, and more (~40 in total)'

export interface ModelDisplay {
  /** The AuraScribe-branded name shown to the user. */
  title: string
  /** The open model that powers it — credited under the name. */
  poweredBy: string
  /** How language selection works. */
  detects: 'auto' | 'single' | 'english'
  /** The languages this model covers, in plain text. */
  languages: string
}

/** Map a backend model to how it should be presented. */
export function modelDisplay(m: ModelInfo): ModelDisplay {
  switch (m.engine) {
    case 'moonshine':
      return {
        title: m.id.includes('tiny') ? 'AuraScribe English Mini' : 'AuraScribe English',
        poweredBy: 'Moonshine',
        detects: 'english',
        languages: 'English only — the fastest, most accurate choice for English.',
      }
    case 'parakeet':
      return {
        title: 'AuraScribe European',
        poweredBy: 'NVIDIA Parakeet',
        detects: 'auto',
        languages: `Auto-detects 25 European languages: ${EUROPEAN_LANGS}.`,
      }
    case 'dolphin':
      return {
        title: 'AuraScribe Asian',
        poweredBy: 'Dolphin',
        detects: 'auto',
        languages: `Auto-detects ~40 Asian languages: ${ASIAN_LANGS}. (Not Malayalam or Kannada — those have their own models.)`,
      }
    case 'nemoctc': {
      const lang = m.id.endsWith('-ml') ? 'Malayalam' : m.id.endsWith('-kn') ? 'Kannada' : 'an Indian language'
      return {
        title: `AuraScribe ${lang}`,
        poweredBy: 'AI4Bharat IndicConformer',
        detects: 'single',
        languages: `${lang} only — dedicated, accurate model for ${lang}.`,
      }
    }
    default:
      return {
        title: m.name || m.id,
        poweredBy: 'Whisper',
        detects: m.multilingual ? 'auto' : 'english',
        languages: m.multilingual ? 'Multiple languages.' : 'English only.',
      }
  }
}

/** Short name for compact places (Dictate screen, sidebar). */
export function modelShortName(loadedId: string | null, models: ModelInfo[]): string {
  if (!loadedId) return ''
  const m = models.find((x) => x.id === loadedId)
  return m ? modelDisplay(m).title : loadedId
}
