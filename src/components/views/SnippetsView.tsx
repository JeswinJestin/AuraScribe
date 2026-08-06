'use client'

import { useCallback, useEffect, useState } from 'react'
import { Plus, Trash2, ArrowRight } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { SnippetEntry } from '@/lib/ipc'
import { PageHeader, EmptyState, ErrorNote } from '@/components/ui'

export function SnippetsView() {
  const [items, setItems] = useState<SnippetEntry[]>([])
  const [trigger, setTrigger] = useState('')
  const [expansion, setExpansion] = useState('')
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setItems(await ipc.getSnippets())
    } catch (e) {
      setError(String(e))
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const add = async () => {
    if (!trigger.trim() || !expansion.trim()) return
    setError(null)
    try {
      await ipc.addSnippet(trigger.trim(), expansion.trim())
      setTrigger('')
      setExpansion('')
      await load()
    } catch (e) {
      setError(String(e))
    }
  }

  const remove = async (id: number) => {
    try {
      await ipc.deleteSnippet(id)
      await load()
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <div>
      <PageHeader
        title="Snippets"
        description="Say a short phrase, get the long text you would rather not repeat."
      />

      <div className="mb-3.5 space-y-3 rounded-[14px] border bg-card p-[18px]">
        <label className="block">
          <span className="mb-1 block text-xs font-medium">When you say</span>
          <input
            className="input"
            placeholder="my email"
            value={trigger}
            onChange={(e) => setTrigger(e.target.value)}
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-xs font-medium">Insert this</span>
          <textarea
            className="input h-20 py-2"
            placeholder="you@example.com"
            value={expansion}
            onChange={(e) => setExpansion(e.target.value)}
          />
        </label>
        <button
          onClick={add}
          disabled={!trigger.trim() || !expansion.trim()}
          className="btn-primary w-full"
        >
          <Plus className="h-4 w-4" />
          Add snippet
        </button>
      </div>

      {error && (
        <div className="mb-3">
          <ErrorNote>{error}</ErrorNote>
        </div>
      )}

      {items.length === 0 ? (
        <EmptyState
          title="No snippets yet"
          hint="Save an email address, a link or a standard reply, and say its name to insert it."
        />
      ) : (
        <div className="flex flex-col gap-1.5">
          {items.map((item) => (
            <div key={item.id} className="panel flex items-start gap-3 px-3 py-2">
              <span className="mono shrink-0 text-sm text-muted-foreground">{item.trigger}</span>
              <ArrowRight className="mt-1 h-3 w-3 shrink-0 text-muted-foreground/60" />
              <span className="line-clamp-2 text-sm">{item.expansion}</span>
              <button
                onClick={() => remove(item.id)}
                className="btn-ghost btn-sm ml-auto"
                aria-label={`Remove ${item.trigger}`}
              >
                <Trash2 className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
