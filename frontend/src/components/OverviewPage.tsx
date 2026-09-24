import { useMemo } from 'react'
import type { FilePrediction, RepositoryAnalysis } from '../types'
import { formatDate, formatNumber } from '../types'
import { bucketSeries } from '../lib/analysis'
import { activityLevel, changeCounts, directoryStats } from '../lib/files'
import { riskDistribution } from '../lib/predictions'
import { Donut } from './Donut'
import { Panel } from './Panel'
import { Tooltip } from './Tooltip'
import { TimelineChart } from './TimelineChart'
import { DataTable } from './DataTable'

export function OverviewPage({
  analysis,
  sourceLabel,
  predictions,
  predictionsAvailable,
  onOpenFile,
  onOpenArea,
  onOpenHistory,
}: {
  analysis: RepositoryAnalysis
  sourceLabel: string
  predictions: FilePrediction[]
  predictionsAvailable: boolean
  onOpenFile: (path: string) => void
  onOpenArea: (directory: string) => void
  onOpenHistory: () => void
}) {
  const distribution = useMemo(
    () => riskDistribution(predictions),
    [predictions],
  )

  const areas = useMemo(() => directoryStats(analysis), [analysis])
  const areaPeak = useMemo(
    () => areas.reduce((max, area) => Math.max(max, area.changes), 0),
    [areas],
  )

  const buckets = useMemo(
    () => bucketSeries(analysis.timeline.entries ?? []),
    [analysis],
  )
  const totalAreaCommits = useMemo(() => {
    let total = 0
    for (const area of areas) total += area.changes
    return total
  }, [areas])

  const recommended = useMemo(() => {
    const items: { title: string; sub: string; file: string }[] = []
    const topPair = [...(analysis.cochange.pairs ?? [])].sort(
      (a, b) => b.count - a.count,
    )[0]
    if (topPair) {
      items.push({
        title: `${topPair.file_a} has strong historical coupling with ${topPair.file_b}`,
        sub: `Changed together ${formatNumber(topPair.count)}× — inspect either file for evidence.`,
        file: topPair.file_a,
      })
    }
    const topRisk = [...predictions]
      .filter((item) => item.label === 1)
      .sort((a, b) => b.probability - a.probability)[0]
    if (topRisk) {
      items.push({
        title: `${topRisk.file_path} is flagged at ${(topRisk.probability * 100).toFixed(0)}% rework risk`,
        sub: 'Open its prediction to see why, then check the evidence.',
        file: topRisk.file_path,
      })
    }
    const counts = changeCounts(analysis)
    const mostActive = [...counts.entries()].sort((a, b) => b[1] - a[1])[0]
    if (mostActive) {
      items.push({
        title: `${mostActive[0]} changed ${formatNumber(mostActive[1])}× — the most active file`,
        sub: 'See what it touched and what tends to change with it.',
        file: mostActive[0],
      })
    }
    return items.slice(0, 3)
  }, [analysis, predictions])

  const signals = useMemo(
    () => [
      {
        key: 'complexity',
        label: 'Avg complexity',
        term: 'Complexity',
        value: analysis.complexity.average_complexity.toFixed(1),
        sub: `${formatNumber(analysis.complexity.complex_files)} complex files`,
      },
      {
        key: 'churn',
        label: 'Total churn',
        term: 'Churn',
        value: formatNumber(analysis.history.total_churn),
        sub: `${formatNumber(analysis.history.total_additions)} additions`,
      },
      {
        key: 'coupling',
        label: 'Co-change pairs',
        term: 'Coupling',
        value: formatNumber(analysis.cochange.total_pairs),
        sub: `${formatNumber(analysis.dependencies.total_dependencies)} dependencies`,
      },
      {
        key: 'predictions',
        label: 'Flagged files',
        term: 'Prediction',
        value: predictionsAvailable ? formatNumber(distribution.positive) : '—',
        sub: predictionsAvailable ? `of ${formatNumber(predictions.length)} analyzed` : 'model pending',
      },
    ],
    [analysis, distribution, predictionsAvailable, predictions.length],
  )

  const period =
    analysis.history.first_commit && analysis.history.last_commit
      ? `${formatDate(analysis.history.first_commit)} → ${formatDate(analysis.history.last_commit)}`
      : 'No dated history'

  return (
    <div className="overview-page">
      <div className="page-head">
        <p className="eyebrow">Repository</p>
        <h1>{sourceLabel}</h1>
        <p>Repository intelligence and evolution analysis.</p>
      </div>

      <div className="section-block">
        <div className="summary-row">
          <div className="summary-card">
            <div className="stat-label">Directories</div>
            <div className="stat-value tnum">{formatNumber(areas.length)}</div>
            <div className="stat-sub">{formatNumber(analysis.source.source_files)} files</div>
          </div>
          <div className="summary-card">
            <div className="stat-label">Commits</div>
            <div className="stat-value tnum">{formatNumber(analysis.history.total_commits)}</div>
            <div className="stat-sub">{formatNumber(analysis.history.active_contributors)} contributors</div>
          </div>
          <div className="summary-card">
            <div className="stat-label">Analysis period</div>
            <div className="stat-value period">{period}</div>
            <div className="stat-sub">{formatNumber(totalAreaCommits)} file changes</div>
          </div>
          <div className="summary-card">
            <div className="stat-label">Predictions</div>
            <div className="stat-value tnum">
              {predictionsAvailable ? formatNumber(predictions.length) : '—'}
            </div>
            <div className="stat-sub">
              {predictionsAvailable
                ? `${formatNumber(distribution.positive)} flagged files`
                : 'model pending'}
            </div>
          </div>
        </div>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Repository activity</h2>
          <p>Commits and files changed per week</p>
        </div>
        <Panel title="Activity" hint="From recorded commit history">
          <TimelineChart
            buckets={buckets}
            label={`Repository activity chart: ${analysis.history.total_commits} commits`}
          />
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Repository areas</h2>
          <p>Directories by change activity — select a row to explore it</p>
        </div>
        <Panel title="Areas" hint="Aggregated from analyzed files">
          <DataTable
            labelledBy="Repository areas"
            emptyText="No repository areas found. Analysis needs parsed source files with history."
            maxRows={20}
            defaultSort={{ key: 'changes', dir: 'desc' }}
            rows={areas}
            columns={[
              {
                key: 'directory',
                label: 'Area',
                sortable: true,
                sortValue: (row) => row.directory,
                render: (row) => (
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => onOpenArea(row.directory)}
                  >
                    {row.directory}
                  </button>
                ),
              },
              {
                key: 'files',
                label: 'Files',
                sortable: true,
                align: 'right',
                sortValue: (row) => row.files,
                render: (row) => formatNumber(row.files),
              },
              {
                key: 'changes',
                label: 'Changes',
                sortable: true,
                align: 'right',
                sortValue: (row) => row.changes,
                render: (row) => formatNumber(row.changes),
              },
              {
                key: 'contributors',
                label: 'Contributors',
                sortable: true,
                align: 'right',
                sortValue: (row) => row.contributors,
                render: (row) => formatNumber(row.contributors),
              },
              {
                key: 'activity',
                label: 'Activity',
                sortable: true,
                sortValue: (row) => row.changes,
                render: (row) => {
                  const level = activityLevel(row.changes, areaPeak)
                  return (
                    <span
                      className={`status ${level === 'high' ? 'increasing' : level === 'medium' ? 'stable' : 'weak'}`}
                      title="Activity relative to the most active area in this repository"
                    >
                      {level === 'high' ? 'High' : level === 'medium' ? 'Medium' : 'Low'}
                    </span>
                  )
                },
              },
            ]}
          />
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>What deserves attention?</h2>
          <p>Only what existing data supports</p>
        </div>
        <Panel title="Recommended Investigation" hint="Click a card to open the file">
          {recommended.length === 0 ? (
            <p className="history-note">
              Not enough history yet to recommend an investigation.
            </p>
          ) : (
            <div className="attention-grid">
              {recommended.map((item) => (
                <article key={item.title} className="attention-card">
                  <div className="attention-top">
                    <span className="attention-file">{item.file}</span>
                  </div>
                  <p className="attention-why">{item.title}</p>
                  <p className="history-note">{item.sub}</p>
                  <div className="attention-actions">
                    <button
                      type="button"
                      className="flow-ghost"
                      onClick={() => onOpenFile(item.file)}
                    >
                      Open file →
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Repository Signals</h2>
          <p>Complexity, change frequency, coupling, prediction activity</p>
        </div>
        <Panel title="Signals" hint="Observed repository measurements">
          <div className="stat-grid">
            {signals.map((signal) => (
              <div className="stat" key={signal.key}>
                <div className="stat-value tnum">{signal.value}</div>
                <div className="stat-label">
                  <Tooltip term={signal.term}>{signal.label}</Tooltip>
                </div>
                <div className="stat-sub">{signal.sub}</div>
              </div>
            ))}
          </div>
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Change Risk Summary</h2>
          <p>Potentially affected files</p>
        </div>
        <Panel title="Predicted Rework Distribution" hint="Model output · uncalibrated">
          {!predictionsAvailable ? (
            <p className="history-note">
              No prediction data is available for this repository yet.
            </p>
          ) : predictions.length === 0 ? (
            <p className="history-note">
              Predictions need parsed source files with repository history.
            </p>
          ) : (
            <div className="risk-donut-wrap">
              <Donut
                size={172}
                thickness={22}
                segments={[
                  { value: distribution.high, color: 'var(--danger)', name: 'High confidence' },
                  { value: distribution.medium, color: 'var(--warn)', name: 'Medium confidence' },
                  { value: distribution.low, color: 'var(--teal)', name: 'Low confidence' },
                ]}
                label={`Rework confidence: high ${distribution.high}, medium ${distribution.medium}, low ${distribution.low}`}
              >
                <div className="donut-value tnum">{formatNumber(distribution.positive)}</div>
                <div className="donut-caption">flagged</div>
              </Donut>
              <div className="risk-donut-legend">
                <div className="risk-donut-total">
                  of {formatNumber(predictions.length)} analyzed
                </div>
                <div className="risk-dist-row">
                  <span className="dot" style={{ background: 'var(--danger)' }} />
                  <span className="lbl">High confidence</span>
                  <span className="val">{formatNumber(distribution.high)}</span>
                </div>
                <div className="risk-dist-row">
                  <span className="dot" style={{ background: 'var(--warn)' }} />
                  <span className="lbl">Medium confidence</span>
                  <span className="val">{formatNumber(distribution.medium)}</span>
                </div>
                <div className="risk-dist-row">
                  <span className="dot" style={{ background: 'var(--teal)' }} />
                  <span className="lbl">Low confidence</span>
                  <span className="val">{formatNumber(distribution.low)}</span>
                </div>
              </div>
            </div>
          )}
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Recent Activity</h2>
          <p>Latest commits</p>
        </div>
        <Panel title="Recent Commits" hint={`${formatNumber(analysis.timeline.total_commits)} total`}>
          {(analysis.timeline.entries ?? []).length === 0 ? (
            <p className="history-note">No commit history available.</p>
          ) : (
            <>
              <ul className="activity-list">
                {[...(analysis.timeline.entries ?? [])].slice(-5).reverse().map((entry) => (
                  <li key={entry.sha} className="activity-row">
                    <code>{entry.sha.slice(0, 7)}</code>
                    <button
                      type="button"
                      className="link-button activity-msg"
                      onClick={onOpenHistory}
                      title="Open history timeline"
                    >
                      {entry.message || '(no message)'}
                    </button>
                    <span className="activity-sub">
                      {entry.author ?? 'unknown'} · {formatDate(entry.date)} · {entry.files.length} files
                    </span>
                  </li>
                ))}
              </ul>
              <div style={{ marginTop: 14 }}>
                <button
                  type="button"
                  className="flow-ghost"
                  onClick={onOpenHistory}
                >
                  View all history →
                </button>
              </div>
            </>
          )}
        </Panel>
      </div>
    </div>
  )
}
