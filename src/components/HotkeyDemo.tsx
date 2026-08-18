'use client'

import { useEffect, useRef, useState } from 'react'
import { Mic, RotateCcw } from 'lucide-react'
import { playKeyPress, playMicOn, playMicOff, playDemoVoice } from '@/lib/demoSounds'

/**
 * The heart of the onboarding walkthrough (step 2): a short, colour-matched motion graphic that
 * *shows* how AuraScribe works — press the hotkey, the mic lights up, you speak, and text appears
 * where your cursor is — with matching sound effects.
 *
 * Plays through ONCE when it appears (and again on "Replay"), then holds on the finished frame, so
 * the sound cues aren't repetitive. Driven by a small JS timeline (not CSS loops) so it can respect
 * `prefers-reduced-motion` by jumping straight to the finished frame with no motion or sound. This
 * is a deliberate, documented exception to DESIGN.md's "only the signal meter moves" rule — it
 * exists only on the first-run surface.
 * See docs/superpowers/specs/2026-08-18-spotlight-onboarding-design.md.
 */

/** The line that "types" into the field. Keep it short, specific, no em dashes. If you record a
 *  voice line (public/onboarding-voice.mp3), make it say exactly this. */
const DEMO_TEXT = 'Schedule my email for 9 AM.'

// Order matters — this is the real dictation flow: press the keys, the mic turns on, you speak,
// the mic turns off, and only THEN does the text land at the cursor.
type Phase = 'idle' | 'press' | 'listen' | 'speak' | 'stop' | 'type' | 'hold'
const DUR: Record<Phase, number> = {
  idle: 500,
  press: 550,
  listen: 550,
  speak: 2100, // long enough for the ~1.85s voice line to finish before the mic-off cue
  stop: 650,
  type: 1400,
  hold: 0, // terminal — holds until replay
}

/** Turn a stored hotkey string ("Ctrl+Shift+Space", "Super+Shift+Space") into keycap labels. */
function keycaps(hotkey: string): { label: string; wide: boolean }[] {
  const map: Record<string, string> = { Super: 'Cmd', Ctrl: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Space: 'Space' }
  return hotkey
    .split('+')
    .filter(Boolean)
    .map((k) => ({ label: map[k] ?? k, wide: k === 'Space' }))
}

