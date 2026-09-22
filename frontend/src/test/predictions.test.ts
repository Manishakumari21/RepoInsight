import { describe, expect, it } from 'vitest'
import {
  filterPredictions,
  probabilityBand,
  riskDistribution,
  sortPredictions,
} from '../lib/predictions'
import { makePrediction } from './fixtures'

const predictions = [
  makePrediction('src/auth.ts', 0.78),
  makePrediction('src/user.ts', 0.64),
  makePrediction('src/api.ts', 0.12),
  makePrediction('src/main.ts', 0.9),
]

const stats = {
  'src/auth.ts': { changes: 5, complexity: 9, dependencies: 3 },
  'src/user.ts': { changes: 2, complexity: 4, dependencies: 1 },
  'src/api.ts': { changes: 8, complexity: 2, dependencies: 6 },
  'src/main.ts': { changes: 1, complexity: 1, dependencies: 0 },
}

describe('probabilityBand', () => {
  it('buckets strength with documented thresholds', () => {
    expect(probabilityBand(0.9)).toBe('high')
    expect(probabilityBand(0.7)).toBe('medium')
    expect(probabilityBand(0.55)).toBe('low')
    expect(probabilityBand(0.1)).toBe('high')
  })
})

describe('filterPredictions', () => {
  it('filters by label', () => {
    const out = filterPredictions(
      predictions,
      { query: '', label: 'positive', band: 'all', model: 'all' },
      'logistic_regression',
    )
    expect(out.map((p) => p.file_path).sort()).toEqual([
      'src/auth.ts',
      'src/main.ts',
      'src/user.ts',
    ])
  })

  it('filters by band and search query', () => {
    const out = filterPredictions(
      predictions,
      { query: 'auth', label: 'all', band: 'medium', model: 'all' },
      'logistic_regression',
    )
    expect(out.map((p) => p.file_path)).toEqual(['src/auth.ts'])
  })

  it('respects the model filter', () => {
    const out = filterPredictions(
      predictions,
      { query: '', label: 'all', band: 'all', model: 'other' },
      'logistic_regression',
    )
    expect(out).toEqual([])
  })
})

describe('sortPredictions', () => {
  it('sorts by probability descending with filename tiebreak', () => {
    const out = sortPredictions(predictions, 'probability', 'desc', stats)
    expect(out.map((p) => p.file_path)).toEqual([
      'src/main.ts',
      'src/auth.ts',
      'src/user.ts',
      'src/api.ts',
    ])
  })

  it('sorts by stats keys', () => {
    const out = sortPredictions(predictions, 'changes', 'desc', stats)
    expect(out[0].file_path).toBe('src/api.ts')
    const byName = sortPredictions(predictions, 'file', 'asc', stats)
    expect(byName.map((p) => p.file_path)).toEqual([
      'src/api.ts',
      'src/auth.ts',
      'src/main.ts',
      'src/user.ts',
    ])
  })
})

describe('riskDistribution', () => {
  it('counts bands and positives', () => {
    expect(riskDistribution(predictions)).toEqual({
      high: 2,
      medium: 2,
      low: 0,
      positive: 3,
    })
  })
})
