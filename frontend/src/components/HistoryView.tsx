import { useState } from 'react'
import type { ChangeTimeline } from '../types'
import { formatDate, shortSha } from '../types'
import { Panel } from './Panel'

export function HistoryView({
  timeline,
  onOpenFile,
}: {
  timeline: ChangeTimeline
  onOpenFile: (path: string) => void
}) {
  const [selected, setSelected] = useState<string | null>(null)
  const entries = [...(timeline.entries ?? [])].reverse()

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
        <ul className="commit-list">
          {entries.slice(0, 100).map((entry) => (
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
        {entries.length > 100 && (
          <p className="history-note">Latest 100 commits shown.</p>
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
          </>
        )}
      </Panel>
    </div>
  )
}
