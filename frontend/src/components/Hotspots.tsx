import type { Hotspot } from '../types'
import { formatNumber } from '../types'
import { severityColor, severityOf, type Severity } from '../lib/palette'
import { Donut } from './Donut'
import { Empty } from './Status'
import { Panel } from './Panel'

function HotspotRow({ hotspot, rank }: { hotspot: Hotspot; rank: number }) {
  const percent = Math.min(hotspot.score, 100)
  const severity = severityOf(hotspot.score)

  return (
    <div className={`hotspot-row severity-${severity}`}>
      <div className="hotspot-rank">{String(rank).padStart(2, '0')}</div>
      <div className="hotspot-body">
        <div className="hotspot-top">
          <div className="hotspot-id">
            <span className="hotspot-path" title={hotspot.path}>
              {hotspot.path}
            </span>
            <span className={`severity ${severity}`}>{severity}</span>
          </div>
          <span className="hotspot-score">{Math.round(hotspot.score)}</span>
        </div>
        <div className="hotspot-bar">
          <div
            className={`hotspot-fill ${severity}`}
            style={{ width: `${percent}%` }}
          />
        </div>
        <div className="hotspot-reasons">{hotspot.reasons.join(' · ')}</div>
      </div>
    </div>
  )
}

export function Hotspots({ hotspots }: { hotspots: Hotspot[] }) {
  const top = hotspots.slice(0, 10)
  const counts: Record<Severity, number> = { high: 0, medium: 0, low: 0 }
  for (const hotspot of hotspots) counts[severityOf(hotspot.score)] += 1
  const lead = hotspots[0]

  return (
    <Panel id="hotspots" title="Hotspots" hint="Top files by risk score">
      <div className="hotspot-summary">
        <Donut
          size={80}
          thickness={10}
          segments={[
            { value: counts.high, color: severityColor('high') },
            { value: counts.medium, color: severityColor('medium') },
            { value: counts.low, color: severityColor('low') },
          ]}
        >
          <div className="donut-value">{formatNumber(hotspots.length)}</div>
          <div className="donut-caption">files</div>
        </Donut>
        <div className="hotspot-legend">
          <span className="legend-row">
            <span className="swatch" style={{ background: severityColor('high') }} />
            High
            <b>{counts.high}</b>
          </span>
          <span className="legend-row">
            <span className="swatch" style={{ background: severityColor('medium') }} />
            Medium
            <b>{counts.medium}</b>
          </span>
          <span className="legend-row">
            <span className="swatch" style={{ background: severityColor('low') }} />
            Low
            <b>{counts.low}</b>
          </span>
        </div>
        {lead && (
          <div className="hotspot-lead">
            <div className="lead-label">Top risk file</div>
            <div className="lead-path" title={lead.path}>
              {lead.path}
            </div>
            <div
              className="lead-score"
              style={{ color: severityColor(severityOf(lead.score)) }}
            >
              score {Math.round(lead.score)}
            </div>
          </div>
        )}
      </div>
      {top.length === 0 ? (
        <Empty text="No hotspots computed for this repository." />
      ) : (
        top.map((hotspot, index) => (
          <HotspotRow key={hotspot.path} hotspot={hotspot} rank={index + 1} />
        ))
      )}
    </Panel>
  )
}