import type { ReactNode } from 'react'
import { NAV_ITEMS, type ViewId } from '../lib/sections'
import { Logo } from './Logo'

const ICONS: Record<ViewId, ReactNode> = {
  overview: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="3" y="3" width="7" height="9" rx="1.5" />
      <rect x="14" y="3" width="7" height="5" rx="1.5" />
      <rect x="14" y="12" width="7" height="9" rx="1.5" />
      <rect x="3" y="16" width="7" height="5" rx="1.5" />
    </svg>
  ),
  structure: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M4 4h6l2 2h8v14H4z" />
      <path d="M9 12h6M9 15.5h6" />
    </svg>
  ),
  dependencies: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="6" cy="6" r="2.4" />
      <circle cx="18" cy="8" r="2.4" />
      <circle cx="10" cy="18" r="2.4" />
      <path d="M8.2 7l7.4.7M7 8.2l2 7.3M16 10l-4.4 6" />
    </svg>
  ),
  timeline: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M4 6v5h5" />
      <path d="M4.5 11a8 8 0 1 1-1 5" />
      <path d="M12 8v4l3 2" />
    </svg>
  ),
  coupling: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M8 12h8" />
      <circle cx="5" cy="12" r="2.2" />
      <circle cx="19" cy="12" r="2.2" />
    </svg>
  ),
  predictions: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 17l5-6 4 3 6-8" />
      <path d="M14 6h4v4" />
      <circle cx="8" cy="11" r="1.4" fill="currentColor" stroke="none" />
    </svg>
  ),
  evidence: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 8v4l2.5 2.5" />
    </svg>
  ),
}

const GROUPS = [...new Set(NAV_ITEMS.map((item) => item.group))]

export function Sidebar({
  sourceLabel,
  branch,
  view,
  onNavigate,
  predictionCount,
  modelOn,
  collapsed,
  onToggleCollapse,
}: {
  sourceLabel: string
  branch: string
  view: ViewId
  onNavigate: (view: ViewId) => void
  predictionCount: number
  modelOn: boolean
  collapsed: boolean
  onToggleCollapse: () => void
}) {
  return (
    <aside className="sidebar" aria-label="Repository navigation">
      <div className="sidebar-brand">
        <span className="brand-mark" aria-hidden="true"><Logo size={20} /></span>
        <div>
          <div className="brand-title">RepoInsight</div>
          <div className="brand-sub">change intelligence</div>
        </div>
      </div>

      <div className="sidebar-repo" title={sourceLabel}>
        <span className="sidebar-repo-label">Repository</span>
        <span className="sidebar-repo-name">{sourceLabel}</span>
        {branch && <span className="branch-chip">{branch}</span>}
      </div>

      <button
        type="button"
        className="sidebar-collapse"
        onClick={onToggleCollapse}
        aria-expanded={!collapsed}
        aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      >
        {collapsed ? '» Expand' : '« Collapse'}
      </button>

      <nav className="sidebar-nav" aria-label="Analysis views">
        {GROUPS.map((group) => (
          <div className="nav-group" key={group}>
            <div className="nav-label">{group}</div>
            {NAV_ITEMS.filter((item) => item.group === group).map((item) => (
              <button
                key={item.id}
                type="button"
                className={`nav-button${view === item.id ? ' active' : ''}`}
                aria-current={view === item.id ? 'page' : undefined}
                onClick={() => onNavigate(item.id)}
                title={`${item.label} — ${item.tooltip}`}
              >
                <span className="nav-icon" aria-hidden="true">{ICONS[item.id]}</span>
                <span className="nav-text">{item.label}</span>
                {item.id === 'predictions' && predictionCount > 0 && (
                  <span className="nav-meta">{predictionCount}</span>
                )}
              </button>
            ))}
          </div>
        ))}
      </nav>

      <div className="sidebar-footer">
        <span>
          <span className="model-dot" aria-hidden="true" />{' '}
          {modelOn ? 'model active · uncalibrated' : 'model pending'}
        </span>
        <span>evidence ≠ prediction</span>
      </div>
    </aside>
  )
}
