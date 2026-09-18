import { useEffect, useState } from 'react'
import { fetchReworkEvaluation } from '../api'
import type { ReworkEvaluationReport } from '../types'
import { formatNumber } from '../types'

function metric(value: number | null): string {
  return value == null ? 'n/a' : value.toFixed(3)
}

export function ReworkEvaluation() {
  const [report, setReport] = useState<ReworkEvaluationReport | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    fetchReworkEvaluation()
      .then((result) => {
        if (!cancelled) setReport(result)
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Evaluation unavailable')
        }
      })
    return () => {
      cancelled = true
    }
  }, [])

  if (error) {
    return <p className="history-note">Rework evaluation unavailable: {error}</p>
  }

  if (!report) {
    return <p className="history-note">Loading rework evaluation…</p>
  }

  const { metrics } = report

  return (
    <div>
      <div className="metrics">
        <div className="metric">
          <div className="metric-value">{metric(metrics.precision)}</div>
          <div className="metric-label">Precision</div>
        </div>
        <div className="metric">
          <div className="metric-value">{metric(metrics.recall)}</div>
          <div className="metric-label">Recall</div>
        </div>
        <div className="metric">
          <div className="metric-value">{metric(metrics.false_positive_rate)}</div>
          <div className="metric-label">False positive rate</div>
        </div>
      </div>
      <div className="history-stats">
        <span className="chip">TP {formatNumber(metrics.tp)}</span>
        <span className="chip chip-warn">FP {formatNumber(metrics.fp)}</span>
        <span className="chip chip-ok">TN {formatNumber(metrics.tn)}</span>
        <span className="chip">FN {formatNumber(metrics.fn)}</span>
      </div>
      <p className="history-note">
        Self-evaluation of the rework detector on RepoInsight&apos;s own reviewed
        history ({report.dataset}, {formatNumber(report.pairs.length)} pairs) —
        observed measurement, not a prediction. Recall is n/a: the sample
        contains no actual rework.
      </p>
    </div>
  )
}
