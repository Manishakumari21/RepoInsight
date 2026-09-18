export type SectionId =
  | 'overview'
  | 'files'
  | 'hotspots'
  | 'dependencies'
  | 'history'
  | 'sequences'
  | 'propagation'
  | 'examples'
  | 'settings'

export interface NavItem {
  id: SectionId
  label: string
}

export const NAV_SECTIONS: NavItem[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'files', label: 'Files' },
  { id: 'hotspots', label: 'Hotspots' },
  { id: 'dependencies', label: 'Dependencies' },
  { id: 'history', label: 'History' },
  { id: 'sequences', label: 'Sequences' },
  { id: 'propagation', label: 'Propagation' },
  { id: 'examples', label: 'Examples' },
  { id: 'settings', label: 'Settings' },
]