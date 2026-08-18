'use client'

import { useCallback, useEffect, useState, type CSSProperties, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ArrowLeft, ArrowRight, Check } from 'lucide-react'
import { HotkeyDemo } from './HotkeyDemo'

/**
 * First-run (and replayable) walkthrough. Highlights one thing at a time and dims + blurs the rest.
 * Three stops: Welcome → an animated "how it works" demo → a real spotlight on the "add a model"
 * action. Skippable at every step. No dependency — the cut-out is four blurred panels tiling around
 * the target rect (no CSS-mask fragility), tracked every frame so it follows layout/scroll.
 *
 * Design: docs/superpowers/specs/2026-08-18-spotlight-onboarding-design.md
 */

type Rect = { top: number; left: number; width: number; height: number }

type Stop = {
  key: string
  eyebrow: string
  title: string
  body?: ReactNode
  /** Spotlight step: selectors tried in order; the first found element is highlighted. */
  targets?: string[]
  /** Render the motion demo above the body text. */
  demo?: boolean
}

const DIM = 'rgba(6, 9, 20, 0.62)'
const BLUR: CSSProperties = { backdropFilter: 'blur(3px)', WebkitBackdropFilter: 'blur(3px)' }
const PAD = 8 // breathing room around the highlighted element
const RADIUS = 14
const CARD_W = 360

function resolveTarget(selectors?: string[]): HTMLElement | null {
  if (!selectors) return null
  for (const s of selectors) {
    const el = document.querySelector(s)
    if (el) return el as HTMLElement
  }
  return null
}

/** Inline keycaps for hotkey text, e.g. "Ctrl + Shift + Space" (or "Cmd + Shift + Space" on macOS,
 *  where the stored combo uses "Super"). Labels match the demo keycaps. */
const KEY_LABEL: Record<string, string> = { Super: 'Cmd', Ctrl: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Space: 'Space' }
function HotkeyInline({ combo }: { combo: string }) {
  const parts = combo.split('+').filter(Boolean)
  return (
    <span className="inline-flex flex-wrap items-center gap-1 align-middle">
      {parts.map((k, i) => (
        <span key={i} className="inline-flex items-center gap-1">
          <kbd className="kbd">{KEY_LABEL[k] ?? k}</kbd>
          {i < parts.length - 1 && <span className="text-muted-foreground">+</span>}
        </span>
      ))}
    </span>
  )
}

/** The dim + blur surround with a rectangular hole, plus an indigo ring around the target. */
function SpotlightMask({ hole }: { hole: Rect }) {
  const base: CSSProperties = { position: 'absolute', background: DIM, ...BLUR }
  const belowTop = hole.top + hole.height
  const rightLeft = hole.left + hole.width
  return (
    <>
      <div style={{ ...base, top: 0, left: 0, width: '100vw', height: Math.max(0, hole.top) }} />
      <div style={{ ...base, top: belowTop, left: 0, width: '100vw', height: `calc(100vh - ${belowTop}px)` }} />
      <div style={{ ...base, top: hole.top, left: 0, width: Math.max(0, hole.left), height: hole.height }} />
      <div style={{ ...base, top: hole.top, left: rightLeft, width: `calc(100vw - ${rightLeft}px)`, height: hole.height }} />
      <div
        style={{
          position: 'absolute',
          top: hole.top,
          left: hole.left,
          width: hole.width,
          height: hole.height,
          borderRadius: RADIUS,
          boxShadow: '0 0 0 2px hsl(var(--primary)), 0 0 24px 4px hsl(var(--primary) / 0.4)',
          pointerEvents: 'none',
        }}
      />
    </>
  )
}

/** Position the card: centered for card/demo steps, beside the target for a spotlight. */
function computeCardStyle(hole: Rect | null): CSSProperties {
  if (!hole) return { top: '50%', left: '50%', transform: 'translate(-50%, -50%)' }
  const vw = window.innerWidth
  const vh = window.innerHeight
  const left = Math.min(Math.max(hole.left + hole.width / 2 - CARD_W / 2, 16), vw - CARD_W - 16)
  const spaceBelow = vh - (hole.top + hole.height)
  if (spaceBelow > 260) return { top: hole.top + hole.height + 16, left }
  return { top: hole.top - 16, left, transform: 'translateY(-100%)' }
}

