import type { FilePrediction, RepositoryAnalysis } from '../types'
import { formatBytes, formatDate, formatNumber, shortSha } from '../types'
import { buildFileStats, cochangePartners } from '../lib/files'

function directionMark(direction: string): string {
  if (direction === 'supports') return '▲'
  if (direction === 'contradicts') return '▼'
  return '·'
}

export function FileDrawer({
  path,
  prediction,
  analysis,
  onClose,
  onOpenFile,
  onOpenGraph,
  onGoHistory,
}: {
  path: string
  prediction: FilePrediction | null
  analysis: RepositoryAnalysis
  onClose: () => void
  onOpenFile: (path: string) => void
  onOpenGraph?: (path: string) => void
  onGoHistory?: () => void
}) {
  const stats = buildFileStats(analysis)[path]
  const structural = analysis.structural_features?.find(
    (item) => item.file_path === path,
  )
  const related = cochangePartners(analysis, path, 5)
  const relatedPeak = related.reduce((max, item) => Math.max(max, item.count), 1)
  const dependencies = (analysis.propagation?.edges ?? [])
    .filter((edge) => edge.source === path || edge.target === path)
    .sort((a, b) => b.strength - a.strength)
    .slice(0, 6)
  const activity = (analysis.timeline?.entries ?? []).filter((entry) =>
    (entry.files ?? []).includes(path),
  )
  const recentActivity = [...activity].reverse().slice(0, 3)
  const verdict = prediction
    ? prediction.label === 1
      ? 'Needs follow-up'
      : 'Looks fine'
    : null

  return (
    <div className="drawer-overlay" onClick={onClose}>
      <aside
        className="drawer"
        role="dialog"
        aria-modal="true"
        aria-label={`Details for ${path}`}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="drawer-head">
          <div>
            <div className="panel-sub-title">File details</div>
            <h2 className="drawer-title">{path}</h2>
            {prediction && (
              <div style={{ display: 'flex', gap: 8, marginTop: 10, flexWrap: 'wrap' }}>
                <span className={`severity ${prediction.label === 1 ? 'high' : 'low'}`}>
                  {prediction.label === 1 ? 'high rework risk' : 'low rework risk'}
                </span>
                <span className="chip">
                  {(prediction.probability * 100).toFixed(1)}% · {prediction.confidence_level} confidence
                </span>
              </div>
            )}
          </div>
          <button type="button" className="drawer-close" onClick={onClose} aria-label="Close file details">
            ✕
          </button>
        </div>

        <section aria-label="Prediction">
          <h3 className="drawer-section">Prediction</h3>
          {!prediction ? (
            <p className="history-note">
              No prediction data is available for this file yet.
            </p>
          ) : (
            <>
              <div className="plain-card" role="note" aria-label="Plain language summary">
                <span className="plain-icon" aria-hidden="true">
                  {prediction.label === 1 ? (
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--warn)" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
                      <path d="M12 3l10 17H2z" />
                      <path d="M12 10v4M12 17.5v.5" />
                    </svg>
                  ) : (
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--teal)" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
                      <circle cx="12" cy="12" r="9" />
                      <path d="M8.5 12.5l2.5 2.5 4.5-5.5" />
                    </svg>
                  )}
                </span>
                <p>
                  <strong>What this means: </strong>
                  {prediction.label === 1
                    ? `changing this file will probably need follow-up fixes about ${(prediction.probability * 100).toFixed(0)}% of the time.`
                    : 'changing this file will probably not need follow-up fixes.'}{' '}
                  This is the model&apos;s best guess from past history — not a guarantee.
                </p>
              </div>
              <div className="stat-grid">
                <div className="stat">
                  <div className="stat-value">
                    {(prediction.probability * 100).toFixed(1)}%
                  </div>
                  <div className="stat-label">Chance of extra fix-up work</div>
                </div>
                <div className="stat">
                  <div className="stat-value">
                    {verdict}
                  </div>
                  <div className="stat-label">Verdict</div>
                </div>
                <div className="stat">
                  <div className="stat-value">{prediction.confidence_level}</div>
                  <div className="stat-label">
                    Confidence{prediction.calibrated ? ' · calibrated' : ' · uncalibrated'}
                  </div>
                </div>
              </div>
              {onOpenGraph && (
                <button
                  type="button"
                  className="flow-button drawer-graph-button"
                  onClick={() => onOpenGraph(path)}
                >
                  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" aria-hidden="true">
                    <circle cx="6" cy="6" r="2.4" />
                    <circle cx="18" cy="8" r="2.4" />
                    <circle cx="10" cy="18" r="2.4" />
                    <path d="M8.2 7l7.4.7M7 8.2l2 7.3M16 10l-4.4 6" />
                  </svg>
                  See this file in the change graph
                </button>
              )}
            </>
          )}
        </section>

        {prediction && (
          <section aria-label="Why this prediction">
            <h3 className="drawer-section">Why we think so</h3>
            <p className="eyebrow">Signals from past history</p>
            <ul className="evidence-list">
              {prediction.top_evidence.map((entry) => (
                <li key={entry.feature} className="evidence-row">
                  <span className="evidence-mark" aria-hidden="true">
                    {directionMark(entry.direction)}
                  </span>
                  <span className="evidence-body">
                    <strong>{entry.feature}</strong>
                    <em>
                      {entry.raw_value === null ? 'n/a' : formatNumber(entry.raw_value)} ·{' '}
                      {entry.direction === 'unknown'
                        ? 'associated with the prediction'
                        : `${entry.direction === 'supports' ? 'supports' : 'contradicts'} a positive prediction`}
                    </em>
                  </span>
                  <span className="evidence-value">
                    {entry.contribution >= 0 ? '+' : ''}
                    {entry.contribution.toFixed(2)}
                  </span>
                </li>
              ))}
            </ul>
            {prediction.top_evidence.length === 0 && (
              <p className="history-note">No feature evidence available.</p>
            )}
          </section>
        )}

        <section aria-label="Predicted changes">
          <h3 className="drawer-section">Predicted changes</h3>
          {related.length === 0 ? (
            <p className="history-note">
              No files have repeatedly changed alongside this one.
            </p>
          ) : (
            <ul className="related-list">
              {related.map((item) => (
                <li key={item.path} className="related-row">
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => onOpenFile(item.path)}
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
        </section>

        <section aria-label="Dependencies">
          <h3 className="drawer-section">Dependencies</h3>
          {dependencies.length === 0 ? (
            <p className="history-note">No recorded relationships for this file.</p>
          ) : (
            <ul className="graph-neighbors">
              {dependencies.map((edge) => {
                const other = edge.source === path ? edge.target : edge.source
                const direction = edge.source === path ? 'out' : 'in'
                return (
                  <li key={`${edge.source}-${edge.target}`}>
                    <button
                      type="button"
                      className="link-button"
                      onClick={() => onOpenFile(other)}
                    >
                      {other}
                    </button>{' '}
                    <span className="muted">
                      · {direction} · strength {edge.strength}
                    </span>
                  </li>
                )
              })}
            </ul>
          )}
        </section>

        <section aria-label="Historical evidence">
          <h3 className="drawer-section">What happened before</h3>
          <p className="history-note" style={{ marginTop: 0 }}>
            Real past changes involving this file — not guesses.
          </p>
          {!prediction || prediction.historical_examples.length === 0 ? (
            <p className="history-note">No relevant historical examples found.</p>
          ) : (
            <ul className="example-list">
              {prediction.historical_examples.map((example, index) => (
                <li key={`${example.commit_sha}-${example.event_type}-${index}`} className="example-row">
                  <div className="example-top">
                    <code>{shortSha(example.commit_sha)}</code>
                    <span className="chip">{example.event_type.replace('_', ' ')}</span>
                  </div>
                  {example.related_files.length > 0 && (
                    <div className="example-files">
                      {example.related_files.map((file) => (
                        <button
                          key={file}
                          type="button"
                          className="link-button"
                          onClick={() => onOpenFile(file)}
                        >
                          {file}
                        </button>
                      ))}
                    </div>
                  )}
                </li>
              ))}
            </ul>
          )}
        </section>

        {prediction && prediction.recommendations.length > 0 && (
          <section aria-label="Recommendations">
            <h3 className="drawer-section">What to do about it</h3>
            <p className="eyebrow">Suggestions — not guaranteed outcomes</p>
            <ul className="reco-list">
              {prediction.recommendations.map((text) => (
                <li key={text}>{text}</li>
              ))}
            </ul>
          </section>
        )}

        <section aria-label="Historical activity">
          <h3 className="drawer-section">Historical activity</h3>
          {activity.length === 0 ? (
            <p className="history-note">This file appears in no recorded commits.</p>
          ) : (
            <>
              <p className="history-note" role="status">
                Changed in {formatNumber(activity.length)}{' '}
                {activity.length === 1 ? 'commit' : 'commits'}.
              </p>
              <ul className="graph-neighbors">
                {recentActivity.map((entry) => (
                  <li key={entry.sha}>
                    <code>{shortSha(entry.sha)}</code>{' '}
                    <span className="muted">
                      {entry.message || '(no message)'} · {formatDate(entry.date)}
                    </span>
                  </li>
                ))}
              </ul>
              {onGoHistory && (
                <button
                  type="button"
                  className="flow-ghost"
                  onClick={onGoHistory}
                >
                  View full history →
                </button>
              )}
            </>
          )}
        </section>

        <section aria-label="File facts">
          <h3 className="drawer-section">File at a glance</h3>
          {stats ? (
            <div className="fact-grid">
              <span>Size</span>
              <span>{formatBytes(stats.size)}</span>
              <span>How tangled the code is</span>
              <span>{formatNumber(stats.complexity)}</span>
              <span>Connected files</span>
              <span>{formatNumber(stats.dependencies)}</span>
              <span>Times changed before</span>
              <span>{formatNumber(stats.changes)}</span>
              {structural && (
                <>
                  <span>Functions</span>
                  <span>{formatNumber(structural.function_count)}</span>
                  <span>Lines</span>
                  <span>{formatNumber(structural.lines_of_code)}</span>
                </>
              )}
            </div>
          ) : (
            <p className="history-note">No structural facts for this file.</p>
          )}
        </section>
      </aside>
    </div>
  )
}
