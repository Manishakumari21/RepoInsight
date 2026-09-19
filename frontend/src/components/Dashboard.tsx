import type { RepositoryAnalysis } from '../types'
import { Overview } from './Overview'
import { Risk } from './Risk'
import { Complexity } from './Complexity'
import { Hotspots } from './Hotspots'
import { Explorer } from './Explorer'
import { Dependencies } from './Dependencies'
import { History } from './History'
import { PropagationGraph } from './PropagationGraph'
import { Cochange } from './Cochange'
import { Sequences } from './Sequences'
import { PropagationHistory } from './PropagationHistory'
import { HistoricalExamples } from './HistoricalExamples'
import { DatasetReadiness } from './DatasetReadiness'
import { Settings } from './Settings'
import { NoticeBanner } from './Status'

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
      {analysis.timeline.truncated && (
        <NoticeBanner message="Partial analysis — showing what completed. Some history may be truncated." />
      )}
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
        <History
          history={analysis.history}
          totalLines={source.total_lines}
          timeline={analysis.timeline}
        />
      </div>
      <PropagationGraph propagation={analysis.propagation} />
      <Cochange pairs={analysis.cochange.pairs} total={analysis.cochange.total_pairs} />
      <Sequences sequences={analysis.sequences.sequences} windowSeconds={analysis.sequences.window_seconds} />
      <PropagationHistory timeline={analysis.timeline} followups={analysis.followups} rework={analysis.rework} />
      <HistoricalExamples examples={analysis.examples.examples} total={analysis.examples.total} />
      <DatasetReadiness analysis={analysis} />
      <Settings repository={analysis.repository} />
    </div>
  )
}