import type { ChangeTimeline, HistoryAnalysis } from '../types'
import { formatDate, formatNumber } from '../types'
import { commitsPerMonth, formatRate, share } from '../lib/derive'
import { weeklyBuckets } from '../lib/analysis'
import { Panel } from './Panel'
import { Stat } from './primitives'

function ActivityStrip({ timeline }: { timeline: ChangeTimeline }) {
  const buckets = weeklyBuckets(timeline.entries)
  if (buckets.length === 0) return null
  const peak = Math.max(...buckets)
  const total = buckets.reduce((sum, count) => sum + count, 0)
  return (
    <div className="activity-strip">
      <div className="bar-top">
        <span className="bar-label">Commits per week</span>
        <span className="bar-value tnum">
          {total} commits · peak {peak}/wk
        </span>
      </div>
      <svg
        className="activity-bars"
        viewBox={`0 0 ${buckets.length * 10} 44`}
        preserveAspectRatio="none"
        role="img"
        aria-label={`Weekly commit activity: ${total} commits total, peak ${peak} per week`}
      >
        {buckets.map((count, index) => {
          const height = peak > 0 ? Math.max((count / peak) * 40, count > 0 ? 3 : 0) : 0
          return (
            <rect
              key={index}
              x={index * 10 + 2}
              y={44 - height}
              width={6}
              height={height}
              rx={1.5}
            >
              <title>
                Week {index + 1}: {count} commit{count === 1 ? '' : 's'}
              </title>
            </rect>
          )
        })}
      </svg>
    </div>
  )
}

export function History({
  history,
  totalLines,
  timeline,
}: {
  history: HistoryAnalysis
  totalLines: number
  timeline: ChangeTimeline
}) {
  const total = history.total_additions + history.total_deletions
  const addPct = total > 0 ? (history.total_additions / total) * 100 : 0
  const rate = commitsPerMonth(
    history.total_commits,
    history.first_commit,
    history.last_commit,
  )
  const avgLinesPerCommit =
    history.total_commits > 0 ? history.total_churn / history.total_commits : 0
  const filesPerCommit =
    history.total_commits > 0
      ? history.changed_files / history.total_commits
      : 0
  const churnShare = share(history.total_churn, totalLines)

  return (
    <Panel id="history" title="History" hint="Commit activity over the analyzed sample">
      <ActivityStrip timeline={timeline} />
      <div className="metrics">
        <Stat label="Commits analyzed" value={formatNumber(history.total_commits)} />
        <Stat label="Contributors" value={formatNumber(history.active_contributors)} />
        <Stat label="Files changed" value={formatNumber(history.changed_files)} />
      </div>
      <div className="bars">
        <div className="bar-row">
          <div className="bar-top">
            <span className="bar-label">Additions vs. deletions</span>
            <span className="bar-value">
              +{formatNumber(history.total_additions)} / −{formatNumber(history.total_deletions)}
            </span>
          </div>
          <div className="bar-track churn-track">
            <div className="bar-fill add" style={{ width: `${addPct}%` }} />
            <div className="bar-fill del" style={{ width: `${100 - addPct}%` }} />
          </div>
        </div>
      </div>
      <div className="churn-legend">
        <span>
          <span className="swatch add" />
          Additions
        </span>
        <span>
          <span className="swatch del" />
          Deletions
        </span>
        <span>Churn {formatNumber(history.total_churn)}</span>
      </div>
      <div className="history-stats">
        <span className="chip">
          {rate != null ? `≈${formatRate(rate)} commits/mo` : 'Commits —'}
        </span>
        <span className="chip">
          {avgLinesPerCommit > 0 ? `${avgLinesPerCommit.toFixed(0)} lines/commit` : 'Lines/commit —'}
        </span>
        <span className="chip">
          {filesPerCommit > 0 ? `${filesPerCommit.toFixed(1)} files/commit` : 'Files/commit —'}
        </span>
        <span className="chip">{`${(churnShare * 100).toFixed(0)}% of LOC churned`}</span>
      </div>
      <div className="history-range">
        {formatDate(history.first_commit)} → {formatDate(history.last_commit)}
      </div>
      <p className="history-note">
        Files changed counts distinct files touched across the analyzed commit history
        (sampled up to 1000 commits), not the repository&apos;s current total file count.
      </p>
    </Panel>
  )
}