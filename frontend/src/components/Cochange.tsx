import type { CochangePair } from '../types'
import { formatNumber } from '../types'
import { Empty } from './Status'
import { Panel } from './Panel'

function CochangeRow({ pair, rank }: { pair: CochangePair; rank: number }) {
  return (
    <div className="cochange-row">
      <span className="hotspot-rank">{String(rank).padStart(2, '0')}</span>
      <div className="cochange-files">
        <span className="hotspot-path" title={pair.file_a}>
          {pair.file_a}
        </span>
        <span className="cochange-plus">↔</span>
        <span className="hotspot-path" title={pair.file_b}>
          {pair.file_b}
        </span>
      </div>
      <span className="cochange-count">×{pair.count}</span>
    </div>
  )
}

export function Cochange({
  pairs,
  total,
}: {
  pairs: CochangePair[]
  total: number
}) {
  const top = pairs.slice(0, 15)
  const lead = pairs[0]

  return (
    <Panel title="Co-change Patterns" hint={`${formatNumber(total)} pairs detected in history`}>
      {top.length === 0 ? (
        <Empty text="No co-change pairs detected." />
      ) : (
        <>
          {lead && (
            <div className="lead-callout">
              <span className="lead-label">Strongest pair</span>
              <span className="lead-path" title={lead.file_a}>
                {lead.file_a}
              </span>
              <span className="cochange-plus">↔</span>
              <span className="lead-path" title={lead.file_b}>
                {lead.file_b}
              </span>
              <span className="cochange-count">×{lead.count}</span>
            </div>
          )}
          {top.map((pair, index) => (
            <CochangeRow
              key={`${pair.file_a}-${pair.file_b}`}
              pair={pair}
              rank={index + 1}
            />
          ))}
        </>
      )}
    </Panel>
  )
}