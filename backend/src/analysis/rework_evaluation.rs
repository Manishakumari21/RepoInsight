use std::collections::HashMap;

use serde::Serialize;

use crate::github::commits::{ChangedFile, Commit, CommitStats, FileChangeStatus};

use super::history;
use super::propagation_history::{self, PropagationConfig};

pub const REWORK_LABELS_CSV: &str =
    include_str!("../../../tests/backend/evaluation/rework_labels.csv");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelRow {
    pub previous_sha: String,
    pub current_sha: String,
    pub actual_rework: bool,
}

pub fn parse_labels(csv: &str) -> Result<Vec<LabelRow>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(csv.as_bytes());

    let mut rows = Vec::new();

    for (index, record) in reader.records().enumerate() {
        let line_number = index + 1;
        let record = record.map_err(|err| format!("line {line_number}: {err}"))?;

        if record.len() == 1 && record[0].trim().is_empty() {
            continue;
        }
        if record.iter().all(|field| field.trim().is_empty()) {
            continue;
        }
        if index == 0 && record.len() >= 3 && record.get(0) == Some("previous_sha") {
            continue;
        }

        if record.len() != 3 {
            return Err(format!("line {line_number}: expected 3 columns"));
        }
        if record[0].is_empty() || record[1].is_empty() {
            return Err(format!("line {line_number}: empty SHA"));
        }
        let actual_rework = match record[2].to_ascii_lowercase().as_str() {
            "true" => true,
            "false" => false,
            other => {
                return Err(format!(
                    "line {line_number}: actual_rework must be true/false, got {other:?}"
                ));
            }
        };

        rows.push(LabelRow {
            previous_sha: record[0].to_owned(),
            current_sha: record[1].to_owned(),
            actual_rework,
        });
    }

    Ok(rows)
}

pub fn confusion_counts(
    labels: &[LabelRow],
    predicted: &HashMap<(String, String), bool>,
) -> (usize, usize, usize, usize) {
    let mut tp = 0;
    let mut fp = 0;
    let mut tn = 0;
    let mut fn_count = 0;

    for label in labels {
        let key = (label.previous_sha.clone(), label.current_sha.clone());
        let predicted_rework = predicted.get(&key).copied().unwrap_or(false);

        match (predicted_rework, label.actual_rework) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, false) => tn += 1,
            (false, true) => fn_count += 1,
        }
    }

    (tp, fp, tn, fn_count)
}

pub fn precision(true_positives: usize, false_positives: usize) -> Option<f64> {
    let denominator = true_positives + false_positives;
    if denominator == 0 {
        return None;
    }
    Some(true_positives as f64 / denominator as f64)
}

pub fn recall(true_positives: usize, false_negatives: usize) -> Option<f64> {
    let denominator = true_positives + false_negatives;
    if denominator == 0 {
        return None;
    }
    Some(true_positives as f64 / denominator as f64)
}

