use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::dataset::DatasetRow;
use crate::github::commits::Commit;

const WEIGHTS_JSON: &str = include_str!("weights.json");

pub const MODEL_NAME: &str = "logistic_regression";
pub const LABEL_THRESHOLD: f64 = 0.5;
pub const TOP_EVIDENCE_COUNT: usize = 5;
pub const MAX_HISTORICAL_EXAMPLES: usize = 5;
pub const MAX_RECOMMENDATIONS: usize = 3;

const LOW_THRESHOLD: f64 = 0.6;
const HIGH_THRESHOLD: f64 = 0.8;

pub const FEATURE_ORDER: [&str; 23] = [
    "file_size_bytes",
    "lines_of_code",
    "function_count",
    "cyclomatic_complexity",
    "max_nesting_depth",
    "incoming_dependencies",
    "outgoing_dependencies",
    "coupling",
    "num_dependents",
    "previous_change_count",
    "historical_churn",
    "contributor_count",
    "time_since_last_change_secs",
    "recent_change_frequency",
    "historical_cochange_frequency",
    "recent_change_sequence_count",
    "previous_files_changed_count",
    "change_window_count",
    "avg_change_delay_seconds",
    "change_order_count",
    "propagation_frequency",
    "followup_frequency",
    "historical_rework_frequency",
];

const FEATURE_META: [(&str, &str, &str); 23] = [
    (
        "file_size_bytes",
        "structural",
        "Size of the source file in bytes.",
    ),
    (
        "lines_of_code",
        "structural",
        "Lines of code in the source file.",
    ),
    (
        "function_count",
        "structural",
        "Number of functions defined in the file.",
    ),
    (
        "cyclomatic_complexity",
        "structural",
        "Cyclomatic complexity of the file.",
    ),
    (
        "max_nesting_depth",
        "structural",
        "Maximum nesting depth in the file.",
    ),
    (
        "incoming_dependencies",
        "structural",
        "Number of incoming dependencies on the file.",
    ),
    (
        "outgoing_dependencies",
        "structural",
        "Number of outgoing dependencies from the file.",
    ),
    (
        "coupling",
        "structural",
        "Coupling of the file to other files.",
    ),
    (
        "num_dependents",
        "structural",
        "Number of files depending on this file.",
    ),
    (
        "previous_change_count",
        "historical",
        "How often the file changed before the target commit.",
    ),
    (
        "historical_churn",
        "historical",
        "Total added plus deleted lines for the file in prior history.",
    ),
    (
        "contributor_count",
        "historical",
        "Number of distinct contributors touching the file before.",
    ),
    (
        "time_since_last_change_secs",
        "historical",
        "Seconds since the file last changed before the target commit.",
    ),
    (
        "recent_change_frequency",
        "historical",
        "How frequently the file changed in the recent window.",
    ),
    (
        "historical_cochange_frequency",
        "historical",
        "How often the file changed together with other files.",
    ),
    (
        "recent_change_sequence_count",
        "temporal",
        "Recent ordered change sequences involving the file.",
    ),
    (
        "previous_files_changed_count",
        "temporal",
        "Files changed in commits preceding the target commit.",
    ),
    (
        "change_window_count",
        "temporal",
        "Changes to the file inside the recent change window.",
    ),
    (
        "avg_change_delay_seconds",
        "temporal",
        "Average delay between consecutive changes to the file.",
    ),
    (
        "change_order_count",
        "temporal",
        "Ordered change events involving the file.",
    ),
    (
        "propagation_frequency",
        "temporal",
        "How often changes propagated through this file.",
    ),
    (
        "followup_frequency",
        "temporal",
        "How often changes to the file were followed by further changes.",
    ),
    (
        "historical_rework_frequency",
        "temporal",
        "How often changes to the file were followed by rework.",
    ),
];

#[derive(Debug, Clone, Deserialize)]
struct Weights {
    model_name: String,
    feature_order: Vec<String>,
    median: Vec<Option<f64>>,
    scaler_mean: Vec<f64>,
    scaler_scale: Vec<f64>,
    coefficients: Vec<f64>,
    intercept: f64,
}

