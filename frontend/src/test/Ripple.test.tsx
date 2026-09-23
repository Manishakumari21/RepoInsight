import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { RippleForecast } from '../components/RippleForecast'
import type { RippleResponse } from '../types'

const response: RippleResponse = {
  source: 'login.ts',
  candidates: [
    {
      file: 'session.ts',
      probability: 0.86,
      rank: 1,
      reasons: ['Changed together in 11 historical commits.'],
      historical_examples: [
        {
          commit_sha: 'abc123def',
          timestamp: 100,
          date: null,
          event_type: 'co_change',
          related_files: ['login.ts', 'session.ts'],
        },
      ],
    },
    {
      file: 'auth.test.ts',
      probability: 0.81,
      rank: 2,
      reasons: ['Followed in 7 comparable changes.'],
      historical_examples: [],
    },
  ],
  ripple_paths: [
    {
      nodes: ['login.ts', 'session.ts', 'auth.test.ts'],
      edge_scores: [0.86, 0.81],
      confidence: 0.835,
      evidence: ['Changed together in 11 historical commits.'],
    },
  ],
  missing_impact: [
    {
      file: 'session.ts',
      probability: 0.86,
      message:
        'These files frequently changed in comparable historical changes. Consider reviewing session.ts (historical follow probability: 86%).',
    },
  ],
  model: {
    name: 'ripple_baseline',
    calibrated: false,
    threshold: 0.5,
    weights: { dependency: 0.2, cochange: 0.3, temporal: 0.35, recent: 0.15 },
    training: 'baseline',
  },
}

function renderRipple(overrides = {}) {
  return render(
    <RippleForecast
      files={['login.ts', 'session.ts', 'auth.test.ts']}
      response={response}
      loading={false}
      error={null}
      onRetry={() => {}}
      onRequest={() => {}}
      onOpenFile={() => {}}
      {...overrides}
    />,
  )
}

describe('RippleForecast', () => {
  it('renders candidates with probabilities and ranks', () => {
    renderRipple()
    expect(screen.getAllByText('session.ts').length).toBeGreaterThan(0)
    expect(screen.getByText('86%')).toBeInTheDocument()
    expect(screen.getAllByText('auth.test.ts').length).toBeGreaterThan(0)
  })

  it('renders the predicted propagation path in order', () => {
    renderRipple()
    const path = screen.getByLabelText('Predicted ripple path')
    expect(path).toHaveTextContent('login.ts')
    expect(path).toHaveTextContent('session.ts')
    expect(path).toHaveTextContent('auth.test.ts')
  })

  it('expands evidence and historical examples', () => {
    renderRipple()
    fireEvent.click(screen.getAllByText('Evidence')[0])
    expect(
      screen.getByText('Changed together in 11 historical commits.'),
    ).toBeInTheDocument()
    expect(screen.getByText(/abc123d/)).toBeInTheDocument()
  })

  it('shows neutral missing-impact warnings', () => {
    renderRipple()
    expect(screen.getByLabelText('Potential missing impact')).toBeInTheDocument()
    expect(screen.getAllByText(/Consider reviewing/).length).toBeGreaterThan(0)
    expect(screen.queryByText(/mistake/i)).not.toBeInTheDocument()
  })

  it('requests a ripple for the selected source file', () => {
    const onRequest = vi.fn()
    render(
      <RippleForecast
        files={['login.ts']}
        response={null}
        loading={false}
        error={null}
        onRetry={() => {}}
        onRequest={onRequest}
        onOpenFile={() => {}}
      />,
    )
    fireEvent.change(screen.getByLabelText('Select a ripple source file'), {
      target: { value: 'login.ts' },
    })
    fireEvent.click(screen.getByText('Forecast ripple'))
    expect(onRequest).toHaveBeenCalledWith('login.ts')
  })

  it('shows an empty-path message when evidence is weak', () => {
    renderRipple({
      response: { ...response, ripple_paths: [] },
    })
    expect(screen.getByText(/No ripple path/)).toBeInTheDocument()
  })
})
