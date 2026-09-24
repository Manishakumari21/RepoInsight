import type { FilePrediction, RepositoryAnalysis } from '../types'
import { FileInspector } from './FileInspector'

export function FileDrawer({
  path,
  prediction,
  analysis,
  onClose,
  onOpenFile,
  onOpenGraph,
  onGoHistory,
}: {
  path: string
  prediction: FilePrediction | null
  analysis: RepositoryAnalysis
  onClose: () => void
  onOpenFile: (path: string) => void
  onOpenGraph?: (path: string) => void
  onGoHistory?: () => void
}) {
  return (
    <div className="drawer-overlay" onClick={onClose}>
      <aside
        className="drawer"
        role="dialog"
        aria-modal="true"
        aria-label={`Details for ${path}`}
        onClick={(event) => event.stopPropagation()}
      >
        <FileInspector
          path={path}
          prediction={prediction}
          analysis={analysis}
          onClose={onClose}
          onOpenFile={onOpenFile}
          onOpenGraph={onOpenGraph}
          onGoHistory={onGoHistory}
        />
      </aside>
    </div>
  )
}
