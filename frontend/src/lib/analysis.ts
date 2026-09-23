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
