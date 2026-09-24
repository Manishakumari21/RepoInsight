import type { ActiveSource } from '../lib/repoSource'
import { sourceLabel } from '../lib/repoSource'
import { Logo } from './Logo'

export function TopBar({
  source,
  branch,
  refreshing,
  onSearch,
  onRefresh,
  onImport,
}: {
  source: ActiveSource
  branch: string
  refreshing: boolean
  onSearch: () => void
  onRefresh: () => void
  onImport: () => void
}) {
  return (
    <header className="topbar">
      <div className="topbar-brand" title="RepoInsight — repository intelligence workbench">
        <span className="brand-mark" aria-hidden="true">
          <Logo size={18} />
        </span>
        <span className="brand-title">RepoInsight</span>
      </div>
      <div className="topbar-repo" title={sourceLabel(source)}>
        <span className="repo-chip">{sourceLabel(source)}</span>
        {branch && (
          <span
            className="branch-chip"
            title="Analysis branch. Switching branches is not supported by the analysis API."
          >
            {branch}
          </span>
        )}
      </div>
      <button
        type="button"
        className="topbar-search"
        onClick={onSearch}
        aria-label="Search files and commits"
        title="Search files and commits"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" aria-hidden="true">
          <circle cx="11" cy="11" r="7" />
          <path d="M20 20l-3.5-3.5" />
        </svg>
        <span className="topbar-search-text">Search files, commits…</span>
        <kbd aria-hidden="true">⌘K</kbd>
      </button>
      <div className="topbar-actions">
        <button
          type="button"
          className="ghost-button topbar-refresh"
          onClick={onRefresh}
          disabled={refreshing}
          aria-busy={refreshing}
          aria-label="Refresh analysis"
          title="Re-fetch repository analysis and predictions"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            className={refreshing ? 'spin' : undefined}
          >
            <path d="M20 11a8 8 0 1 0-2.3 6.3" />
            <path d="M20 5v6h-6" />
          </svg>
          {refreshing ? 'Refreshing…' : 'Refresh'}
        </button>
        <button type="button" className="dash-import" onClick={onImport}>
          Import Repository
        </button>
      </div>
    </header>
  )
}
