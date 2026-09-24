import { useCallback, useEffect, useMemo, useState } from 'react'
import type {
  FilePrediction,
  ImpactResponse,
  PredictionsResponse,
  RepositoryAnalysis,
  RippleResponse,
} from '../types'
import type { ActiveSource } from '../lib/repoSource'
import { fetchLocalImpact, fetchImpact, fetchLocalRipple, fetchRipple } from '../api'
import { buildFileStats, collectFilePaths } from '../lib/files'
import { NAV_ITEMS, type PredictTab, type ViewId } from '../lib/sections'
import { useMediaQuery } from '../lib/useMediaQuery'
import { TopBar } from './TopBar'
import { Sidebar } from './Sidebar'
import { Breadcrumbs } from './Breadcrumbs'
import { SubTabs } from './SubTabs'
import { CommandPalette, type PaletteSelection } from './CommandPalette'
import { OverviewPage } from './OverviewPage'
import { StructureView } from './StructureView'
import { PredictView } from './PredictView'
import { RippleForecast } from './RippleForecast'
import { ImpactSimulator } from './ImpactSimulator'
import { TimelineView } from './TimelineView'
import { CouplingView } from './CouplingView'
import { EvidenceView } from './EvidenceView'
import { FileInspector } from './FileInspector'
import { FileDrawer } from './FileDrawer'
import { Hotspots } from './Hotspots'
import { Dependencies } from './Dependencies'
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
  onRefresh,
  refreshing,
}: {
  source: ActiveSource
  analysis: RepositoryAnalysis
  predictions: PredictionsResponse | null
  predictionsLoading: boolean
  predictionsError: string | null
  onRetryPredictions: () => void
  onImport: () => void
  onRefresh: () => void
  refreshing: boolean
}) {
  const [view, setView] = useState<ViewId>('overview')
  const [predictTab, setPredictTab] = useState<PredictTab>('predicted')
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [graphFocus, setGraphFocus] = useState<string | null>(null)
  const [structureQuery, setStructureQuery] = useState('')
  const [focusSha, setFocusSha] = useState<string | null>(null)
  const [paletteOpen, setPaletteOpen] = useState(false)
  const [collapsed, setCollapsed] = useState(false)
  const [ripple, setRipple] = useState<RippleResponse | null>(null)
  const [rippleLoading, setRippleLoading] = useState(false)
  const [rippleError, setRippleError] = useState<string | null>(null)
  const [rippleSource, setRippleSource] = useState<string | null>(null)
  const [impact, setImpact] = useState<ImpactResponse | null>(null)
  const [impactLoading, setImpactLoading] = useState(false)
  const [impactError, setImpactError] = useState<string | null>(null)
  const [impactDescription, setImpactDescription] = useState<string | null>(null)
  const wide = useMediaQuery('(min-width: 1100px)')

  const stats = useMemo(() => buildFileStats(analysis), [analysis])
  const paths = useMemo(() => collectFilePaths(analysis), [analysis])
  const predictionMap = useMemo(() => {
    const map = new Map<string, FilePrediction>()
    for (const item of predictions?.predictions ?? []) map.set(item.file_path, item)
    return map
  }, [predictions])
  const predictedSet = useMemo(() => new Set(predictionMap.keys()), [predictionMap])

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        setPaletteOpen((open) => !open)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  function openFile(path: string) {
    setSelectedFile(path)
  }

  function focusAndOpen(path: string) {
    setGraphFocus(path)
    setSelectedFile(path)
  }

  function openGraph(path: string) {
    setGraphFocus(path)
    setView('structure')
  }

  function openArea(directory: string) {
    setStructureQuery(directory === '(root)' ? '' : directory)
    setView('structure')
  }

  function openHistory() {
    setView('timeline')
  }

  function openPaletteSelection(selection: PaletteSelection) {
    setPaletteOpen(false)
    if (selection.kind === 'file' && selection.file) {
      setSelectedFile(selection.file)
      setGraphFocus(selection.file)
      setView('structure')
    } else if (selection.kind === 'commit' && selection.sha) {
      setFocusSha(selection.sha)
      setView('timeline')
    }
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

  function openGraphFromInspector(path: string) {
    setSelectedFile(wide ? selectedFile : null)
    setGraphFocus(path)
    setView('structure')
  }

  const navigate = useCallback((next: ViewId) => {
    setView(next)
  }, [])

  const trail = useMemo(() => {
    const item = NAV_ITEMS.find((entry) => entry.id === view)
    return [{ label: item?.label ?? view }]
  }, [view])

  return (
    <div className={`shell${collapsed ? ' collapsed' : ''}`}>
      <Sidebar
        sourceLabel={source.kind === 'github' ? `${source.owner}/${source.repo}` : source.path}
        branch={analysis.repository.default_branch}
        view={view}
        onNavigate={navigate}
        predictionCount={predictions?.predictions.length ?? 0}
        modelOn={predictions !== null}
        collapsed={collapsed}
        onToggleCollapse={() => setCollapsed((value) => !value)}
      />
      <div className="main">
        <TopBar
          source={source}
          branch={analysis.repository.default_branch}
          refreshing={refreshing}
          onSearch={() => setPaletteOpen(true)}
          onRefresh={onRefresh}
          onImport={onImport}
        />
        <main className="content workbench" id="main-content" tabIndex={-1}>
          <Breadcrumbs trail={trail} file={selectedFile} onHome={() => setView('overview')} />
          <div className="dashboard-body">
            {analysis.timeline.truncated && (
              <NoticeBanner message="Partial analysis — showing what completed. Some history may be truncated." />
            )}

            {view === 'overview' && (
              <OverviewPage
                analysis={analysis}
                sourceLabel={source.kind === 'github' ? `${source.owner}/${source.repo}` : source.path}
                predictions={predictions?.predictions ?? []}
                predictionsAvailable={predictions !== null}
                onOpenFile={openFile}
                onOpenArea={openArea}
                onOpenHistory={openHistory}
              />
            )}

            {view === 'structure' && (
              <StructureView
                paths={paths}
                predictions={predictionMap}
                stats={stats}
                edges={analysis.propagation.edges}
                focus={graphFocus}
                treeQuery={structureQuery}
                predicted={predictedSet}
                onOpenFile={openFile}
                onFocus={setGraphFocus}
                onViewPrediction={openFile}
                onGoHistory={openHistory}
              />
            )}

            {view === 'dependencies' && (
              <>
                <div className="page-head">
                  <h1>Dependencies</h1>
                  <p>
                    Structural links between files, such as imports, extracted
                    from source code.
                  </p>
                </div>
                <Dependencies
                  dependencies={analysis.dependencies}
                  sourceFiles={analysis.source.source_files}
                />
              </>
            )}

            {view === 'timeline' && (
              <>
                <TimelineView
                  timeline={analysis.timeline}
                  focusFile={selectedFile}
                  focusSha={focusSha}
                  onOpenFile={openFile}
                  onOpenPrediction={openFile}
                  onOpenGraph={openGraph}
                />
                <DatasetReadiness analysis={analysis} />
                <Settings repository={analysis.repository} />
              </>
            )}

            {view === 'coupling' && (
              <CouplingView
                analysis={analysis}
                onOpenFile={openFile}
                onHighlight={(path) => {
                  setGraphFocus(path)
                  setView('structure')
                }}
              />
            )}

            {view === 'predictions' && (
              <>
                <SubTabs
                  tabs={[
                    { id: 'predicted', label: 'Predicted changes' },
                    { id: 'related', label: 'Related files' },
                    { id: 'impact', label: 'Impact simulator' },
                  ]}
                  active={predictTab}
                  onChange={setPredictTab}
                  label="Prediction views"
                />
                {predictTab === 'predicted' && (
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
                  </>
                )}
                {predictTab === 'related' && (
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
                )}
                {predictTab === 'impact' && (
                  <ImpactSimulator
                    response={impact}
                    loading={impactLoading}
                    error={impactError}
                    onRetry={() => {
                      if (impactDescription) void requestImpact(impactDescription)
                    }}
                    onRequest={(description) => void requestImpact(description)}
                    onOpenFile={openFile}
                    onOpenGraph={() => setView('structure')}
                  />
                )}
              </>
            )}

            {view === 'evidence' && (
              <EvidenceView
                predictions={predictions?.predictions ?? null}
                loading={predictionsLoading}
                error={predictionsError}
                onOpenFile={openFile}
              />
            )}
          </div>
        </main>
      </div>

      {selectedFile && wide && (
        <aside className="inspector" aria-label="File inspector">
          <FileInspector
            path={selectedFile}
            prediction={predictionMap.get(selectedFile) ?? null}
            analysis={analysis}
            onClose={() => setSelectedFile(null)}
            onOpenFile={focusAndOpen}
            onOpenGraph={openGraphFromInspector}
            onGoHistory={openHistory}
          />
        </aside>
      )}
      {selectedFile && !wide && (
        <FileDrawer
          path={selectedFile}
          prediction={predictionMap.get(selectedFile) ?? null}
          analysis={analysis}
          onClose={() => setSelectedFile(null)}
          onOpenFile={focusAndOpen}
          onOpenGraph={openGraphFromInspector}
          onGoHistory={openHistory}
        />
      )}
      {paletteOpen && (
        <CommandPalette
          files={paths}
          commits={analysis.timeline.entries ?? []}
          onSelect={openPaletteSelection}
          onClose={() => setPaletteOpen(false)}
        />
      )}
    </div>
  )
}
