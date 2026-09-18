import type { HistoricalExample } from '../types'
import { formatDelay, formatNumber, shortSha } from '../types'
import { Empty } from './Status'
import { Panel } from './Panel'

function ExampleRow({ example }: { example: HistoricalExample }) {
  return (
    <div className="example-row">
      <div className="example-top">
        <span className="hotspot-path" title={example.source_file}>
          {example.source_file}
        </span>
        <span className="cochange-plus">→</span>
        <span className="hotspot-path" title={example.target_file}>
          {example.target_file}
        </span>
        <span className="cochange-count">×{example.occurrences}</span>
        <span className="seq-delay">{formatDelay(example.avg_delay_seconds)} avg</span>
      </div>
      <div className="example-sequence" title={example.sequence.join(' → ')}>
        {example.sequence.join(' → ')}
      </div>
      <div className="example-meta">
        {example.signals.map((signal) => (
          <span className="chip" key={signal}>
            {signal}
          </span>
        ))}
        <span className="chip">{example.evidence_type}</span>
        {example.example_shas.length > 0 && (
          <span className="timeline-meta">
            e.g. {example.example_shas.map(shortSha).join(', ')}
          </span>
        )}
      </div>
    </div>
  )
}

export function HistoricalExamples({
  examples,
  total,
}: {
  examples: HistoricalExample[]
  total: number
}) {
  const top = examples.slice(0, 12)

  return (
    <Panel
      id="examples"
      title="Historical Examples"
      hint={`${formatNumber(total)} representative observed cases`}
    >
      <p className="history-note">
        Historical evidence — representative past propagations with supporting signals and
        example commits. These explain what happened, not what will happen.
      </p>
      {top.length === 0 ? (
        <Empty text="No historical examples in this history sample." />
      ) : (
        top.map((example) => <ExampleRow key={example.id} example={example} />)
      )}
    </Panel>
  )
}
