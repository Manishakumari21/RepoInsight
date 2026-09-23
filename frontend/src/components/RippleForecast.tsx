import { useState } from 'react'
import type { RippleResponse } from '../types'
import { shortSha } from '../types'
import { Panel } from './Panel'

function pct(probability: number): string {
  return `${Math.round(probability * 100)}%`
}

export function RippleForecast({
  files,
  response,
  loading,
  error,
  onRetry,
  onRequest,
  onOpenFile,
}: {
  files: string[]
  response: RippleResponse | null
  loading: boolean
  error: string | null
  onRetry: () => void
  onRequest: (source: string) => void
  onOpenFile: (path: string) => void
}) {
  const [source, setSource] = useState('')
  const [expanded, setExpanded] = useState<string | null>(null)

  const query = source.trim()
  const matches =
    query.length === 0
      ? []
      : files.filter((file) => file.toLowerCase().includes(query.toLowerCase())).slice(0, 8)

  return (
    <>
      <div className="page-head">
        <h1>Change Ripple Forecast</h1>
        <p>
          When a file changes, which other files historically tend to change
          afterward? Ripple edges are directional follow-up predictions, not
          structural dependencies.
        </p>
      </div>
      <Panel
        title="Ripple Forecast"
        hint="Structural + co-change + temporal follow-up baseline"
      >
        <div className="graph-controls">
          <input
            className="filter-input"
            type="search"
            value={source}
            onChange={(event) => setSource(event.target.value)}
            placeholder="Select a source file…"
            aria-label="Select a ripple source file"
          />
          <button
            type="button"
            className="flow-button"
            disabled={!query || loading}
            onClick={() => onRequest(query)}
          >
            {loading ? 'Forecasting…' : 'Forecast ripple'}
          </button>
        </div>
        {matches.length > 0 && (
          <ul className="graph-matches" role="listbox" aria-label="Matching files">
            {matches.map((file) => (
              <li key={file} role="option" aria-selected={false}>
                <button
                  type="button"
                  className="link-button"
                  onClick={() => {
                    setSource(file)
                    onRequest(file)
                  }}
                >
                  {file}
                </button>
              </li>
            ))}
          </ul>
        )}
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
            Select a file to forecast its likely change ripple. Evidence comes
            from observed repository history — never fabricated.
          </p>
        )}
        {response && (
          <div className="ripple-results">
            <p className="history-note" role="status">
              Source {response.source} · {response.candidates.length} candidates ·
              model {response.model.name} (uncalibrated baseline)
            </p>
            {response.ripple_paths.length > 0 ? (
              <div className="ripple-path" aria-label="Predicted ripple path">
                {response.ripple_paths[0].nodes.map((node, index) => (
                  <div key={node} className="ripple-step">
                    <div className="ripple-file">
                      <span className="eyebrow">
                        {index === 0 ? 'Source' : `Step ${index}`}
                      </span>
                      <button
                        type="button"
                        className="link-button"
                        onClick={() => onOpenFile(node)}
                      >
                        {node}
                      </button>
                    </div>
                    {index < response.ripple_paths[0].edge_scores.length && (
                      <div className="ripple-arrow" aria-hidden="true">
                        ↓ {pct(response.ripple_paths[0].edge_scores[index])}
                      </div>
                    )}
                  </div>
                ))}
                <p className="history-note">
                  Path confidence {pct(response.ripple_paths[0].confidence)} ·
                  mean of edge scores, max depth {response.ripple_paths[0].nodes.length - 1}
                </p>
              </div>
            ) : (
              <p className="history-note">
                No ripple path — no edge from {response.source} meets the
                confidence threshold. Weak evidence is not forced into a path.
              </p>
            )}
            {response.candidates.length > 0 && (
              <ul className="ripple-candidates">
                {response.candidates.map((candidate) => (
                  <li key={candidate.file} className="ripple-candidate">
                    <div className="ripple-candidate-head">
                      <button
                        type="button"
                        className="link-button"
                        onClick={() => onOpenFile(candidate.file)}
                      >
                        {candidate.file}
                      </button>
                      <span className="tnum" aria-label={`probability ${pct(candidate.probability)}`}>
                        {pct(candidate.probability)}
                      </span>
                      <span className="muted">rank {candidate.rank}</span>
                      <button
                        type="button"
                        className="ghost-button"
                        aria-expanded={expanded === candidate.file}
                        onClick={() =>
                          setExpanded((current) =>
                            current === candidate.file ? null : candidate.file,
                          )
                        }
                      >
                        {expanded === candidate.file ? 'Hide evidence' : 'Evidence'}
                      </button>
                    </div>
                    {expanded === candidate.file && (
                      <div className="ripple-evidence">
                        <ul>
                          {candidate.reasons.map((reason) => (
                            <li key={reason}>{reason}</li>
                          ))}
                        </ul>
                        {candidate.historical_examples.length > 0 && (
                          <>
                            <p className="eyebrow">Historical examples</p>
                            <ul>
                              {candidate.historical_examples.map((example) => (
                                <li key={`${example.commit_sha}-${example.event_type}`}>
                                  <span className="mono">{shortSha(example.commit_sha)}</span>{' '}
                                  · {example.event_type} ·{' '}
                                  {example.related_files.join(', ')}
                                </li>
                              ))}
                            </ul>
                          </>
                        )}
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            )}
            {response.missing_impact.length > 0 && (
              <div className="ripple-missing" role="note" aria-label="Potential missing impact">
                <h3>Potential missing impact</h3>
                <p className="history-note">
                  These files frequently changed in comparable historical
                  changes. Consider reviewing them.
                </p>
                <ul>
                  {response.missing_impact.map((item) => (
                    <li key={item.file}>
                      <button
                        type="button"
                        className="link-button"
                        onClick={() => onOpenFile(item.file)}
                      >
                        {item.file}
                      </button>{' '}
                      <span className="muted">
                        · historical follow probability {pct(item.probability)}
                      </span>
                      <p className="history-note">{item.message}</p>
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
