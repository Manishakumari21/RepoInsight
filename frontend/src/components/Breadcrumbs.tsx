export interface Crumb {
  label: string
  onSelect?: () => void
}

export function Breadcrumbs({
  trail,
  file,
  onHome,
}: {
  trail: Crumb[]
  file: string | null
  onHome: () => void
}) {
  return (
    <nav className="crumbs" aria-label="Breadcrumb">
      <button type="button" onClick={onHome}>
        RepoInsight
      </button>
      {trail.map((crumb, index) => (
        <span key={`${crumb.label}-${index}`}>
          <span className="crumb-sep" aria-hidden="true">
            ›
          </span>
          {crumb.onSelect ? (
            <button type="button" onClick={crumb.onSelect}>
              {crumb.label}
            </button>
          ) : (
            <span className="crumb-current">{crumb.label}</span>
          )}
        </span>
      ))}
      {file && (
        <>
          <span className="crumb-sep" aria-hidden="true">
            ›
          </span>
          <span className="crumb-current crumb-file">{file}</span>
        </>
      )}
    </nav>
  )
}
