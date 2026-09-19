import type { Hotspot } from '../types'
import { formatNumber } from '../types'
import { severityOf, type Severity } from '../lib/palette'
import { Panel } from './Panel'

export function Explorer({ hotspots }: { hotspots: Hotspot[] }) {
  const sorted = [...hotspots].sort((a, b) => b.score - a.score)
  const counts: Record<Severity, number> = { high: 0, medium: 0, low: 0 }
  for (const hotspot of sorted) counts[severityOf(hotspot.score)] += 1

  return (
    <Panel id="files" title="File Risk Explorer" hint={`${sorted.length} files assessed`}>
      <div className="explorer-status">
        <span className="chip">{formatNumber(sorted.length)} files</span>
        <span className="chip chip-danger">{counts.high} high</span>
        <span className="chip chip-warn">{counts.medium} medium</span>
        <span className="chip chip-ok">{counts.low} low</span>
        <span className="explorer-hint">
          Sort by risk score · severity thresholds 70+ high, 40+ medium
        </span>
      </div>
      <div className="table-scroll">
        <table className="file-table">
          <caption className="visually-hidden">
            Files ranked by risk score with severity and reasons
          </caption>
          <thead>
            <tr>
              <th className="col-rank" scope="col">#</th>
              <th className="col-path" scope="col">Path</th>
              <th className="col-severity" scope="col">Severity</th>
              <th className="col-score" scope="col">Score</th>
              <th className="col-reasons" scope="col">Reasons</th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((hotspot, index) => {
              const severity = severityOf(hotspot.score)
              return (
                <tr className={`row-${severity}`} key={hotspot.path}>
                  <td className="col-rank">{index + 1}</td>
                  <td className="col-path" title={hotspot.path}>
                    {hotspot.path}
                  </td>
                  <td className="col-severity">
                    <span className={`severity ${severity}`}>{severity}</span>
                  </td>
                  <td className="col-score">{hotspot.score.toFixed(1)}</td>
                  <td className="col-reasons">{hotspot.reasons.join(', ')}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </Panel>
  )
}