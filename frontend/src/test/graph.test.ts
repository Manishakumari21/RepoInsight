import { describe, expect, it } from 'vitest'
import {
  buildNeighborhood,
  edgeMatches,
  layoutCircle,
  layoutRadial,
  nodeSignalCounts,
} from '../lib/graph'
import type { PropagationEdge } from '../types'

const edges: PropagationEdge[] = [
  { source: 'a.ts', target: 'b.ts', dependency: true, temporal: false, cochange: false, strength: 5 },
  { source: 'b.ts', target: 'c.ts', dependency: false, temporal: true, cochange: false, strength: 2 },
  { source: 'a.ts', target: 'c.ts', dependency: false, temporal: false, cochange: true, strength: 3 },
  { source: 'c.ts', target: 'd.ts', dependency: true, temporal: true, cochange: true, strength: 1 },
]

describe('edgeMatches', () => {
  it('filters by relationship type', () => {
    expect(edges.filter((e) => edgeMatches(e, 'dependency')).length).toBe(2)
    expect(edges.filter((e) => edgeMatches(e, 'temporal')).length).toBe(2)
    expect(edges.filter((e) => edgeMatches(e, 'cochange')).length).toBe(2)
    expect(edges.filter((e) => edgeMatches(e, 'all')).length).toBe(4)
  })

  it('filters prediction edges to the predicted set', () => {
    const predicted = new Set(['a.ts'])
    expect(edges.filter((e) => edgeMatches(e, 'prediction', predicted)).length).toBe(2)
    expect(edges.filter((e) => edgeMatches(e, 'prediction', new Set()))).toEqual([])
    expect(edges.filter((e) => edgeMatches(e, 'prediction'))).toEqual([])
  })

  it('filters test and config files by path kind', () => {
    const kindEdges: PropagationEdge[] = [
      { source: 'src/auth.ts', target: 'tests/auth.test.ts', dependency: true, temporal: false, cochange: false, strength: 2 },
      { source: 'src/auth.ts', target: 'config/auth.yml', dependency: true, temporal: false, cochange: false, strength: 1 },
      { source: 'src/a.ts', target: 'src/b.ts', dependency: true, temporal: false, cochange: false, strength: 1 },
    ]
    expect(kindEdges.filter((e) => edgeMatches(e, 'tests')).length).toBe(1)
    expect(kindEdges.filter((e) => edgeMatches(e, 'config')).length).toBe(1)
    expect(kindEdges.filter((e) => edgeMatches(e, 'all')).length).toBe(3)
  })
})

describe('buildNeighborhood', () => {
  it('returns depth-1 neighbors of the focus file', () => {
    const data = buildNeighborhood(edges, 'a.ts', [], 1, 'all', 60)
    expect(data.nodes.map((n) => n.id).sort()).toEqual(['a.ts', 'b.ts', 'c.ts'])
    expect(data.links.length).toBe(3)
  })

  it('expands to depth 2', () => {
    const data = buildNeighborhood(edges, 'a.ts', [], 2, 'all', 60)
    expect(data.nodes.map((n) => n.id).sort()).toEqual(['a.ts', 'b.ts', 'c.ts', 'd.ts'])
  })

  it('respects edge-type filtering', () => {
    const data = buildNeighborhood(edges, 'a.ts', [], 2, 'dependency', 60)
    expect(data.nodes.map((n) => n.id).sort()).toEqual(['a.ts', 'b.ts'])
  })

  it('caps nodes and falls back to top-connected without focus', () => {
    const capped = buildNeighborhood(edges, 'a.ts', [], 2, 'all', 2)
    expect(capped.nodes.length).toBeLessThanOrEqual(2)
    expect(capped.nodes[0].id).toBe('a.ts')
    const top = buildNeighborhood(edges, null, [], 1, 'all', 2)
    expect(top.nodes.length).toBe(2)
  })

  it('is deterministic', () => {
    const first = buildNeighborhood(edges, 'b.ts', [], 2, 'all', 60)
    const second = buildNeighborhood(edges, 'b.ts', [], 2, 'all', 60)
    expect(first).toEqual(second)
  })
})

describe('layoutRadial', () => {
  it('centers the focus node with neighbors on a ring', () => {
    const data = buildNeighborhood(edges, 'a.ts', [], 1, 'all', 60)
    const positions = layoutRadial(data.nodes, data.links, 'a.ts', 760, 520)
    expect(positions.get('a.ts')).toEqual({ x: 380, y: 260 })
    expect(positions.get('b.ts')).not.toEqual({ x: 380, y: 260 })
    expect(positions.get('c.ts')).not.toEqual({ x: 380, y: 260 })
  })

  it('is deterministic', () => {
    const data = buildNeighborhood(edges, 'b.ts', [], 2, 'all', 60)
    const first = layoutRadial(data.nodes, data.links, 'b.ts', 760, 520)
    const second = layoutRadial(data.nodes, data.links, 'b.ts', 760, 520)
    expect(first).toEqual(second)
  })
})

describe('nodeSignalCounts', () => {
  it('splits dependency direction and counts signals', () => {
    const counts = nodeSignalCounts(edges, 'b.ts', 'all')

    expect(counts.dependenciesOut).toBe(0)
    expect(counts.dependentsIn).toBe(1)
    expect(counts.temporal).toBe(1)
    expect(counts.cochange).toBe(0)
    expect(counts.neighbors.map((n) => n.id)).toEqual(['a.ts', 'c.ts'])
  })

  it('respects the edge filter', () => {
    const counts = nodeSignalCounts(edges, 'c.ts', 'dependency')

    expect(counts.dependenciesOut).toBe(1)
    expect(counts.dependentsIn).toBe(0)
    expect(counts.temporal).toBe(1)
    expect(counts.cochange).toBe(1)
    expect(counts.neighbors.map((n) => n.id)).toEqual(['d.ts'])
  })
})
describe('layoutCircle', () => {
  it('places a single node in the center', () => {
    const positions = layoutCircle([{ id: 'a.ts', degree: 0 }], 720, 460)
    expect(positions.get('a.ts')).toEqual({ x: 360, y: 230 })
  })

  it('places nodes deterministically', () => {
    const nodes = [
      { id: 'a.ts', degree: 2 },
      { id: 'b.ts', degree: 1 },
    ]
    expect(layoutCircle(nodes, 720, 460)).toEqual(layoutCircle(nodes, 720, 460))
  })
})
