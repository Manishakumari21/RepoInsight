import { useId, useState } from 'react'
import { glossaryDefinition } from '../lib/glossary'

export function Tooltip({
  term,
  text,
  children,
}: {
  term?: string
  text?: string
  children?: React.ReactNode
}) {
  const [open, setOpen] = useState(false)
  const id = useId()
  const definition = text ?? (term ? glossaryDefinition(term) : null)
  if (!definition) return <>{children ?? term}</>
  return (
    <span className="tooltip-wrap">
      <button
        type="button"
        className="tooltip-term"
        aria-describedby={id}
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
      >
        {children ?? term}
        <span className="tooltip-mark" aria-hidden="true">
          ?
        </span>
      </button>
      {open && (
        <span className="tooltip-bubble" role="tooltip" id={id}>
          {definition}
        </span>
      )}
    </span>
  )
}
