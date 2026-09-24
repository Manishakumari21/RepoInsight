import { useEffect, useMemo, useRef, useState } from 'react'
import type { TimelineEntry } from '../types'
import { shortSha } from '../types'

export interface PaletteSelection {
  kind: 'file' | 'commit'
  file?: string
  sha?: string
}

export function CommandPalette({
  files,
  commits,
  onSelect,
  onClose,
}: {
  files: string[]
  commits: TimelineEntry[]
  onSelect: (selection: PaletteSelection) => void
  onClose: () => void
}) {
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  const results = useMemo(() => {
    const term = query.trim().toLowerCase()
    if (!term) return []
    const matchedFiles = files
      .filter((file) => file.toLowerCase().includes(term))
      .slice(0, 6)
      .map((file) => ({ kind: 'file' as const, label: file, file }))
    const matchedCommits = commits
      .filter(
        (entry) =>
          entry.message.toLowerCase().includes(term) ||
          entry.sha.toLowerCase().startsWith(term),
      )
      .slice(-6)
      .reverse()
      .map((entry) => ({
        kind: 'commit' as const,
        label: `${shortSha(entry.sha)} ${entry.message || '(no message)'}`,
        sha: entry.sha,
      }))
    return [...matchedFiles, ...matchedCommits].slice(0, 10)
  }, [query, files, commits])

  useEffect(() => {
    setActive(0)
  }, [query])

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault()
      onClose()
    } else if (event.key === 'ArrowDown') {
      event.preventDefault()
      setActive((index) => Math.min(index + 1, results.length - 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setActive((index) => Math.max(index - 1, 0))
    } else if (event.key === 'Enter' && results[active]) {
      event.preventDefault()
      onSelect(results[active])
    }
  }

  return (
    <div
      className="palette-overlay"
      onClick={onClose}
      onKeyDown={(event) => {
        if (event.key === 'Escape') onClose()
      }}
    >
      <div
        className="palette"
        role="dialog"
        aria-modal="true"
        aria-label="Search files and commits"
        onClick={(event) => event.stopPropagation()}
      >
        <input
          ref={inputRef}
          className="palette-input"
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Search files, commits, symbols…"
          aria-label="Search files and commits"
          aria-expanded={results.length > 0}
          aria-activedescendant={
            results.length > 0 ? `palette-option-${active}` : undefined
          }
          role="combobox"
          aria-autocomplete="list"
          aria-controls="palette-list"
        />
        {query.trim() && results.length === 0 && (
          <p className="history-note palette-empty" role="status">
            No files or commits match “{query.trim()}”.
          </p>
        )}
        {results.length > 0 && (
          <ul className="palette-list" id="palette-list" role="listbox">
            {results.map((result, index) => (
              <li key={`${result.kind}-${result.label}`} role="presentation">
                <button
                  type="button"
                  id={`palette-option-${index}`}
                  role="option"
                  aria-selected={index === active}
                  className={`palette-option${index === active ? ' active' : ''}`}
                  onClick={() => onSelect(result)}
                  onMouseEnter={() => setActive(index)}
                >
                  <span className="chip">{result.kind}</span>
                  <span className="palette-label">{result.label}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
