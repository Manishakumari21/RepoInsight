import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { ImpactSimulator } from '../components/ImpactSimulator'
import type { ImpactResponse } from '../types'

const response: ImpactResponse = {
  change_description: 'Replace JWT authentication with OAuth2',
  intent: {
    operation: 'replace',
    domains: ['authentication'],
    terms: ['authentication', 'jwt', 'oauth2'],
    current_technology: 'jwt',
    target_technology: 'oauth2',
  },
  impact: [
    {
      path: 'src/auth/login.ts',
      level: 'high',
      score: 0.91,
      categories: ['direct'],
      evidence: ['Path matches change concept: auth, authentication.'],
    },
    {
      path: 'tests/auth.test.ts',
      level: 'medium',
      score: 0.42,
      categories: ['test'],
      evidence: ['Path matches change concept: auth.'],
    },
  ],
  capabilities: [
    {
      name: 'authentication',
      files: ['src/auth/login.ts', 'tests/auth.test.ts'],
    },
  ],
  checklist: [
    {
      label: 'Review authentication',
      files: ['src/auth/login.ts', 'tests/auth.test.ts'],
    },
    {
      label: 'Review tests',
      files: ['tests/auth.test.ts'],
    },
  ],
  graph: {
    nodes: [
      {
        id: 'change',
        kind: 'change',
        label: 'Replace JWT authentication with OAuth2',
        level: null,
      },
      {
        id: 'capability:authentication',
        kind: 'capability',
        label: 'authentication',
        level: 'high',
      },
      {
        id: 'src/auth/login.ts',
        kind: 'file',
        label: 'src/auth/login.ts',
        level: 'high',
      },
    ],
    edges: [
      {
        from: 'change',
        to: 'capability:authentication',
        evidence: '2 impacted files in authentication.',
      },
      {
        from: 'capability:authentication',
        to: 'src/auth/login.ts',
        evidence: 'Path matches change concept: auth, authentication.',
      },
    ],
  },
  model: {
    name: 'impact_baseline',
    calibrated: false,
    threshold: 0.15,
    weights: { semantic: 0.35, dependency: 0.2, historical: 0.3, kind: 0.15 },
    training: 'baseline',
  },
}

function renderSimulator(overrides = {}) {
  return render(
    <ImpactSimulator
      response={response}
      loading={false}
      error={null}
      onRetry={() => {}}
      onRequest={() => {}}
      onOpenFile={() => {}}
      onOpenGraph={() => {}}
      {...overrides}
    />,
  )
}

describe('ImpactSimulator', () => {
  it('renders detected intent without claiming certainty', () => {
    renderSimulator()
    expect(screen.getByText(/Operation: replace/)).toBeInTheDocument()
    expect(screen.getByText(/Current: jwt/)).toBeInTheDocument()
    expect(screen.queryByText(/mistake/i)).not.toBeInTheDocument()
  })

  it('groups impacted files by level', () => {
    renderSimulator()
    expect(screen.getByLabelText('high impact')).toBeInTheDocument()
    expect(screen.getByLabelText('medium impact')).toBeInTheDocument()
    expect(screen.getAllByText('src/auth/login.ts').length).toBeGreaterThan(0)
    expect(screen.getByText('91%')).toBeInTheDocument()
  })

  it('expands evidence on demand', () => {
    renderSimulator()
    fireEvent.click(screen.getAllByText('Evidence')[0])
    expect(
      screen.getByText('Path matches change concept: auth, authentication.'),
    ).toBeInTheDocument()
  })

  it('renders the review checklist with file links', () => {
    renderSimulator()
    expect(screen.getByLabelText('Review checklist')).toBeInTheDocument()
    expect(screen.getByText('Review authentication')).toBeInTheDocument()
  })

  it('renders the capability impact map', () => {
    renderSimulator()
    expect(screen.getByLabelText('Impact map')).toBeInTheDocument()
    expect(screen.getByText('authentication')).toBeInTheDocument()
  })

  it('submits the described change for analysis', () => {
    const onRequest = vi.fn()
    render(
      <ImpactSimulator
        response={null}
        loading={false}
        error={null}
        onRetry={() => {}}
        onRequest={onRequest}
        onOpenFile={() => {}}
        onOpenGraph={() => {}}
      />,
    )
    fireEvent.change(screen.getByLabelText('Describe your planned change'), {
      target: { value: 'Replace JWT authentication with OAuth2' },
    })
    fireEvent.click(screen.getByText('Analyze'))
    expect(onRequest).toHaveBeenCalledWith(
      'Replace JWT authentication with OAuth2',
    )
  })

  it('navigates to a file from the impact list', () => {
    const onOpenFile = vi.fn()
    renderSimulator({ onOpenFile })
    fireEvent.click(screen.getAllByText('src/auth/login.ts')[0])
    expect(onOpenFile).toHaveBeenCalledWith('src/auth/login.ts')
  })
})
