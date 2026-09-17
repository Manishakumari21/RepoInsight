import type { RepositoryAnalysis } from '../types'
import { Overview } from './Overview'
import { Risk } from './Risk'
import { Complexity } from './Complexity'
import { Hotspots } from './Hotspots'
import { Explorer } from './Explorer'
import { Dependencies } from './Dependencies'
import { History } from './History'
import { Cochange } from './Cochange'
import { Settings } from './Settings'

export function Dashboard({
  owner,
  repo,
  analysis,
}: {
  owner: string
  repo: string
  analysis: RepositoryAnalysis
}) {
  const { source, complexity, dependencies } = analysis

  return (
    <div className="dashboard-body">
      <Overview owner={owner} repo={repo} analysis={analysis} />
      <div className="grid-2">
        <Risk
          difficulty={analysis.difficulty}
          sourceFiles={source.source_files}
          complexFiles={complexity.complex_files}
          hotspots={analysis.hotspots}
          connectedFiles={dependencies.connected_files}
        />
        <Complexity complexity={complexity} source={source} />
      </div>
      <Hotspots hotspots={analysis.hotspots} />
      <Explorer hotspots={analysis.hotspots} />
      <div className="grid-2">
        <Dependencies
          dependencies={dependencies}
          sourceFiles={source.source_files}
        />
        <History history={analysis.history} totalLines={source.total_lines} />
      </div>
      <Cochange pairs={analysis.cochange.pairs} total={analysis.cochange.total_pairs} />
      <Settings repository={analysis.repository} />
    </div>
  )
}