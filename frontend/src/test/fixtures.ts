import type { FilePrediction } from '../types'

export function makePrediction(
  file_path: string,
  probability: number,
): FilePrediction {
  return {
    file_path,
    probability,
    label: probability >= 0.5 ? 1 : 0,
    confidence_level: 'high',
    calibrated: false,
    top_evidence: [
      {
        feature: 'historical_rework_frequency',
        group: 'temporal',
        description: 'How often changes to the file were followed by rework.',
        raw_value: 3,
        transformed_value: 1.2,
        contribution: 2.5,
        direction: 'supports',
      },
    ],
    supporting_evidence: [],
    contradicting_evidence: [],
    historical_examples: [
      {
        commit_sha: 'abc123def456',
        timestamp: 1700000000,
        date: '2023-11-14T00:00:00Z',
        file_path,
        event_type: 'co_change',
        related_files: ['src/other.ts'],
      },
    ],
    recommendations: [`Consider reviewing related changes in src/other.ts before modifying ${file_path}.`],
  }
}
