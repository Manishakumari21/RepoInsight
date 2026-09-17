export interface RepositoryAnalysis {
  repository: RepositoryInfo
  source: SourceAnalysis
  complexity: ComplexityAnalysis
  history: HistoryAnalysis
  dependencies: DependencyAnalysis
  cochange: CochangeAnalysis
  temporal: TemporalAnalysis
  propagation: PropagationAnalysis
  hotspots: Hotspot[]
  difficulty: DifficultyScore
}

export interface RepositoryInfo {
  name: string
  default_branch: string
  total_files: number
}

export interface SourceAnalysis {
  source_files: number
  total_lines: number
  total_size_bytes: number
}

export interface ComplexityAnalysis {
  average_complexity: number
  max_complexity: number
  complex_files: number
}

export interface HistoryAnalysis {
  total_commits: number
  active_contributors: number
  changed_files: number
  total_additions: number
  total_deletions: number
  total_churn: number
  first_commit: string | null
  last_commit: string | null
}

export interface DependencyAnalysis {
  total_dependencies: number
  connected_files: number
  highly_connected_files: number
  resolved_dependencies: number
  top_coupled_files: CoupledFile[]
}

export interface CoupledFile {
  path: string
  coupling: number
}

export interface CochangeAnalysis {
  pairs: CochangePair[]
  total_pairs: number
}

export interface CochangePair {
  file_a: string
  file_b: string
  count: number
}

export interface TemporalAnalysis {
  pairs: TemporalPair[]
}

export interface TemporalPair {
  source: string
  target: string
  occurrences: number
  avg_delay_seconds: number
}

export interface PropagationAnalysis {
  edges: PropagationEdge[]
}

export interface PropagationEdge {
  source: string
  target: string
  dependency: boolean
  temporal: boolean
  cochange: boolean
  strength: number
}

export interface Hotspot {
  path: string
  score: number
  reasons: string[]
}

export interface DifficultyScore {
  score: number
  level: string
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export function formatNumber(value: number): string {
  return value.toLocaleString('en-US')
}

export function formatDate(iso: string | null): string {
  if (!iso) return '—'

  const date = new Date(iso)

  return Number.isNaN(date.getTime())
    ? iso
    : date.toLocaleDateString('en-US')
}