import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { Predictions } from '../components/Predictions'
import { makePrediction } from './fixtures'

const predictions = [
  makePrediction('src/auth.ts', 0.78),
  makePrediction('src/user.ts', 0.12),
]

const stats = {
  'src/auth.ts': { changes: 1, complexity: 1, dependencies: 1 },
  'src/user.ts': { changes: 1, complexity: 1, dependencies: 1 },
}

function renderPredictions(overrides = {}) {
  return render(
    <Predictions
      predictions={predictions}
      modelName="logistic_regression"
      availableModels={['logistic_regression']}
      stats={stats}
      loading={false}
      error={null}
      onRetry={() => {}}
      onSelect={() => {}}
      {...overrides}
    />,
  )
}

describe('Predictions', () => {
  it('renders the prediction list with probabilities', () => {
    renderPredictions()
    expect(screen.getByText('src/auth.ts')).toBeInTheDocument()
    expect(screen.getByText('78.0%')).toBeInTheDocument()
    expect(screen.getByText('12.0%')).toBeInTheDocument()
  })

  it('filters by search query', () => {
    renderPredictions()
    fireEvent.change(screen.getByLabelText('Search predicted files'), {
      target: { value: 'auth' },
    })
    expect(screen.getByText('src/auth.ts')).toBeInTheDocument()
    expect(screen.queryByText('src/user.ts')).not.toBeInTheDocument()
  })

  it('filters by predicted label', () => {
    renderPredictions()
    fireEvent.change(screen.getByLabelText('Filter by predicted label'), {
      target: { value: 'negative' },
    })
    expect(screen.queryByText('src/auth.ts')).not.toBeInTheDocument()
    expect(screen.getByText('src/user.ts')).toBeInTheDocument()
  })

  it('sorts by file name', () => {
    renderPredictions()
    fireEvent.click(screen.getByText(/^File/))
    const rows = screen.getAllByRole('row').slice(1)
    expect(rows[0]).toHaveTextContent('src/auth.ts')
  })

  it('selects a file on click', () => {
    const onSelect = vi.fn()
    renderPredictions({ onSelect })
    fireEvent.click(screen.getByText('src/auth.ts'))
    expect(onSelect).toHaveBeenCalledWith('src/auth.ts')
  })

  it('shows loading, error and empty states', () => {
    const { unmount } = renderPredictions({ loading: true, predictions: null })
    expect(screen.getByText('Loading predictions…')).toBeInTheDocument()
    unmount()

    const onRetry = vi.fn()
    renderPredictions({ predictions: null, error: 'boom', onRetry })
    expect(
      screen.getByText('No prediction data is available for this repository yet.'),
    ).toBeInTheDocument()
    fireEvent.click(screen.getByText('Retry'))
    expect(onRetry).toHaveBeenCalled()
  })

  it('shows an empty state when no predictions exist', () => {
    renderPredictions({ predictions: [] })
    expect(
      screen.getByText('Predictions need parsed source files with repository history.'),
    ).toBeInTheDocument()
  })
})
