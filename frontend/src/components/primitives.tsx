import type { ReactNode } from 'react'
import { useEffect, useState } from 'react'

export function Stat({
  label,
  value,
  sub,
  tone,
}: {
  label: string
  value: string
  sub?: string
  tone?: string
}) {
  return (
    <div className="stat-card">
      <div className="stat-label">{label}</div>
      <div
        className="stat-value tnum"
        style={tone ? { color: tone } : undefined}
      >
        {value}
      </div>
      {sub && <div className="stat-sub">{sub}</div>}
    </div>
  )
}

export function Meter({
  label,
  detail,
  ratio,
  hint,
}: {
  label: string
  detail?: string
  ratio: number
  hint?: string
}) {
  const percent = Math.min(Math.max(ratio * 100, 0), 100)
  const tone =
    percent >= 70 ? 'danger' : percent >= 40 ? 'warn' : ''
  const summary = hint ?? `${label}: ${percent.toFixed(0)} percent${detail ? `, ${detail}` : ''}`
  return (
    <div className="bar-row">
      <div className="bar-top">
        <span className="bar-label">
          {label}
          {detail && <em className="bar-sub">{detail}</em>}
        </span>
        <span className="bar-value tnum">{percent.toFixed(0)}%</span>
      </div>
      <div
        className="bar-track"
        role="img"
        aria-label={summary}
        title={summary}
      >
        <div
          className={`bar-fill ${tone}`.trim()}
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  )
}

export function SeverityBadge({ level }: { level: string }) {
  const normalized = level.toLowerCase()
  const severity =
    normalized === 'high' ? 'high' : normalized === 'medium' ? 'medium' : 'low'
  return <span className={`severity ${severity}`}>{severity}</span>
}

export type EvidenceKind = 'observed' | 'heuristic' | 'upcoming'

export function EvidenceBadge({ kind }: { kind: EvidenceKind }) {
  const label =
    kind === 'observed'
      ? 'Observed'
      : kind === 'heuristic'
        ? 'Heuristic'
        : 'Not yet live'
  const description =
    kind === 'observed'
      ? 'From commit history'
      : kind === 'heuristic'
        ? 'Calculated score, not ML'
        : 'ML risk prediction (planned)'
  return (
    <span className={`evidence-badge ${kind}`} title={description}>
      {label}
    </span>
  )
}

const SIGNAL_TONES: Record<string, string> = {
  dependency: 'var(--accent)',
  temporal: 'var(--warn)',
  'co-change': 'var(--ok)',
}

export function SignalDots({ signals }: { signals: string[] }) {
  return (
    <span className="signal-dots" title={signals.join(', ')}>
      {signals.map((signal) => (
        <span
          key={signal}
          className="signal-dot"
          style={{ background: SIGNAL_TONES[signal] ?? 'var(--muted)' }}
        />
      ))}
      <span className="signal-names">{signals.join(' + ')}</span>
    </span>
  )
}

export function useMounted(): boolean {
  const [mounted, setMounted] = useState(false)
  useEffect(() => {
    const frame = requestAnimationFrame(() => setMounted(true))
    return () => cancelAnimationFrame(frame)
  }, [])
  return mounted
}

export function Skeleton({ label }: { label: string }) {
  return (
    <div className="skeleton" role="img" aria-label={label}>
      <span className="visually-hidden">{label}</span>
    </div>
  )
}

export function DashboardSkeleton(): ReactNode {
  return (
    <div className="dashboard-body" role="status" aria-label="Loading repository analysis">
      <div className="hero">
        <Skeleton label="Loading repository title" />
        <div className="stat-grid">
          {Array.from({ length: 6 }, (_, index) => (
            <Skeleton key={index} label={`Loading statistic ${index + 1}`} />
          ))}
        </div>
      </div>
      <div className="grid-2">
        <Skeleton label="Loading risk overview" />
        <Skeleton label="Loading complexity overview" />
      </div>
      <Skeleton label="Loading hotspot list" />
      <span className="visually-hidden">Loading repository analysis…</span>
    </div>
  )
}
