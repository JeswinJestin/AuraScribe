'use client'

import { useState, type ReactNode } from 'react'
import {
  Mic,
  Download,
  Keyboard,
  Wand2,
  Sparkles,
  ArrowRight,
  ArrowLeft,
  Check,
  ShieldCheck,
} from 'lucide-react'

/**
 * First-run walkthrough. Shown once, when `settings.onboarded` is false (a fresh install).
 * It never blocks the app permanently: "Skip" and the final "Start dictating" both call
 * `onFinish`, which persists `onboarded = true` so it does not return.
 *
 * Deliberately does *not* download a model or change settings for the user — it explains, and
 * hands off to Settings. Doing side-effectful things from a walkthrough is how you end up with
 * a surprised user and a half-downloaded model.
 */

/** Render "Shift+Ctrl+Space" as individual keycaps. */
function HotkeyKeys({ combo }: { combo: string }) {
  const parts = combo.split('+').filter(Boolean)
  return (
    <span className="inline-flex flex-wrap items-center gap-1">
      {parts.map((k, i) => (
        <span key={i} className="inline-flex items-center gap-1">
          <kbd className="kbd">{k === 'Space' ? 'Space' : k}</kbd>
          {i < parts.length - 1 && <span className="text-muted-foreground">+</span>}
        </span>
      ))}
    </span>
  )
}

type Step = {
  icon: ReactNode
  eyebrow: string
  title: string
  body: ReactNode
}

export function Onboarding({
  hotkey,
  onFinish,
  onOpenModels,
}: {
  hotkey: string
  /** Persist onboarded = true. */
  onFinish: () => void
  /** Finish onboarding and jump to Settings (voice model). */
  onOpenModels: () => void
}) {
  const [step, setStep] = useState(0)

  const steps: Step[] = [
    {
      icon: <Mic className="h-6 w-6" />,
      eyebrow: 'Step 1 of 5 · Welcome',
      title: 'Your voice, on your machine',
      body: (
        <>
          <p>
            AuraScribe turns speech into clean, punctuated text right where your cursor is — in
            any app. Press a key, talk, and your words appear.
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            {['Free forever', '100% on-device', 'No account, no cloud'].map((t) => (
              <span
                key={t}
                className="rounded-full border border-primary/25 bg-primary/10 px-3 py-1 text-xs font-medium text-primary"
              >
                {t}
              </span>
            ))}
          </div>
        </>
      ),
    },
    {
      icon: <Download className="h-6 w-6" />,
      eyebrow: 'Step 2 of 5 · Voice model',
      title: 'Pick a voice model, once',
      body: (
        <>
          <p>
            The model does the listening, right here on your computer. You download it a single
            time, then it works offline forever.
          </p>
          <p className="mt-3">
            We recommend <span className="mono text-foreground">moonshine-base-en</span> — the
            fastest engine we ship and the most accurate on English. On a slower machine,{' '}
            <span className="mono text-foreground">moonshine-tiny-en</span> is lighter and nearly
            as good.
          </p>
          <button onClick={onOpenModels} className="btn-secondary btn-sm mt-4">
            <Download className="h-3.5 w-3.5" />
            Open model settings
          </button>
        </>
      ),
    },
    {
      icon: <Keyboard className="h-6 w-6" />,
      eyebrow: 'Step 3 of 5 · The hotkey',
      title: 'One key, anywhere',
      body: (
        <>
          <p>
            Your dictation shortcut is{' '}
            <span className="whitespace-nowrap">
              <HotkeyKeys combo={hotkey} />
            </span>
            . It works in every application, even when this window is closed.
          </p>
          <p className="mt-3">
            <span className="font-medium text-foreground">Tap</span> it to start, tap again to
            stop — or switch to <span className="font-medium text-foreground">hold</span> mode in
            Settings to talk only while the key is held. You can rebind it any time.
          </p>
        </>
      ),
    },
    {
      icon: <Wand2 className="h-6 w-6" />,
      eyebrow: 'Step 4 of 5 · Cleanup, Words & Snippets',
      title: 'It tidies up as you talk',
      body: (
        <>
          <p>
            Cleanup fixes punctuation and capitalisation and drops filler words like “um” and
            “uh” — all locally, with no delay.
          </p>
          <ul className="mt-3 space-y-2">
            <li className="flex gap-2">
              <ArrowRight className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
              <span>
                <span className="font-medium text-foreground">Words</span> — teach it names and
                jargon so “kubernetes” is always written “Kubernetes”.
              </span>
            </li>
            <li className="flex gap-2">
              <ArrowRight className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
              <span>
                <span className="font-medium text-foreground">Snippets</span> — say “my email” and
                get your full address inserted for you.
              </span>
            </li>
          </ul>
        </>
      ),
    },
    {
      icon: <Sparkles className="h-6 w-6" />,
      eyebrow: 'Step 5 of 5 · Make it yours',
      title: 'A look you can change',
      body: (
        <>
          <p>
            You’re seeing the <span className="font-medium text-foreground">Glass</span>{' '}
            appearance — the default. Prefer something plainer? Settings → Appearance also has
            Light, Dark, and Match&nbsp;system.
          </p>
          <div className="mt-4 flex items-start gap-2.5 rounded-xl border border-primary/20 bg-primary/5 p-3">
            <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
            <p className="text-[13px] text-muted-foreground">
              Everything stays on this device. The only time AuraScribe uses the network is to
              download a voice model — never your audio, never your text.
            </p>
          </div>
        </>
      ),
    },
  ]

  const isLast = step === steps.length - 1
  const current = steps[step]

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      {/* Scrim over the app while onboarding is open. */}
      <div className="absolute inset-0 bg-background/70 backdrop-blur-md" />

      <div className="glass panel relative z-10 w-full max-w-lg overflow-hidden rounded-2xl border p-8 shadow-2xl">
        <div key={step} className="fade-up">
          <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-primary/12 text-primary">
            {current.icon}
          </div>
          <p className="eyebrow mt-5">{current.eyebrow}</p>
          <h2 className="font-display mt-1 text-[26px] font-medium leading-tight tracking-tight">
            {current.title}
          </h2>
          <div className="mt-3 space-y-1 text-sm leading-relaxed text-muted-foreground">
            {current.body}
          </div>
        </div>

        {/* Progress dots. */}
        <div className="mt-8 flex items-center gap-1.5">
          {steps.map((_, i) => (
            <button
              key={i}
              onClick={() => setStep(i)}
              aria-label={`Go to step ${i + 1}`}
              className={`h-1.5 rounded-full transition-all ${
                i === step ? 'w-6 bg-primary' : 'w-1.5 bg-foreground/20 hover:bg-foreground/40'
              }`}
            />
          ))}
        </div>

        <div className="mt-5 flex items-center justify-between gap-3">
          <div>
            {step > 0 ? (
              <button onClick={() => setStep((s) => s - 1)} className="btn-ghost btn-sm">
                <ArrowLeft className="h-3.5 w-3.5" />
                Back
              </button>
            ) : (
              <button onClick={onFinish} className="btn-ghost btn-sm">
                Skip
              </button>
            )}
          </div>

          {isLast ? (
            <button onClick={onFinish} className="btn-primary">
              <Check className="h-4 w-4" />
              Start dictating
            </button>
          ) : (
            <button onClick={() => setStep((s) => s + 1)} className="btn-primary">
              Next
              <ArrowRight className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
