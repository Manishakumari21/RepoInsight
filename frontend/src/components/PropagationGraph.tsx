import type { PropagationAnalysis } from '../types'
import { formatNumber } from '../types'
import { edgeSignals } from '../lib/analysis'
import { Empty } from './Status'
import { Panel } from './Panel'
import { SignalDots, useMounted } from './primitives'

function shortName(path: string): string {
  const parts = path.split('/')
  const file = parts.at(-1) ?? path
  return file.length > 22 ? `…${file.slice(-21)}` : file
}

function FlowDiagram({
  edges,
}: {
  edges: PropagationAnalysis['edges']
}) {
  const shown = edges.slice(0, 6)
  const sources = [...new Set(shown.map((edge) => edge.source))]
  const targets = [...new Set(shown.map((edge) => edge.target))]
  const peak = shown.reduce((max, edge) => Math.max(max, edge.strength), 0)
  const height = Math.max(sources.length, targets.length, 1) * 34 + 24
  const yFor = (index: number) => 20 + index * 34

  return (
    <svg
      className="flow-diagram"
      viewBox={`0 0 640 ${height}`}
      role="img"
      aria-label={`Strongest ${shown.length} propagation links from ${sources.length} sources to ${targets.length} targets`}
    >
      {shown.map((edge) => {
        const y1 = yFor(sources.indexOf(edge.source))
        const y2 = yFor(targets.indexOf(edge.target))
        return (
          <path
            key={`${edge.source}→${edge.target}`}
            d={`M150 ${y1} C 300 ${y1}, 340 ${y2}, 490 ${y2}`}
            fill="none"
            stroke="var(--accent)"
            strokeOpacity={0.25 + (peak > 0 ? (edge.strength / peak) * 0.65 : 0)}
            strokeWidth={peak > 0 ? 1 + (edge.strength / peak) * 4 : 1}
          >
            <title>{`${edge.source} → ${edge.target} (strength ${edge.strength})`}</title>
          </path>
        )
      })}
      {sources.map((source, index) => (
        <text key={source} x={8} y={yFor(index) + 4} className="flow-label">
          <title>{source}</title>
          {shortName(source)}
        </text>
      ))}
      {targets.map((target, index) => (
        <text
          key={target}
          x={632}
          y={yFor(index) + 4}
          textAnchor="end"
          className="flow-label"
        >
          <title>{target}</title>
          {shortName(target)}
        </text>
      ))}
    </svg>
  )
}

export function PropagationGraph({
  propagation,
}: {
  propagation: PropagationAnalysis
}) {
  const mounted = useMounted()
  const edges = [...propagation.edges].sort(
    (a, b) =>
      b.strength - a.strength ||
      a.source.localeCompare(b.source) ||
      a.target.localeCompare(b.target),
  )
  const top = edges.slice(0, 12)
  const peak = top.reduce((max, edge) => Math.max(max, edge.strength), 0)

  return (
    <Panel
      title="Propagation Graph"
      hint="Strongest source → target links · observed evidence"
    >
      <div className="legend-row signal-legend">
        <SignalDots signals={['dependency']} />
        <SignalDots signals={['temporal']} />
        <SignalDots signals={['co-change']} />
      </div>
      {edges.length === 0 ? (
        <Empty text="No propagation links detected in this history sample." />
      ) : (
        <>
          <FlowDiagram edges={edges} />
          {top.map((edge) => (
          <div
            className="cochange-row"
            key={`${edge.source}→${edge.target}`}
          >
            <div className="cochange-files">
              <span className="hotspot-path" title={edge.source}>
                {edge.source}
              </span>
              <span className="cochange-plus" aria-hidden="true">
                →
              </span>
              <span className="hotspot-path" title={edge.target}>
                {edge.target}
              </span>
            </div>
            <SignalDots signals={edgeSignals(edge)} />
            <span
              className="cochange-count tnum"
              title={`Combined strength ${edge.strength}`}
            >
              ×{edge.strength}
            </span>
            <span
              className="prop-bar"
              role="img"
              aria-label={`Strength ${edge.strength} of peak ${peak}`}
              title={`Combined strength ${edge.strength} — ${edgeSignals(edge).join(' + ')}`}
            >
              <span
                className="prop-fill"
                style={{
                  width:
                    mounted && peak > 0
                      ? `${(edge.strength / peak) * 100}%`
                      : '0%',
                }}
              />
            </span>
            </div>
          ))}
          </>
      )}
      <p className="history-note">
        Links combine dependency, temporal, and co-change evidence from
        repository history — what happened, not what will happen (
        {formatNumber(propagation.edges.length)} total).
      </p>
    </Panel>
  )
}
