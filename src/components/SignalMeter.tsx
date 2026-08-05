'use client'

/**
 * The signal meter — AuraScribe's one signature element.
 *
 * Flat and dim when idle, ticks rise while listening, a single band sweeps while
 * transcribing. It reports state the way a piece of audio equipment would, which is
 * also the fastest way to answer the only question that matters mid-dictation:
 * "is this thing hearing me?"
 */

const TICKS = 21

// Fixed pseudo-random heights so the meter looks like a waveform rather than a
// mechanical sine, without re-randomising on every render.
const HEIGHTS = [
  0.35, 0.6, 0.45, 0.85, 0.55, 1, 0.7, 0.4, 0.9, 0.5, 0.75, 0.45, 0.95, 0.6, 0.35, 0.8,
  0.5, 0.7, 0.4, 0.55, 0.3,
]

export type MeterState = 'idle' | 'listening' | 'processing'

export function SignalMeter({ state }: { state: MeterState }) {
  if (state === 'processing') {
    return (
      <div
        className="relative h-10 w-full overflow-hidden rounded-[var(--radius)] bg-secondary"
        role="img"
        aria-label="Transcribing"
      >
        <div className="absolute inset-y-0 w-1/4 signal-sweep bg-[hsl(var(--standby))]/25" />
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="mono text-[11px] uppercase tracking-widest text-[hsl(var(--standby))]">
            transcribing
          </span>
        </div>
      </div>
    )
  }

  const listening = state === 'listening'

  return (
    <div
      className="flex h-10 items-center justify-center gap-[3px] rounded-[var(--radius)] bg-secondary px-3"
      role="img"
      aria-label={listening ? 'Listening' : 'Idle'}
    >
      {Array.from({ length: TICKS }).map((_, i) => (
        <span
          key={i}
          className={`w-[3px] rounded-full ${
            listening
              ? 'signal-tick bg-[hsl(var(--record))]'
              : 'bg-muted-foreground/25'
          }`}
          style={{
            height: listening ? `${HEIGHTS[i] * 22}px` : '3px',
            animationDelay: listening ? `${(i % 7) * 90}ms` : undefined,
          }}
        />
      ))}
    </div>
  )
}
