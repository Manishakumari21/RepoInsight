import type { HistoryAnalysis } from '../types'
import { formatDate, formatNumber } from '../types'
import { commitsPerMonth, formatRate, share } from '../lib/derive'
import { Panel } from './Panel'

export function History({
  history,
  totalLines,
}: {
  history: HistoryAnalysis
  totalLines: number
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
      <div className="metrics">
        <div className="metric">
          <div className="metric-value">{formatNumber(history.total_commits)}</div>
          <div className="metric-label">Commits analyzed</div>
        </div>
        <div className="metric">
          <div className="metric-value">{formatNumber(history.active_contributors)}</div>
          <div className="metric-label">Contributors</div>
        </div>
        <div className="metric">
          <div className="metric-value">{formatNumber(history.changed_files)}</div>
          <div className="metric-label">Files changed</div>
        </div>
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