fn weights() -> &'static Weights {
    static CELL: OnceLock<Weights> = OnceLock::new();
    CELL.get_or_init(|| {
        let parsed: Weights = serde_json::from_str(WEIGHTS_JSON).expect("prediction weights parse");
        let expected: Vec<String> = FEATURE_ORDER
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        assert_eq!(parsed.feature_order, expected, "weights feature order");
        assert_eq!(parsed.model_name, MODEL_NAME, "weights model name");
        parsed
    })
}

pub fn raw_values(row: &DatasetRow) -> [Option<f64>; 23] {
    let structural = &row.structural;
    let historical = &row.historical;
    let temporal = &row.temporal;
    [
        Some(structural.file_size_bytes as f64),
        Some(structural.lines_of_code as f64),
        Some(structural.function_count as f64),
        Some(structural.cyclomatic_complexity as f64),
        Some(structural.max_nesting_depth as f64),
        Some(structural.incoming_dependencies as f64),
        Some(structural.outgoing_dependencies as f64),
        Some(structural.coupling as f64),
        Some(structural.num_dependents as f64),
        Some(historical.previous_change_count as f64),
        Some(historical.historical_churn as f64),
        Some(historical.contributor_count as f64),
        historical
            .time_since_last_change_secs
            .map(|value| value as f64),
        Some(historical.recent_change_frequency),
        Some(historical.historical_cochange_frequency as f64),
        Some(temporal.recent_change_sequence_count as f64),
        Some(temporal.previous_files_changed_count as f64),
        Some(temporal.change_window_count as f64),
        temporal.avg_change_delay_seconds.map(|value| value as f64),
        Some(temporal.change_order_count as f64),
        Some(temporal.propagation_frequency as f64),
        Some(temporal.followup_frequency as f64),
        Some(temporal.historical_rework_frequency as f64),
    ]
}

fn sigmoid(score: f64) -> f64 {
    1.0 / (1.0 + (-score).exp())
}

pub fn predict_proba(raw: &[Option<f64>; 23]) -> f64 {
    use ndarray::Array1;

    let loaded = weights();
    let scaled = Array1::from(
        (0..23)
            .map(|index| {
                let imputed = raw[index].unwrap_or_else(|| loaded.median[index].unwrap_or(0.0));
                (imputed - loaded.scaler_mean[index]) / loaded.scaler_scale[index]
            })
            .collect::<Vec<_>>(),
    );
    let coefficients = Array1::from(loaded.coefficients.clone());
    let score = loaded.intercept + coefficients.dot(&scaled);
    sigmoid(score)
}

