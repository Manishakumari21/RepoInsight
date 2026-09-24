import { probabilityBand } from './predictions'

export type PredictionLevel = 'Likely' | 'Possible' | 'Low likelihood'
export type CouplingStrength = 'Strong' | 'Moderate' | 'Weak'

export function predictionLevel(probability: number, label: number): PredictionLevel {
  if (label !== 1) return 'Low likelihood'
  const band = probabilityBand(probability)
  if (band === 'high') return 'Likely'
  if (band === 'medium') return 'Possible'
  return 'Low likelihood'
}

export function predictionLevelClass(level: PredictionLevel): string {
  if (level === 'Likely') return 'likely'
  if (level === 'Possible') return 'possible'
  return 'low'
}

export function couplingStrengthClass(strength: CouplingStrength): string {
  return strength.toLowerCase()
}

export function trendClass(trend: string): string {
  if (trend === 'increasing') return 'increasing'
  if (trend === 'decreasing') return 'low'
  if (trend === 'stable') return 'stable'
  return 'weak'
}