pub fn false_positive_rate(false_positives: usize, true_negatives: usize) -> Option<f64> {
    let denominator = false_positives + true_negatives;
    if denominator == 0 {
        return None;
    }
    Some(false_positives as f64 / denominator as f64)
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationMetrics {
    pub tp: usize,
    pub fp: usize,
    pub tn: usize,
    #[serde(rename = "fn")]
    pub fn_count: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub false_positive_rate: Option<f64>,
}

pub fn evaluate(
    labels: &[LabelRow],
    predicted: &HashMap<(String, String), bool>,
) -> EvaluationMetrics {
    let (tp, fp, tn, fn_count) = confusion_counts(labels, predicted);

    EvaluationMetrics {
        tp,
        fp,
        tn,
        fn_count,
        precision: precision(tp, fp),
        recall: recall(tp, fn_count),
        false_positive_rate: false_positive_rate(fp, tn),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluatedPair {
    pub previous_sha: String,
    pub current_sha: String,
    pub actual_rework: bool,
    pub predicted_rework: bool,
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationReport {
    pub dataset: String,
    pub pairs: Vec<EvaluatedPair>,
    pub metrics: EvaluationMetrics,
}

pub fn run_evaluation() -> EvaluationReport {
    let labels = parse_labels(REWORK_LABELS_CSV).unwrap_or_default();
    let commits = evaluation_commits();
    let config = PropagationConfig::default();

    let (_, history_metrics) = history::analyze(&commits);
    let rework =
        propagation_history::detect_rework(&commits, &[], &history_metrics.cochange_pairs, &config);

    let mut predicted: HashMap<(String, String), bool> = HashMap::new();
    let mut rule_for: HashMap<(String, String), String> = HashMap::new();
    for event in &rework.events {
        let key = (event.initial_sha.clone(), event.rework_sha.clone());
        predicted.insert(key.clone(), true);
        rule_for.entry(key).or_insert_with(|| event.rule.clone());
    }

    let pairs = labels
        .iter()
        .map(|label| {
            let key = (label.previous_sha.clone(), label.current_sha.clone());
            EvaluatedPair {
                previous_sha: label.previous_sha.clone(),
                current_sha: label.current_sha.clone(),
                actual_rework: label.actual_rework,
                predicted_rework: predicted.get(&key).copied().unwrap_or(false),
                rule: rule_for.get(&key).cloned(),
            }
        })
        .collect::<Vec<_>>();

    let metrics = evaluate(&labels, &predicted);

    EvaluationReport {
        dataset: "tests/backend/evaluation/rework_labels.csv".to_owned(),
        pairs,
        metrics,
    }
}

fn changed(path: &str, status: FileChangeStatus) -> ChangedFile {
    ChangedFile {
        filename: path.to_owned(),
        additions: 1,
        deletions: 0,
        changes: 1,
        status,
    }
}

fn eval_commit(
    sha: &str,
    timestamp: i64,
    message: &str,
    files: Vec<(&str, FileChangeStatus)>,
) -> Commit {
    Commit {
        sha: sha.to_owned(),
        message: message.to_owned(),
        author: None,
        timestamp: Some(timestamp),
        date: None,
        url: String::new(),
        stats: CommitStats::default(),
        files: files
            .into_iter()
            .map(|(path, status)| changed(path, status))
            .collect(),
    }
}

pub fn evaluation_commits() -> Vec<Commit> {
    use FileChangeStatus::{Added as A, Modified as M, Removed as D};

    vec![
        eval_commit(
            "147c64ad8aa72f96423e7102b0791b6426f5497d",
            1788464481,
            "Initial project setup",
            vec![
                (".gitignore", A),
                ("README.md", A),
                ("backend/Cargo.lock", A),
                ("backend/Cargo.toml", A),
                ("backend/src/main.rs", A),
                ("docs/architecture.md", A),
                ("frontend/README.md", A),
                ("ml/requirements.txt", A),
                ("ml/src/__init__.py", A),
                ("phases.md", A),
            ],
        ),
        eval_commit(
            "39cdeeabdc2babac8e3e236500abe31bfdd490b5",
            1788464524,
            "Initial project setup",
            vec![(".gitignore", M)],
        ),
        eval_commit(
            "852b566b028d2c3f8946f8d3fa341d43627e0df5",
            1788465420,
            "Complete Phase 1 project foundation",
            vec![
                ("docs/architecture.md", M),
                ("frontend/.gitignore", A),
                ("frontend/README.md", M),
                ("frontend/eslint.config.js", A),
                ("frontend/index.html", A),
                ("frontend/package-lock.json", A),
                ("frontend/package.json", A),
                ("frontend/public/favicon.svg", A),
                ("frontend/public/icons.svg", A),
                ("frontend/src/App.css", A),
                ("frontend/src/App.tsx", A),
                ("frontend/src/assets/hero.png", A),
                ("frontend/src/assets/react.svg", A),
                ("frontend/src/assets/vite.svg", A),
                ("frontend/src/index.css", A),
                ("frontend/src/main.tsx", A),
                ("frontend/tsconfig.app.json", A),
                ("frontend/tsconfig.json", A),
                ("frontend/tsconfig.node.json", A),
                ("frontend/vite.config.ts", A),
                ("ml/requirements.txt", M),
            ],
        ),
        eval_commit(
            "2befeffedf15030b783c52e10eb56020f381dc93",
            1788629857,
            "feat: complete GitHub repository collector",
            vec![
                ("backend/Cargo.lock", M),
                ("backend/Cargo.toml", M),
                ("backend/src/github/client.rs", A),
                ("backend/src/github/commits.rs", A),
                ("backend/src/github/files.rs", A),
                ("backend/src/github/mod.rs", A),
                ("backend/src/github/repository.rs", A),
                ("backend/src/main.rs", M),
            ],
        ),
        eval_commit(
            "c0a89c13b92acd7965ae0b17412b0a5e842b70b9",
            1788629915,
            "chore: remove Rust build artifacts from repository",
            vec![],
        ),
        eval_commit(
            "302fc08f98881b50b016d1d424292dddc6bc9a0a",
            1788807031,
            "feat: complete repository analysis phase",
            vec![
                ("README.md", M),
                ("backend/Cargo.lock", M),
                ("backend/Cargo.toml", M),
                ("backend/src/analysis/analyzer.rs", A),
                ("backend/src/analysis/complexity.rs", A),
                ("backend/src/analysis/dependencies.rs", A),
                ("backend/src/analysis/history.rs", A),
                ("backend/src/analysis/mod.rs", A),
                ("backend/src/analysis/models.rs", A),
                ("backend/src/analysis/parsers/mod.rs", A),
                ("backend/src/analysis/parsers/tree_sitter.rs", A),
                ("backend/src/analysis/repository.rs", A),
                ("backend/src/analysis/scoring.rs", A),
                ("backend/src/analysis/source.rs", A),
                ("backend/src/github/client.rs", M),
                ("backend/src/github/commits.rs", M),
                ("phases.md", M),
            ],
        ),
        eval_commit(
            "5525a4dee0cb2a3253c40dbec92d7026b2d4eb9c",
            1789669301,
            "Phase 4: temporal change propagation + resilient GitHub fetching",
            vec![
                ("README.md", M),
                ("backend/Cargo.lock", M),
                ("backend/Cargo.toml", M),
                ("backend/src/analysis/analyzer.rs", M),
                ("backend/src/analysis/complexity.rs", M),
                ("backend/src/analysis/dependencies.rs", M),
                ("backend/src/analysis/history.rs", M),
                ("backend/src/analysis/mod.rs", M),
                ("backend/src/analysis/models.rs", M),
                ("backend/src/analysis/parsers/tree_sitter.rs", M),
                ("backend/src/analysis/repository.rs", D),
                ("backend/src/analysis/scoring.rs", M),
                ("backend/src/analysis/source.rs", M),
                ("backend/src/cache.rs", A),
                ("backend/src/github/client.rs", M),
                ("backend/src/github/commits.rs", M),
                ("backend/src/github/files.rs", M),
                ("backend/src/github/repository.rs", M),
                ("backend/src/main.rs", M),
                ("frontend/index.html", M),
                ("frontend/src/App.css", M),
                ("frontend/src/App.tsx", M),
                ("frontend/src/api.ts", A),
                ("frontend/src/components/Cochange.tsx", A),
                ("frontend/src/components/Complexity.tsx", A),
                ("frontend/src/components/Dashboard.tsx", A),
                ("frontend/src/components/Dependencies.tsx", A),
                ("frontend/src/components/Donut.tsx", A),
                ("frontend/src/components/Explorer.tsx", A),
                ("frontend/src/components/History.tsx", A),
                ("frontend/src/components/Hotspots.tsx", A),
                ("frontend/src/components/Landing.tsx", A),
                ("frontend/src/components/Overview.tsx", A),
                ("frontend/src/components/Panel.tsx", A),
                ("frontend/src/components/Risk.tsx", A),
                ("frontend/src/components/Settings.tsx", A),
                ("frontend/src/components/Sidebar.tsx", A),
                ("frontend/src/components/Status.tsx", A),
                ("frontend/src/components/Topbar.tsx", A),
                ("frontend/src/config.ts", A),
                ("frontend/src/index.css", M),
                ("frontend/src/lib/derive.ts", A),
                ("frontend/src/lib/githubUrl.ts", A),
                ("frontend/src/lib/palette.ts", A),
                ("frontend/src/nav.ts", A),
                ("frontend/src/types.ts", A),
                ("frontend/vite.config.ts", M),
                ("phases.md", M),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(previous: &str, current: &str, actual: bool) -> LabelRow {
        LabelRow {
            previous_sha: previous.to_owned(),
            current_sha: current.to_owned(),
            actual_rework: actual,
        }
    }

    fn predictions(pairs: Vec<(&str, &str, bool)>) -> HashMap<(String, String), bool> {
        pairs
            .into_iter()
            .map(|(previous, current, predicted)| {
                ((previous.to_owned(), current.to_owned()), predicted)
            })
            .collect()
    }

    #[test]
    fn confusion_matrix_counts_each_cell() {
        let labels = vec![
            label("a", "b", true),
            label("c", "d", true),
            label("e", "f", false),
            label("g", "h", false),
        ];
        let predicted = predictions(vec![
            ("a", "b", true),
            ("c", "d", false),
            ("e", "f", true),
            ("g", "h", false),
        ]);

        assert_eq!(confusion_counts(&labels, &predicted), (1, 1, 1, 1));
    }

    #[test]
    fn missing_predictions_count_as_negative() {
        let labels = vec![label("a", "b", true), label("c", "d", false)];

        assert_eq!(confusion_counts(&labels, &HashMap::new()), (0, 0, 1, 1));
    }

    #[test]
    fn precision_divides_by_predicted_positives() {
        assert_eq!(precision(2, 2), Some(0.5));
        assert_eq!(precision(0, 3), Some(0.0));
    }

    #[test]
    fn recall_divides_by_actual_positives() {
        assert_eq!(recall(1, 3), Some(0.25));
        assert_eq!(recall(0, 2), Some(0.0));
    }

    #[test]
    fn false_positive_rate_divides_by_actual_negatives() {
        assert_eq!(false_positive_rate(1, 3), Some(0.25));
        assert_eq!(false_positive_rate(0, 5), Some(0.0));
    }

    #[test]
    fn zero_denominators_yield_none() {
        assert_eq!(precision(0, 0), None);
        assert_eq!(recall(0, 0), None);
        assert_eq!(false_positive_rate(0, 0), None);
    }

    #[test]
    fn labels_csv_parses_to_six_reviewed_pairs() {
        use std::collections::HashSet;

        let labels = parse_labels(REWORK_LABELS_CSV).expect("labels should parse");

        assert_eq!(labels.len(), 6);
        assert!(labels.iter().all(|label| !label.actual_rework));

        let distinct: HashSet<&String> = labels
            .iter()
            .flat_map(|label| [&label.previous_sha, &label.current_sha])
            .collect();
        assert_eq!(distinct.len(), 7);
    }

    #[test]
    fn malformed_labels_are_rejected() {
        assert!(parse_labels("previous_sha,current_sha,actual_rework\na,b,maybe").is_err());
        assert!(parse_labels("previous_sha,current_sha,actual_rework\na,b").is_err());
        assert!(parse_labels("previous_sha,current_sha,actual_rework\n,b,false").is_err());
    }

    #[test]
    fn detector_outcome_on_own_history_is_measured() {
        let report = run_evaluation();

        assert_eq!(report.pairs.len(), 6);

        let metrics = &report.metrics;
        assert_eq!(
            (metrics.tp, metrics.fp, metrics.tn, metrics.fn_count),
            (0, 2, 4, 0)
        );
        assert_eq!(metrics.precision, Some(0.0));
        assert_eq!(metrics.recall, None);
        assert_eq!(metrics.false_positive_rate, Some(2.0 / 6.0));

        let flagged: Vec<&EvaluatedPair> = report
            .pairs
            .iter()
            .filter(|pair| pair.predicted_rework)
            .collect();
        assert_eq!(flagged.len(), 2);
        assert!(
            flagged
                .iter()
                .all(|pair| pair.rule.as_deref() == Some("repeated-touch"))
        );
    }
}