pub fn confidence_level(probability: f64) -> &'static str {
    let strength = probability.max(1.0 - probability);
    if strength >= HIGH_THRESHOLD {
        "high"
    } else if strength >= LOW_THRESHOLD {
        "medium"
    } else {
        "low"
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FeatureEvidence {
    pub feature: String,
    pub group: String,
    pub description: String,
    pub raw_value: Option<f64>,
    pub transformed_value: Option<f64>,
    pub contribution: f64,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoricalExample {
    pub commit_sha: String,
    pub timestamp: Option<i64>,
    pub date: Option<String>,
    pub file_path: String,
    pub event_type: String,
    pub related_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilePrediction {
    pub file_path: String,
    pub probability: f64,
    pub label: u8,
    pub confidence_level: String,
    pub calibrated: bool,
    pub top_evidence: Vec<FeatureEvidence>,
    pub supporting_evidence: Vec<FeatureEvidence>,
    pub contradicting_evidence: Vec<FeatureEvidence>,
    pub historical_examples: Vec<HistoricalExample>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub calibrated: bool,
    pub threshold: f64,
    pub available_models: Vec<String>,
    pub training: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredictionsResponse {
    pub model: ModelInfo,
    pub predictions: Vec<FilePrediction>,
}

fn build_evidence(raw: &[Option<f64>; 23]) -> Vec<FeatureEvidence> {
    let loaded = weights();
    let mut entries = Vec::with_capacity(23);
    for index in 0..23 {
        let (name, group, description) = FEATURE_META[index];
        let imputed = raw[index].unwrap_or_else(|| loaded.median[index].unwrap_or(0.0));
        let transformed = (imputed - loaded.scaler_mean[index]) / loaded.scaler_scale[index];
        let contribution = loaded.coefficients[index] * transformed;
        let direction = if contribution > 0.0 {
            "supports"
        } else if contribution < 0.0 {
            "contradicts"
        } else {
            "unknown"
        };
        entries.push(FeatureEvidence {
            feature: name.to_owned(),
            group: group.to_owned(),
            description: description.to_owned(),
            raw_value: raw[index],
            transformed_value: Some(transformed),
            contribution,
            direction: direction.to_owned(),
        });
    }
    entries
}

fn top_evidence(evidence: &[FeatureEvidence]) -> Vec<FeatureEvidence> {
    let mut ordered = evidence.to_vec();
    ordered.sort_by(|a, b| {
        b.contribution
            .abs()
            .partial_cmp(&a.contribution.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.feature.cmp(&b.feature))
    });
    ordered.truncate(TOP_EVIDENCE_COUNT);
    ordered
}

fn commit_files(commit: &Commit) -> Vec<String> {
    let mut files: Vec<String> = commit
        .files
        .iter()
        .map(|file| file.filename.clone())
        .collect();
    files.sort();
    files.dedup();
    files
}

fn historical_examples(commits: &[Commit], file: &str) -> Vec<HistoricalExample> {
    let mut examples = Vec::new();
    for commit in commits {
        let files = commit_files(commit);
        if !files.contains(&file.to_owned()) {
            continue;
        }
        examples.push(HistoricalExample {
            commit_sha: commit.sha.clone(),
            timestamp: commit.timestamp,
            date: commit.date.clone(),
            file_path: file.to_owned(),
            event_type: "prior_change".to_owned(),
            related_files: Vec::new(),
        });
        let related: Vec<String> = files.into_iter().filter(|name| name != file).collect();
        if !related.is_empty() {
            examples.push(HistoricalExample {
                commit_sha: commit.sha.clone(),
                timestamp: commit.timestamp,
                date: commit.date.clone(),
                file_path: file.to_owned(),
                event_type: "co_change".to_owned(),
                related_files: related,
            });
        }
    }
    examples.sort_by(|a, b| {
        b.timestamp
            .unwrap_or(0)
            .cmp(&a.timestamp.unwrap_or(0))
            .then_with(|| a.commit_sha.cmp(&b.commit_sha))
            .then_with(|| a.event_type.cmp(&b.event_type))
    });
    examples.truncate(MAX_HISTORICAL_EXAMPLES);
    examples
}

fn raw_of(evidence: &[FeatureEvidence], feature: &str) -> Option<f64> {
    evidence
        .iter()
        .find(|entry| entry.feature == feature)?
        .raw_value
}

fn supports(evidence: &[FeatureEvidence], feature: &str) -> bool {
    evidence
        .iter()
        .any(|entry| entry.feature == feature && entry.direction == "supports")
}

fn recommendations(
    file: &str,
    evidence: &[FeatureEvidence],
    examples: &[HistoricalExample],
) -> Vec<String> {
    let mut out = Vec::new();
    if raw_of(evidence, "historical_rework_frequency").is_some_and(|value| value > 0.0)
        && supports(evidence, "historical_rework_frequency")
    {
        out.push(format!(
            "Historical changes suggest reviewing past rework in {file} before modifying it."
        ));
    }
    if let Some(example) = examples
        .iter()
        .find(|item| item.event_type == "co_change" && !item.related_files.is_empty())
    {
        out.push(format!(
            "Consider reviewing related changes in {} before modifying {file}.",
            example.related_files[0]
        ));
    }
    if raw_of(evidence, "followup_frequency").is_some_and(|value| value > 0.0)
        && supports(evidence, "followup_frequency")
    {
        out.push(format!(
            "Check whether recent follow-up changes affect {file}."
        ));
    }
    if raw_of(evidence, "propagation_frequency").is_some_and(|value| value > 0.0)
        && supports(evidence, "propagation_frequency")
    {
        out.push(format!(
            "Historical changes suggest checking downstream files that previously changed after {file}."
        ));
    }
    if raw_of(evidence, "coupling").is_some_and(|value| value > 0.0)
        && supports(evidence, "coupling")
    {
        out.push(format!(
            "Consider reviewing coupled files of {file}; its coupling is associated with rework."
        ));
    }
    out.truncate(MAX_RECOMMENDATIONS);
    out
}

fn latest_rows<'a>(rows: &'a [DatasetRow]) -> Vec<&'a DatasetRow> {
    let mut latest: HashMap<&str, &DatasetRow> = HashMap::new();
    for row in rows {
        let replace = match latest.get(row.file_path.as_str()) {
            None => true,
            Some(current) => {
                (row.timestamp, row.commit_sha.as_str())
                    > (current.timestamp, current.commit_sha.as_str())
            }
        };
        if replace {
            latest.insert(row.file_path.as_str(), row);
        }
    }
    let mut ordered: Vec<&DatasetRow> = latest.into_values().collect();
    ordered.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    ordered
}

pub fn predict_files(rows: &[DatasetRow], commits: &[Commit]) -> PredictionsResponse {
    let mut predictions = Vec::new();
    for row in latest_rows(rows) {
        let raw = raw_values(row);
        let probability = predict_proba(&raw);
        let label = u8::from(probability >= LABEL_THRESHOLD);
        let evidence = build_evidence(&raw);
        let supporting = evidence
            .iter()
            .filter(|entry| entry.direction == "supports")
            .cloned()
            .collect::<Vec<_>>();
        let mut contradicting = evidence
            .iter()
            .filter(|entry| entry.direction == "contradicts")
            .cloned()
            .collect::<Vec<_>>();
        contradicting.sort_by(|a, b| {
            a.contribution
                .partial_cmp(&b.contribution)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.feature.cmp(&b.feature))
        });
        let examples = historical_examples(commits, &row.file_path);
        let recs = recommendations(&row.file_path, &evidence, &examples);
        predictions.push(FilePrediction {
            file_path: row.file_path.clone(),
            probability,
            label,
            confidence_level: confidence_level(probability).to_owned(),
            calibrated: false,
            top_evidence: top_evidence(&evidence),
            supporting_evidence: supporting,
            contradicting_evidence: contradicting,
            historical_examples: examples,
            recommendations: recs,
        });
    }
    PredictionsResponse {
        model: ModelInfo {
            name: MODEL_NAME.to_owned(),
            calibrated: false,
            threshold: LABEL_THRESHOLD,
            available_models: vec![MODEL_NAME.to_owned()],
            training: "logistic_regression baseline trained on ml/data/dataset.jsonl train split; see ml/scripts/export_baseline_weights.py".to_owned(),
        },
        predictions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE_RAW: [Option<f64>; 23] = [
        Some(8224.0),
        Some(259.0),
        Some(18.0),
        Some(9.0),
        Some(5.0),
        Some(0.0),
        Some(3.0),
        Some(3.0),
        Some(0.0),
        Some(3.0),
        Some(210.0),
        Some(1.0),
        Some(13.0),
        Some(0.0000011574074074074074),
        Some(77.0),
        Some(3.0),
        Some(74.0),
        Some(2.0),
        Some(458827.0),
        Some(7.0),
        Some(35.0),
        Some(7.0),
        Some(3.0),
    ];

    #[test]
    fn weights_cover_all_23_features_in_order() {
        let loaded = weights();
        assert_eq!(loaded.feature_order.len(), 23);
        assert_eq!(loaded.coefficients.len(), 23);
        assert_eq!(loaded.median.len(), 23);
    }

    #[test]
    fn inference_matches_python_baseline() {
        let probability = predict_proba(&REFERENCE_RAW);
        assert!(
            (probability - 0.03526788170277072).abs() < 1e-9,
            "{probability}"
        );
    }

    #[test]
    fn missing_values_fall_back_deterministically() {
        let empty: [Option<f64>; 23] = [None; 23];
        let first = predict_proba(&empty);
        let second = predict_proba(&empty);
        assert_eq!(first, second);
        assert!((0.0..=1.0).contains(&first));
    }

    #[test]
    fn label_and_confidence_thresholds() {
        assert_eq!(
            u8::from(predict_proba(&REFERENCE_RAW) >= LABEL_THRESHOLD),
            0
        );
        assert_eq!(confidence_level(0.9), "high");
        assert_eq!(confidence_level(0.7), "medium");
        assert_eq!(confidence_level(0.55), "low");
        assert_eq!(confidence_level(0.1), "high");
    }

    #[test]
    fn evidence_direction_follows_sign() {
        let evidence = build_evidence(&REFERENCE_RAW);
        assert_eq!(evidence.len(), 23);
        for entry in &evidence {
            if entry.contribution > 0.0 {
                assert_eq!(entry.direction, "supports");
            } else if entry.contribution < 0.0 {
                assert_eq!(entry.direction, "contradicts");
            } else {
                assert_eq!(entry.direction, "unknown");
            }
        }
    }

    #[test]
    fn latest_row_wins_per_file() {
        use crate::analysis::{
            features::StructuralFeatures, historical_features::HistoricalFeatures,
            temporal_features::TemporalFeatures,
        };
        fn row(sha: &str, ts: i64, churn: u64) -> DatasetRow {
            DatasetRow {
                commit_sha: sha.to_owned(),
                timestamp: ts,
                file_path: "a.rs".to_owned(),
                structural: StructuralFeatures {
                    file_path: "a.rs".to_owned(),
                    file_size_bytes: 10,
                    lines_of_code: 5,
                    function_count: 1,
                    cyclomatic_complexity: 1,
                    max_nesting_depth: 1,
                    incoming_dependencies: 0,
                    outgoing_dependencies: 0,
                    coupling: 0,
                    num_dependents: 0,
                },
                historical: HistoricalFeatures {
                    file_path: "a.rs".to_owned(),
                    previous_change_count: 1,
                    historical_churn: churn,
                    contributor_count: 1,
                    time_since_last_change_secs: Some(60),
                    recent_change_frequency: 0.0,
                    historical_cochange_frequency: 0,
                },
                temporal: TemporalFeatures {
                    file_path: "a.rs".to_owned(),
                    recent_change_sequence_count: 0,
                    previous_files_changed_count: 0,
                    change_window_count: 0,
                    avg_change_delay_seconds: None,
                    change_order_count: 0,
                    propagation_frequency: 0,
                    followup_frequency: 0,
                    historical_rework_frequency: 0,
                },
                label: 0,
            }
        }
        let rows = vec![row("aaa", 200, 99), row("bbb", 100, 1)];
        let response = predict_files(&rows, &[]);
        assert_eq!(response.predictions.len(), 1);
        let churn = response.predictions[0]
            .top_evidence
            .iter()
            .chain(response.predictions[0].supporting_evidence.iter())
            .chain(response.predictions[0].contradicting_evidence.iter())
            .find(|entry| entry.feature == "historical_churn")
            .and_then(|entry| entry.raw_value);
        assert_eq!(churn, Some(99.0));
    }

    #[test]
    fn examples_are_newest_first_and_capped() {
        use crate::github::commits::{ChangedFile, CommitStats, FileChangeStatus};
        fn commit(sha: &str, ts: i64, files: &[&str]) -> Commit {
            Commit {
                sha: sha.to_owned(),
                message: String::new(),
                author: None,
                timestamp: Some(ts),
                date: None,
                url: String::new(),
                stats: CommitStats::default(),
                files: files
                    .iter()
                    .map(|name| ChangedFile {
                        filename: (*name).to_owned(),
                        additions: 1,
                        deletions: 0,
                        changes: 1,
                        status: FileChangeStatus::Modified,
                    })
                    .collect(),
            }
        }
        let commits = vec![
            commit("aaa", 100, &["a.rs"]),
            commit("bbb", 200, &["a.rs", "b.rs"]),
            commit("ccc", 300, &["other.rs"]),
        ];
        let examples = historical_examples(&commits, "a.rs");
        assert_eq!(examples.len(), 3);
        assert_eq!(examples[0].commit_sha, "bbb");
        assert_eq!(examples[0].event_type, "co_change");
        assert_eq!(examples[0].related_files, vec!["b.rs"]);
        assert!(examples.iter().all(|item| item.file_path == "a.rs"));
        assert!(historical_examples(&commits, "missing.rs").is_empty());
    }

    #[test]
    fn recommendations_require_evidence() {
        let evidence = build_evidence(&REFERENCE_RAW);
        let recs = recommendations("a.rs", &[], &[]);
        assert!(recs.is_empty());
        let _ = evidence;
    }
}
