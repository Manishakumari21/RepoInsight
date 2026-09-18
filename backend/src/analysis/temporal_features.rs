use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::analysis::models::{PropagationAnalysis, TemporalAnalysis};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TemporalFeatures {
    pub file_path: String,
    pub recent_change_sequence_count: usize,
    pub previous_files_changed_count: usize,
    pub change_window_count: usize,
    pub avg_change_delay_seconds: Option<i64>,
    pub change_order_count: usize,
    pub propagation_frequency: usize,
    pub followup_frequency: usize,
    pub historical_rework_frequency: usize,
}

/// Per-file temporal signals over one considered commit set.
///
/// `file_times` maps each file to its sorted change timestamps within the
/// set; `ref_ts` is the reference time (history end for snapshots, target
/// time `T` for dataset rows); only information in the set may be passed.
pub fn build_temporal_features(
    temporal: &TemporalAnalysis,
    propagation: &PropagationAnalysis,
    followup_frequency: &HashMap<String, usize>,
    rework_frequency: &HashMap<String, usize>,
    file_times: &HashMap<String, Vec<i64>>,
    ref_ts: i64,
    window_secs: i64,
) -> Vec<TemporalFeatures> {
    let mut features: HashMap<String, TemporalFeatures> = HashMap::new();

    for pair in &temporal.pairs {
        for file in [&pair.source, &pair.target] {
            features
                .entry(file.clone())
                .or_insert_with(|| empty_features(file));
        }
        features
            .get_mut(&pair.source)
            .expect("inserted above")
            .recent_change_sequence_count += pair.occurrences;
    }

    for edge in &propagation.edges {
        for file in [&edge.source, &edge.target] {
            features
                .entry(file.clone())
                .or_insert_with(|| empty_features(file))
                .propagation_frequency += edge.strength;
        }
    }

    for (file, count) in followup_frequency {
        features
            .entry(file.clone())
            .or_insert_with(|| empty_features(file))
            .followup_frequency += *count;
    }

    for (file, count) in rework_frequency {
        features
            .entry(file.clone())
            .or_insert_with(|| empty_features(file))
            .historical_rework_frequency += *count;
    }

    let others = file_times.len();
    for entry in features.values_mut() {
        let times = file_times.get(&entry.file_path);

        entry.previous_files_changed_count = others.saturating_sub(usize::from(times.is_some()));
        entry.change_window_count = times
            .map(|stamps| {
                stamps
                    .iter()
                    .filter(|ts| ref_ts - **ts >= 0 && ref_ts - **ts <= window_secs)
                    .count()
            })
            .unwrap_or(0);
        entry.avg_change_delay_seconds = times.and_then(|stamps| mean_gap(stamps));
        entry.change_order_count = times
            .and_then(|stamps| stamps.iter().max())
            .map(|latest| {
                let mut all: HashSet<i64> = HashSet::new();
                for stamps in file_times.values() {
                    all.extend(stamps.iter().copied());
                }
                all.iter().filter(|ts| **ts <= *latest).count()
            })
            .unwrap_or(0);
    }

    let mut result: Vec<_> = features.into_values().collect();
    result.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    result
}

fn mean_gap(stamps: &[i64]) -> Option<i64> {
    if stamps.len() < 2 {
        return None;
    }
    let total: i64 = stamps.windows(2).map(|pair| pair[1] - pair[0]).sum();
    Some(total / (stamps.len() - 1) as i64)
}

impl TemporalFeatures {
    /// Zero signals for files absent from the considered evidence.
    pub fn empty(file_path: &str) -> Self {
        empty_features(file_path)
    }
}

fn empty_features(file_path: &str) -> TemporalFeatures {
    TemporalFeatures {
        file_path: file_path.to_owned(),
        recent_change_sequence_count: 0,
        previous_files_changed_count: 0,
        change_window_count: 0,
        avg_change_delay_seconds: None,
        change_order_count: 0,
        propagation_frequency: 0,
        followup_frequency: 0,
        historical_rework_frequency: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::models::{PropagationEdge, TemporalPair};

    fn pair(source: &str, target: &str, occurrences: usize, avg: i64) -> TemporalPair {
        TemporalPair {
            source: source.to_owned(),
            target: target.to_owned(),
            occurrences,
            avg_delay_seconds: avg,
        }
    }

    fn times(pairs: &[(&str, Vec<i64>)]) -> HashMap<String, Vec<i64>> {
        pairs
            .iter()
            .map(|(file, stamps)| ((*file).to_owned(), stamps.clone()))
            .collect()
    }

    #[test]
    fn aggregates_sequence_and_propagation_counts() {
        let temporal = TemporalAnalysis {
            pairs: vec![pair("a.rs", "b.rs", 3, 100), pair("a.rs", "c.rs", 2, 50)],
        };
        let propagation = PropagationAnalysis {
            edges: vec![PropagationEdge {
                source: "a.rs".to_owned(),
                target: "b.rs".to_owned(),
                dependency: false,
                temporal: true,
                cochange: false,
                strength: 4,
            }],
        };

        let result = build_temporal_features(
            &temporal,
            &propagation,
            &HashMap::new(),
            &HashMap::new(),
            &times(&[("a.rs", vec![100, 200, 300]), ("b.rs", vec![150])]),
            300,
            7 * 24 * 60 * 60,
        );

        let file = result
            .iter()
            .find(|entry| entry.file_path == "a.rs")
            .unwrap();
        assert_eq!(file.recent_change_sequence_count, 5);
        assert_eq!(file.propagation_frequency, 4);
        assert_eq!(file.previous_files_changed_count, 1);
        assert_eq!(file.change_window_count, 3);
        assert_eq!(file.avg_change_delay_seconds, Some(100));
        assert_eq!(file.change_order_count, 4);
    }

    #[test]
    fn avg_delay_needs_two_changes() {
        let temporal = TemporalAnalysis { pairs: vec![] };
        let propagation = PropagationAnalysis { edges: vec![] };

        let result = build_temporal_features(
            &temporal,
            &propagation,
            &HashMap::from([("solo.rs".to_owned(), 2)]),
            &HashMap::from([("solo.rs".to_owned(), 1)]),
            &times(&[("solo.rs", vec![500])]),
            500,
            60,
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].avg_change_delay_seconds, None);
        assert_eq!(result[0].followup_frequency, 2);
        assert_eq!(result[0].historical_rework_frequency, 1);
        assert_eq!(result[0].change_window_count, 1);
    }

    #[test]
    fn unknown_files_get_zero_time_fields() {
        let temporal = TemporalAnalysis {
            pairs: vec![pair("$dep", "a.rs", 1, 10)],
        };
        let propagation = PropagationAnalysis { edges: vec![] };

        let result = build_temporal_features(
            &temporal,
            &propagation,
            &HashMap::new(),
            &HashMap::new(),
            &times(&[("a.rs", vec![10])]),
            10,
            60,
        );

        let file = result
            .iter()
            .find(|entry| entry.file_path == "$dep")
            .unwrap();
        assert_eq!(file.avg_change_delay_seconds, None);
        assert_eq!(file.change_order_count, 0);
        assert_eq!(file.previous_files_changed_count, 1);
    }
}
