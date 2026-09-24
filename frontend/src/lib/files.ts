import type {
  HistoricalFeatures,
  RepositoryAnalysis,
  StructuralFeatures,
} from '../types'

export interface FileStats {
  path: string
  changes: number
  complexity: number
  dependencies: number
  size: number
}

export function collectFilePaths(analysis: RepositoryAnalysis): string[] {
  const paths = new Set<string>()
  for (const item of analysis.structural_features ?? []) paths.add(item.file_path)
  for (const item of analysis.hotspots ?? []) paths.add(item.path)
  for (const entry of analysis.timeline.entries ?? []) {
    for (const file of entry.files ?? []) paths.add(file)
  }
  return [...paths].sort((a, b) => a.localeCompare(b))
}

export function buildFileStats(analysis: RepositoryAnalysis): Record<string, FileStats> {
  const structural = new Map<string, StructuralFeatures>()
  for (const item of analysis.structural_features ?? []) {
    structural.set(item.file_path, item)
  }
  const historical = new Map<string, HistoricalFeatures>()
  for (const item of analysis.historical_features ?? []) {
    historical.set(item.file_path, item)
  }
  const stats: Record<string, FileStats> = {}
  for (const path of collectFilePaths(analysis)) {
    stats[path] = {
      path,
      changes: historical.get(path)?.previous_change_count ?? 0,
      complexity: structural.get(path)?.cyclomatic_complexity ?? 0,
      dependencies:
        (structural.get(path)?.incoming_dependencies ?? 0) +
        (structural.get(path)?.outgoing_dependencies ?? 0),
      size: structural.get(path)?.file_size_bytes ?? 0,
    }
  }
  return stats
}

export interface FileTreeNode {
  name: string
  path: string
  children: FileTreeNode[]
  isFile: boolean
}

export function buildFileTree(paths: string[]): FileTreeNode[] {
  const root: FileTreeNode[] = []
  const dirs = new Map<string, FileTreeNode>()
  const byPath = new Map<string, FileTreeNode>()

  const ensureDir = (path: string): FileTreeNode => {
    const existing = byPath.get(path)
    if (existing) return existing
    const slash = path.lastIndexOf('/')
    const name = slash < 0 ? path : path.slice(slash + 1)
    const node: FileTreeNode = { name, path, children: [], isFile: false }
    byPath.set(path, node)
    dirs.set(path, node)
    if (slash < 0) {
      root.push(node)
    } else {
      ensureDir(path.slice(0, slash)).children.push(node)
    }
    return node
  }

  for (const full of paths) {
    const slash = full.lastIndexOf('/')
    const name = slash < 0 ? full : full.slice(slash + 1)
    const node: FileTreeNode = { name, path: full, children: [], isFile: true }
    if (slash < 0) {
      root.push(node)
    } else {
      ensureDir(full.slice(0, slash)).children.push(node)
    }
  }

  const sortTree = (nodes: FileTreeNode[]) => {
    nodes.sort((a, b) => {
      if (a.isFile !== b.isFile) return a.isFile ? 1 : -1
      return a.name.localeCompare(b.name)
    })
    for (const node of nodes) sortTree(node.children)
  }
  sortTree(root)
  return root
}

export function changeCounts(analysis: RepositoryAnalysis): Map<string, number> {
  const counts = new Map<string, number>()
  for (const entry of analysis.timeline.entries ?? []) {
    for (const file of entry.files ?? []) {
      counts.set(file, (counts.get(file) ?? 0) + 1)
    }
  }
  return counts
}

export type FileKind = 'test' | 'config' | 'source'

export function fileKind(path: string): FileKind {
  const lower = path.toLowerCase()
  const segments = lower.split('/')
  const file = segments.pop() ?? lower
  if (
    lower.includes('/test/') ||
    lower.includes('/tests/') ||
    lower.includes('__tests__') ||
    file.includes('.test.') ||
    file.includes('_test.') ||
    file.startsWith('test_') ||
    file === 'conftest.py'
  ) {
    return 'test'
  }
  if (
    file.endsWith('.yml') ||
    file.endsWith('.yaml') ||
    file.endsWith('.toml') ||
    file.endsWith('.ini') ||
    file.startsWith('.env') ||
    lower.includes('/config/') ||
    lower.includes('/settings/') ||
    file === 'config' ||
    file === 'settings'
  ) {
    return 'config'
  }
  return 'source'
}

export interface RelatedFile {
  path: string
  count: number
}

export function cochangePartners(
  analysis: RepositoryAnalysis,
  path: string,
  limit = 5,
): RelatedFile[] {
  const partners: RelatedFile[] = []
  for (const pair of analysis.cochange?.pairs ?? []) {
    if (pair.file_a === path) partners.push({ path: pair.file_b, count: pair.count })
    else if (pair.file_b === path) partners.push({ path: pair.file_a, count: pair.count })
  }
  partners.sort((a, b) => b.count - a.count || a.path.localeCompare(b.path))
  return partners.slice(0, Math.max(limit, 1))
}

export interface DirectoryStats {
  directory: string
  files: number
  lines: number
  changes: number
  contributors: number
  testFiles: number
}

export function directoryStats(analysis: RepositoryAnalysis): DirectoryStats[] {
  const lines = new Map<string, number>()
  for (const item of analysis.structural_features ?? []) {
    lines.set(item.file_path, item.lines_of_code)
  }
  const changes = changeCounts(analysis)
  const authors = new Map<string, Set<string>>()
  for (const entry of analysis.timeline.entries ?? []) {
    if (!entry.author) continue
    for (const file of entry.files ?? []) {
      const slash = file.lastIndexOf('/')
      const dir = slash < 0 ? '(root)' : file.slice(0, slash)
      let set = authors.get(dir)
      if (!set) {
        set = new Set<string>()
        authors.set(dir, set)
      }
      set.add(entry.author)
    }
  }
  const byDir = new Map<string, DirectoryStats>()
  const ensure = (directory: string): DirectoryStats => {
    let stats = byDir.get(directory)
    if (!stats) {
      stats = {
        directory,
        files: 0,
        lines: 0,
        changes: 0,
        contributors: 0,
        testFiles: 0,
      }
      byDir.set(directory, stats)
    }
    return stats
  }
  const paths = new Set<string>()
  for (const item of analysis.structural_features ?? []) paths.add(item.file_path)
  for (const file of changes.keys()) paths.add(file)
  for (const path of paths) {
    const slash = path.lastIndexOf('/')
    const dir = slash < 0 ? '(root)' : path.slice(0, slash)
    const stats = ensure(dir)
    stats.files += 1
    stats.lines += lines.get(path) ?? 0
    stats.changes += changes.get(path) ?? 0
    if (fileKind(path) === 'test') stats.testFiles += 1
  }
  for (const [dir, set] of authors) ensure(dir).contributors = set.size
  return [...byDir.values()].sort((a, b) => b.changes - a.changes || a.directory.localeCompare(b.directory))
}

export type ActivityLevel = 'high' | 'medium' | 'low'

export function activityLevel(changes: number, peak: number): ActivityLevel {
  if (peak <= 0 || changes <= 0) return 'low'
  const ratio = changes / peak
  if (ratio >= 0.6) return 'high'
  if (ratio >= 0.25) return 'medium'
  return 'low'
}
