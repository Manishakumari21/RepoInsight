import { describe, expect, it } from 'vitest'
import { buildFileTree, collectFilePaths } from '../lib/files'
import type { RepositoryAnalysis } from '../types'

function analysisWith(paths: { structural: string[]; hotspots: string[]; timeline: string[][] }): RepositoryAnalysis {
  return {
    structural_features: paths.structural.map((file_path) => ({ file_path })) as never,
    historical_features: [],
    temporal_features: [],
    hotspots: paths.hotspots.map((path) => ({ path })) as never,
    timeline: {
      entries: paths.timeline.map((files, index) => ({
        sha: `sha${index}`,
        order: index,
        timestamp: index,
        date: null,
        author: null,
        message: '',
        files,
        additions: 0,
        deletions: 0,
      })),
      total_commits: paths.timeline.length,
      truncated: false,
    },
  } as unknown as RepositoryAnalysis
}

describe('collectFilePaths', () => {
  it('unions sources deterministically', () => {
    const analysis = analysisWith({
      structural: ['b.ts', 'a.ts'],
      hotspots: ['c.ts'],
      timeline: [['d.ts', 'a.ts']],
    })
    expect(collectFilePaths(analysis)).toEqual(['a.ts', 'b.ts', 'c.ts', 'd.ts'])
  })
})

describe('buildFileTree', () => {
  it('nests directories with files sorted after directories', () => {
    const tree = buildFileTree(['backend/src/main.ts', 'backend/src/auth.ts', 'README.md'])
    expect(tree.map((n) => n.name)).toEqual(['backend', 'README.md'])
    const backend = tree[0]
    expect(backend.isFile).toBe(false)
    expect(backend.children.map((n) => n.name)).toEqual(['src'])
    expect(backend.children[0].children.map((n) => n.name)).toEqual([
      'auth.ts',
      'main.ts',
    ])
  })

  it('handles an empty repository', () => {
    expect(buildFileTree([])).toEqual([])
  })
})
