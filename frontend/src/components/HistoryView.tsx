import { useState } from 'react'
import type { ChangeTimeline } from '../types'
import { formatDate, shortSha } from '../types'
import { Panel } from './Panel'

export function HistoryView({
  timeline,
  focusFile,
  onOpenFile,
  onOpenPrediction,
  onOpenGraph,
}: {
  timeline: ChangeTimeline
  focusFile?: string | null
  onOpenFile: (path: string) => void
  onOpenPrediction?: (path: string) => void
  onOpenGraph?: (path: string) => void
}) {
  const [selected, setSelected] = useState<string | null>(null)
  const [filter, setFilter] = useState('')
  const entries = [...(timeline.entries ?? [])].reverse()

  const activeFilter = (focusFile ?? filter).trim().toLowerCase()
  const visible = activeFilter
    ? entries.filter((entry) =>
        (entry.files ?? []).some((file) =>
          file.toLowerCase().includes(activeFilter),
        ),
      )
    : entries

  const active = selected
    ? entries.find((entry) => entry.sha === selected) ?? null
    : null

  if (entries.length === 0) {
    return (
      <Panel title="History" hint="Repository commits">
        <div className="empty-state">
          <p>No commit history is available for this repository yet.</p>
        </div>
      </Panel>
    )
  }

  return (
    <div className="grid-2 history-grid">
      <Panel title="History" hint={`${timeline.total_commits} commits`}>
        <div className="graph-controls">
          <input
            className="filter-input"
            type="search"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            placeholder={focusFile ? `Filtering: ${focusFile}` : 'Filter by file…'}
            aria-label="Filter commits by file"
          />
          {filter.trim() && (
            <button
              type="button"
              className="ghost-button"
              onClick={() => setFilter('')}
            >
              Clear
            </button>
          )}
        </div>
        {visible.length === 0 ? (
          <p className="history-note" role="status">
            No commits touch {activeFilter || 'this filter'} yet.
          </p>
        ) : (
          <ul className="commit-list">
            {visible.slice(0, 100).map((entry) => (
              <li key={entry.sha}>
                <button
                  type="button"
                  className={`commit-row${selected === entry.sha ? ' active' : ''}`}
                  onClick={() => setSelected(entry.sha)}
                  aria-pressed={selected === entry.sha}
                  aria-label={`Commit ${shortSha(entry.sha)}: ${entry.message}`}
                >
                  <code>{shortSha(entry.sha)}</code>
                  <span className="commit-message">{entry.message || '(no message)'}</span>
                  <span className="muted">
                    {entry.author ?? 'unknown'} · {formatDate(entry.date)}
                  </span>
                  <span className="muted">{entry.files.length} files</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {visible.length > 100 && (
          <p className="history-note">Latest 100 matching commits shown.</p>
        )}
      </Panel>
      <Panel title="Commit Details" hint="Changed files">
        {!active ? (
          <p className="history-note">Select a commit to see its changed files.</p>
        ) : (
          <>
            <p>
              <code>{shortSha(active.sha)}</code> · {active.message || '(no message)'}
            </p>
            <p className="history-note">
              {active.author ?? 'unknown'} · {formatDate(active.date)} · +
              {active.additions} −{active.deletions}
            </p>
            <ul className="graph-neighbors">
              {active.files.map((file) => (
                <li key={file}>
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => onOpenFile(file)}
                  >
                    {file}
                  </button>
                </li>
              ))}
            </ul>
            <div className="graph-explain-actions">
              {onOpenPrediction && active.files[0] && (
                <button
                  type="button"
                  className="flow-button"
                  onClick={() => onOpenPrediction(active.files[0])}
                >
                  View prediction →
                </button>
              )}
              {onOpenGraph && active.files[0] && (
                <button
                  type="button"
                  className="flow-ghost"
                  onClick={() => onOpenGraph(active.files[0])}
                >
                  Focus in graph
                </button>
              )}
            </div>
          </>
        )}
      </Panel>
    </div>
  )
}
