// Shared speaker-transcript helpers for the Notetaker — used by both the in-app view and the
// floating panel so the two surfaces stay identical (Operate-mode consistency). Groups attributed
// lines into speaker turns and assigns each speaker a stable, theme-aware colour.

import type { Speaker } from '@/lib/ipc'

/** One attributed transcript line (saved segment or live window). */
export interface SpeakerLine {
  offset_ms: number
  text: string
  speaker_cluster: number
}

/** A run of consecutive lines by the same speaker — one labelled, coloured block in the UI. */
export interface SpeakerTurn {
  cluster: number
  label: string
  color: string
  isSelf: boolean
  lines: SpeakerLine[]
}

// Guest hues, cycled by appearance order. Mid-lightness so they read on both light and dark cards;
// "You" always uses the app's primary. Tunable — matches the coloured speaker names in the design.
const GUEST_COLORS = [
  'hsl(32 78% 50%)', // amber
  'hsl(265 68% 62%)', // violet
  'hsl(168 60% 40%)', // teal
  'hsl(340 68% 58%)', // rose
  'hsl(150 52% 42%)', // green
  'hsl(210 70% 56%)', // blue
]

/** Build a cluster → label map from the backend speaker list (handles is_self as number or bool). */
export function labelMap(speakers: Speaker[]): Map<number, string> {
  const m = new Map<number, string>()
  for (const s of speakers) m.set(s.cluster, s.label)
  return m
}

/** A speaker's display colour: primary for "You", else a stable guest hue by cluster order. */
export function speakerColor(cluster: number, guestIndex: number): string {
  if (cluster === 0) return 'hsl(var(--primary))'
  if (cluster < 0) return 'hsl(var(--muted-foreground))'
  return GUEST_COLORS[guestIndex % GUEST_COLORS.length]
}

/**
 * Group ordered lines into consecutive same-speaker turns, resolving each speaker's label (from the
 * backend map, falling back to "You" / "Speaker N") and a stable colour. Guest colours/numbers are
 * assigned by first appearance so they don't jump as names resolve.
 */
export function groupTurns(lines: SpeakerLine[], labels: Map<number, string>): SpeakerTurn[] {
  // Stable guest ordinal per cluster, by first appearance.
  const guestOrder = new Map<number, number>()
  let nextGuest = 0
  const guestIndexOf = (cluster: number) => {
    if (cluster <= 0) return -1
    if (!guestOrder.has(cluster)) guestOrder.set(cluster, nextGuest++)
    return guestOrder.get(cluster)!
  }

  const turns: SpeakerTurn[] = []
  for (const line of lines) {
    const cluster = line.speaker_cluster
    const gi = guestIndexOf(cluster)
    const last = turns[turns.length - 1]
    if (last && last.cluster === cluster) {
      last.lines.push(line)
      continue
    }
    const fallback = cluster === 0 ? 'You' : cluster < 0 ? 'Speaker' : `Speaker ${gi + 1}`
    turns.push({
      cluster,
      label: labels.get(cluster) ?? fallback,
      color: speakerColor(cluster, gi),
      isSelf: cluster === 0,
      lines: [line],
    })
  }
  return turns
}
