import { useMemo, useState } from 'react'
import type { FilePrediction, RepositoryAnalysis } from '../types'
import { cochangePartners } from '../lib/files'
import { Panel } from './Panel'
import { Predictions } from './Predictions'

function toneFor(probability: number, label: number): string {
  if (label !== 1) return 'var(--teal)'
  const pct = probability * 100
  if (pct >= 80) return 'var(--danger)'
  if (pct >= 60) return 'var(--warn)'
  return 'var(--cyan)'
}

export function PredictView({
  analysis,
  paths,
  predictions,
  prediction,
  selectedFile,
  modelName,
  availableModels,
  stats,
  loading,
  error,
  onRetry,
  onPickFile,
  onWhy,
  onOpenGraph,
}: {
  analysis: RepositoryAnalysis
  paths: string[]
  predictions: FilePrediction[] | null
  prediction: FilePrediction | null
  selectedFile: string | null
  modelName: string
  availableModels: string[]
  stats: Record<string, { changes: number; complexity: number; dependencies: number }>
  loading: boolean
  error: string | null
  onRetry: () => void
  onPickFile: (path: string) => void
  onWhy: (path: string) => void
  onOpenGraph?: (path: string) => void
}) {
  const [query, setQuery] = useState('')

  const matches = useMemo(() => {
    const term = query.trim().toLowerCase()
    if (!term) return []
    return paths.filter((file) => file.toLowerCase().includes(term)).slice(0, 8)
  }, [query, paths])

  const related = useMemo(
    () => (selectedFile ? cochangePartners(analysis, selectedFile, 5) : []),
    [analysis, selectedFile],
  )
  const relatedPeak = useMemo(
    () => related.reduce((max, item) => Math.max(max, item.count), 1),
    [related],
  )

  const pct = prediction ? prediction.probability * 100 : 0

  return (
    <>
      <div className="page-head">
        <h1>Change Prediction</h1>
        <p>
          {predictions ? `${predictions.length} files analyzed · ` : ''}
          model {modelName} · uncalibrated probability
        </p>
      </div>
      <div className="grid-2 predict-grid">
        <Panel title="Selected File" hint="Pick a file to predict">
          <div className="graph-controls">
            <input
              className="filter-input"
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search files…"
              aria-label="Select a file to predict"
            />
          </div>
          {matches.length > 0 && (
            <ul className="graph-matches" role="listbox" aria-label="Matching files">
              {matches.map((file) => (
                <li key={file} role="option" aria-selected={selectedFile === file}>
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => {
                      onPickFile(file)
                      setQuery('')
                    }}
                  >
                    {file}
                  </button>
                </li>
              ))}
            </ul>
          )}
          {!selectedFile ? (
            <p className="history-note">
              Select a file above, or pick any file from the table below, to
              see its prediction and likely related changes.
            </p>
          ) : (
            <>
              <p className="history-note" role="status">
                Selected: <strong>{selectedFile}</strong>
              </p>
              {!prediction ? (
                <p className="history-note">
                  No prediction data is available for this file yet.
                </p>
              ) : (
                <>
                  <div className="attention-prob">
                    <span className="track" aria-hidden="true">
                      <span
                        className="fill"
                        style={{
                          width: `${pct.toFixed(1)}%`,
                          background: toneFor(prediction.probability, prediction.label),
                        }}
                      />
                    </span>
                    <span className="pct tnum">{pct.toFixed(0)}%</span>
                  </div>
                  <p className="history-note">
                    {prediction.label === 1 ? 'At risk' : 'Looks OK'} ·{' '}
                    {prediction.confidence_level} confidence ·{' '}
                    {prediction.historical_examples.length} historical examples
                  </p>
                  <div className="attention-actions">
                    <button
                      type="button"
                      className="flow-button"
                      onClick={() => onWhy(selectedFile)}
                    >
                      Why? View evidence →
                    </button>
                    {onOpenGraph && (
                      <button
                        type="button"
                        className="flow-ghost"
                        onClick={() => onOpenGraph(selectedFile)}
                      >
                        Open graph
                      </button>
                    )}
                  </div>
                </>
              )}
            </>
          )}
        </Panel>
        <Panel title="Likely Related Changes" hint="Historical co-change partners">
          {!selectedFile ? (
            <p className="history-note">
              Related files appear here once a file is selected.
            </p>
          ) : related.length === 0 ? (
            <p className="history-note">
              No historical relationship found for {selectedFile} — it has not
              repeatedly changed alongside other files.
            </p>
          ) : (
            <ul className="related-list">
              {related.map((item) => (
                <li key={item.path} className="related-row">
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => onWhy(item.path)}
                    title={`Open evidence for ${item.path}`}
                  >
                    {item.path}
                  </button>
                  <span className="related-bar" aria-hidden="true">
                    <span
                      className="related-fill"
                      style={{ width: `${(item.count / relatedPeak) * 100}%` }}
                    />
                  </span>
                  <span className="tnum muted">×{item.count}</span>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>
      <Predictions
        predictions={predictions}
        modelName={modelName}
        availableModels={availableModels}
        stats={stats}
        loading={loading}
        error={error}
        onRetry={onRetry}
        onSelect={onWhy}
        onOpenGraph={onOpenGraph}
      />
    </>
  )
}
