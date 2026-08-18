// Tiny Web Audio sound effects for the onboarding "See it work" demo (HotkeyDemo).
//
// Synthesized in the browser — no audio files, no dependency, no weight. These are illustrative
// cues that play ONCE per demo run (not on a loop), started from within a user gesture (the user
// clicked "Next" to reach step 2), so the autoplay policy allows them. Everything fails silently if
// Web Audio is unavailable or blocked — a missing sound must never break the walkthrough.

let ctx: AudioContext | null = null

function audio(): AudioContext | null {
  if (typeof window === 'undefined') return null
  if (!ctx) {
    const AC = window.AudioContext || (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!AC) return null
    try {
      ctx = new AC()
    } catch {
      return null
    }
  }
  if (ctx.state === 'suspended') ctx.resume().catch(() => {})
  return ctx
}

/** A short, soft tactile "click" — the key press. */
export function playKeyPress() {
  const c = audio()
  if (!c) return
  try {
    const dur = 0.05
    const buffer = c.createBuffer(1, Math.floor(c.sampleRate * dur), c.sampleRate)
    const data = buffer.getChannelData(0)
    for (let i = 0; i < data.length; i++) {
      const t = i / data.length
      // Decaying noise burst → a mechanical-ish click.
      data[i] = (Math.random() * 2 - 1) * Math.pow(1 - t, 3)
    }
    const src = c.createBufferSource()
    src.buffer = buffer
    const filter = c.createBiquadFilter()
    filter.type = 'bandpass'
    filter.frequency.value = 2200
    filter.Q.value = 0.7
    const gain = c.createGain()
    gain.gain.value = 0.16
    src.connect(filter)
    filter.connect(gain)
    gain.connect(c.destination)
    src.start()
  } catch {
    /* best-effort */
  }
}

/** A soft falling two-note chime — the mic switching off ("done"). Mirrors playMicOn. */
export function playMicOff() {
  chime([880, 620])
}

/** A soft rising two-note chime — the mic switching on ("I'm listening"). */
export function playMicOn() {
  chime([660, 880])
}

/** Play a short two-note chime at the given frequencies. */
function chime(freqs: number[]) {
  const c = audio()
  if (!c) return
  try {
    const now = c.currentTime
    freqs.forEach((freq, i) => {
      const osc = c.createOscillator()
      osc.type = 'sine'
      osc.frequency.value = freq
      const g = c.createGain()
      const start = now + i * 0.09
      g.gain.setValueAtTime(0.0001, start)
      g.gain.exponentialRampToValueAtTime(0.11, start + 0.02)
      g.gain.exponentialRampToValueAtTime(0.0001, start + 0.15)
      osc.connect(g)
      g.connect(c.destination)
      osc.start(start)
      osc.stop(start + 0.17)
    })
  } catch {
    /* best-effort */
  }
}

/**
 * Play the spoken demo line once, if present. Drop an ElevenLabs (or any) recording at
 * `public/onboarding-voice.mp3` that says the DEMO_TEXT line and it plays here automatically;
 * until then this no-ops (the 404 rejects `play()`, which we swallow).
 */
export function playDemoVoice() {
  if (typeof window === 'undefined') return
  try {
    const a = new Audio('/onboarding-voice.mp3')
    a.volume = 0.9
    a.play().catch(() => {})
  } catch {
    /* best-effort */
  }
}
