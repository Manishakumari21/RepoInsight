import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { RepoGraph } from '../components/RepoGraph'
import { FileExplorer } from '../components/FileExplorer'
import { HistoryView } from '../components/HistoryView'
import { Header } from '../components/Header'
import type { PropagationEdge } from '../types'

const edges: PropagationEdge[] = [
  { source: 'a.ts', target: 'b.ts', dependency: true, temporal: false, cochange: false, strength: 5 },
  { source: 'b.ts', target: 'c.ts', dependency: false, temporal: true, cochange: false, strength: 2 },
]

describe('RepoGraph', () => {
  it('renders nodes and selects through the accessible fallback list', () => {
    const onFocus = vi.fn()
    const onOpenFile = vi.fn()
    render(
      <RepoGraph
        edges={edges}
        files={['a.ts', 'b.ts', 'c.ts']}
        focus="a.ts"
        onFocus={onFocus}
        onOpenFile={onOpenFile}
      />,
    )
    expect(screen.getByText(/Showing 2 files/)).toBeInTheDocument()
    expect(screen.getByText('Selected: a.ts', { exact: false })).toBeInTheDocument()
    fireEvent.click(screen.getByText('b.ts'))
    expect(onFocus).toHaveBeenCalledWith('b.ts')
    fireEvent.click(screen.getByText('Open file details'))
    expect(onOpenFile).toHaveBeenCalledWith('a.ts')
  })

  it('filters by relationship type and resets', () => {
    const onFocus = vi.fn()
    render(
      <RepoGraph
        edges={edges}
        files={['a.ts', 'b.ts', 'c.ts']}
        focus={null}
        onFocus={onFocus}
        onOpenFile={() => {}}
      />,
    )
    fireEvent.change(screen.getByLabelText('Filter graph by relationship type'), {
      target: { value: 'dependency' },
    })
    expect(screen.getByText(/Showing 2 files/)).toBeInTheDocument()
    fireEvent.click(screen.getByText('Reset view'))
    expect(onFocus).toHaveBeenCalledWith(null)
  })

  it('shows an empty state without relationships', () => {
    render(
      <RepoGraph edges={[]} files={[]} focus={null} onFocus={() => {}} onOpenFile={() => {}} />,
    )
    expect(
      screen.getByText('No connections found yet.', { exact: false }),
    ).toBeInTheDocument()
  })
})

describe('FileExplorer', () => {
  it('searches and opens files', () => {
    const onOpenFile = vi.fn()
    render(
      <FileExplorer
        paths={['src/auth.ts', 'src/user.ts']}
        predictions={new Map()}
        stats={{}}
        onOpenFile={onOpenFile}
      />,
    )
    fireEvent.change(screen.getByLabelText('Search files'), {
      target: { value: 'auth' },
    })
    expect(screen.getByText('auth.ts')).toBeInTheDocument()
    expect(screen.queryByText('user.ts')).not.toBeInTheDocument()
    fireEvent.click(screen.getByText('auth.ts'))
    expect(onOpenFile).toHaveBeenCalledWith('src/auth.ts')
  })

  it('shows an empty state', () => {
    render(
      <FileExplorer paths={[]} predictions={new Map()} stats={{}} onOpenFile={() => {}} />,
    )
    expect(
      screen.getByText('No files are available for this repository yet.'),
    ).toBeInTheDocument()
  })
})

describe('HistoryView', () => {
  const timeline = {
    entries: [
      {
        sha: 'abc123',
        order: 0,
        timestamp: 1700000000,
        date: '2023-11-14T00:00:00Z',
        author: 'dev',
        message: 'fix auth',
        files: ['src/auth.ts', 'src/user.ts'],
        additions: 10,
        deletions: 2,
      },
    ],
    total_commits: 1,
    truncated: false,
  } as never

  it('selects a commit and opens its files', () => {
    const onOpenFile = vi.fn()
    render(<HistoryView timeline={timeline} onOpenFile={onOpenFile} />)
    fireEvent.click(screen.getByRole('button', { name: /Commit abc123/ }))
    expect(screen.getByText('src/auth.ts')).toBeInTheDocument()
    fireEvent.click(screen.getByText('src/user.ts'))
    expect(onOpenFile).toHaveBeenCalledWith('src/user.ts')
  })

  it('shows an empty state', () => {
    render(
      <HistoryView
        timeline={{ entries: [], total_commits: 0, truncated: false }}
        onOpenFile={() => {}}
      />,
    )
    expect(
      screen.getByText('No commit history is available for this repository yet.'),
    ).toBeInTheDocument()
  })
})

describe('Header', () => {
  it('switches sections and imports', () => {
    const onSection = vi.fn()
    const onImport = vi.fn()
    render(
      <Header
        sourceLabel="owner/repo"
        branch="main"
        section="overview"
        onSection={onSection}
        onImport={onImport}
      />,
    )
    for (const tab of ['Overview', 'Structure', 'Timeline', 'Coupling', 'Predictions', 'Evidence']) {
      expect(screen.getByRole('button', { name: tab })).toBeInTheDocument()
    }
    fireEvent.click(screen.getByRole('button', { name: 'Structure' }))
    expect(onSection).toHaveBeenCalledWith('structure')
    fireEvent.click(screen.getByText('Import Repository'))
    expect(onImport).toHaveBeenCalled()
  })
})
