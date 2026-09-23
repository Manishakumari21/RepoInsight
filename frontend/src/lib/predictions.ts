import type { FilePrediction } from '../types'

export type LabelFilter = 'all' | 'positive' | 'negative'
export type BandFilter = 'all' | 'high' | 'medium' | 'low'
export type SortKey =
  | 'probability'
  | 'file'
  | 'changes'
  | 'complexity'
  | 'dependencies'
export type SortDir = 'asc' | 'desc'

export interface PredictionFilters {
  query: string
  label: LabelFilter
  band: BandFilter
  model: string
}


export function probabilityBand(probability: number): 'high' | 'medium' | 'low' {
  const strength = Math.max(probability, 1 - probability)
  if (strength >= 0.8) return 'high'
  if (strength >= 0.6) return 'medium'
  return 'low'
}

export function filterPredictions(
  predictions: FilePrediction[],
  filters: PredictionFilters,
  modelName: string,
): FilePrediction[] {
  const query = filters.query.trim().toLowerCase()
  return predictions.filter((item) => {
    if (filters.model !== 'all' && filters.model !== modelName) return false
    if (filters.label === 'positive' && item.label !== 1) return false
    if (filters.label === 'negative' && item.label !== 0) return false
    if (filters.band !== 'all' && probabilityBand(item.probability) !== filters.band) {
      return false
    }
    if (query && !item.file_path.toLowerCase().includes(query)) return false
    return true
  })
}

export function sortPredictions(
  predictions: FilePrediction[],
  key: SortKey,
  dir: SortDir,
  stats: Record<string, { changes: number; complexity: number; dependencies: number }>,
): FilePrediction[] {
  const ordered = [...predictions]
  const factor = dir === 'asc' ? 1 : -1
  ordered.sort((a, b) => {
    switch (key) {
      case 'probability':
        return (a.probability - b.probability) * factor || a.file_path.localeCompare(b.file_path)
      case 'file':
        return a.file_path.localeCompare(b.file_path) * factor
      case 'changes':
      case 'complexity':
      case 'dependencies': {
        const left = stats[a.file_path]?.[key] ?? 0
        const right = stats[b.file_path]?.[key] ?? 0
        return (left - right) * factor || a.file_path.localeCompare(b.file_path)
      }
    }
  })
  return ordered
}

export function riskDistribution(predictions: FilePrediction[]): {
  high: number
  medium: number
  low: number
  positive: number
} {
  let high = 0
  let medium = 0
  let low = 0
  let positive = 0
  for (const item of predictions) {
    const band = probabilityBand(item.probability)
    if (band === 'high') high += 1
    else if (band === 'medium') medium += 1
    else low += 1
    if (item.label === 1) positive += 1
  }
  return { high, medium, low, positive }
}