export function HotkeyDemo({ hotkey }: { hotkey: string }) {
  const [reduced, setReduced] = useState(false)
  const [phase, setPhase] = useState<Phase>('idle')
  const [typed, setTyped] = useState(0)
  const [runId, setRunId] = useState(0)
  const timers = useRef<ReturnType<typeof setTimeout>[]>([])

  useEffect(() => {
    const m = window.matchMedia('(prefers-reduced-motion: reduce)')
    const apply = () => setReduced(m.matches)
    apply()
    m.addEventListener('change', apply)
    return () => m.removeEventListener('change', apply)
  }, [])

  // Play the timeline once per runId. Reduced-motion → straight to the finished frame, no sound.
  useEffect(() => {
    timers.current.forEach(clearTimeout)
    timers.current = []
    if (reduced) {
      setPhase('hold')
      setTyped(DEMO_TEXT.length)
      return
    }
    setPhase('idle')
    setTyped(0)
    let at = 0
    const schedule = (phaseDur: number, fn: () => void) => {
      timers.current.push(setTimeout(fn, at))
      at += phaseDur
    }
    schedule(DUR.idle, () => setPhase('idle'))
    schedule(DUR.press, () => {
      setPhase('press')
      playKeyPress()
    })
    schedule(DUR.listen, () => {
      setPhase('listen')
      playMicOn()
    })
    schedule(DUR.speak, () => {
      setPhase('speak')
      playDemoVoice()
    })
    schedule(DUR.stop, () => {
      setPhase('stop')
      playMicOff()
    })
    schedule(DUR.type, () => setPhase('type'))
    schedule(0, () => setPhase('hold'))
    return () => timers.current.forEach(clearTimeout)
  }, [runId, reduced])

  // Typewriter, scoped to the 'type' phase.
  useEffect(() => {
    if (reduced || phase === 'hold') {
      setTyped(DEMO_TEXT.length)
      return
    }
    if (phase !== 'type') {
      if (phase === 'idle') setTyped(0)
      return
    }
    setTyped(0)
    let i = 0
    const iv = setInterval(() => {
      i += 1
      setTyped(i)
      if (i >= DEMO_TEXT.length) clearInterval(iv)
    }, DUR.type / DEMO_TEXT.length)
    return () => clearInterval(iv)
  }, [phase, reduced])

  const keysPressed = phase === 'press' || phase === 'listen'
  // Mic is lit only while listening/speaking; it goes dark when recording stops, before the text.
  const micOn = phase === 'listen' || phase === 'speak'
  const listening = phase === 'listen' || phase === 'speak'

  return (
    <div className="flex flex-col items-center gap-4 py-1">
      {/* Keys */}
      <div className="flex items-center gap-1.5">
        {keycaps(hotkey).map((k, i) => (
          <Keycap key={i} label={k.label} pressed={keysPressed} wide={k.wide} />
        ))}
      </div>

      {/* Mic + signal */}
      <div className="flex items-center gap-3">
        <div
          className="relative grid place-items-center rounded-full"
          style={{
            width: 52,
            height: 52,
            border: '1px solid',
            borderColor: micOn ? 'hsl(var(--primary))' : 'hsl(var(--border))',
            background: micOn ? 'hsl(var(--primary) / 0.18)' : 'rgba(255,255,255,0.04)',
            color: micOn ? 'hsl(var(--primary))' : 'hsl(var(--muted-foreground))',
            boxShadow: listening
              ? '0 0 0 6px hsl(var(--primary) / 0.12), 0 0 26px 4px hsl(var(--primary) / 0.4)'
              : 'none',
            transition: 'all 220ms ease',
          }}
        >
          <Mic className="h-[22px] w-[22px]" strokeWidth={2} />
        </div>

        <div className="flex items-end gap-[3px]" style={{ height: 26 }}>
          {[0, 1, 2, 3, 4].map((i) => (
            <span
              key={i}
              className={listening ? 'tick-pulse' : ''}
              style={{
                width: 3,
                height: 26,
                borderRadius: 2,
                background: listening ? 'hsl(var(--primary))' : 'hsl(var(--muted-foreground) / 0.4)',
                transform: listening ? undefined : 'scaleY(0.4)',
                transformOrigin: 'bottom',
                animationDelay: `${i * 90}ms`,
                transition: 'background 220ms ease',
              }}
            />
          ))}
        </div>
      </div>

      {/* The text landing where your cursor is */}
      <div
        className="flex min-h-[42px] w-full max-w-[280px] items-center rounded-[10px] px-3 py-2 text-[13px]"
        style={{
          border: '1px solid hsl(var(--border))',
          background: 'rgba(255,255,255,0.05)',
          color: 'hsl(var(--foreground))',
        }}
      >
        <span>
          {DEMO_TEXT.slice(0, typed)}
          {phase !== 'hold' && (
            <span
              className="tour-caret ml-[1px] inline-block"
              style={{ width: 1.5, height: 15, transform: 'translateY(3px)', background: 'hsl(var(--primary))' }}
            />
          )}
        </span>
      </div>

      {/* Replay — re-runs the animation + sound on demand. */}
      <button
        onClick={() => setRunId((r) => r + 1)}
        className="inline-flex items-center gap-1.5 text-[11.5px] text-muted-foreground transition-colors hover:text-foreground"
      >
        <RotateCcw className="h-3 w-3" />
        Replay
      </button>
    </div>
  )
}

function Keycap({ label, pressed, wide }: { label: string; pressed: boolean; wide?: boolean }) {
  return (
    <div
      className="mono flex items-center justify-center"
      style={{
        minWidth: wide ? 74 : 46,
        padding: '7px 10px',
        borderRadius: 8,
        fontSize: 12,
        fontWeight: 600,
        border: '1px solid',
        borderColor: pressed ? 'hsl(var(--primary))' : 'hsl(var(--border))',
        color: pressed ? '#fff' : 'hsl(var(--muted-foreground))',
        background: pressed ? 'hsl(var(--primary) / 0.9)' : 'rgba(255,255,255,0.05)',
        boxShadow: pressed ? '0 0 18px 2px hsl(var(--primary) / 0.5)' : 'none',
        transform: pressed ? 'translateY(2px)' : 'none',
        transition: 'all 160ms ease',
      }}
    >
      {label}
    </div>
  )
}
