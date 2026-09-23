export type DashboardSection = 'overview' | 'explore' | 'predict' | 'history'

export const SECTIONS: { id: DashboardSection; label: string }[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'explore', label: 'Explore' },
  { id: 'predict', label: 'Predict' },
  { id: 'history', label: 'History' },
]

export type ExploreTab = 'graph' | 'files' | 'dependencies'
export type PredictTab = 'predictions' | 'ripple' | 'simulator'
export type HistoryTab = 'timeline' | 'patterns'

export const EXPLORE_TABS: { id: ExploreTab; label: string }[] = [
  { id: 'graph', label: 'Graph' },
  { id: 'files', label: 'Files' },
  { id: 'dependencies', label: 'Dependencies' },
]

export const PREDICT_TABS: { id: PredictTab; label: string }[] = [
  { id: 'predictions', label: 'Predictions' },
  { id: 'ripple', label: 'Ripple' },
  { id: 'simulator', label: 'Simulator' },
]

export const HISTORY_TABS: { id: HistoryTab; label: string }[] = [
  { id: 'timeline', label: 'Timeline' },
  { id: 'patterns', label: 'Patterns' },
]
