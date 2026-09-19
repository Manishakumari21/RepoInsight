import type { CochangePair } from '../types'
import { formatNumber } from '../types'
import { Empty } from './Status'
import { Panel } from './Panel'
import { useMounted } from './primitives'

function CochangeRow({
  pair,
  rank,
  peak,
}: {
  pair: CochangePair
  rank: number
  peak: number
}) {
  const mounted = useMounted()
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
      <span
        className="prop-bar"
        role="img"
        aria-label={`${pair.count} co-changes of peak ${peak}: ${pair.file_a} and ${pair.file_b}`}
        title={`${pair.count} co-changes — ${pair.file_a} ↔ ${pair.file_b}`}
      >
        <span
          className="prop-fill"
          style={{ width: mounted && peak > 0 ? `${(pair.count / peak) * 100}%` : '0%' }}
        />
      </span>
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
  const ranked = [...pairs].sort(
    (a, b) =>
      b.count - a.count ||
      a.file_a.localeCompare(b.file_a) ||
      a.file_b.localeCompare(b.file_b),
  )
  const top = ranked.slice(0, 15)
  const lead = ranked[0]
  const peak = ranked.reduce((max, pair) => Math.max(max, pair.count), 0)

  return (
    <Panel title="Co-change Patterns" hint={`${formatNumber(total)} pairs detected in history`}>
      {top.length === 0 ? (
        <Empty text="No repeated co-change pairs found yet. This usually means the repo's history is short or changes are well-isolated." />
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
              peak={peak}
            />
          ))}
        </>
      )}
    </Panel>
  )
}