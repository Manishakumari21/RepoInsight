import type { RepositoryAnalysis } from '../types'
import { formatNumber } from '../types'
import { Panel } from './Panel'
import { Stat } from './primitives'

export interface DatasetFeatureSamples {
  structural_features?: { file_path: string }[]
  historical_features?: { file_path: string }[]
  temporal_features?: { file_path: string }[]
}

export function DatasetReadiness({
  analysis,
}: {
  analysis: RepositoryAnalysis & Partial<DatasetFeatureSamples>
}) {
  const structural = analysis.structural_features ?? null
  const historical = analysis.historical_features ?? null
  const temporal = analysis.temporal_features ?? null

  return (
    <Panel
      title="ML Dataset Readiness"
      hint="Phase 5 feature coverage · observed counts"
    >
      {structural === null ||
      historical === null ||
      temporal === null ? (
        <p className="history-note">
          Feature samples are unavailable from this backend version — upgrade
          the analysis service to expose structural, historical, and temporal
          feature vectors. No predictions are shown.
        </p>
      ) : (
        <>
          <div className="stat-grid readiness-grid">
            <Stat
              label="Structural vectors"
              value={formatNumber(structural.length)}
              sub="per parsed file"
            />
            <Stat
              label="Historical vectors"
              value={formatNumber(historical.length)}
              sub="per changed file"
            />
            <Stat
              label="Temporal vectors"
              value={formatNumber(temporal.length)}
              sub="per propagated file"
            />
          </div>
          <p className="history-note">
            Counts of ML-ready feature vectors computed from repository
            history. Dataset construction and model training remain future
            work — nothing here is a prediction.
          </p>
        </>
      )}
    </Panel>
  )
}
