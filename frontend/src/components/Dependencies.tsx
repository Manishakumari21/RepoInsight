import type { DependencyAnalysis } from '../types'
import { formatNumber } from '../types'
import { share } from '../lib/derive'
import { Donut } from './Donut'
import { Empty } from './Status'
import { Panel } from './Panel'

export function Dependencies({
  dependencies,
  sourceFiles,
}: {
  dependencies: DependencyAnalysis
  sourceFiles: number
}) {
  const coverage = share(dependencies.connected_files, sourceFiles)
  const standalone = Math.max(sourceFiles - dependencies.connected_files, 0)
  const lead = dependencies.top_coupled_files[0]
  const peakCoupling = Math.max(
    1,
    ...dependencies.top_coupled_files.map((file) => file.coupling),
  )

  return (
    <Panel
      id="dependencies"
      title="Dependencies"
      hint={`${formatNumber(dependencies.resolved_dependencies)} dependency paths resolved`}
    >
      <div className="dep-top">
        <Donut
          segments={[
            { value: dependencies.connected_files, color: 'var(--accent)' },
            { value: standalone, color: 'var(--ok)' },
          ]}
        >
          <div className="donut-value">{(coverage * 100).toFixed(0)}%</div>
          <div className="donut-caption">coupled</div>
        </Donut>
        <div className="dep-side">
          <div className="metrics">
            <div className="metric">
              <div className="metric-value">{formatNumber(dependencies.total_dependencies)}</div>
              <div className="metric-label">Total dependencies</div>
            </div>
            <div className="metric">
              <div className="metric-value">{formatNumber(dependencies.connected_files)}</div>
              <div className="metric-label">Connected files</div>
            </div>
            <div className="metric">
              <div className="metric-value">{formatNumber(dependencies.highly_connected_files)}</div>
              <div className="metric-label">Highly connected</div>
            </div>
          </div>
          <div className="bar-row">
            <div className="bar-top">
              <span className="bar-label">Files in a dependency network</span>
              <span className="bar-value">
                {formatNumber(dependencies.connected_files)} of {formatNumber(sourceFiles)}
              </span>
            </div>
            <div className="bar-track">
              <div
                className="bar-fill"
                style={{ width: `${Math.min(coverage * 100, 100)}%` }}
              />
            </div>
          </div>
        </div>
      </div>

      <div className="panel-sub-title">Top coupled files</div>
      {dependencies.top_coupled_files.length === 0 ? (
        <Empty text="No coupled files detected." />
      ) : (
        <>
          {lead && (
            <div className="lead-callout">
              <span className="lead-label">Most coupled</span>
              <span className="lead-path" title={lead.path}>
                {lead.path}
              </span>
              <span className="lead-score">{formatNumber(lead.coupling)}</span>
            </div>
          )}
          {dependencies.top_coupled_files.map((file) => (
            <div className="coupled-row" key={file.path}>
              <div className="coupled-top">
                <span className="coupled-path" title={file.path}>
                  {file.path}
                </span>
                <span className="coupled-value">
                  {formatNumber(file.coupling)} links
                </span>
              </div>
              <div className="bar-track">
                <div
                  className="bar-fill"
                  style={{ width: `${Math.min((file.coupling / peakCoupling) * 100, 100)}%` }}
                />
              </div>
            </div>
          ))}
        </>
      )}
    </Panel>
  )
}