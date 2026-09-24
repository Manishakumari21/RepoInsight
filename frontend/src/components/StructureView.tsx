import type { FilePrediction, PropagationEdge } from '../types'
import { FileExplorer } from './FileExplorer'
import { RepoGraph } from './RepoGraph'
import { Panel } from './Panel'

export function StructureView({
  paths,
  predictions,
  stats,
  edges,
  focus,
  treeQuery,
  predicted,
  onOpenFile,
  onFocus,
  onViewPrediction,
  onGoHistory,
}: {
  paths: string[]
  predictions: Map<string, FilePrediction>
  stats: Record<string, { changes: number; complexity: number; dependencies: number; size: number }>
  edges: PropagationEdge[]
  focus: string | null
  treeQuery: string
  predicted: Set<string>
  onOpenFile: (path: string) => void
  onFocus: (path: string | null) => void
  onViewPrediction: (path: string) => void
  onGoHistory: () => void
}) {
  return (
    <>
      <div className="page-head">
        <h1>Repository Structure</h1>
        <p>
          Repository tree (left), dependency neighborhood graph (center), file inspector (right).
          Filter the graph by relationship: Dependencies, Temporal, Co-change, Tests, Configuration.
        </p>
      </div>
      <div className="structure-grid">
        <Panel title="Repository tree" hint={`${paths.length} files`}>
          <FileExplorer
            paths={paths}
            predictions={predictions}
            stats={stats}
            onOpenFile={onOpenFile}
            initialQuery={treeQuery}
          />
        </Panel>
        <RepoGraph
          edges={edges}
          files={paths}
          focus={focus}
          onFocus={onFocus}
          onOpenFile={onOpenFile}
          onViewPrediction={onViewPrediction}
          onGoHistory={onGoHistory}
          predicted={predicted}
          chrome="compact"
        />
      </div>
    </>
  )
}
