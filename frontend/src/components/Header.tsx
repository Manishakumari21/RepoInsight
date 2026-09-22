import type { DashboardSection } from '../lib/sections'
import { SECTIONS } from '../lib/sections'

export function Header({
  sourceLabel,
  branch,
  section,
  onSection,
  onImport,
}: {
  sourceLabel: string
  branch: string
  section: DashboardSection
  onSection: (section: DashboardSection) => void
  onImport: () => void
}) {
  return (
    <header className="dash-header">
      <div className="dash-repo" title={sourceLabel}>
        <span className="repo-chip">{sourceLabel}</span>
        {branch && <span className="branch-chip">{branch}</span>}
      </div>
      <nav className="dash-tabs" aria-label="Dashboard sections">
        {SECTIONS.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`dash-tab${section === item.id ? ' active' : ''}`}
            aria-current={section === item.id ? 'page' : undefined}
            onClick={() => onSection(item.id)}
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
