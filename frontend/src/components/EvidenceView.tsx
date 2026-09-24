import { useMemo, useState } from 'react'
import type { FilePrediction } from '../types'
import { formatNumber, shortSha } from '../types'
import { predictionLevel, predictionLevelClass } from '../lib/labels'
import { Panel } from './Panel'
import { Tooltip } from './Tooltip'
import { EmptyState } from './EmptyState'

export function EvidenceView({
  predictions,
  loading,
  error,
  onOpenFile,
}: {
  predictions: FilePrediction[] | null
  loading: boolean
  error: string | null
  onOpenFile: (path: string) => void
}) {
  const [selected, setSelected] = useState<string | null>(null)

  const ranked = useMemo(() => {
    if (!predictions) return []
    return [...predictions].sort((a, b) => b.probability - a.probability)
  }, [predictions])

  const active =
    ranked.find((item) => item.file_path === selected) ??
    ranked.find((item) => item.label === 1) ??
    ranked[0] ??
    null

  return (
    <>
      <div className="page-head">
        <h1>Prediction Evidence</h1>
        <p>
          See the repository history and relationships behind each
          prediction. Evidence is observed fact — the prediction is the
          model&apos;s interpretation of it.
        </p>
      </div>
      {loading && (
        <p className="history-note" role="status">
          Loading predictions…
        </p>
      )}
      {error && !loading && (
        <div className="empty-state" role="alert">
          <p>Prediction evidence is unavailable.</p>
          <p className="history-note">{error}</p>
        </div>
      )}
      {!loading && !error && ranked.length === 0 && (
        <div className="empty-state">
          <p>No predictions to explain yet.</p>
          <p className="history-note">
            Predictions need parsed source files with repository history.
          </p>
        </div>
      )}
      {!loading && !error && ranked.length > 0 && active && (
        <div className="grid-2 evidence-grid">
          <Panel title="Predictions" hint={`${ranked.length} files analyzed`}>
            <ul className="evidence-pick-list">
              {ranked.slice(0, 60).map((item) => (
                <li key={item.file_path}>
                  <button
                    type="button"
                    className={`evidence-pick${active.file_path === item.file_path ? ' active' : ''}`}
                    onClick={() => setSelected(item.file_path)}
                    aria-pressed={active.file_path === item.file_path}
                  >
                    <span className="evidence-pick-file mono">{item.file_path}</span>
                    <span className="tnum muted">
                      {(item.probability * 100).toFixed(0)}%
                    </span>
                    <span
                      className={`status ${predictionLevelClass(predictionLevel(item.probability, item.label))}`}
                    >
                      {predictionLevel(item.probability, item.label)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
            {ranked.length > 60 && (
              <p className="history-note">First 60 predictions shown.</p>
            )}
          </Panel>
          <div className="evidence-main">
            <Panel
              title="Evidence timeline"
              hint={`Why ${active.file_path} was flagged`}
            >
              <ol className="evidence-steps">
                {active.top_evidence.map((entry, index) => (
                  <li key={entry.feature} className="evidence-step">
                    <span className="evidence-step-num" aria-hidden="true">
                      {index + 1}
                    </span>
                    <div>
                      <strong>{entry.feature}</strong>
                      <p>
                        {entry.description} Observed value:{' '}
                        {entry.raw_value === null
                          ? 'n/a'
                          : formatNumber(entry.raw_value)}
                        .
                      </p>
                    </div>
                  </li>
                ))}
              </ol>
              {active.top_evidence.length === 0 && (
                <EmptyState title="No feature evidence was recorded for this prediction." body="The model scored this file, but no individual feature contribution was stored. Historical examples below may still help." />
              )}
              {active.historical_examples.length > 0 ? (
                <>
                  <p className="eyebrow">
                    <Tooltip term="Historical example" />s
                  </p>
                  <ul className="example-list">
                    {active.historical_examples.map((example, index) => (
                      <li
                        key={`${example.commit_sha}-${example.event_type}-${index}`}
                        className="example-row"
                      >
                        <div className="example-top">
                          <code>{shortSha(example.commit_sha)}</code>
                          <span className="chip">
                            {example.event_type.replace('_', ' ')}
                          </span>
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
                </>
              ) : (
                <EmptyState title="No matching historical change sets were found." body="No sampled commit resembles this prediction yet. This can happen on short histories." />
              )}
            </Panel>
            <Panel title="Explanation" hint="What this means">
              <p>
                <Tooltip term="Prediction" />: changing{' '}
                <strong>{active.file_path}</strong> will probably need
                follow-up fixes about{' '}
                {(active.probability * 100).toFixed(0)}% of the time.
              </p>
              <p className="history-note">
                <Tooltip term="Confidence" />: {active.confidence_level}
                {active.calibrated ? ' · calibrated' : ' · uncalibrated'}.
                This is the model&apos;s best guess — not a guarantee.
              </p>
              {active.recommendations.length > 0 && (
                <>
                  <p className="eyebrow">Suggestions — not guaranteed outcomes</p>
                  <ul className="reco-list">
                    {active.recommendations.map((text) => (
                      <li key={text}>{text}</li>
                    ))}
                  </ul>
                </>
              )}
              <button
                type="button"
                className="flow-ghost"
                onClick={() => onOpenFile(active.file_path)}
              >
                Open file details →
              </button>
            </Panel>
          </div>
        </div>
      )}
    </>
  )
}
