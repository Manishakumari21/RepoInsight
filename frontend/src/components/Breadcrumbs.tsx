import type { DashboardSection } from '../lib/sections'
import { SECTIONS } from '../lib/sections'

const LABELS = new Map<DashboardSection, string>(
  SECTIONS.map((item) => [item.id, item.label]),
)

export function Breadcrumbs({
  section,
  file,
  onSection,
}: {
  section: DashboardSection
  file: string | null
  onSection: (section: DashboardSection) => void
}) {
  return (
    <nav className="crumbs" aria-label="Breadcrumb">
      <button type="button" onClick={() => onSection('overview')}>
        RepoInsight
      </button>
      <span className="crumb-sep" aria-hidden="true">›</span>
      {file ? (
        <>
          <button type="button" onClick={() => onSection(section)}>
            {LABELS.get(section) ?? section}
          </button>
          <span className="crumb-sep" aria-hidden="true">›</span>
          <span className="crumb-current crumb-file">{file}</span>
        </>
      ) : (
        <span className="crumb-current">{LABELS.get(section) ?? section}</span>
      )}
    </nav>
  )
}
