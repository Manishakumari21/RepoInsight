import type { ComplexityAnalysis, SourceAnalysis } from '../types'
import { formatNumber } from '../types'
import { severityColor, severityOf } from '../lib/palette'
import { Donut } from './Donut'
import { Panel } from './Panel'

function barClass(ratio: number): string {
  if (ratio >= 0.66) return 'danger'
  if (ratio >= 0.4) return 'warn'
  return ''
}

export function Complexity({
  complexity,
  source,
}: {
  complexity: ComplexityAnalysis
  source: SourceAnalysis
}) {
  const avgRatio =
    complexity.max_complexity > 0
      ? complexity.average_complexity / complexity.max_complexity
      : 0
  const complexRatio =
    source.source_files > 0 ? complexity.complex_files / source.source_files : 0
  const clearFiles = Math.max(source.source_files - complexity.complex_files, 0)
  const severity = severityOf(complexRatio * 100)

  return (
    <Panel title="Complexity" hint="Cyclomatic complexity overview">
      <div className="complex-grid">
        <div className="complex-donut">
          <Donut
            segments={[
              { value: complexity.complex_files, color: severityColor(severity) },
              { value: clearFiles, color: 'var(--accent)' },
            ]}
          >
            <div className="donut-value">{(complexRatio * 100).toFixed(0)}%</div>
            <div className="donut-caption">complex</div>
          </Donut>
          <div className="donut-legend">
            Complex vs. clear source files
          </div>
        </div>

        <div className="complex-metrics">
          <div className="metrics">
            <div className="metric">
              <div className="metric-value">{complexity.average_complexity.toFixed(2)}</div>
              <div className="metric-label">Avg cyclomatic</div>
            </div>
            <div className="metric">
              <div className="metric-value">{complexity.max_complexity}</div>
              <div className="metric-label">Max complexity</div>
            </div>
            <div className="metric">
              <div className="metric-value">{formatNumber(complexity.complex_files)}</div>
              <div className="metric-label">Complex files</div>
            </div>
          </div>
          <div className="bars">
            <div className="bar-row">
              <div className="bar-top">
                <span className="bar-label">Avg vs. peak complexity</span>
                <span className="bar-value">{(avgRatio * 100).toFixed(0)}%</span>
              </div>
              <div className="bar-track">
                <div
                  className={`bar-fill ${barClass(avgRatio)}`.trim()}
                  style={{ width: `${Math.min(avgRatio * 100, 100)}%` }}
                />
              </div>
            </div>
            <div className="bar-row">
              <div className="bar-top">
                <span className="bar-label">Complex files in codebase</span>
                <span className="bar-value">
                  {formatNumber(complexity.complex_files)} of{' '}
                  {formatNumber(source.source_files)}
                </span>
              </div>
              <div className="bar-track">
                <div
                  className={`bar-fill ${barClass(complexRatio)}`.trim()}
                  style={{ width: `${Math.min(complexRatio * 100, 100)}%` }}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </Panel>
  )
}