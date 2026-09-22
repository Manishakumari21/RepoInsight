export function FlowNext({
  title,
  sub,
  primary,
  onPrimary,
  secondary,
  onSecondary,
}: {
  title: string
  sub: string
  primary: string
  onPrimary: () => void
  secondary?: string
  onSecondary?: () => void
}) {
  return (
    <div className="flow-next">
      <div className="flow-next-text">
        <div className="flow-next-title">{title}</div>
        <div className="flow-next-sub">{sub}</div>
      </div>
      <div className="flow-next-actions">
        {secondary && onSecondary && (
          <button type="button" className="flow-ghost" onClick={onSecondary}>
            {secondary}
          </button>
        )}
        <button type="button" className="flow-button" onClick={onPrimary}>
          {primary} <span className="arrow" aria-hidden="true">→</span>
        </button>
      </div>
    </div>
  )
}
