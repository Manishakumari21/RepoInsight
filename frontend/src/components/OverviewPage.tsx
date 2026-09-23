import { useMemo } from 'react'
import type { FilePrediction, RepositoryAnalysis } from '../types'
import { formatDate, formatNumber, shortSha } from '../types'
import { changeCounts } from '../lib/files'
import { riskDistribution } from '../lib/predictions'
import type { DashboardSection } from '../lib/sections'
import { Overview } from './Overview'
import { Risk } from './Risk'
import { Complexity } from './Complexity'
import { Donut } from './Donut'
import { Panel } from './Panel'

function edgeKind(edge: { dependency: boolean; temporal: boolean; cochange: boolean }): {
  key: string
  label: string
  className: string
} {
  if (edge.dependency) return { key: 'dep', label: 'dependency', className: '' }
  if (edge.cochange) return { key: 'co', label: 'co-change', className: 'co' }
  return { key: 'tm', label: 'propagation', className: 'tm' }
}

export function OverviewPage({
  analysis,
  sourceLabel,
  predictions,
  predictionsAvailable,
  onOpenFile,
  onOpenGraph,
  onSection,
}: {
  analysis: RepositoryAnalysis
  sourceLabel: string
  predictions: FilePrediction[]
  predictionsAvailable: boolean
  onOpenFile: (path: string) => void
  onOpenGraph: (path: string) => void
  onSection: (section: DashboardSection) => void
}) {
  const mostChanged = useMemo(() => {
    const counts = changeCounts(analysis)
    return [...counts.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .slice(0, 6)
  }, [analysis])

  const distribution = useMemo(
    () => riskDistribution(predictions),
    [predictions],
  )

  const connectedCounts = useMemo(() => {
    const counts = new Map<string, number>()
    for (const edge of analysis.propagation.edges ?? []) {
      counts.set(edge.source, (counts.get(edge.source) ?? 0) + 1)
      counts.set(edge.target, (counts.get(edge.target) ?? 0) + 1)
    }
    return counts
  }, [analysis])

  const previewEdges = useMemo(
    () => [...(analysis.propagation.edges ?? [])].slice(0, 5),
    [analysis],
  )

  const recentCommits = useMemo(
    () => [...(analysis.timeline.entries ?? [])].slice(-5).reverse(),
    [analysis],
  )

  const signals = useMemo(
    () => [
      {
        key: 'complexity',
        label: 'Avg complexity',
        value: analysis.complexity.average_complexity.toFixed(1),
        sub: `${formatNumber(analysis.complexity.complex_files)} complex files`,
      },
      {
        key: 'churn',
        label: 'Total churn',
        value: formatNumber(analysis.history.total_churn),
        sub: `${formatNumber(analysis.history.total_additions)} additions`,
      },
      {
        key: 'coupling',
        label: 'Dependencies',
        value: formatNumber(analysis.dependencies.total_dependencies),
        sub: `${formatNumber(analysis.dependencies.connected_files)} connected files`,
      },
      {
        key: 'predictions',
        label: 'Flagged files',
        value: predictionsAvailable ? formatNumber(distribution.positive) : '—',
        sub: predictionsAvailable ? `of ${formatNumber(predictions.length)} analyzed` : 'model pending',
      },
    ],
    [analysis, distribution, predictionsAvailable, predictions.length],
  )

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
    const latest = [...(analysis.timeline.entries ?? [])].pop()
    const recentFile = latest?.files?.[0]
    if (recentFile) {
      items.push({
        title: `${recentFile} changed most recently`,
        sub: `${latest?.message || '(no message)'} — see what it touched.`,
        file: recentFile,
      })
    }
    return items.slice(0, 3)
  }, [analysis, predictions])

  const totalBands = Math.max(distribution.high + distribution.medium + distribution.low, 1)
  const bandRows = [
    { key: 'high', label: 'High confidence', value: distribution.high, color: 'var(--danger)' },
    { key: 'medium', label: 'Medium confidence', value: distribution.medium, color: 'var(--warn)' },
    { key: 'low', label: 'Low confidence', value: distribution.low, color: 'var(--teal)' },
  ]

  return (
    <div className="overview-page">
      <Overview sourceLabel={sourceLabel} analysis={analysis} />

      <div className="section-block">
        <div className="summary-row">
          <div className="summary-card">
            <div className="stat-label">Files</div>
            <div className="stat-value tnum">{formatNumber(analysis.source.source_files)}</div>
            <div className="stat-sub">{formatNumber(analysis.repository.total_files)} total</div>
          </div>
          <div className="summary-card">
            <div className="stat-label">Commits</div>
            <div className="stat-value tnum">{formatNumber(analysis.history.total_commits)}</div>
            <div className="stat-sub">{formatNumber(analysis.history.active_contributors)} contributors</div>
          </div>
          <div className="summary-card">
            <div className="stat-label">Dependencies</div>
            <div className="stat-value tnum">{formatNumber(analysis.dependencies.total_dependencies)}</div>
            <div className="stat-sub">{formatNumber(analysis.dependencies.connected_files)} connected files</div>
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
          <h2>Repository Signals</h2>
          <p>Complexity, change frequency, coupling, prediction activity</p>
        </div>
        <Panel title="Signals" hint="Observed repository measurements">
          <div className="stat-grid">
            {signals.map((signal) => (
              <div className="stat" key={signal.key}>
                <div className="stat-value tnum">{signal.value}</div>
                <div className="stat-label">{signal.label}</div>
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
                segments={bandRows.map((row) => ({
                  value: row.value,
                  color: row.color,
                  name: row.label,
                }))}
                label={`Rework confidence: high ${distribution.high}, medium ${distribution.medium}, low ${distribution.low}`}
              >
                <div className="donut-value tnum">{formatNumber(distribution.positive)}</div>
                <div className="donut-caption">flagged</div>
              </Donut>
              <div className="risk-donut-legend">
                <div className="risk-donut-total">
                  of {formatNumber(predictions.length)} analyzed
                </div>
                {bandRows.map((row) => (
                  <div className="risk-dist-row" key={row.key}>
                    <span className="dot" style={{ background: row.color }} />
                    <span className="lbl">{row.label}</span>
                    <span className="val">
                      {formatNumber(row.value)} · {((row.value / totalBands) * 100).toFixed(0)}%
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Recommended Investigation</h2>
          <p>Where to look next, from real repository data</p>
        </div>
        <Panel title="Investigate Next" hint="Click a card to open the file">
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
          <h2>Recently Important Changes</h2>
          <p>Most frequently changed files</p>
        </div>
        <Panel title="Files Requiring Attention" hint="From commit history">
          {mostChanged.length === 0 ? (
            <p className="history-note">No file changes recorded.</p>
          ) : (
            <ul className="important-list">
              {mostChanged.map(([file, count]) => (
                <li key={file} className="important-row">
                  <div className="important-main">
                    <div className="important-file">{file}</div>
                    <div className="important-meta">
                      <span>Changed <b>{formatNumber(count)}×</b></span>
                      <span>Potential impact · <b>{formatNumber(connectedCounts.get(file) ?? 0)}</b> connected files</span>
                    </div>
                  </div>
                  <div className="important-action">
                    <button
                      type="button"
                      className="flow-ghost"
                      onClick={() => onOpenFile(file)}
                    >
                      View analysis →
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Change Propagation Preview</h2>
          <p>Strongest file relationships</p>
        </div>
        <Panel title="Propagation Preview" hint="Top coupled pairs">
          {previewEdges.length === 0 ? (
            <p className="history-note">No propagation relationships detected yet.</p>
          ) : (
            <>
              <div className="mini-graph">
                {previewEdges.map((edge, index) => {
                  const kind = edgeKind(edge)
                  return (
                    <div className="mini-edge" key={`${edge.source}-${edge.target}-${index}`}>
                      <span className="src">{edge.source}</span>
                      <span className={`conn ${kind.className}`}>
                        <i aria-hidden="true" />
                        {kind.label}
                      </span>
                      <span className="dst">{edge.target}</span>
                    </div>
                  )
                })}
              </div>
              <div style={{ marginTop: 14 }}>
                <button
                  type="button"
                  className="flow-ghost"
                  onClick={() => onOpenGraph(previewEdges[0].source)}
                >
                  Explore full graph →
                </button>
              </div>
            </>
          )}
        </Panel>
      </div>

      <div className="section-block">
        <div className="section-head">
          <h2>Recent Activity</h2>
          <p>Latest commits</p>
        </div>
        <Panel title="Recent Commits" hint={`${formatNumber(analysis.timeline.total_commits)} total`}>
          {recentCommits.length === 0 ? (
            <p className="history-note">No commit history available.</p>
          ) : (
            <>
              <ul className="activity-list">
                {recentCommits.map((entry) => (
                  <li key={entry.sha} className="activity-row">
                    <code>{shortSha(entry.sha)}</code>
                    <button
                      type="button"
                      className="link-button activity-msg"
                      onClick={() => onSection('history')}
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
                  onClick={() => onSection('history')}
                >
                  View all history →
                </button>
              </div>
            </>
          )}
        </Panel>
      </div>

      <div className="grid-2">
        <Risk
          difficulty={analysis.difficulty}
          sourceFiles={analysis.source.source_files}
          complexFiles={analysis.complexity.complex_files}
          hotspots={analysis.hotspots}
          connectedFiles={analysis.dependencies.connected_files}
        />
        <Complexity complexity={analysis.complexity} source={analysis.source} />
      </div>
    </div>
  )
}
