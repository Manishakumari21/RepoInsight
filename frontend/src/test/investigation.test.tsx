import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { Sidebar } from '../components/Sidebar'
import { Breadcrumbs } from '../components/Breadcrumbs'
import { PredictView } from '../components/PredictView'
import { OverviewPage } from '../components/OverviewPage'
import { HistoryView } from '../components/HistoryView'
import { makePrediction } from './fixtures'
import type { RepositoryAnalysis } from '../types'

function analysisStub(): RepositoryAnalysis {
  return {
    repository: { name: 'repo', default_branch: 'main', total_files: 3 },
    source: { source_files: 3, total_lines: 100, total_size_bytes: 1000 },
    structural_features: [],
    historical_features: [],
    temporal_features: [],
    complexity: { average_complexity: 2.5, max_complexity: 5, complex_files: 1 },
    history: {
      total_commits: 2,
      active_contributors: 1,
      changed_files: 2,
      total_additions: 10,
      total_deletions: 2,
      total_churn: 12,
      first_commit: '2023-11-14T00:00:00Z',
      last_commit: '2023-11-15T00:00:00Z',
    },
    dependencies: {
      total_dependencies: 2,
      connected_files: 2,
      highly_connected_files: 0,
      resolved_dependencies: 2,
      top_coupled_files: [],
    },
    cochange: {
      pairs: [{ file_a: 'src/auth.ts', file_b: 'src/session.ts', count: 11 }],
      total_pairs: 1,
    },
    temporal: { pairs: [] },
    propagation: { edges: [] },
    timeline: {
      entries: [
        {
          sha: 'abc123',
          order: 0,
          timestamp: 1700000000,
          date: '2023-11-14T00:00:00Z',
          author: 'dev',
          message: 'fix auth',
          files: ['src/auth.ts'],
          additions: 10,
          deletions: 2,
        },
      ],
      total_commits: 1,
      truncated: false,
    },
    sequences: { sequences: [], window_seconds: 604800 },
    followups: { followups: [], total: 0, window_seconds: 604800 },
    rework: { events: [], total: 0, window_seconds: 1209600 },
    examples: { examples: [], total: 0 },
    hotspots: [{ path: 'src/auth.ts', score: 80, reasons: [] }],
    difficulty: { score: 60, level: 'medium' },
  } as unknown as RepositoryAnalysis
}

describe('Investigation navigation', () => {
  it('shows four coherent sections', () => {
    render(
      <Sidebar
        sourceLabel="owner/repo"
        branch="main"
        section="overview"
        onSection={() => {}}
        predictionCount={0}
        modelOn={false}
        collapsed={false}
        onToggleCollapse={() => {}}
      />,
    )
    for (const item of ['Overview', 'Explore', 'Predict', 'History']) {
      expect(screen.getByTitle(item)).toBeInTheDocument()
    }
    expect(screen.queryByTitle('Predictions')).not.toBeInTheDocument()
    expect(screen.queryByTitle('Ripple')).not.toBeInTheDocument()
  })

  it('renders Repository / Section / Sub / File breadcrumbs', () => {
    render(
      <Breadcrumbs
        section="predict"
        subLabel="Predictions"
        file="src/auth.ts"
        onSection={() => {}}
      />,
    )
    expect(screen.getByText('RepoInsight')).toBeInTheDocument()
    expect(screen.getByText('Predict')).toBeInTheDocument()
    expect(screen.getByText('Predictions')).toBeInTheDocument()
    expect(screen.getByText('src/auth.ts')).toBeInTheDocument()
  })
})

describe('PredictView', () => {
  const predictions = [makePrediction('src/auth.ts', 0.86)]

  function renderView(overrides = {}) {
    return render(
      <PredictView
        analysis={analysisStub()}
        paths={['src/auth.ts', 'src/session.ts']}
        predictions={predictions}
        prediction={predictions[0]}
        selectedFile="src/auth.ts"
        modelName="logistic_regression"
        availableModels={['logistic_regression']}
        stats={{}}
        loading={false}
        error={null}
        onRetry={() => {}}
        onPickFile={() => {}}
        onWhy={() => {}}
        {...overrides}
      />,
    )
  }

  it('shows the selected file prediction with a Why action', () => {
    const onWhy = vi.fn()
    renderView({ onWhy, selectedFile: null, prediction: null })
    fireEvent.change(screen.getByLabelText('Select a file to predict'), {
      target: { value: 'auth' },
    })
    expect(screen.getAllByText('src/auth.ts').length).toBeGreaterThan(0)
  })

  it('lists likely related changes with evidence navigation', () => {
    const onWhy = vi.fn()
    renderView({ onWhy })
    expect(screen.getByText('src/session.ts')).toBeInTheDocument()
    expect(screen.getByText('×11')).toBeInTheDocument()
    fireEvent.click(screen.getByText('Why? View evidence →'))
    expect(onWhy).toHaveBeenCalledWith('src/auth.ts')
  })

  it('explains an empty relationship honestly', () => {
    renderView({ selectedFile: 'src/lonely.ts', prediction: null })
    expect(
      screen.getByText(/No historical relationship found/),
    ).toBeInTheDocument()
  })
})

describe('OverviewPage investigation', () => {
  it('recommends real investigations and navigates to files', () => {
    const onOpenFile = vi.fn()
    render(
      <OverviewPage
        analysis={analysisStub()}
        sourceLabel="owner/repo"
        predictions={[makePrediction('src/auth.ts', 0.9)]}
        predictionsAvailable
        onOpenFile={onOpenFile}
        onOpenGraph={() => {}}
        onSection={() => {}}
      />,
    )
    expect(screen.getByText('Repository Signals')).toBeInTheDocument()
    expect(screen.getByText('Recommended Investigation')).toBeInTheDocument()
    fireEvent.click(screen.getAllByText('Open file →')[0])
    expect(onOpenFile).toHaveBeenCalled()
  })

  it('navigates recent activity to history', () => {
    const onSection = vi.fn()
    render(
      <OverviewPage
        analysis={analysisStub()}
        sourceLabel="owner/repo"
        predictions={[]}
        predictionsAvailable={false}
        onOpenFile={() => {}}
        onOpenGraph={() => {}}
        onSection={onSection}
      />,
    )
    fireEvent.click(screen.getByText('fix auth'))
    expect(onSection).toHaveBeenCalledWith('history')
  })
})

describe('HistoryView investigation', () => {
  const timeline = analysisStub().timeline

  it('filters commits by file', () => {
    render(<HistoryView timeline={timeline} onOpenFile={() => {}} />)
    fireEvent.change(screen.getByLabelText('Filter commits by file'), {
      target: { value: 'other.ts' },
    })
    expect(screen.getByText(/No commits touch/)).toBeInTheDocument()
  })

  it('navigates from a commit to prediction and graph', () => {
    const onOpenPrediction = vi.fn()
    const onOpenGraph = vi.fn()
    render(
      <HistoryView
        timeline={timeline}
        onOpenFile={() => {}}
        onOpenPrediction={onOpenPrediction}
        onOpenGraph={onOpenGraph}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: /Commit abc123/ }))
    fireEvent.click(screen.getByText('View prediction →'))
    expect(onOpenPrediction).toHaveBeenCalledWith('src/auth.ts')
    fireEvent.click(screen.getByText('Focus in graph'))
    expect(onOpenGraph).toHaveBeenCalledWith('src/auth.ts')
  })
})
