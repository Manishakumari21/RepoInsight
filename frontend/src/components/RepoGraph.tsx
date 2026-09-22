import { useMemo, useRef, useState } from 'react'
import type { PropagationEdge } from '../types'
import {
  buildNeighborhood,
  layoutCircle,
  layoutRadial,
  nodeSignalCounts,
  type EdgeFilter,
} from '../lib/graph'
import { Panel } from './Panel'

const WIDTH = 760
const HEIGHT = 520
const MAX_NODES = 24

function shortName(path: string): string {
  return path.length > 30 ? `…${path.slice(-29)}` : path
}

export function RepoGraph({
  edges,
  files,
  focus,
  onFocus,
  onOpenFile,
  onViewPrediction,
  onGoHistory,
}: {
  edges: PropagationEdge[]
  files: string[]
  focus: string | null
  onFocus: (path: string | null) => void
  onOpenFile: (path: string) => void
  onViewPrediction?: (path: string) => void
  onGoHistory?: () => void
}) {
  const [query, setQuery] = useState('')
  const [edgeFilter, setEdgeFilter] = useState<EdgeFilter>('all')
  const [depth, setDepth] = useState(1)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [zoom, setZoom] = useState(1)
  const drag = useRef<{ x: number; y: number } | null>(null)

  const data = useMemo(
    () => buildNeighborhood(edges, focus, files, depth, edgeFilter, MAX_NODES),
    [edges, focus, files, depth, edgeFilter],
  )
  const positions = useMemo(
    () =>
      focus
        ? layoutRadial(data.nodes, data.links, focus, WIDTH, HEIGHT)
        : layoutCircle(data.nodes, WIDTH, HEIGHT),
    [data, focus],
  )
  const matches = useMemo(() => {
    const term = query.trim().toLowerCase()
    if (!term) return []
    return files.filter((file) => file.toLowerCase().includes(term)).slice(0, 8)
  }, [query, files])

  const counts = useMemo(
    () => (focus ? nodeSignalCounts(edges, focus, edgeFilter) : null),
    [edges, focus, edgeFilter],
  )

  const selected = focus ? data.nodes.find((node) => node.id === focus) : null

  const topFiles = useMemo(() => {
    const degree = new Map<string, number>()
    for (const edge of edges) {
      degree.set(edge.source, (degree.get(edge.source) ?? 0) + 1)
      degree.set(edge.target, (degree.get(edge.target) ?? 0) + 1)
    }
    return [...degree.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .slice(0, 5)
      .map(([id]) => id)
  }, [edges])

  function zoomBy(factor: number) {
    setZoom((value) => Math.min(3, Math.max(0.5, value * factor)))
  }

  function fitView() {
    setPan({ x: 0, y: 0 })
    setZoom(1)
  }

  if (edges.length === 0) {
    return (
      <>
        <div className="page-head">
          <h1>Change Graph</h1>
          <p>Files and how they connect.</p>
        </div>
        <Panel title="Repository Graph" hint="Files and relationships">
          <div className="empty-state graph-empty">
            <svg width="72" height="56" viewBox="0 0 72 56" fill="none" aria-hidden="true">
              <circle cx="14" cy="12" r="7" stroke="var(--border)" strokeWidth="2" strokeDasharray="4 3" />
              <circle cx="58" cy="12" r="7" stroke="var(--border)" strokeWidth="2" strokeDasharray="4 3" />
              <circle cx="36" cy="44" r="7" stroke="var(--border)" strokeWidth="2" strokeDasharray="4 3" />
            </svg>
            <p><strong>No connections found yet.</strong></p>
            <p className="history-note" style={{ textAlign: 'center' }}>
              This usually means the repository history is too short or files
              rarely change together. The commit history below still shows
              what changed and when.
            </p>
          </div>
        </Panel>
      </>
    )
  }

  return (
    <>
      <div className="page-head">
        <h1>Change Graph</h1>
        <p>
          {focus
            ? `Centered on ${focus} — dependencies, co-changes, and likely propagation.`
            : 'Start from one file — the graph shows its direct neighborhood, not the whole repository.'}
        </p>
      </div>
      <Panel title="Repository Graph" hint="Files and relationships">
        <div className="legend-swatches" aria-label="Graph legend">
          <span className="legend-sw">
            <svg width="34" height="10" aria-hidden="true">
              <circle cx="5" cy="5" r="4" fill="var(--surface-elev)" stroke="var(--cyan)" strokeWidth="1.5" />
              <line x1="12" y1="5" x2="32" y2="5" stroke="var(--primary)" strokeWidth="2" />
            </svg>
            File <span className="hint">→ dependency</span>
          </span>
          <span className="legend-sw">
            <svg width="34" height="10" aria-hidden="true">
              <line x1="2" y1="5" x2="32" y2="5" stroke="var(--teal)" strokeWidth="3" />
            </svg>
            <span className="hint">━ frequently changed together</span>
          </span>
          <span className="legend-sw">
            <svg width="34" height="10" aria-hidden="true">
              <line x1="2" y1="5" x2="26" y2="5" stroke="var(--warn)" strokeWidth="2" strokeDasharray="5 3" />
              <path d="M26 2l6 3-6 3z" fill="var(--warn)" />
            </svg>
            <span className="hint">⇢ possible change propagation</span>
          </span>
        </div>

        <div className="graph-controls">
          <input
            className="filter-input"
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search files to focus…"
            aria-label="Search files to focus in graph"
          />
          <label className="filter-field">
            Relationship
            <select
              value={edgeFilter}
              onChange={(event) => setEdgeFilter(event.target.value as EdgeFilter)}
              aria-label="Filter graph by relationship type"
            >
              <option value="all">All</option>
              <option value="dependency">Dependency</option>
              <option value="temporal">Temporal</option>
              <option value="cochange">Co-change</option>
            </select>
          </label>
          <label className="filter-field">
            Depth {depth}
            <span className="depth-slider">
              <input
                type="range"
                min={1}
                max={2}
                step={1}
                value={depth}
                onChange={(event) => setDepth(Number(event.target.value))}
                aria-label="Graph neighborhood depth"
              />
            </span>
          </label>
          <button
            type="button"
            className="ghost-button"
            onClick={() => {
              onFocus(null)
              fitView()
            }}
          >
            Reset view
          </button>
        </div>

        {matches.length > 0 && (
          <ul className="graph-matches" role="listbox" aria-label="Matching files">
            {matches.map((file) => (
              <li key={file} role="option" aria-selected={focus === file}>
                <button type="button" className="link-button" onClick={() => onFocus(file)}>
                  {file}
                </button>
              </li>
            ))}
          </ul>
        )}

        {!focus && (
          <div className="quick-pick">
            <span className="quick-pick-label">
              No file selected — start with a highly connected file:
            </span>
            {topFiles.map((file) => (
              <button
                key={file}
                type="button"
                className="quick-chip"
                onClick={() => onFocus(file)}
                title={`Center graph on ${file}`}
              >
                {shortName(file)}
              </button>
            ))}
          </div>
        )}

        <p className="history-note" role="status">
          Showing {data.nodes.length} files · {data.links.length} relationships
          {focus ? ` · focused on ${focus}` : ' · top connected files'}
        </p>
        <p className="history-note" style={{ marginTop: 4 }}>
          How to read this: each circle is a file. A line means the files are
          connected — tap any circle to center it and see why it matters.
        </p>

        <div className="graph-layout">
          <div className="graph-canvas-wrap">
            <div className="graph-zoom" role="group" aria-label="Graph zoom controls">
              <button type="button" onClick={() => zoomBy(1.2)} aria-label="Zoom in">+</button>
              <button type="button" onClick={() => zoomBy(1 / 1.2)} aria-label="Zoom out">−</button>
              <button type="button" onClick={fitView} aria-label="Fit graph to view" style={{ fontSize: 12 }}>Fit</button>
            </div>
            <svg
              className="graph-canvas"
              viewBox={`${-pan.x} ${-pan.y} ${WIDTH / zoom} ${HEIGHT / zoom}`}
              role="img"
              aria-label={focus ? `Relationship graph around ${focus}` : 'Repository relationship graph'}
              onMouseDown={(event) => {
                drag.current = { x: event.clientX, y: event.clientY }
              }}
              onMouseMove={(event) => {
                if (!drag.current) return
                const scale = WIDTH / zoom / event.currentTarget.clientWidth
                setPan((prev) => ({
                  x: prev.x - (event.clientX - drag.current!.x) * scale,
                  y: prev.y - (event.clientY - drag.current!.y) * scale,
                }))
                drag.current = { x: event.clientX, y: event.clientY }
              }}
              onMouseUp={() => {
                drag.current = null
              }}
              onMouseLeave={() => {
                drag.current = null
              }}
              onWheel={(event) => {
                event.preventDefault()
                zoomBy(event.deltaY > 0 ? 0.9 : 1.1)
              }}
            >
              <defs>
                <marker
                  id="graph-arrow"
                  viewBox="0 0 10 10"
                  refX="9"
                  refY="5"
                  markerWidth="7"
                  markerHeight="7"
                  orient="auto-start-reverse"
                >
                  <path d="M0 0L10 5L0 10z" fill="var(--warn)" />
                </marker>
              </defs>
              {data.links.map((link, index) => {
                const a = positions.get(link.source)
                const b = positions.get(link.target)
                if (!a || !b) return null
                const active =
                  focus !== null && (link.source === focus || link.target === focus)
                const kind = link.signals.includes('dependency')
                  ? 'dependency'
                  : link.signals.includes('co-change')
                    ? 'cochange'
                    : link.signals.includes('temporal')
                      ? 'temporal'
                      : 'none'
                const isTemporal = kind === 'temporal'
                return (
                  <line
                    key={`${link.source}-${link.target}-${index}`}
                    x1={a.x}
                    y1={a.y}
                    x2={b.x}
                    y2={b.y}
                    className={`graph-edge${active ? ' active' : ''} edge-${kind}`}
                    strokeWidth={kind === 'cochange' ? 2.5 : 1.5}
                    markerEnd={isTemporal ? 'url(#graph-arrow)' : undefined}
                  />
                )
              })}
              {data.nodes.map((node) => {
                const position = positions.get(node.id)
                if (!position) return null
                const isFocus = node.id === focus
                return (
                  <g key={node.id} transform={`translate(${position.x},${position.y})`}>
                    <title>{`${node.id} · ${node.degree} connections`}</title>
                    {isFocus && <circle r={20} className="graph-halo" />}
                    <circle
                      r={isFocus ? 13 : 5 + Math.min(node.degree, 8)}
                      className={isFocus ? 'graph-node-center' : 'graph-node'}
                      onClick={() => onFocus(node.id)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault()
                          onFocus(node.id)
                        }
                      }}
                      tabIndex={0}
                      role="button"
                      aria-label={`Focus ${node.id}`}
                    />
                    {(isFocus || node.degree > 2) && (
                      <text y={-16} textAnchor="middle" className="graph-label">
                        {shortName(node.id)}
                      </text>
                    )}
                  </g>
                )
              })}
            </svg>
          </div>

          <div className="graph-side">
            <div className="graph-legend" aria-label="Relationship legend">
              <span className="legend-item edge-dependency">dependency</span>
              <span className="legend-item edge-temporal">temporal</span>
              <span className="legend-item edge-cochange">co-change</span>
            </div>
            {selected && counts ? (
              <div className="graph-explain">
                <span className="role">Selected file</span>
                <span className="visually-hidden">Selected: {selected.id}</span>
                <h3>{selected.id}</h3>
                {counts.neighbors.length === 0 ? (
                  <p className="history-note" style={{ marginTop: 8 }}>
                    <strong>No connections detected for this file.</strong>{' '}
                    It hasn&apos;t changed together with other files and has no
                    recorded dependencies. Try one of the suggested files above,
                    or check its history instead.
                  </p>
                ) : (
                <>
                <p className="history-note" style={{ marginTop: 6 }}>
                  Why is this file important? Its direct neighborhood, split by
                  relationship type.
                </p>
                <div className="graph-counts">
                  <div className="graph-count">
                    <div className="n tnum">{counts.dependenciesOut}</div>
                    <div className="l">Dependencies</div>
                  </div>
                  <div className="graph-count">
                    <div className="n tnum">{counts.dependentsIn}</div>
                    <div className="l">Dependents</div>
                  </div>
                  <div className="graph-count">
                    <div className="n tnum">{counts.cochange}</div>
                    <div className="l">Co-changes</div>
                  </div>
                  <div className="graph-count">
                    <div className="n tnum">{counts.temporal}</div>
                    <div className="l">Propagation</div>
                  </div>
                </div>
                <p className="eyebrow">Connected files</p>
                <ul className="graph-neighbors">
                  {counts.neighbors.slice(0, 12).map((item) => (
                    <li key={`${item.id}-${item.direction}`}>
                      <button type="button" className="link-button" onClick={() => onFocus(item.id)}>
                        {item.id}
                      </button>{' '}
                      <span className="muted">· {item.signals.join(' + ')}</span>
                    </li>
                  ))}
                </ul>
                <div className="graph-explain-actions">
                  <button
                    type="button"
                    className="flow-button"
                    onClick={() => (onViewPrediction ?? onOpenFile)(selected.id)}
                  >
                    View prediction →
                  </button>
                  <button
                    type="button"
                    className="flow-ghost"
                    onClick={() => onOpenFile(selected.id)}
                  >
                    Open file details
                  </button>
                  {onGoHistory && (
                    <button
                      type="button"
                      className="flow-ghost"
                      onClick={onGoHistory}
                    >
                      View history
                    </button>
                  )}
                </div>
                </>
                )}
              </div>
            ) : (
              <div className="graph-details">
                <p className="history-note" style={{ margin: 0 }}>
                  Select a node to center it and see why it matters:
                  dependencies, dependents, co-changes, and predicted
                  propagation — or search to focus a file.
                </p>
              </div>
            )}
          </div>
        </div>
      </Panel>
    </>
  )
}
