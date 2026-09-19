import type { ChangeTimeline } from '../types'

const WEEK_SECS = 7 * 24 * 60 * 60

export function weeklyBuckets(entries: ChangeTimeline['entries']): number[] {
  const stamps: number[] = []
  for (const entry of entries) {
    if (typeof entry.timestamp === 'number') {
      stamps.push(entry.timestamp)
    } else if (entry.date) {
      const parsed = Date.parse(entry.date) / 1000
      if (Number.isFinite(parsed)) stamps.push(parsed)
    }
  }
  if (stamps.length === 0) return []
  stamps.sort((a, b) => a - b)
  const start = Math.floor(stamps[0] / WEEK_SECS) * WEEK_SECS
  const end = Math.ceil((stamps[stamps.length - 1] + 1) / WEEK_SECS) * WEEK_SECS
  const buckets = new Array<number>(Math.max((end - start) / WEEK_SECS, 1)).fill(0)
  for (const stamp of stamps) {
    const index = Math.min(
      Math.floor((stamp - start) / WEEK_SECS),
      buckets.length - 1,
    )
    buckets[index] += 1
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
