import * as ipc from './ipc'

/**
 * A shareable card, rendered entirely on-device with the Canvas API (no dependency, no upload),
 * then handed to the backend to save as a PNG. Used for the streak and the yearly recap.
 */
export interface ShareCardSpec {
  filename: string
  /** Small top label, e.g. "AuraScribe · Your 2026". */
  kicker: string
  /** The big number, e.g. "6.5 hr" or "10". */
  headline: string
  /** One line under the headline. */
  headlineSub: string
  /** Up to 4 supporting stats. */
  stats: { value: string; label: string }[]
  footer?: string
}

// Instrument palette (matches the app's dark tokens). System fonts so canvas needs no webfont.
const SANS = 'system-ui, -apple-system, Segoe UI, sans-serif'
const MONO = 'ui-monospace, "SF Mono", Menlo, Consolas, monospace'

export async function renderAndSaveCard(spec: ShareCardSpec): Promise<string> {
  const S = 1080
  const canvas = document.createElement('canvas')
  canvas.width = S
  canvas.height = S
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('Canvas is not available')

  const cyan = '#3ECEDE'
  const fg = '#E7ECF2'
  const faint = '#8A93A0'
  const M = 100

  // Background + hairline frame.
  ctx.fillStyle = '#0E1116'
  ctx.fillRect(0, 0, S, S)
  ctx.strokeStyle = '#242A33'
  ctx.lineWidth = 2
  ctx.strokeRect(44, 44, S - 88, S - 88)

  // Wordmark (three rising bars) + kicker.
  const by = 150
  ctx.fillStyle = fg
  ;[28, 50, 18].forEach((h, i) => ctx.fillRect(M + i * 18, by + (50 - h), 11, h))
  ctx.fillStyle = faint
  ctx.font = `600 30px ${SANS}`
  ctx.textBaseline = 'alphabetic'
  ctx.fillText(spec.kicker, M + 78, by + 36)

  // Headline (mono, tabular feel) + accent underline + sub.
  ctx.fillStyle = fg
  ctx.font = `700 150px ${MONO}`
  ctx.fillText(spec.headline, M, 420)
  ctx.fillStyle = cyan
  ctx.fillRect(M, 452, 130, 8)
  ctx.fillStyle = faint
  ctx.font = `400 40px ${SANS}`
  ctx.fillText(spec.headlineSub, M, 522)

  // Supporting stats, two columns.
  const cols = 2
  const cw = (S - 2 * M) / cols
  const gy = 660
  spec.stats.slice(0, 4).forEach((s, i) => {
    const cx = M + (i % cols) * cw
    const cy = gy + Math.floor(i / cols) * 140
    ctx.fillStyle = fg
    ctx.font = `700 66px ${MONO}`
    ctx.fillText(s.value, cx, cy)
    ctx.fillStyle = faint
    ctx.font = `500 28px ${SANS}`
    ctx.fillText(s.label, cx, cy + 40)
  })

  // Footer.
  ctx.fillStyle = cyan
  ctx.font = `600 30px ${SANS}`
  ctx.fillText(spec.footer ?? '100% offline · free · open source', M, S - 100)

  const blob = await new Promise<Blob | null>((res) => canvas.toBlob(res, 'image/png'))
  if (!blob) throw new Error('Could not render the card')
  const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()))
  return ipc.saveShareImage(spec.filename, bytes)
}
