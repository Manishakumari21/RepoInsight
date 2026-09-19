import type { RepositoryAnalysis } from '../types'
import { formatBytes, formatDate, formatNumber } from '../types'
import { difficultyColor } from '../lib/palette'
import { commitsPerMonth, formatRate } from '../lib/derive'
import { Stat } from './primitives'

export function Overview({
  owner,
  repo,
  analysis,
}: {
  owner: string
  repo: string
  analysis: RepositoryAnalysis
}) {
  const { repository, source, history, dependencies, hotspots, difficulty } =
    analysis
  const rate = commitsPerMonth(
    history.total_commits,
    history.first_commit,
    history.last_commit,
  )
  const flagged = hotspots.filter((item) => item.score >= 40).length
  const levelTone = difficultyColor(difficulty.level)

  return (
    <section id="overview" className="overview">
      <div className="hero" data-reveal>
        <div className="hero-eyebrow">Repository Analysis</div>
        <h1 className="hero-title">{repository.name}</h1>
        <p className="hero-sub">
          {owner}/{repo} · {repository.default_branch} branch
        </p>
        <div className="hero-chips">
          <span className="chip">
            {formatNumber(source.source_files)} source files
          </span>
          <span className="chip">{formatNumber(history.total_commits)} commits sampled</span>
          <span className="chip">
            {formatDate(history.first_commit)} → {formatDate(history.last_commit)}
          </span>
        </div>
      </div>

      <div className="insights">
        <Stat
          value={`${difficulty.level} · ${Math.round(difficulty.score)}`}
          label="Repository risk"
          tone={levelTone}
        />
        <Stat
          value={rate != null ? `≈${formatRate(rate)}/mo` : '—'}
          label="Commit velocity"
        />
        <Stat
          value={`+${formatNumber(history.total_additions)} · −${formatNumber(history.total_deletions)}`}
          label="Lines added · deleted"
        />
        <Stat
          value={`${formatNumber(flagged)} · ${formatNumber(hotspots.length)}`}
          label="Flagged · hotspots"
        />
      </div>

      <div className="stat-grid">
        <Stat label="Total Files" value={formatNumber(repository.total_files)} />
        <Stat
          label="Source Files"
          value={formatNumber(source.source_files)}
          sub={formatBytes(source.total_size_bytes)}
        />
        <Stat label="Lines of Code" value={formatNumber(source.total_lines)} />
        <Stat label="Commits" value={formatNumber(history.total_commits)} />
        <Stat label="Contributors" value={formatNumber(history.active_contributors)} />
        <Stat
          label="Dependencies"
          value={formatNumber(dependencies.total_dependencies)}
          sub={`${formatNumber(dependencies.connected_files)} connected files`}
        />
      </div>
    </section>
  )
}