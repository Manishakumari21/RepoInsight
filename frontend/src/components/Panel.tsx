import type { ReactNode } from 'react'

export function Panel({
  title,
  hint,
  id,
  children,
}: {
  title: string
  hint?: string
  id?: string
  children: ReactNode
}) {
  return (
    <section className="panel" id={id}>
      <div className="panel-head">
        <h2>{title}</h2>
        {hint && <p className="panel-hint">{hint}</p>}
      </div>
      {children}
    </section>
  )
}