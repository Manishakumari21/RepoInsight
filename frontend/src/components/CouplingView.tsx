import { useMemo, useState } from 'react'
import type { RepositoryAnalysis } from '../types'
import { formatNumber, shortSha } from '../types'
import { changeCounts } from '../lib/files'
import { couplingStrengthClass, trendClass } from '../lib/labels'
import { pairTrend, type CouplingTrend } from '../lib/analysis'
import { DataTable } from './DataTable'
import { Panel } from './Panel'
import { Tooltip } from './Tooltip'
import { EmptyState } from './EmptyState'

interface CouplingRow {
  fileA: string
  fileB: string
  count: number
  changesA: number
  changesB: number
  trend: CouplingTrend
  recent: number
  older: number
}

function strengthOf(count: number, peak: number): 'Strong' | 'Moderate' | 'Weak' {
  if (peak <= 0) return 'Weak'
  const ratio = count / peak
  if (ratio >= 0.66) return 'Strong'
  if (ratio >= 0.33) return 'Moderate'
  return 'Weak'
}

export function CouplingView({
  analysis,
  onOpenFile,
  onHighlight,
}: {
  analysis: RepositoryAnalysis
  onOpenFile: (path: string) => void
  onHighlight: (path: string) => void
}) {
  const [minCount, setMinCount] = useState(2)
  const [directory, setDirectory] = useState('')
  const [timeRange, setTimeRange] = useState<'all' | 'recent'>('all')
  const [relationship, setRelationship] = useState<'all' | 'dependency' | 'temporal'>('all')
  const [selected, setSelected] = useState<string | null>(null)

  const counts = useMemo(() => changeCounts(analysis), [analysis])

  const rows = useMemo<CouplingRow[]>(() => {
    const dir = directory.trim().toLowerCase()
    const edges = analysis.propagation.edges ?? []
    const hasRelation = (a: string, b: string, kind: 'dependency' | 'temporal') =>
      edges.some(
        (e) =>
          ((e.source === a && e.target === b) || (e.source === b && e.target === a)) &&
          (kind === 'dependency' ? e.dependency : e.temporal),
      )
    return (analysis.cochange.pairs ?? [])
      .filter((pair) => pair.count >= minCount)
      .filter(
        (pair) =>
          !dir ||
          pair.file_a.toLowerCase().includes(dir) ||
          pair.file_b.toLowerCase().includes(dir),
      )
      .filter((pair) => {
        if (relationship === 'all') return true
        return hasRelation(pair.file_a, pair.file_b, relationship)
      })
      .map((pair) => {
        const { trend, recent, older } = pairTrend(
          analysis.timeline.entries ?? [],
          pair.file_a,
          pair.file_b,
        )
        return {
          fileA: pair.file_a,
          fileB: pair.file_b,
          count: pair.count,
          changesA: counts.get(pair.file_a) ?? 0,
          changesB: counts.get(pair.file_b) ?? 0,
          trend: timeRange === 'recent' && recent === 0 ? 'unknown' : trend,
          recent,
          older,
        }
      })
      .filter((row) => (timeRange === 'recent' ? row.recent > 0 : true))
      .sort((a, b) => b.count - a.count || a.fileA.localeCompare(b.fileA))
  }, [analysis, counts, minCount, directory, timeRange, relationship])

  const peak = rows.reduce((max, row) => Math.max(max, row.count), 0)

  const active = selected
    ? rows.find((row) => `${row.fileA}↔${row.fileB}` === selected) ?? null
    : null
  const activeExamples = useMemo(() => {
    if (!active) return []
    return (analysis.timeline.entries ?? [])
      .filter(
        (entry) =>
          entry.files.includes(active.fileA) && entry.files.includes(active.fileB),
      )
      .slice(-5)
      .reverse()
  }, [analysis, active])

  return (
    <>
      <div className="page-head">
        <h1>Change Coupling</h1>
        <p>
          Files that repeatedly change together may have an implicit
          relationship — even without a direct dependency.
        </p>
      </div>
      <Panel title="Coupling filters" hint="Narrow the ranked table">
        <div className="filter-bar">
          <label className="filter-field">
            Minimum co-changes
            <input
              className="filter-input"
              type="number"
              min={1}
              value={minCount}
              onChange={(event) =>
                setMinCount(Math.max(1, Number(event.target.value) || 1))
              }
              aria-label="Minimum co-changes"
            />
          </label>
          <label className="filter-field">
            Directory
            <input
              className="filter-input"
              type="search"
              value={directory}
              onChange={(event) => setDirectory(event.target.value)}
              placeholder="src/auth…"
              aria-label="Filter by directory"
            />
          </label>
          <label className="filter-field">
            Time range
            <select value={timeRange} onChange={(e) => setTimeRange(e.target.value as 'all' | 'recent')} aria-label="Filter by time range">
              <option value="all">All history</option>
              <option value="recent">Active recently</option>
            </select>
          </label>
          <label className="filter-field">
            Relationship
            <select value={relationship} onChange={(e) => setRelationship(e.target.value as 'all' | 'dependency' | 'temporal')} aria-label="Filter by relationship type">
              <option value="all">Co-change only</option>
              <option value="dependency">Also dependency-linked</option>
              <option value="temporal">Also temporally ordered</option>
            </select>
          </label>
        </div>
      </Panel>
      <Panel
        title="Ranked coupling"
        hint={`${formatNumber(rows.length)} pairs · strength is relative to this repository`}
      >
        <DataTable<CouplingRow>
          labelledBy="Coupling table"
          emptyText="No coupling data. Coupling analysis requires repository history with multiple changes."
          maxRows={100}
          defaultSort={{ key: 'count', dir: 'desc' }}
          rows={rows}
          columns={[
            {
              key: 'fileA',
              label: 'File A',
              sortable: true,
              sortValue: (row) => row.fileA,
              render: (row) => (
                <button
                  type="button"
                  className="link-button"
                  onClick={() => onOpenFile(row.fileA)}
                >
                  {row.fileA}
                </button>
              ),
            },
            {
              key: 'fileB',
              label: 'File B',
              sortable: true,
              sortValue: (row) => row.fileB,
              render: (row) => (
                <button
                  type="button"
                  className="link-button"
                  onClick={() => onOpenFile(row.fileB)}
                >
                  {row.fileB}
                </button>
              ),
            },
            {
              key: 'count',
              label: 'Co-changes',
              sortable: true,
              align: 'right',
              sortValue: (row) => row.count,
              render: (row) => `×${row.count}`,
            },
            {
              key: 'totals',
              label: 'Total changes',
              align: 'right',
              render: (row) => (
                <span className="muted">
                  {row.changesA} / {row.changesB}
                </span>
              ),
            },
            {
              key: 'strength',
              label: 'Coupling strength',
              sortable: true,
              sortValue: (row) => row.count,
              render: (row) => {
                const strength = strengthOf(row.count, peak)
                return (
                  <span
                    className={`status ${couplingStrengthClass(strength)}`}
                  >
                    {strength}
                  </span>
                )
              },
            },
            {
              key: 'trend',
              label: 'Recent trend',
              sortable: true,
              sortValue: (row) => row.trend,
              render: (row) => (
                <span className={`status ${trendClass(row.trend)}`}>
                  {row.trend === 'unknown' ? '—' : row.trend}
                </span>
              ),
            },
            {
              key: 'inspect',
              label: 'Inspect',
              render: (row) => (
                <span style={{ display: 'inline-flex', gap: 8 }}>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={() =>
                      setSelected(
                        selected === `${row.fileA}↔${row.fileB}`
                          ? null
                          : `${row.fileA}↔${row.fileB}`,
                      )
                    }
                    aria-expanded={selected === `${row.fileA}↔${row.fileB}`}
                  >
                    Why?
                  </button>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={() => onHighlight(row.fileA)}
                    title="Highlight these files in the graph"
                  >
                    Graph
                  </button>
                </span>
              ),
            },
          ]}
        />
        <p className="history-note">
          <Tooltip term="Coupling" /> · <Tooltip term="Co-change" /> · strength
          is ranked relative to this table, not an absolute score.
        </p>
      </Panel>
      {active && (
        <Panel
          title={`Why are these files coupled?`}
          hint={`${active.fileA} ↔ ${active.fileB}`}
        >
          <ul className="evidence-list">
            <li className="evidence-row">
              <span className="evidence-body">
                Co-changed <strong>×{active.count}</strong> · observed across{' '}
                {formatNumber(active.recent + active.older)} shared change sets
                ({active.recent} recent, {active.older} older).
              </span>
            </li>
            <li className="evidence-row">
              <span className="evidence-body">
                Recent coupling trend:{' '}
                <strong>
                  {active.trend === 'unknown' ? 'no dated history' : active.trend}
                </strong>
                .
              </span>
            </li>
            <li className="evidence-row">
              <span className="evidence-body">
                Individual activity: {active.fileA} changed{' '}
                {formatNumber(active.changesA)}×, {active.fileB} changed{' '}
                {formatNumber(active.changesB)}×.
              </span>
            </li>
          </ul>
          {activeExamples.length > 0 && (
            <>
              <p className="eyebrow">Historical examples</p>
              <ul className="example-list">
                {activeExamples.map((entry) => (
                  <li key={entry.sha} className="example-row">
                    <div className="example-top">
                      <code>{shortSha(entry.sha)}</code>
                      <span className="muted">{entry.message || '(no message)'}</span>
                    </div>
                  </li>
                ))}
              </ul>
            </>
          )}
          {activeExamples.length === 0 && <EmptyState title="No matching historical change sets were found." body="The pair co-changed, but no individual commit in the sampled history contains both files together. Try widening the time range." />}
        </Panel>
      )}
    </>
  )
}
