import { describe, expect, it } from 'vitest'
import { buildFileTree, cochangePartners, collectFilePaths, fileKind } from '../lib/files'
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

describe('fileKind', () => {
  it('classifies test, config, and source paths', () => {
    expect(fileKind('tests/auth.test.ts')).toBe('test')
    expect(fileKind('src/__tests__/auth.ts')).toBe('test')
    expect(fileKind('src/auth_test.py')).toBe('test')
    expect(fileKind('config/auth.yml')).toBe('config')
    expect(fileKind('.env.example')).toBe('config')
    expect(fileKind('src/auth.ts')).toBe('source')
  })
})

describe('cochangePartners', () => {
  it('ranks partners by co-change count', () => {
    const analysis = {
      cochange: {
        pairs: [
          { file_a: 'a.ts', file_b: 'b.ts', count: 3 },
          { file_a: 'c.ts', file_b: 'a.ts', count: 7 },
          { file_a: 'a.ts', file_b: 'd.ts', count: 1 },
        ],
        total_pairs: 3,
      },
    } as unknown as RepositoryAnalysis
    expect(cochangePartners(analysis, 'a.ts', 5)).toEqual([
      { path: 'c.ts', count: 7 },
      { path: 'b.ts', count: 3 },
      { path: 'd.ts', count: 1 },
    ])
    expect(cochangePartners(analysis, 'missing.ts')).toEqual([])
  })
})
