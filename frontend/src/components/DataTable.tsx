import { useMemo, useState } from 'react'

export interface TableColumn<T> {
  key: string
  label: string
  sortable?: boolean
  align?: 'left' | 'right'
  render: (row: T) => React.ReactNode
  sortValue?: (row: T) => string | number
}

export function DataTable<T>({
  columns,
  rows,
  maxRows,
  emptyText,
  labelledBy,
  defaultSort,
}: {
  columns: TableColumn<T>[]
  rows: T[]
  maxRows?: number
  emptyText: string
  labelledBy: string
  defaultSort?: { key: string; dir: 'asc' | 'desc' }
}) {
  const [sortKey, setSortKey] = useState(defaultSort?.key ?? '')
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>(defaultSort?.dir ?? 'desc')

  const visible = useMemo(() => {
    const column = columns.find((item) => item.key === sortKey)
    if (!column?.sortable) return rows
    const factor = sortDir === 'asc' ? 1 : -1
    return [...rows].sort((a, b) => {
      const left = column.sortValue?.(a) ?? ''
      const right = column.sortValue?.(b) ?? ''
      if (left < right) return -1 * factor
      if (left > right) return 1 * factor
      return 0
    })
  }, [columns, rows, sortKey, sortDir])

  function toggle(key: string) {
    if (key === sortKey) {
      setSortDir((dir) => (dir === 'asc' ? 'desc' : 'asc'))
    } else {
      setSortKey(key)
      setSortDir('desc')
    }
  }

  if (rows.length === 0) {
    return (
      <div className="empty-state">
        <p>{emptyText}</p>
      </div>
    )
  }
  const shown = maxRows ? visible.slice(0, maxRows) : visible
  return (
    <>
      <div className="table-wrap">
        <table className="data-table" aria-label={labelledBy}>
          <thead>
            <tr>
              {columns.map((column) => (
                <th
                  key={column.key}
                  className={column.align === 'right' ? 'num' : undefined}
                >
                  {column.sortable ? (
                    <button
                      type="button"
                      className="th-sort"
                      onClick={() => toggle(column.key)}
                      aria-label={`Sort by ${column.label}`}
                    >
                      {column.label}{' '}
                      {sortKey === column.key ? (sortDir === 'asc' ? '▲' : '▼') : ''}
                    </button>
                  ) : (
                    column.label
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {shown.map((row, index) => (
              <tr key={index}>
                {columns.map((column) => (
                  <td
                    key={column.key}
                    className={column.align === 'right' ? 'num tnum' : undefined}
                  >
                    {column.render(row)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {maxRows && visible.length > maxRows && (
        <p className="history-note">
          First {maxRows} rows shown — refine the search.
        </p>
      )}
    </>
  )
}
