import type { ReactNode } from 'react'
import { EvidenceBadge, type EvidenceKind } from './primitives'

export function Panel({
  title,
  hint,
  id,
  tone,
  children,
}: {
  title: string
  hint?: string
  id?: string
  tone?: EvidenceKind
  children: ReactNode
}) {
  return (
    <section className={`panel${tone ? ` tone-${tone}` : ''}`} id={id}>
      <div className="panel-head">
        <h2>{title}</h2>
        <div className="panel-meta">
          {tone && <EvidenceBadge kind={tone} />}
          {hint && <p className="panel-hint">{hint}</p>}
        </div>
      </div>
      {children}
    </section>
  )
}