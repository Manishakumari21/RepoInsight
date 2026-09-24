import type { DifficultyScore, Hotspot } from '../types'
import { formatNumber } from '../types'
import { difficultyColor, severityOf } from '../lib/palette'
import { share } from '../lib/derive'
import { Panel } from './Panel'

export function Risk({
  difficulty,
  sourceFiles,
  complexFiles,
  hotspots,
  connectedFiles,
}: {
  difficulty: DifficultyScore
  sourceFiles: number
  complexFiles: number
  hotspots: Hotspot[]
  connectedFiles: number
}) {
  const color = difficultyColor(difficulty.level)
  const percent = Math.min(difficulty.score, 100)
  const flagged = hotspots.filter((item) => item.score >= 40).length

  const drivers = [
    {
      label: 'Complex files',
      detail: `${formatNumber(complexFiles)} of ${formatNumber(sourceFiles)} files`,
      ratio: share(complexFiles, sourceFiles),
    },
    {
      label: 'Hotspot coverage',
      detail: `${formatNumber(hotspots.length)} assessed · ${formatNumber(flagged)} flagged`,
      ratio: share(hotspots.length, sourceFiles),
    },
    {
      label: 'Coupling density',
      detail: `${formatNumber(connectedFiles)} connected files`,
      ratio: share(connectedFiles, sourceFiles),
    },
  ]

  return (
    <Panel title="Repository Risk" hint="Heuristic aggregate, not an ML prediction">
      <div className="risk-grid">
        <div className="gauge-wrap">
          <div
            className="gauge"
            role="progressbar"
            aria-label={`Repository difficulty score ${Math.round(difficulty.score)} of 100, level ${difficulty.level}`}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(difficulty.score)}
            style={{
              background: `conic-gradient(${color} ${percent}%, var(--bg-elev) ${percent}% 100%)`,
            }}
          >
            <div className="gauge-hole">
              <div className="gauge-score">{Math.round(difficulty.score)}</div>
              <div className="gauge-max">of 100</div>
            </div>
          </div>
          <div
            className="risk-level"
            style={{
              color,
              border: `1px solid ${color}`,
              background: `color-mix(in srgb, ${color} 12%, transparent)`,
            }}
          >
            {difficulty.level.toUpperCase()}
          </div>
          <div className="risk-scale">
            <span>Low risk</span>
            <span>·</span>
            <span>High risk</span>
          </div>
        </div>

        <div className="risk-drivers">
          <div className="panel-sub-title">What drives this score?</div>
          {drivers.map((driver) => {
            const severity = severityOf(driver.ratio * 100)
            const fillClass =
              severity === 'high'
                ? 'danger'
                : severity === 'medium'
                  ? 'warn'
                  : ''
            return (
              <div className="bar-row" key={driver.label}>
                <div className="bar-top">
                  <span className="bar-label">
                    {driver.label}
                    <em className="bar-sub">{driver.detail}</em>
                  </span>
                  <span className="bar-value">
                    {(driver.ratio * 100).toFixed(0)}%
                  </span>
                </div>
                <div className="bar-track">
                  <div
                    className={`bar-fill ${fillClass}`.trim()}
                    style={{ width: `${Math.min(driver.ratio * 100, 100)}%` }}
                  />
                </div>
              </div>
            )
          })}
          <p className="bar-foot">
            Shares of the {formatNumber(sourceFiles)} source files that drive
            complexity, hotspot, and coupling signals.
          </p>
        </div>
      </div>
    </Panel>
  )
}