import type { ChangeSequence } from '../types'
import { formatDelay, formatNumber } from '../types'
import { Empty } from './Status'
import { Panel } from './Panel'

function SequenceRow({ sequence, rank }: { sequence: ChangeSequence; rank: number }) {
  return (
    <div className="cochange-row">
      <span className="hotspot-rank">{String(rank).padStart(2, '0')}</span>
      <div className="cochange-files">
        {sequence.files.map((file, index) => (
          <span key={`${file}-${index}`} className="seq-chain">
            {index > 0 && <span className="cochange-plus">→</span>}
            <span className="hotspot-path" title={file}>
              {file}
            </span>
          </span>
        ))}
      </div>
      <span className="chip">{sequence.kind}</span>
      <span className="cochange-count">×{sequence.occurrences}</span>
      <span className="seq-delay">{formatDelay(sequence.avg_delay_seconds)} avg</span>
    </div>
  )
}

export function Sequences({
  sequences,
  windowSeconds,
}: {
  sequences: ChangeSequence[]
  windowSeconds: number
}) {
  const top = sequences.slice(0, 15)

  return (
    <Panel
      id="sequences"
      title="Change Sequences"
      hint={`Observed order within ${formatDelay(windowSeconds)} windows`}
    >
      <p className="history-note">
        Historical evidence — file order actually observed in consecutive commits, not a
        prediction of future changes.
      </p>
      {top.length === 0 ? (
        <Empty text="No change sequences detected in this history sample." />
      ) : (
        <>
          <div className="explorer-status">
            <span className="chip">{formatNumber(sequences.length)} sequences</span>
            <span className="explorer-hint">Ranked by occurrences · then average delay</span>
          </div>
          {top.map((sequence, index) => (
            <SequenceRow
              key={sequence.files.join('→')}
              sequence={sequence}
              rank={index + 1}
            />
          ))}
        </>
      )}
    </Panel>
  )
}