export function SpotlightTour({
  hotkey,
  onFinish,
}: {
  hotkey: string
  /** Persist onboarded = true (first run) and close the tour. Skip and Finish both call this. */
  onFinish: () => void
}) {
  const [step, setStep] = useState(0)
  const [rect, setRect] = useState<Rect | null>(null)
  const [mounted, setMounted] = useState(false)

  useEffect(() => setMounted(true), [])

  const stops: Stop[] = [
    {
      key: 'welcome',
      eyebrow: 'Welcome',
      title: 'Your voice, on your machine',
      body: (
        <>
          <p>
            AuraScribe turns speech into clean, punctuated text right where your cursor is — in any
            app. This quick tour takes about fifteen seconds.
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            {['Free forever', '100% on-device', 'No account, no cloud'].map((t) => (
              <span
                key={t}
                className="rounded-full border px-3 py-1 text-xs font-medium"
                style={{
                  borderColor: 'hsl(var(--primary) / 0.3)',
                  background: 'hsl(var(--primary) / 0.12)',
                  color: 'hsl(var(--primary))',
                }}
              >
                {t}
              </span>
            ))}
          </div>
        </>
      ),
    },
    {
      key: 'how',
      eyebrow: 'How it works',
      title: 'Press, speak, done',
      demo: true,
      body: (
        <p>
          Press <HotkeyInline combo={hotkey} /> anywhere — even with this window closed — to start
          dictating. Tap again to stop, and your words land at the cursor.
        </p>
      ),
    },
    {
      key: 'model',
      eyebrow: 'One-time setup',
      title: 'Add a voice model',
      targets: ['[data-tour="download-model"]', '[data-tour="record"]'],
      body: (
        <p>
          Everything starts with one voice model. Download it once — then AuraScribe runs completely
          offline, forever. It’s the only thing you need to do to begin.
        </p>
      ),
    },
  ]

  const current = stops[step]
  const isLast = step === stops.length - 1
  const isSpotlight = !!current.targets

  // Track the highlighted element every frame while a spotlight step is active, so the cut-out
  // follows it through layout shifts / scrolls / resizes. Falls back to a centered card if no
  // target is found (e.g. a context without the real Dictate view).
  useEffect(() => {
    if (!isSpotlight) {
      setRect(null)
      return
    }
    let raf = 0
    let alive = true
    let scrolled = false
    const tick = () => {
      if (!alive) return
      const el = resolveTarget(current.targets)
      if (el) {
        if (!scrolled) {
          el.scrollIntoView({ block: 'center', behavior: 'smooth' })
          scrolled = true
        }
        const r = el.getBoundingClientRect()
        setRect((prev) => {
          const next = { top: r.top, left: r.left, width: r.width, height: r.height }
          if (
            prev &&
            Math.round(prev.top) === Math.round(next.top) &&
            Math.round(prev.left) === Math.round(next.left) &&
            Math.round(prev.width) === Math.round(next.width) &&
            Math.round(prev.height) === Math.round(next.height)
          ) {
            return prev
          }
          return next
        })
      } else {
        setRect(null)
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => {
      alive = false
      cancelAnimationFrame(raf)
    }
    // current.targets is stable per step; step covers the dependency.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step, isSpotlight])

  const next = useCallback(() => {
    if (isLast) onFinish()
    else setStep((s) => s + 1)
  }, [isLast, onFinish])

  if (!mounted) return null

  const hole: Rect | null = rect
    ? { top: rect.top - PAD, left: rect.left - PAD, width: rect.width + PAD * 2, height: rect.height + PAD * 2 }
    : null
  const cardStyle = computeCardStyle(hole)

  const overlay = (
    <div className="fixed inset-0 z-[100]" role="dialog" aria-modal="true" aria-label="AuraScribe walkthrough">
      {hole ? (
        <SpotlightMask hole={hole} />
      ) : (
        <div className="absolute inset-0" style={{ background: DIM, ...BLUR }} />
      )}

      {/* Positioning wrapper (no animation, so its transform never fights fade-up). */}
      <div className="absolute z-[101]" style={cardStyle}>
        <div
          key={step}
          className="glass panel fade-up rounded-2xl border p-6 shadow-2xl"
          style={{
            width: CARD_W,
            maxWidth: 'calc(100vw - 32px)',
            // Never let a tall step (the demo) push its controls off a short window.
            maxHeight: 'calc(100vh - 32px)',
            overflowY: 'auto',
          }}
        >
          <p className="eyebrow">{current.eyebrow}</p>
          <h2 className="font-display mt-1 text-[22px] font-medium leading-tight tracking-tight">
            {current.title}
          </h2>

          {current.demo && (
            <div
              className="mt-4 rounded-xl border p-3"
              style={{ borderColor: 'hsl(var(--border))', background: 'rgba(255,255,255,0.03)' }}
            >
              <HotkeyDemo hotkey={hotkey} />
            </div>
          )}

          <div className="mt-3 space-y-2 text-[13.5px] leading-relaxed text-muted-foreground">
            {current.body}
          </div>

          <div className="mt-5 flex items-center gap-1.5">
            {stops.map((_, i) => (
              <span
                key={i}
                className="h-1.5 rounded-full transition-all"
                style={{
                  width: i === step ? 22 : 6,
                  background: i === step ? 'hsl(var(--primary))' : 'hsl(var(--muted-foreground) / 0.4)',
                }}
              />
            ))}
          </div>

          <div className="mt-4 flex items-center justify-between gap-3">
            {step > 0 ? (
              <button onClick={() => setStep((s) => s - 1)} className="btn-ghost btn-sm">
                <ArrowLeft className="h-3.5 w-3.5" />
                Back
              </button>
            ) : (
              <span />
            )}
            <div className="flex items-center gap-2">
              {/* Skip sits next to Next in a muted tone, on the first two steps only. The last
                  step has no skip — the user just starts dictating. */}
              {!isLast && (
                <button onClick={onFinish} className="btn-ghost btn-sm">
                  Skip
                </button>
              )}
              <button onClick={next} className="btn-primary">
                {isLast ? (
                  <>
                    <Check className="h-4 w-4" />
                    Start dictating
                  </>
                ) : (
                  <>
                    Next
                    <ArrowRight className="h-4 w-4" />
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )

  return createPortal(overlay, document.body)
}
