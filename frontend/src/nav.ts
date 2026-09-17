export type SectionId =
  | 'overview'
  | 'files'
  | 'hotspots'
  | 'dependencies'
  | 'history'
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
  { id: 'settings', label: 'Settings' },
]