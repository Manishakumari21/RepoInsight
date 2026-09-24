import { useMemo, useState } from 'react'
import type { ChangeTimeline } from '../types'
import { bucketSeries } from '../lib/analysis'
import { HistoryView } from './HistoryView'
import { TimelineChart } from './TimelineChart'
import { Panel } from './Panel'

type RangePreset = 'all' | '30' | '90' | '365'
type ChangeType = 'all' | 'additions' | 'deletions' | 'mixed'

export function TimelineView({
  timeline,
  focusFile,
  focusSha,
  onOpenFile,
  onOpenPrediction,
  onOpenGraph,
}: {
  timeline: ChangeTimeline
  focusFile: string | null
  focusSha: string | null
  onOpenFile: (path: string) => void
  onOpenPrediction: (path: string) => void
  onOpenGraph: (path: string) => void
}) {
  const [range, setRange] = useState<RangePreset>('all')
  const [author, setAuthor] = useState('all')
  const [directory, setDirectory] = useState('')
  const [file, setFile] = useState('')
  const [changeType, setChangeType] = useState<ChangeType>('all')

  const authors = useMemo(() => {
    const names = new Set<string>()
    for (const entry of timeline.entries ?? []) {
      if (entry.author) names.add(entry.author)
    }
    return [...names].sort((a, b) => a.localeCompare(b))
  }, [timeline])

  const newest = useMemo(() => {
    let max = 0
    for (const entry of timeline.entries ?? []) {
      if (typeof entry.timestamp === 'number') max = Math.max(max, entry.timestamp)
    }
    return max
  }, [timeline])

  const filtered = useMemo(() => {
    const cutoff =
      range === 'all' || newest === 0 ? 0 : newest - Number(range) * 24 * 60 * 60
    const dir = directory.trim().toLowerCase()
    const name = file.trim().toLowerCase()
    return (timeline.entries ?? []).filter((entry) => {
      if (typeof entry.timestamp === 'number' && entry.timestamp < cutoff) {
        return false
      }
      if (author !== 'all' && entry.author !== author) return false
      if (dir && !entry.files.some((item) => item.toLowerCase().includes(dir))) {
        return false
      }
      if (name && !entry.files.some((item) => item.toLowerCase().includes(name))) {
        return false
      }
      if (changeType === 'additions' && entry.additions <= entry.deletions) {
        return false
      }
      if (changeType === 'deletions' && entry.deletions <= entry.additions) {
        return false
      }
      if (
        changeType === 'mixed' &&
        (entry.additions === 0 || entry.deletions === 0)
      ) {
        return false
      }
      return true
    })
  }, [timeline, range, newest, author, directory, file, changeType])

  const buckets = useMemo(() => bucketSeries(filtered), [filtered])
  const shown: ChangeTimeline = useMemo(
    () => ({
      ...timeline,
      entries: filtered,
      total_commits: filtered.length,
      truncated: false,
    }),
    [timeline, filtered],
  )
  const isFiltered =
    range !== 'all' ||
    author !== 'all' ||
    directory.trim() !== '' ||
    file.trim() !== '' ||
    changeType !== 'all'

  return (
    <>
      <div className="page-head">
        <h1>Repository Timeline</h1>
        <p>
          Commit activity over time. Filter to a period, author, or area —
          every number below comes from recorded commits.
        </p>
      </div>
      <Panel title="Activity" hint="Commits and files changed per week">
        <TimelineChart
          buckets={buckets}
          label={`Commit activity chart: ${filtered.length} commits shown`}
        />
      </Panel>
      <Panel title="History filters" hint="Narrow the change sets">
        <div className="filter-bar">
          <label className="filter-field">
            Date range
            <select
              value={range}
              onChange={(event) => setRange(event.target.value as RangePreset)}
              aria-label="Filter by date range"
            >
              <option value="all">All time</option>
              <option value="30">Last 30 days</option>
              <option value="90">Last 90 days</option>
              <option value="365">Last year</option>
            </select>
          </label>
          <label className="filter-field">
            Author
            <select
              value={author}
              onChange={(event) => setAuthor(event.target.value)}
              aria-label="Filter by author"
            >
              <option value="all">All authors</option>
              {authors.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
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
            File
            <input
              className="filter-input"
              type="search"
              value={file}
              onChange={(event) => setFile(event.target.value)}
              placeholder="session.ts…"
              aria-label="Filter timeline by file"
            />
          </label>
          <label className="filter-field">
            Change type
            <select
              value={changeType}
              onChange={(event) => setChangeType(event.target.value as ChangeType)}
              aria-label="Filter by change type"
              title="Mostly additions, mostly deletions, or mixed — computed from recorded additions and deletions."
            >
              <option value="all">All changes</option>
              <option value="additions">Mostly additions</option>
              <option value="deletions">Mostly deletions</option>
              <option value="mixed">Mixed</option>
            </select>
          </label>
        </div>
        {isFiltered && (
          <p className="history-note" role="status">
            Showing {filtered.length} of {timeline.entries.length} commits.
          </p>
        )}
      </Panel>
      <HistoryView
        timeline={shown}
        focusFile={focusFile}
        focusSha={focusSha}
        onOpenFile={onOpenFile}
        onOpenPrediction={onOpenPrediction}
        onOpenGraph={onOpenGraph}
      />
    </>
  )
}
