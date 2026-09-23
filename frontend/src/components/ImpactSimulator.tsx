import { useState } from 'react'
import type { ImpactItem, ImpactResponse } from '../types'
import { Panel } from './Panel'

function pct(score: number): string {
  return `${Math.round(score * 100)}%`
}

function levelLabel(level: ImpactItem['level']): string {
  switch (level) {
    case 'high':
      return 'High-impact candidate'
    case 'medium':
      return 'Medium-impact candidate'
    case 'low':
      return 'Low-impact candidate'
  }
}

export function ImpactSimulator({
  response,
  loading,
  error,
  onRetry,
  onRequest,
  onOpenFile,
  onOpenGraph,
}: {
  response: ImpactResponse | null
  loading: boolean
  error: string | null
  onRetry: () => void
  onRequest: (description: string) => void
  onOpenFile: (path: string) => void
  onOpenGraph: () => void
}) {
  const [description, setDescription] = useState('')
  const [expanded, setExpanded] = useState<string | null>(null)
  const [checked, setChecked] = useState<string[]>([])

  const query = description.trim()
  const groups: { level: ImpactItem['level']; items: ImpactItem[] }[] = response
    ? (['high', 'medium', 'low'] as const).map((level) => ({
        level,
        items: response.impact.filter((item) => item.level === level),
      }))
    : []

  function toggleCheck(label: string) {
    setChecked((current) =>
      current.includes(label)
        ? current.filter((entry) => entry !== label)
        : [...current, label],
    )
  }

  return (
    <>
      <div className="page-head">
        <h1>Change Impact Simulator</h1>
        <p>
          Describe a planned change before making it. RepoInsight estimates
          which parts of the repository would need attention, with evidence
          for every candidate.
        </p>
      </div>
      <Panel
        title="Impact Simulator"
        hint="Deterministic intent + repository capability baseline"
      >
        <div className="graph-controls">
          <input
            className="filter-input"
            type="search"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            placeholder="Describe your planned change…"
            aria-label="Describe your planned change"
          />
          <button
            type="button"
            className="flow-button"
            disabled={!query || loading}
            onClick={() => onRequest(query)}
          >
            {loading ? 'Analyzing…' : 'Analyze'}
          </button>
        </div>
        {error && (
          <div className="error-banner" role="alert">
            <span>{error}</span>{' '}
            <button type="button" className="ghost-button" onClick={onRetry}>
              Retry
            </button>
          </div>
        )}
        {!response && !loading && !error && (
          <p className="history-note">
            Try “Replace JWT authentication with OAuth2” or “Add caching to
            the API”. Nothing is modified — this is analysis only.
          </p>
        )}
        {response && (
          <div className="impact-results">
            <p className="history-note" role="status">
              {response.impact.length} impacted files · model{' '}
              {response.model.name} (uncalibrated baseline)
            </p>
            <div className="impact-intent" aria-label="Detected intent">
              <span className="eyebrow">Detected intent</span>
              <p>
                Operation: {response.intent.operation} · Domains:{' '}
                {response.intent.domains.length > 0
                  ? response.intent.domains.join(', ')
                  : 'general'}
                {response.intent.current_technology &&
                  ` · Current: ${response.intent.current_technology}`}
                {response.intent.target_technology &&
                  ` · Target: ${response.intent.target_technology}`}
              </p>
            </div>
            {groups.map(
              (group) =>
                group.items.length > 0 && (
                  <section key={group.level} aria-label={`${group.level} impact`}>
                    <h3 className={`impact-level impact-${group.level}`}>
                      {group.level.toUpperCase()}
                    </h3>
                    <ul className="impact-candidates">
                      {group.items.map((item) => (
                        <li key={item.path} className="impact-candidate">
                          <div className="impact-candidate-head">
                            <button
                              type="button"
                              className="link-button"
                              onClick={() => onOpenFile(item.path)}
                            >
                              {item.path}
                            </button>
                            <span
                              className="tnum"
                              aria-label={`score ${pct(item.score)}`}
                            >
                              {pct(item.score)}
                            </span>
                            <span className="muted">
                              {levelLabel(item.level)}
                            </span>
                            <button
                              type="button"
                              className="ghost-button"
                              aria-expanded={expanded === item.path}
                              onClick={() =>
                                setExpanded((current) =>
                                  current === item.path ? null : item.path,
                                )
                              }
                            >
                              {expanded === item.path
                                ? 'Hide evidence'
                                : 'Evidence'}
                            </button>
                          </div>
                          {expanded === item.path && (
                            <div className="impact-evidence">
                              <p className="muted">
                                Categories: {item.categories.join(', ')}
                              </p>
                              <ul>
                                {item.evidence.map((line) => (
                                  <li key={line}>{line}</li>
                                ))}
                              </ul>
                            </div>
                          )}
                        </li>
                      ))}
                    </ul>
                  </section>
                ),
            )}
            {response.graph.nodes.length > 1 && (
              <div className="impact-map" aria-label="Impact map">
                <h3>Impact map</h3>
                <ul className="impact-tree">
                  {response.graph.nodes
                    .filter((node) => node.kind === 'capability')
                    .map((capability) => (
                      <li key={capability.id}>
                        <strong>{capability.label}</strong>{' '}
                        <span className="muted">
                          ({capability.level}-impact candidate)
                        </span>
                        <ul>
                          {response.graph.edges
                            .filter((edge) => edge.from === capability.id)
                            .map((edge) => (
                              <li key={edge.to}>
                                <button
                                  type="button"
                                  className="link-button"
                                  onClick={() => onOpenFile(edge.to)}
                                >
                                  {edge.to}
                                </button>{' '}
                                <span className="muted">· {edge.evidence}</span>
                              </li>
                            ))}
                        </ul>
                      </li>
                    ))}
                </ul>
                <button
                  type="button"
                  className="flow-ghost"
                  onClick={onOpenGraph}
                >
                  Open dependency graph
                </button>
              </div>
            )}
            {response.checklist.length > 0 && (
              <div className="impact-checklist" aria-label="Review checklist">
                <h3>Review checklist</h3>
                <p className="history-note">
                  Recommendations for review, not automatic changes.
                </p>
                <ul>
                  {response.checklist.map((entry) => (
                    <li key={entry.label}>
                      <label>
                        <input
                          type="checkbox"
                          checked={checked.includes(entry.label)}
                          onChange={() => toggleCheck(entry.label)}
                        />{' '}
                        {entry.label}
                      </label>{' '}
                      <span className="muted">
                        {entry.files.map((file, index) => (
                          <span key={file}>
                            {index > 0 && ', '}
                            <button
                              type="button"
                              className="link-button"
                              onClick={() => onOpenFile(file)}
                            >
                              {file}
                            </button>
                          </span>
                        ))}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}
      </Panel>
    </>
  )
}
