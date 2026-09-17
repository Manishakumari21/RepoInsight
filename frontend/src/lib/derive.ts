export function share(value: number, total: number): number {
  return total > 0 ? Math.min(value / total, 1) : 0
}

export function commitsPerMonth(
  totalCommits: number,
  firstIso: string | null,
  lastIso: string | null,
): number | null {
  if (!firstIso || !lastIso) return null
  const start = new Date(firstIso).getTime()
  const end = new Date(lastIso).getTime()
  if (Number.isNaN(start) || Number.isNaN(end) || end <= start) return null
  const months = Math.max((end - start) / (30 * 24 * 60 * 60 * 1000), 1 / 30)
  const rate = totalCommits / months
  return Number.isFinite(rate) ? rate : null
}

export function formatRate(rate: number): string {
  if (rate >= 10) return Math.round(rate).toLocaleString('en-US')
  return rate.toFixed(1)
}