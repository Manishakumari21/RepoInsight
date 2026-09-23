import { useMemo, useState } from 'react'
import type {
  FilePrediction,
  ImpactResponse,
  PredictionsResponse,
  RepositoryAnalysis,
  RippleResponse,
} from '../types'
import type { ActiveSource } from '../lib/repoSource'
import { sourceLabel } from '../lib/repoSource'
import { fetchLocalImpact, fetchImpact, fetchLocalRipple, fetchRipple } from '../api'
import { buildFileStats, collectFilePaths } from '../lib/files'
import { Header } from './Header'
import { Sidebar } from './Sidebar'
import { Breadcrumbs } from './Breadcrumbs'
import { FlowNext } from './FlowNext'
import type {
  DashboardSection,
  ExploreTab,
  HistoryTab,
  PredictTab,
} from '../lib/sections'
import { EXPLORE_TABS, HISTORY_TABS, PREDICT_TABS } from '../lib/sections'
import { SubTabs } from './SubTabs'
import { OverviewPage } from './OverviewPage'
import { PredictView } from './PredictView'
import { RippleForecast } from './RippleForecast'
import { ImpactSimulator } from './ImpactSimulator'
import { RepoGraph } from './RepoGraph'
import { FileExplorer } from './FileExplorer'
import { HistoryView } from './HistoryView'
import { FileDrawer } from './FileDrawer'
import { Hotspots } from './Hotspots'
import { Dependencies } from './Dependencies'
import { Cochange } from './Cochange'
import { Sequences } from './Sequences'
import { PropagationGraph } from './PropagationGraph'
import { PropagationHistory } from './PropagationHistory'
import { HistoricalExamples } from './HistoricalExamples'
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
  const [exploreTab, setExploreTab] = useState<ExploreTab>('graph')
  const [predictTab, setPredictTab] = useState<PredictTab>('predictions')
  const [historyTab, setHistoryTab] = useState<HistoryTab>('timeline')
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [graphFocus, setGraphFocus] = useState<string | null>(null)
  const [collapsed, setCollapsed] = useState(false)
  const [ripple, setRipple] = useState<RippleResponse | null>(null)
  const [rippleLoading, setRippleLoading] = useState(false)
  const [rippleError, setRippleError] = useState<string | null>(null)
  const [rippleSource, setRippleSource] = useState<string | null>(null)
  const [impact, setImpact] = useState<ImpactResponse | null>(null)
  const [impactLoading, setImpactLoading] = useState(false)
  const [impactError, setImpactError] = useState<string | null>(null)
  const [impactDescription, setImpactDescription] = useState<string | null>(null)

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
    setExploreTab('graph')
    setSection('explore')
  }

  function openPredict(path: string) {
    setSelectedFile(path)
    setPredictTab('predictions')
    setSection('predict')
  }

  function openHistory() {
    setHistoryTab('timeline')
    setSection('history')
  }

  async function requestRipple(file: string) {
    const trimmed = file.trim()
    if (!trimmed) return
    setRippleLoading(true)
    setRippleError(null)
    setRippleSource(trimmed)
    try {
      const result =
        source.kind === 'github'
          ? await fetchRipple(source.owner, source.repo, { source: trimmed })
          : await fetchLocalRipple(source.path, { source: trimmed })
      setRipple(result)
    } catch (err) {
      setRipple(null)
      setRippleError(err instanceof Error ? err.message : 'Ripple request failed')
    } finally {
      setRippleLoading(false)
    }
  }

  async function requestImpact(description: string) {
    const trimmed = description.trim()
    if (!trimmed) return
    setImpactLoading(true)
    setImpactError(null)
    setImpactDescription(trimmed)
    try {
      const result =
        source.kind === 'github'
          ? await fetchImpact(source.owner, source.repo, {
              changeDescription: trimmed,
            })
          : await fetchLocalImpact(source.path, {
              changeDescription: trimmed,
            })
      setImpact(result)
    } catch (err) {
      setImpact(null)
      setImpactError(err instanceof Error ? err.message : 'Impact request failed')
    } finally {
      setImpactLoading(false)
    }
  }

  function openGraphFromDrawer(path: string) {
    setSelectedFile(null)
    setGraphFocus(path)
    setExploreTab('graph')
    setSection('explore')
  }

  const subLabel =
    section === 'explore'
      ? (EXPLORE_TABS.find((tab) => tab.id === exploreTab)?.label ?? '')
      : section === 'predict'
        ? (PREDICT_TABS.find((tab) => tab.id === predictTab)?.label ?? '')
        : section === 'history'
          ? (HISTORY_TABS.find((tab) => tab.id === historyTab)?.label ?? '')
          : ''

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
            subLabel={subLabel || undefined}
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
                  onPrimary={() => setSection('predict')}
                />
              </>
            )}

            {section === 'explore' && (
              <>
                <SubTabs
                  tabs={EXPLORE_TABS}
                  active={exploreTab}
                  onChange={setExploreTab}
                  label="Explore views"
                />
                {exploreTab === 'graph' && (
                  <>
                    <RepoGraph
                      edges={analysis.propagation.edges}
                      files={paths}
                      focus={graphFocus}
                      onFocus={setGraphFocus}
                      onOpenFile={openFile}
                      onViewPrediction={openFile}
                      onGoHistory={openHistory}
                      predicted={new Set(predictionMap.keys())}
                    />
                    <FlowNext
                      title="Check the historical evidence"
                      sub="Validate graph relationships against commit history, sequences, and rework signals."
                      primary="View Historical Evidence"
                      onPrimary={openHistory}
                    />
                  </>
                )}
                {exploreTab === 'files' && (
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
                      onPrimary={() => setExploreTab('graph')}
                    />
                  </>
                )}
                {exploreTab === 'dependencies' && (
                  <Dependencies
                    dependencies={analysis.dependencies}
                    sourceFiles={analysis.source.source_files}
                  />
                )}
              </>
            )}

            {section === 'predict' && (
              <>
                <SubTabs
                  tabs={PREDICT_TABS}
                  active={predictTab}
                  onChange={setPredictTab}
                  label="Prediction views"
                />
                {predictTab === 'predictions' && (
                  <>
                    <PredictView
                      analysis={analysis}
                      paths={paths}
                      predictions={predictions?.predictions ?? null}
                      prediction={
                        selectedFile ? (predictionMap.get(selectedFile) ?? null) : null
                      }
                      selectedFile={selectedFile}
                      modelName={predictions?.model.name ?? 'logistic_regression'}
                      availableModels={predictions?.model.available_models ?? []}
                      stats={stats}
                      loading={predictionsLoading}
                      error={predictionsError}
                      onRetry={onRetryPredictions}
                      onPickFile={openFile}
                      onWhy={openFile}
                      onOpenGraph={openGraph}
                    />
                    <Hotspots hotspots={analysis.hotspots} />
                    <FlowNext
                      title="Check the historical evidence"
                      sub="Validate predictions against the commits and patterns behind them."
                      primary="View Historical Evidence"
                      onPrimary={openHistory}
                    />
                  </>
                )}
                {predictTab === 'ripple' && (
                  <>
                    <RippleForecast
                      files={paths}
                      response={ripple}
                      loading={rippleLoading}
                      error={rippleError}
                      onRetry={() => {
                        if (rippleSource) void requestRipple(rippleSource)
                      }}
                      onRequest={(file) => void requestRipple(file)}
                      onOpenFile={openFile}
                    />
                    <FlowNext
                      title="See how files connect"
                      sub="Open the change graph to explore structural dependencies around the predicted ripple."
                      primary="Explore Change Graph"
                      onPrimary={() =>
                        openGraph(ripple?.source ?? graphFocus ?? paths[0] ?? '')
                      }
                    />
                  </>
                )}
                {predictTab === 'simulator' && (
                  <>
                    <ImpactSimulator
                      response={impact}
                      loading={impactLoading}
                      error={impactError}
                      onRetry={() => {
                        if (impactDescription) void requestImpact(impactDescription)
                      }}
                      onRequest={(description) => void requestImpact(description)}
                      onOpenFile={openFile}
                      onOpenGraph={() => {
                        setExploreTab('graph')
                        setSection('explore')
                      }}
                    />
                    <FlowNext
                      title="See how files connect"
                      sub="Open the change graph to explore structural dependencies around the simulated impact."
                      primary="Explore Change Graph"
                      onPrimary={() => {
                        setExploreTab('graph')
                        setSection('explore')
                      }}
                    />
                  </>
                )}
              </>
            )}

            {section === 'history' && (
              <>
                <SubTabs
                  tabs={HISTORY_TABS}
                  active={historyTab}
                  onChange={setHistoryTab}
                  label="History views"
                />
                {historyTab === 'timeline' && (
                  <>
                    <HistoryView
                      timeline={analysis.timeline}
                      focusFile={selectedFile}
                      onOpenFile={openFile}
                      onOpenPrediction={openPredict}
                      onOpenGraph={openGraph}
                    />
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
                {historyTab === 'patterns' && (
                  <>
                    <Sequences
                      sequences={analysis.sequences.sequences}
                      windowSeconds={analysis.sequences.window_seconds}
                    />
                    <Cochange
                      pairs={analysis.cochange.pairs}
                      total={analysis.cochange.total_pairs}
                    />
                    <PropagationGraph propagation={analysis.propagation} />
                    <PropagationHistory
                      timeline={analysis.timeline}
                      followups={analysis.followups}
                      rework={analysis.rework}
                    />
                    <HistoricalExamples
                      examples={analysis.examples.examples}
                      total={analysis.examples.total}
                    />
                    <FlowNext
                      title="Back to the overview"
                      sub="Return to the command center for risk summary and files requiring attention."
                      primary="Back to Overview"
                      onPrimary={() => setSection('overview')}
                    />
                  </>
                )}
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
          onGoHistory={openHistory}
        />
      )}
    </div>
  )
}
