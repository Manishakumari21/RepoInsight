import { useMemo, useState } from 'react'
import type {
  FilePrediction,
  PredictionsResponse,
  RepositoryAnalysis,
} from '../types'
import type { ActiveSource } from '../lib/repoSource'
import { sourceLabel } from '../lib/repoSource'
import { buildFileStats, collectFilePaths } from '../lib/files'
import { Header } from './Header'
import { Sidebar } from './Sidebar'
import { Breadcrumbs } from './Breadcrumbs'
import { FlowNext } from './FlowNext'
import type { DashboardSection } from '../lib/sections'
import { OverviewPage } from './OverviewPage'
import { Predictions } from './Predictions'
import { RepoGraph } from './RepoGraph'
import { FileExplorer } from './FileExplorer'
import { HistoryView } from './HistoryView'
import { FileDrawer } from './FileDrawer'
import { Hotspots } from './Hotspots'
import { DatasetReadiness } from './DatasetReadiness'
import { Settings } from './Settings'
import { NoticeBanner } from './Status'

export function Dashboard({
  source,
  analysis,
  predictions,
  predictionsLoading,
  predictionsError,
  onRetryPredictions,
  onImport,
}: {
  source: ActiveSource
  analysis: RepositoryAnalysis
  predictions: PredictionsResponse | null
  predictionsLoading: boolean
  predictionsError: string | null
  onRetryPredictions: () => void
  onImport: () => void
}) {
  const [section, setSection] = useState<DashboardSection>('overview')
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [graphFocus, setGraphFocus] = useState<string | null>(null)
  const [collapsed, setCollapsed] = useState(false)

  const label = sourceLabel(source)
  const stats = useMemo(() => buildFileStats(analysis), [analysis])
  const paths = useMemo(() => collectFilePaths(analysis), [analysis])
  const predictionMap = useMemo(() => {
    const map = new Map<string, FilePrediction>()
    for (const item of predictions?.predictions ?? []) map.set(item.file_path, item)
    return map
  }, [predictions])
  const topRisk = useMemo(
    () =>
      [...(predictions?.predictions ?? [])]
        .filter((item) => item.label === 1)
        .sort((a, b) => b.probability - a.probability)
        .slice(0, 3),
    [predictions],
  )

  function openFile(path: string) {
    setSelectedFile(path)
  }

  function focusAndOpen(path: string) {
    setGraphFocus(path)
    setSelectedFile(path)
  }

  function openGraph(path: string) {
    setGraphFocus(path)
    setSection('graph')
  }

  function openGraphFromDrawer(path: string) {
    setSelectedFile(null)
    setGraphFocus(path)
    setSection('graph')
  }

  return (
    <div className={`shell${collapsed ? ' collapsed' : ''}`}>
      <Sidebar
        sourceLabel={label}
        branch={analysis.repository.default_branch}
        section={section}
        onSection={setSection}
        predictionCount={predictions?.predictions.length ?? 0}
        modelOn={predictions !== null}
        collapsed={collapsed}
        onToggleCollapse={() => setCollapsed((value) => !value)}
      />
      <div className="main">
        <Header
          sourceLabel={label}
          branch={analysis.repository.default_branch}
          section={section}
          onSection={setSection}
          onImport={onImport}
        />
        <main className="content" id="main-content" tabIndex={-1}>
          <Breadcrumbs
            section={section}
            file={selectedFile}
            onSection={setSection}
          />
          <div className="dashboard-body">
            {analysis.timeline.truncated && (
              <NoticeBanner message="Partial analysis — showing what completed. Some history may be truncated." />
            )}

            {section === 'overview' && (
              <>
                <OverviewPage
                  analysis={analysis}
                  sourceLabel={label}
                  predictions={predictions?.predictions ?? []}
                  predictionsAvailable={predictions !== null}
                  onOpenFile={openFile}
                  onOpenGraph={openGraph}
                  onSection={setSection}
                />
                <FlowNext
                  title="Continue to predictions"
                  sub={
                    topRisk.length > 0
                      ? `${topRisk.length} high-risk files flagged — inspect probabilities and evidence next.`
                      : 'Inspect per-file rework probabilities and the evidence behind them.'
                  }
                  primary="Explore Predictions"
                  onPrimary={() => setSection('predictions')}
                />
              </>
            )}

            {section === 'predictions' && (
              <>
                <Predictions
                  predictions={predictions?.predictions ?? null}
                  modelName={predictions?.model.name ?? 'logistic_regression'}
                  availableModels={predictions?.model.available_models ?? []}
                  stats={stats}
                  loading={predictionsLoading}
                  error={predictionsError}
                  onRetry={onRetryPredictions}
                  onSelect={openFile}
                  onOpenGraph={openGraph}
                />
                <Hotspots hotspots={analysis.hotspots} />
                <FlowNext
                  title="See how files connect"
                  sub="Open the change graph to explore affected files around the riskiest predictions."
                  primary="Explore Change Graph"
                  onPrimary={() =>
                    openGraph(topRisk[0]?.file_path ?? graphFocus ?? paths[0] ?? '')
                  }
                  secondary="Browse files"
                  onSecondary={() => setSection('files')}
                />
              </>
            )}

            {section === 'graph' && (
              <>
                <RepoGraph
                  edges={analysis.propagation.edges}
                  files={paths}
                  focus={graphFocus}
                  onFocus={setGraphFocus}
                  onOpenFile={openFile}
                  onViewPrediction={openFile}
                  onGoHistory={() => setSection('history')}
                />
                <FlowNext
                  title="Check the historical evidence"
                  sub="Validate graph relationships against commit history, sequences, and rework signals."
                  primary="View Historical Evidence"
                  onPrimary={() => setSection('history')}
                />
              </>
            )}

            {section === 'files' && (
              <>
                <FileExplorer
                  paths={paths}
                  predictions={predictionMap}
                  stats={stats}
                  onOpenFile={openFile}
                />
                <FlowNext
                  title="Visualize file relationships"
                  sub="Jump into the graph to see dependencies and co-change coupling for any file."
                  primary="Explore Change Graph"
                  onPrimary={() => setSection('graph')}
                />
              </>
            )}

            {section === 'history' && (
              <>
                <HistoryView timeline={analysis.timeline} onOpenFile={openFile} />
                <DatasetReadiness analysis={analysis} />
                <Settings repository={analysis.repository} />
                <FlowNext
                  title="Back to the overview"
                  sub="Return to the command center for risk summary and files requiring attention."
                  primary="Back to Overview"
                  onPrimary={() => setSection('overview')}
                />
              </>
            )}
          </div>
        </main>
      </div>

      {selectedFile && (
        <FileDrawer
          path={selectedFile}
          prediction={predictionMap.get(selectedFile) ?? null}
          analysis={analysis}
          onClose={() => setSelectedFile(null)}
          onOpenFile={focusAndOpen}
          onOpenGraph={openGraphFromDrawer}
        />
      )}
    </div>
  )
}
