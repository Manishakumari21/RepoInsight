import { NAV_ITEMS, type ViewId } from '../lib/sections'

export function Header({
  sourceLabel,
  branch,
  section,
  onSection,
  onImport,
}: {
  sourceLabel: string
  branch: string
  section: ViewId
  onSection: (section: ViewId) => void
  onImport: () => void
}) {
  return (
    <header className="dash-header">
      <div className="dash-repo" title={sourceLabel}>
        <span className="repo-chip">{sourceLabel}</span>
        {branch && <span className="branch-chip">{branch}</span>}
      </div>
      <nav className="dash-tabs" aria-label="Dashboard sections">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`dash-tab${section === item.id ? ' active' : ''}`}
            aria-current={section === item.id ? 'page' : undefined}
            onClick={() => onSection(item.id)}
            title={item.tooltip}
          >
            {item.label}
          </button>
        ))}
      </nav>
      <button type="button" className="dash-import" onClick={onImport}>
        Import Repository
      </button>
    </header>
  )
}
