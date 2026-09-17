export function difficultyColor(level: string): string {
  switch (level.toLowerCase()) {
    case 'low':
      return 'var(--ok)'
    case 'medium':
      return 'var(--warn)'
    case 'high':
      return 'var(--danger)'
    default:
      return 'var(--text-h)'
  }
}

export type Severity = 'low' | 'medium' | 'high'

export function severityOf(score: number): Severity {
  if (score >= 70) return 'high'
  if (score >= 40) return 'medium'
  return 'low'
}

export function severityColor(severity: Severity): string {
  switch (severity) {
    case 'high':
      return 'var(--danger)'
    case 'medium':
      return 'var(--warn)'
    case 'low':
      return 'var(--ok)'
  }
}