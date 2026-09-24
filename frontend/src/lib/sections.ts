export type ViewId =
  | 'overview'
  | 'structure'
  | 'dependencies'
  | 'timeline'
  | 'coupling'
  | 'predictions'
  | 'evidence'

export interface NavItem {
  id: ViewId
  label: string
  group: string
  tooltip: string
}

export const NAV_ITEMS: NavItem[] = [
  {
    id: 'overview',
    label: 'Overview',
    group: 'Overview',
    tooltip: 'Repository briefing: activity, areas, and what deserves attention.',
  },
  {
    id: 'structure',
    label: 'Structure',
    group: 'Repository',
    tooltip: 'Explore how the repository is organized: files, modules, dependencies.',
  },
  {
    id: 'dependencies',
    label: 'Dependencies',
    group: 'Repository',
    tooltip: 'Structural links between files, such as imports.',
  },
  {
    id: 'timeline',
    label: 'Timeline',
    group: 'History',
    tooltip: 'Commit activity over time, filterable by author, directory, and change type.',
  },
  {
    id: 'coupling',
    label: 'Coupling',
    group: 'Coupling',
    tooltip: 'Find files that repeatedly change together.',
  },
  {
    id: 'predictions',
    label: 'Predictions',
    group: 'Prediction',
    tooltip: 'See what files are likely to change next, and why.',
  },
  {
    id: 'evidence',
    label: 'Evidence',
    group: 'Explanation',
    tooltip: 'Understand why RepoInsight made a prediction.',
  },
]

export type PredictTab = 'predicted' | 'related' | 'impact'

export const PREDICT_TABS: { id: PredictTab; label: string }[] = [
  { id: 'predicted', label: 'Predicted changes' },
  { id: 'related', label: 'Related files' },
  { id: 'impact', label: 'Impact simulator' },
]
