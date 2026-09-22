import { useMemo, useState } from 'react'
import type { FilePrediction } from '../types'
import {
  filterPredictions,
  sortPredictions,
  type BandFilter,
  type LabelFilter,
  type SortDir,
  type SortKey,
} from '../lib/predictions'
import { Panel } from './Panel'

export function Predictions({
  predictions,
  modelName,
  availableModels,
  stats,
  loading,
  error,
  onRetry,
  onSelect,
  onOpenGraph,
}: {
  predictions: FilePrediction[] | null
  modelName: string
  availableModels: string[]
  stats: Record<string, { changes: number; complexity: number; dependencies: number }>
  loading: boolean
  error: string | null
  onRetry: () => void
  onSelect: (path: string) => void
  onOpenGraph?: (path: string) => void
}) {
  const [query, setQuery] = useState('')
  const [label, setLabel] = useState<LabelFilter>('all')
  const [band, setBand] = useState<BandFilter>('all')
  const [model, setModel] = useState('all')
  const [sortKey, setSortKey] = useState<SortKey>('probability')
  const [sortDir, setSortDir] = useState<SortDir>('desc')

  const visible = useMemo(() => {
    if (!predictions) return []
    const filtered = filterPredictions(
      predictions,
      { query, label, band, model },
      modelName,
    )
    return sortPredictions(filtered, sortKey, sortDir, stats)
  }, [predictions, query, label, band, model, modelName, sortKey, sortDir, stats])

  const attention = useMemo(() => {
    if (!predictions || predictions.length <= 3) return []
    return [...predictions]
      .filter((item) => item.label === 1)
      .sort((a, b) => b.probability - a.probability)
      .slice(0, 4)
  }, [predictions])

  function toggleSort(key: SortKey) {
    if (key === sortKey) {
      setSortDir((dir) => (dir === 'asc' ? 'desc' : 'asc'))
    } else {
      setSortKey(key)
      setSortDir(key === 'file' ? 'asc' : 'desc')
    }
  }

  return (
    <>
      <div className="page-head">
        <h1>Change Propagation Predictions</h1>
        <p>
          {predictions ? `${predictions.length} files analyzed · ` : ''}
          model {modelName} · uncalibrated probability
        </p>
      </div>
      {attention.length > 0 && (
        <div className="attention-grid" aria-label="Files needing attention">
          {attention.map((item) => {
            const pct = item.probability * 100
            const tone =
              pct >= 80 ? 'var(--danger)' : pct >= 60 ? 'var(--warn)' : 'var(--cyan)'
            const why = item.top_evidence[0]
            const cochanges = item.historical_examples.length
            return (
              <article key={item.file_path} className="attention-card">
                <div className="attention-top">
                  <span className="attention-file">{item.file_path}</span>
                  <span className="severity high">High risk</span>
                </div>
                <div className="attention-prob">
                  <span className="track" aria-hidden="true">
                    <span className="fill" style={{ width: `${pct.toFixed(1)}%`, background: tone }} />
                  </span>
                  <span className="pct tnum">{pct.toFixed(0)}%</span>
                </div>
                <div className="attention-facts">
                  <span>Rework probability <b>{pct.toFixed(1)}%</b></span>
                  <span><b>{cochanges}</b> historical examples</span>
                  <span><b>{stats[item.file_path]?.dependencies ?? 0}</b> dependent files</span>
                </div>
                {why && (
                  <p className="attention-why">
                    <span className="k">Why?</span>
                    {why.description}
                  </p>
                )}
                <div className="attention-actions">
                  <button type="button" className="flow-ghost" onClick={() => onSelect(item.file_path)}>
                    View explanation
                  </button>
                  {onOpenGraph && (
                    <button type="button" className="flow-ghost" onClick={() => onOpenGraph(item.file_path)}>
                      Open graph
                    </button>
                  )}
                </div>
              </article>
            )
          })}
        </div>
      )}
    <Panel title="Prediction Explorer" hint="Per-file rework probability">
      {loading && (
        <p className="history-note" role="status">
          Loading predictions…
        </p>
      )}
      {error && !loading && (
        <div className="empty-state" role="alert">
          <p>No prediction data is available for this repository yet.</p>
          <p className="history-note">{error}</p>
          <button type="button" className="landing-button" onClick={onRetry}>
            Retry
          </button>
        </div>
      )}
      {!loading && !error && predictions && predictions.length === 0 && (
        <div className="empty-state">
          <p>No prediction data is available for this repository yet.</p>
          <p className="history-note">
            Predictions need parsed source files with repository history.
          </p>
        </div>
      )}
      {!loading && !error && predictions && predictions.length > 0 && (
        <>
          <div className="filter-bar">
            <input
              className="filter-input"
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search files…"
              aria-label="Search predicted files"
            />
            <label className="filter-field">
              Prediction
              <select
                value={label}
                onChange={(event) => setLabel(event.target.value as LabelFilter)}
                aria-label="Filter by predicted label"
              >
                <option value="all">All</option>
                <option value="positive">At risk</option>
                <option value="negative">OK</option>
              </select>
            </label>
            <label className="filter-field">
              Probability
              <select
                value={band}
                onChange={(event) => setBand(event.target.value as BandFilter)}
                aria-label="Filter by probability band"
              >
                <option value="all">All</option>
                <option value="high">High</option>
                <option value="medium">Medium</option>
                <option value="low">Low</option>
              </select>
            </label>
            <label className="filter-field">
              Model
              <select
                value={model}
                onChange={(event) => setModel(event.target.value)}
                aria-label="Filter by model"
              >
                <option value="all">All</option>
                {availableModels.map((name) => (
                  <option key={name} value={name}>
                    {name}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <p className="history-note" role="status">
            Showing {visible.length} of {predictions.length} files · model {modelName} ·
            uncalibrated probability
          </p>
          <div className="table-wrap">
            <table className="pred-table">
              <thead>
                <tr>
                  <th>
                    <button type="button" className="th-sort" onClick={() => toggleSort('file')}>
                      File {sortKey === 'file' ? (sortDir === 'asc' ? '▲' : '▼') : ''}
                    </button>
                  </th>
                  <th>
                    <button
                      type="button"
                      className="th-sort"
                      onClick={() => toggleSort('probability')}
                    >
                      Probability {sortKey === 'probability' ? (sortDir === 'asc' ? '▲' : '▼') : ''}
                    </button>
                  </th>
                  <th>Label</th>
                  <th>Confidence</th>
                </tr>
              </thead>
              <tbody>
                {visible.slice(0, 200).map((item) => {
                  const pct = item.probability * 100
                  const tone =
                    item.label === 1
                      ? pct >= 80
                        ? 'var(--danger)'
                        : pct >= 60
                          ? 'var(--warn)'
                          : 'var(--cyan)'
                      : 'var(--teal)'
                  return (
                    <tr key={item.file_path} className={item.label === 1 ? 'row-risk' : ''}>
                      <td>
                        <button
                          type="button"
                          className="link-button"
                          onClick={() => onSelect(item.file_path)}
                          title={`Open details for ${item.file_path}`}
                        >
                          {item.file_path}
                        </button>
                      </td>
                      <td>
                        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                          <span
                            aria-hidden="true"
                            style={{
                              display: 'inline-block',
                              width: 56,
                              height: 5,
                              borderRadius: 999,
                              background: 'var(--bg-secondary)',
                              border: '1px solid var(--border-muted)',
                              overflow: 'hidden',
                              verticalAlign: 'middle',
                            }}
                          >
                            <span
                              style={{
                                display: 'block',
                                height: '100%',
                                width: `${pct.toFixed(1)}%`,
                                background: tone,
                              }}
                            />
                          </span>
                          <span className="tnum">{pct.toFixed(1)}%</span>
                        </span>
                      </td>
                      <td>
                        <span className={`severity ${item.label === 1 ? 'high' : 'low'}`}>
                          {item.label === 1 ? 'At risk' : 'OK'}
                        </span>
                      </td>
                      <td className="muted">{item.confidence_level}</td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
          {visible.length > 200 && (
            <p className="history-note">First 200 rows shown — refine the search.</p>
          )}
          {visible.length === 0 && (
            <div className="empty-state">
              <p>No files match these filters.</p>
            </div>
          )}
        </>
      )}
    </Panel>
    </>
  )
}
