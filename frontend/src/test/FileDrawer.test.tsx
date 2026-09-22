import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { FileDrawer } from '../components/FileDrawer'
import { makePrediction } from './fixtures'

const analysis = {
  structural_features: [
    {
      file_path: 'src/auth.ts',
      file_size_bytes: 1200,
      lines_of_code: 80,
      function_count: 4,
      cyclomatic_complexity: 6,
      max_nesting_depth: 2,
      incoming_dependencies: 1,
      outgoing_dependencies: 2,
      coupling: 3,
      num_dependents: 1,
    },
  ],
  historical_features: [],
  timeline: { entries: [], total_commits: 0, truncated: false },
} as never

describe('FileDrawer', () => {
  it('renders prediction, evidence, examples and recommendations', () => {
    render(
      <FileDrawer
        path="src/auth.ts"
        prediction={makePrediction('src/auth.ts', 0.78)}
        analysis={analysis}
        onClose={() => {}}
        onOpenFile={() => {}}
      />,
    )
    expect(screen.getByText('78.0%')).toBeInTheDocument()
    expect(screen.getByText('Needs follow-up')).toBeInTheDocument()
    expect(screen.getByText('historical_rework_frequency')).toBeInTheDocument()
    expect(screen.getByText('co change', { exact: false })).toBeInTheDocument()
    expect(screen.getByText(/Consider reviewing related changes/)).toBeInTheDocument()
    expect(screen.getByText('Signals from past history')).toBeInTheDocument()
    expect(screen.getByText(/not guaranteed outcomes/)).toBeInTheDocument()
  })

  it('distinguishes missing prediction and missing examples', () => {
    const bare = { ...makePrediction('src/auth.ts', 0.1), historical_examples: [], recommendations: [] }
    render(
      <FileDrawer
        path="src/auth.ts"
        prediction={bare}
        analysis={analysis}
        onClose={() => {}}
        onOpenFile={() => {}}
      />,
    )
    expect(
      screen.getByText('No relevant historical examples found.'),
    ).toBeInTheDocument()
  })

  it('opens related files and closes', () => {
    const onOpenFile = vi.fn()
    const onClose = vi.fn()
    render(
      <FileDrawer
        path="src/auth.ts"
        prediction={makePrediction('src/auth.ts', 0.78)}
        analysis={analysis}
        onClose={onClose}
        onOpenFile={onOpenFile}
      />,
    )
    fireEvent.click(screen.getByText('src/other.ts'))
    expect(onOpenFile).toHaveBeenCalledWith('src/other.ts')
    fireEvent.click(screen.getByLabelText('Close file details'))
    expect(onClose).toHaveBeenCalled()
  })

  it('opens the change graph from the drawer', () => {
    const onOpenGraph = vi.fn()
    render(
      <FileDrawer
        path="src/auth.ts"
        prediction={makePrediction('src/auth.ts', 0.78)}
        analysis={analysis}
        onClose={() => {}}
        onOpenFile={() => {}}
        onOpenGraph={onOpenGraph}
      />,
    )
    fireEvent.click(screen.getByText('See this file in the change graph'))
    expect(onOpenGraph).toHaveBeenCalledWith('src/auth.ts')
  })

  it('handles files without prediction data', () => {
    render(
      <FileDrawer
        path="unknown.ts"
        prediction={null}
        analysis={analysis}
        onClose={() => {}}
        onOpenFile={() => {}}
      />,
    )
    expect(
      screen.getByText('No prediction data is available for this file yet.'),
    ).toBeInTheDocument()
  })
})
