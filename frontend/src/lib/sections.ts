export type DashboardSection =
  | 'overview'
  | 'predictions'
  | 'graph'
  | 'files'
  | 'history'

export const SECTIONS: { id: DashboardSection; label: string }[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'predictions', label: 'Predictions' },
  { id: 'graph', label: 'Graph' },
  { id: 'files', label: 'Files' },
  { id: 'history', label: 'History' },
]
