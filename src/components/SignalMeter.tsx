'use client'

/**
 * The signal meter — AuraScribe's one signature element, restyled for the warm-glass system.
 *
 * A row of ticks sitting on the frosted stage. Short and dim when idle; the ticks pulse in
 * warm red while listening; a soft indigo opacity-sweep while transcribing. It answers the
 * only question that matters mid-dictation — "is this thing hearing me?" — without words, and
 * it is the only element in the interface allowed real motion.
 */

// Fixed heights (px) so it reads as a waveform, not a mechanical sine — matches the design.
const HEIGHTS = [
  30, 45, 60, 40, 70, 55, 35, 50, 65, 45, 30, 55, 70, 40, 60, 35, 50, 65, 45, 30, 55, 40, 60, 45,
]

export type MeterState = 'idle' | 'listening' | 'processing'

export function SignalMeter({ state }: { state: MeterState }) {
  const listening = state === 'listening'
  const processing = state === 'processing'

  const scale = listening ? 1 : processing ? 0.55 : 0.24
  const color = listening
    ? 'hsl(var(--record))'
    : processing
    ? 'hsl(var(--primary))'
    : 'hsl(var(--faint) / 0.6)'

  return (
    <div
      className="flex h-[38px] items-end justify-center gap-[3px]"
      role="img"
      aria-label={listening ? 'Listening' : processing ? 'Transcribing' : 'Idle'}
    >
      {HEIGHTS.map((h, i) => (
        <span
          key={i}
          className={`w-[3px] rounded-[2px] ${
            listening ? 'tick-pulse' : processing ? 'soft-sweep' : ''
          }`}
          style={{
            height: `${Math.round(h * scale)}px`,
            background: color,
            animationDelay: state === 'idle' ? undefined : `${(i % 8) * 70}ms`,
          }}
        />
      ))}
    </div>
  )
}
