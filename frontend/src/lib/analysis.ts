import type { ChangeTimeline } from '../types'
import {
  addWeeks,
  differenceInCalendarWeeks,
  fromUnixTime,
  startOfWeek,
} from 'date-fns'

export function weeklyBuckets(entries: ChangeTimeline['entries']): number[] {
  const dates: Date[] = []
  for (const entry of entries) {
    if (typeof entry.timestamp === 'number') {
      dates.push(fromUnixTime(entry.timestamp))
    } else if (entry.date) {
      const parsed = Date.parse(entry.date)
      if (Number.isFinite(parsed)) dates.push(new Date(parsed))
    }
  }
  if (dates.length === 0) return []
  dates.sort((a, b) => a.getTime() - b.getTime())
  const start = startOfWeek(dates[0], { weekStartsOn: 1 })
  const last = dates[dates.length - 1]
  const weekCount = differenceInCalendarWeeks(last, start, {
    weekStartsOn: 1,
  }) + 1

  const weekStarts = Array.from({ length: weekCount }, (_, index) =>
    addWeeks(start, index),
  )
  void weekStarts
  const buckets = new Array<number>(Math.max(weekCount, 1)).fill(0)
  for (const date of dates) {
    const index = Math.min(
      differenceInCalendarWeeks(date, start, { weekStartsOn: 1 }),
      buckets.length - 1,
    )
    buckets[Math.max(index, 0)] += 1
  }
  while (buckets.length > 26) {
    const merged: number[] = []
    for (let index = 0; index < buckets.length; index += 2) {
      merged.push(buckets[index] + (buckets[index + 1] ?? 0))
    }
    buckets.splice(0, buckets.length, ...merged)
  }
  return buckets
}

export interface TimelineBucket {
  start: Date
  label: string
  commits: number
  files: number
  additions: number
  deletions: number
}

function entryDate(entry: {
  timestamp: number | null
  date: string | null
}): Date | null {
  if (typeof entry.timestamp === 'number') {
    return fromUnixTime(entry.timestamp)
  }
  if (entry.date) {
    const parsed = Date.parse(entry.date)
    if (Number.isFinite(parsed)) return new Date(parsed)
  }
  return null
}

export function bucketSeries(
  entries: ChangeTimeline['entries'],
): TimelineBucket[] {
  const dated = entries
    .map((entry) => ({ entry, date: entryDate(entry) }))
    .filter(
      (item): item is { entry: ChangeTimeline['entries'][number]; date: Date } =>
        item.date !== null,
    )
  if (dated.length === 0) return []
  dated.sort((a, b) => a.date.getTime() - b.date.getTime())
  const start = startOfWeek(dated[0].date, { weekStartsOn: 1 })
  const last = dated[dated.length - 1].date
  let weekCount =
    differenceInCalendarWeeks(last, start, { weekStartsOn: 1 }) + 1
  let buckets: TimelineBucket[] = Array.from(
    { length: Math.max(weekCount, 1) },
    (_, index) => {
      const weekStart = addWeeks(start, index)
      return {
        start: weekStart,
        label: `${weekStart.getMonth() + 1}/${weekStart.getDate()}`,
        commits: 0,
        files: 0,
        additions: 0,
        deletions: 0,
      }
    },
  )
  for (const { entry, date } of dated) {
    const index = Math.min(
      Math.max(differenceInCalendarWeeks(date, start, { weekStartsOn: 1 }), 0),
      buckets.length - 1,
    )
    const bucket = buckets[index]
    bucket.commits += 1
    bucket.files += entry.files.length
    bucket.additions += entry.additions
    bucket.deletions += entry.deletions
  }
  while (buckets.length > 26) {
    const merged: TimelineBucket[] = []
    for (let index = 0; index < buckets.length; index += 2) {
      const first = buckets[index]
      const second = buckets[index + 1]
      merged.push(
        second
          ? {
              start: first.start,
              label: first.label,
              commits: first.commits + second.commits,
              files: first.files + second.files,
              additions: first.additions + second.additions,
              deletions: first.deletions + second.deletions,
            }
          : first,
      )
    }
    buckets = merged
    weekCount = buckets.length
  }
  return buckets
}

export type CouplingTrend = 'increasing' | 'stable' | 'decreasing' | 'unknown'

export function pairTrend(
  entries: ChangeTimeline['entries'],
  fileA: string,
  fileB: string,
): { trend: CouplingTrend; recent: number; older: number } {
  const ordered = [...entries].sort((a, b) => (a.order ?? 0) - (b.order ?? 0))
  const hits = ordered.filter(
    (entry) => entry.files.includes(fileA) && entry.files.includes(fileB),
  )
  if (hits.length === 0) return { trend: 'unknown', recent: 0, older: 0 }
  const split = Math.floor(ordered.length * (2 / 3))
  const splitOrder = ordered[split]?.order ?? 0
  const recent = hits.filter((entry) => (entry.order ?? 0) >= splitOrder).length
  const older = hits.length - recent
  const trend: CouplingTrend =
    recent > older ? 'increasing' : recent < older ? 'decreasing' : 'stable'
  return { trend, recent, older }
}

export function fileCommitStamps(
  entries: ChangeTimeline['entries'],
  path: string,
): number[] {
  const stamps: number[] = []
  for (const entry of entries) {
    if (!entry.files.includes(path)) continue
    const date = entryDate(entry)
    if (date) stamps.push(date.getTime())
  }
  return stamps.sort((a, b) => a - b)
}

export function edgeSignals(edge: {
  dependency: boolean
  temporal: boolean
  cochange: boolean
}): string[] {
  const signals: string[] = []
  if (edge.dependency) signals.push('dependency')
  if (edge.temporal) signals.push('temporal')
  if (edge.cochange) signals.push('co-change')
  return signals
}
