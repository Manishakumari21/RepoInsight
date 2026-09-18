export interface RepositoryAnalysis {
  repository: RepositoryInfo
  source: SourceAnalysis
  complexity: ComplexityAnalysis
  history: HistoryAnalysis
  dependencies: DependencyAnalysis
  cochange: CochangeAnalysis
  temporal: TemporalAnalysis
  propagation: PropagationAnalysis
  timeline: ChangeTimeline
  sequences: ChangeSequences
  followups: FollowUpAnalysis
  rework: ReworkAnalysis
  examples: HistoricalExamples
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

export interface TimelineEntry {
  sha: string
  order: number
  timestamp: number | null
  date: string | null
  author: string | null
  message: string
  files: string[]
  additions: number
  deletions: number
}

export interface ChangeTimeline {
  entries: TimelineEntry[]
  total_commits: number
  truncated: boolean
}

export interface ChangeSequence {
  files: string[]
  occurrences: number
  avg_delay_seconds: number
  kind: string
  evidence_type: string
}

export interface ChangeSequences {
  sequences: ChangeSequence[]
  window_seconds: number
}

export interface FollowUp {
  source_sha: string
  source_files: string[]
  followup_sha: string
  followup_files: string[]
  delay_seconds: number
  reason: string
  signals: string[]
  evidence_type: string
}

export interface FollowUpAnalysis {
  followups: FollowUp[]
  total: number
  window_seconds: number
}

export interface ReworkEvent {
  file: string
  initial_sha: string
  rework_sha: string
  delay_seconds: number
  rule: string
  evidence: string
}

export interface ReworkAnalysis {
  events: ReworkEvent[]
  total: number
  window_seconds: number
}

export interface HistoricalExample {
  id: string
  source_file: string
  target_file: string
  sequence: string[]
  occurrences: number
  avg_delay_seconds: number
  signals: string[]
  evidence_type: string
  example_shas: string[]
}

export interface HistoricalExamples {
  examples: HistoricalExample[]
  total: number
}

export interface ReworkEvaluationMetrics {
  tp: number
  fp: number
  tn: number
  fn: number
  precision: number | null
  recall: number | null
  false_positive_rate: number | null
}

export interface ReworkEvaluatedPair {
  previous_sha: string
  current_sha: string
  actual_rework: boolean
  predicted_rework: boolean
  rule: string | null
}

export interface ReworkEvaluationReport {
  dataset: string
  pairs: ReworkEvaluatedPair[]
  metrics: ReworkEvaluationMetrics
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

export function formatDelay(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`
  if (seconds < 86400) return `${(seconds / 3600).toFixed(1)}h`
  return `${(seconds / 86400).toFixed(1)}d`
}

export function shortSha(sha: string): string {
  return sha.slice(0, 7)
}