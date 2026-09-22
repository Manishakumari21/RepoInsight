import { useMemo, useState } from 'react'
import type { FilePrediction } from '../types'
import { buildFileTree } from '../lib/files'
import { Panel } from './Panel'

export function FileExplorer({
  paths,
  predictions,
  stats,
  onOpenFile,
}: {
  paths: string[]
  predictions: Map<string, FilePrediction>
  stats: Record<string, { changes: number; complexity: number; dependencies: number; size: number }>
  onOpenFile: (path: string) => void
}) {
  const [query, setQuery] = useState('')
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set())

  const filtered = useMemo(() => {
    const term = query.trim().toLowerCase()
    if (!term) return paths
    return paths.filter((path) => path.toLowerCase().includes(term))
  }, [paths, query])

  const tree = useMemo(() => buildFileTree(filtered), [filtered])

  function toggle(dir: string) {
    setCollapsed((prev) => {
      const next = new Set(prev)
      if (next.has(dir)) next.delete(dir)
      else next.add(dir)
      return next
    })
  }

  if (paths.length === 0) {
    return (
      <Panel title="File Explorer" hint="Browse analyzed files">
        <div className="empty-state">
          <p>No files are available for this repository yet.</p>
        </div>
      </Panel>
    )
  }

  const renderNodes = (nodes: typeof tree, depth: number): React.ReactNode => (
    <ul className={depth === 0 ? 'tree-root' : 'tree-children'}>
      {nodes.map((node) => {
        if (node.isFile) {
          const prediction = predictions.get(node.path)
          const info = stats[node.path]
          return (
            <li key={node.path} className="tree-file">
              <button
                type="button"
                className="link-button tree-name"
                onClick={() => onOpenFile(node.path)}
                title={node.path}
              >
                {node.name}
              </button>
              {prediction && (
                <span
                  className={`risk-badge${prediction.label === 1 ? ' high' : ''}`}
                  title={`Rework probability ${(prediction.probability * 100).toFixed(1)}%`}
                >
                  {(prediction.probability * 100).toFixed(0)}%
                </span>
              )}
              {info && (
                <span className="tree-meta muted">
                  Δ{info.changes} · C{info.complexity}
                </span>
              )}
            </li>
          )
        }
        const isCollapsed = collapsed.has(node.path)
        return (
          <li key={node.path} className="tree-dir">
            <button
              type="button"
              className="tree-toggle"
              onClick={() => toggle(node.path)}
              aria-expanded={!isCollapsed}
              aria-label={`${isCollapsed ? 'Expand' : 'Collapse'} ${node.path}`}
            >
              {isCollapsed ? '▸' : '▾'} {node.name}/
            </button>
            {!isCollapsed && renderNodes(node.children, depth + 1)}
          </li>
        )
      })}
    </ul>
  )

  return (
    <Panel title="File Explorer" hint="Browse analyzed files">
      <input
        className="filter-input"
        type="search"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="Search files…"
        aria-label="Search files"
      />
      <p className="history-note" role="status">
        {filtered.length} of {paths.length} files
      </p>
      {filtered.length === 0 ? (
        <div className="empty-state">
          <p>No files match this search.</p>
        </div>
      ) : (
        renderNodes(tree, 0)
      )}
    </Panel>
  )
}
