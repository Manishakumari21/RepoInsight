import type { PropagationEdge } from '../types'

export interface GraphNode {
  id: string
  degree: number
}

export interface GraphLink {
  source: string
  target: string
  signals: string[]
  strength: number
}

export interface GraphData {
  nodes: GraphNode[]
  links: GraphLink[]
}

export type EdgeFilter = 'all' | 'dependency' | 'temporal' | 'cochange'

export function edgeMatches(edge: PropagationEdge, filter: EdgeFilter): boolean {
  switch (filter) {
    case 'dependency':
      return edge.dependency
    case 'temporal':
      return edge.temporal
    case 'cochange':
      return edge.cochange
    case 'all':
      return true
  }
}

/**
 * Focused neighborhood around `focus`: the focus node plus nodes reachable
 * within `depth` hops. Bounded by `maxNodes` (highest-strength links win)
 * so large repositories stay usable.
 */
export function buildNeighborhood(
  edges: PropagationEdge[],
  focus: string | null,
  allNodes: string[],
  depth: number,
  filter: EdgeFilter,
  maxNodes: number,
): GraphData {
  const relevant = edges.filter((edge) => edgeMatches(edge, filter))
  if (focus === null) {
    const degree = new Map<string, number>()
    for (const edge of relevant) {
      degree.set(edge.source, (degree.get(edge.source) ?? 0) + 1)
      degree.set(edge.target, (degree.get(edge.target) ?? 0) + 1)
    }
    const top = [...degree.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .slice(0, Math.max(maxNodes, 1))
      .map(([id]) => id)
    return materialize(relevant, new Set(top))
  }

  const adjacency = new Map<string, { to: string; strength: number }[]>()
  const link = (from: string, to: string, strength: number) => {
    const list = adjacency.get(from) ?? []
    list.push({ to, strength })
    adjacency.set(from, list)
  }
  for (const edge of relevant) {
    link(edge.source, edge.target, edge.strength)
    link(edge.target, edge.source, edge.strength)
  }

  const visited = new Set<string>([focus])
  let frontier = [focus]
  for (let hop = 0; hop < Math.max(depth, 1); hop += 1) {
    const next: string[] = []
    for (const node of frontier) {
      const neighbors = (adjacency.get(node) ?? [])
        .filter((item) => !visited.has(item.to))
        .sort((a, b) => b.strength - a.strength || a.to.localeCompare(b.to))
      for (const item of neighbors) {
        if (visited.size >= Math.max(maxNodes, 1)) break
        visited.add(item.to)
        next.push(item.to)
      }
      if (visited.size >= Math.max(maxNodes, 1)) break
    }
    frontier = next
    if (frontier.length === 0) break
  }
  void allNodes
  return materialize(relevant, visited)
}

function materialize(edges: PropagationEdge[], keep: Set<string>): GraphData {
  const degree = new Map<string, number>()
  const links: GraphLink[] = []
  for (const edge of edges) {
    if (!keep.has(edge.source) || !keep.has(edge.target)) continue
    if (edge.source === edge.target) continue
    degree.set(edge.source, (degree.get(edge.source) ?? 0) + 1)
    degree.set(edge.target, (degree.get(edge.target) ?? 0) + 1)
    const signals: string[] = []
    if (edge.dependency) signals.push('dependency')
    if (edge.temporal) signals.push('temporal')
    if (edge.cochange) signals.push('co-change')
    links.push({
      source: edge.source,
      target: edge.target,
      signals,
      strength: edge.strength,
    })
  }
  const nodes: GraphNode[] = [...keep]
    .filter((id) => (degree.get(id) ?? 0) > 0 || keep.size === 1)
    .map((id) => ({ id, degree: degree.get(id) ?? 0 }))
    .sort((a, b) => b.degree - a.degree || a.id.localeCompare(b.id))
  return { nodes, links }
}

/** Circular layout positions, deterministic for a given node order. */
export function layoutCircle(
  nodes: GraphNode[],
  width: number,
  height: number,
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>()
  const cx = width / 2
  const cy = height / 2
  const radius = Math.max(Math.min(width, height) / 2 - 40, 20)
  if (nodes.length === 1) {
    positions.set(nodes[0].id, { x: cx, y: cy })
    return positions
  }
  nodes.forEach((node, index) => {
    const angle = (2 * Math.PI * index) / nodes.length - Math.PI / 2
    positions.set(node.id, {
      x: cx + radius * Math.cos(angle),
      y: cy + radius * Math.sin(angle),
    })
  })
  return positions
}

/**
 * Center-node layout: `focus` sits in the middle, its neighbors form a
 * ring around it. Makes the selected file the visual center of the graph.
 */
export function layoutRadial(
  nodes: GraphNode[],
  links: GraphLink[],
  focus: string,
  width: number,
  height: number,
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>()
  const cx = width / 2
  const cy = height / 2
  positions.set(focus, { x: cx, y: cy })
  const neighborIds = new Set<string>()
  for (const link of links) {
    if (link.source === focus) neighborIds.add(link.target)
    else if (link.target === focus) neighborIds.add(link.source)
  }
  const ring = [...neighborIds].sort((a, b) => a.localeCompare(b))
  const radius = Math.max(Math.min(width, height) / 2 - 56, 40)
  ring.forEach((id, index) => {
    const angle = (2 * Math.PI * index) / ring.length - Math.PI / 2
    positions.set(id, {
      x: cx + radius * Math.cos(angle),
      y: cy + radius * Math.sin(angle),
    })
  })
  // Second-hop leftovers share the outer area deterministically.
  const rest = nodes
    .map((node) => node.id)
    .filter((id) => !positions.has(id))
    .sort((a, b) => a.localeCompare(b))
  rest.forEach((id, index) => {
    const angle = (2 * Math.PI * index) / rest.length - Math.PI / 2
    positions.set(id, {
      x: cx + (radius + 56) * Math.cos(angle),
      y: cy + (radius + 56) * Math.sin(angle),
    })
  })
  return positions
}

export interface NodeSignalCounts {
  dependenciesOut: number
  dependentsIn: number
  cochange: number
  temporal: number
  neighbors: { id: string; signals: string[]; direction: 'out' | 'in' }[]
}

/** Per-signal neighbor breakdown for one node, from directed edges. */
export function nodeSignalCounts(
  edges: PropagationEdge[],
  node: string,
  filter: EdgeFilter,
): NodeSignalCounts {
  const counts: NodeSignalCounts = {
    dependenciesOut: 0,
    dependentsIn: 0,
    cochange: 0,
    temporal: 0,
    neighbors: [],
  }
  const seen = new Set<string>()
  for (const edge of edges) {
    if (!edgeMatches(edge, filter)) continue
    const isSource = edge.source === node
    const isTarget = edge.target === node
    if (!isSource && !isTarget) continue
    const other = isSource ? edge.target : edge.source
    const direction = isSource ? 'out' : 'in'
    if (edge.dependency) {
      if (isSource) counts.dependenciesOut += 1
      else counts.dependentsIn += 1
    }
    if (edge.cochange) counts.cochange += 1
    if (edge.temporal) counts.temporal += 1
    const key = `${other}|${direction}`
    if (!seen.has(key)) {
      seen.add(key)
      const signals: string[] = []
      if (edge.dependency) signals.push('dependency')
      if (edge.temporal) signals.push('temporal')
      if (edge.cochange) signals.push('co-change')
      counts.neighbors.push({ id: other, signals, direction })
    }
  }
  counts.neighbors.sort((a, b) => a.id.localeCompare(b.id))
  return counts
}
