'use client'

import { useCallback, useEffect, useState } from 'react'
import { Copy, Check, Trash2 } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { TranscriptEntry } from '@/lib/ipc'
import { PageHeader, EmptyState, ErrorNote } from '@/components/ui'

function timeAgo(unixSeconds: number) {
  const diff = Date.now() / 1000 - unixSeconds
  if (diff < 60) return 'just now'
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return new Date(unixSeconds * 1000).toLocaleDateString()
}

export function HistoryView() {
  const [items, setItems] = useState<TranscriptEntry[]>([])
  const [error, setError] = useState<string | null>(null)
  const [copiedId, setCopiedId] = useState<number | null>(null)

  const load = useCallback(async () => {
    try {
      setItems(await ipc.getTranscripts(100, 0))
    } catch (e) {
      setError(String(e))
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const copy = async (item: TranscriptEntry) => {
    await navigator.clipboard.writeText(item.cleaned_text || item.raw_text)
    setCopiedId(item.id)
    setTimeout(() => setCopiedId(null), 1400)
  }

  const clearAll = async () => {
    try {
      await ipc.clearTranscripts()
      await load()
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <div className="mx-auto max-w-2xl">
      <PageHeader
        title="History"
        description="Everything you have dictated, stored only on this machine."
        action={
          items.length > 0 ? (
            <button onClick={clearAll} className="btn-danger btn-sm">
              <Trash2 className="h-3 w-3" />
              Clear all
            </button>
          ) : undefined
        }
      />

      {error && (
        <div className="mb-3">
          <ErrorNote>{error}</ErrorNote>
        </div>
      )}

      {items.length === 0 ? (
        <EmptyState
          title="Nothing dictated yet"
          hint="Your dictations will collect here so you can copy anything that landed in the wrong window."
        />
      ) : (
        <div className="flex flex-col gap-2">
          {items.map((item) => (
            <div key={item.id} className="panel p-3">
              <div className="flex items-center justify-between gap-3">
                <span className="mono text-[10px] uppercase tracking-widest text-muted-foreground">
                  {timeAgo(item.timestamp)}
                  {item.audio_ms > 0 && ` · ${(item.audio_ms / 1000).toFixed(1)}s`}
                </span>
                <button onClick={() => copy(item)} className="btn-ghost btn-sm">
                  {copiedId === item.id ? (
                    <Check className="h-3 w-3" />
                  ) : (
                    <Copy className="h-3 w-3" />
                  )}
                  {copiedId === item.id ? 'Copied' : 'Copy'}
                </button>
              </div>
              <p className="mt-1.5 whitespace-pre-wrap text-sm leading-relaxed">
                {item.cleaned_text || item.raw_text}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
