'use client'

import { useCallback, useEffect, useState } from 'react'
import { Plus, Trash2, ArrowRight } from 'lucide-react'
import * as ipc from '@/lib/ipc'
import type { DictionaryEntry } from '@/lib/ipc'
import { PageHeader, EmptyState, ErrorNote } from '@/components/ui'

export function DictionaryView() {
  const [items, setItems] = useState<DictionaryEntry[]>([])
  const [word, setWord] = useState('')
  const [replacement, setReplacement] = useState('')
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setItems(await ipc.getDictionary())
    } catch (e) {
      setError(String(e))
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const add = async () => {
    if (!word.trim() || !replacement.trim()) return
    setError(null)
    try {
      await ipc.addDictionaryEntry({ word: word.trim(), replacement: replacement.trim() })
      setWord('')
      setReplacement('')
      await load()
    } catch (e) {
      setError(String(e))
    }
  }

  const remove = async (id: number) => {
    try {
      await ipc.deleteDictionaryEntry(id)
      await load()
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <div className="mx-auto max-w-2xl">
      <PageHeader
        title="Words"
        description="Names, jargon and acronyms you want spelled your way every time."
      />

      <div className="panel mb-3 p-3">
        <div className="flex items-end gap-2">
          <label className="flex-1">
            <span className="mb-1 block text-xs font-medium">Heard as</span>
            <input
              className="input"
              placeholder="jeswin"
              value={word}
              onChange={(e) => setWord(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && add()}
            />
          </label>
          <ArrowRight className="mb-2.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <label className="flex-1">
            <span className="mb-1 block text-xs font-medium">Written as</span>
            <input
              className="input"
              placeholder="Jeswin"
              value={replacement}
              onChange={(e) => setReplacement(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && add()}
            />
          </label>
          <button onClick={add} disabled={!word.trim() || !replacement.trim()} className="btn-primary">
            <Plus className="h-4 w-4" />
            Add
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-3">
          <ErrorNote>{error}</ErrorNote>
        </div>
      )}

      {items.length === 0 ? (
        <EmptyState
          title="No words yet"
          hint="Add a term above and AuraScribe will correct it in every dictation."
        />
      ) : (
        <div className="flex flex-col gap-1.5">
          {items.map((item) => (
            <div key={item.id} className="panel flex items-center gap-3 px-3 py-2">
              <span className="mono text-sm text-muted-foreground">{item.word}</span>
              <ArrowRight className="h-3 w-3 shrink-0 text-muted-foreground/60" />
              <span className="mono text-sm">{item.replacement}</span>
              <button
                onClick={() => remove(item.id)}
                className="btn-ghost btn-sm ml-auto"
                aria-label={`Remove ${item.word}`}
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
