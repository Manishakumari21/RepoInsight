use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct RepositoryAnalysis {
    pub repository: RepositoryInfo,
    pub source: SourceAnalysis,
    pub structural_features: Vec<crate::analysis::features::StructuralFeatures>,
    pub historical_features: Vec<crate::analysis::historical_features::HistoricalFeatures>,
    pub temporal_features: Vec<crate::analysis::temporal_features::TemporalFeatures>,
    pub complexity: ComplexityAnalysis,
    pub history: HistoryAnalysis,
    pub dependencies: DependencyAnalysis,
    pub cochange: CochangeAnalysis,
    pub temporal: TemporalAnalysis,
    pub propagation: PropagationAnalysis,
    pub timeline: ChangeTimeline,
    pub sequences: ChangeSequences,
    pub followups: FollowUpAnalysis,
    pub rework: ReworkAnalysis,
    pub examples: HistoricalExamples,
    pub hotspots: Vec<Hotspot>,
    pub difficulty: DifficultyScore,
}

#[derive(Debug, Serialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub default_branch: String,
    pub total_files: usize,
}

#[derive(Debug, Serialize)]
pub struct SourceAnalysis {
    pub source_files: usize,
    pub total_lines: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ComplexityAnalysis {
    pub average_complexity: f64,
    pub max_complexity: usize,
    pub complex_files: usize,
}

#[derive(Debug, Serialize)]
pub struct HistoryAnalysis {
    pub total_commits: usize,
    pub active_contributors: usize,
    pub changed_files: usize,
    pub total_additions: u64,
    pub total_deletions: u64,
    pub total_churn: u64,
    pub first_commit: Option<String>,
    pub last_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DependencyAnalysis {
    pub total_dependencies: usize,
    pub connected_files: usize,
    pub highly_connected_files: usize,
    pub resolved_dependencies: usize,
    pub top_coupled_files: Vec<CoupledFile>,
}

#[derive(Debug, Serialize)]
pub struct CoupledFile {
    pub path: String,
    pub coupling: usize,
}

#[derive(Debug, Serialize)]
pub struct CochangeAnalysis {
    pub pairs: Vec<CochangePair>,
    pub total_pairs: usize,
}

#[derive(Debug, Serialize)]
pub struct CochangePair {
    pub file_a: String,
    pub file_b: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct TemporalAnalysis {
    pub pairs: Vec<TemporalPair>,
}

#[derive(Debug, Serialize)]
pub struct TemporalPair {
    pub source: String,
    pub target: String,
    pub occurrences: usize,
    pub avg_delay_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct PropagationAnalysis {
    pub edges: Vec<PropagationEdge>,
}

#[derive(Debug, Serialize)]
pub struct PropagationEdge {
    pub source: String,
    pub target: String,
    pub dependency: bool,
    pub temporal: bool,
    pub cochange: bool,
    pub strength: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeTimeline {
    pub entries: Vec<TimelineEntry>,
    pub total_commits: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub sha: String,
    pub order: usize,
    pub timestamp: Option<i64>,
    pub date: Option<String>,
    pub author: Option<String>,
    pub message: String,
    pub files: Vec<String>,
    pub additions: u64,
    pub deletions: u64,
}

#[derive(Debug, Serialize)]
pub struct ChangeSequences {
    pub sequences: Vec<ChangeSequence>,
    pub window_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct ChangeSequence {
    pub files: Vec<String>,
    pub occurrences: usize,
    pub avg_delay_seconds: i64,
    pub kind: String,
    pub evidence_type: String,
}

#[derive(Debug, Serialize)]
pub struct FollowUpAnalysis {
    pub followups: Vec<FollowUp>,
    pub total: usize,
    pub window_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct FollowUp {
    pub source_sha: String,
    pub source_files: Vec<String>,
    pub followup_sha: String,
    pub followup_files: Vec<String>,
    pub delay_seconds: i64,
    pub reason: String,
    pub signals: Vec<String>,
    pub evidence_type: String,
}

#[derive(Debug, Serialize)]
pub struct ReworkAnalysis {
    pub events: Vec<ReworkEvent>,
    pub total: usize,
    pub window_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct ReworkEvent {
    pub file: String,
    pub initial_sha: String,
    pub rework_sha: String,
    pub delay_seconds: i64,
    pub rule: String,
    pub evidence: String,
}

#[derive(Debug, Serialize)]
pub struct HistoricalExamples {
    pub examples: Vec<HistoricalExample>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct HistoricalExample {
    pub id: String,
    pub source_file: String,
    pub target_file: String,
    pub sequence: Vec<String>,
    pub occurrences: usize,
    pub avg_delay_seconds: i64,
    pub signals: Vec<String>,
    pub evidence_type: String,
    pub example_shas: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Hotspot {
    pub path: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DifficultyScore {
    pub score: f64,
    pub level: String,
}

pub fn build_dependency_analysis(
    metrics: &crate::analysis::dependencies::DependencyMetrics,
) -> DependencyAnalysis {
    let mut top = metrics
        .coupling
        .iter()
        .map(|(path, &coupling)| CoupledFile {
            path: path.clone(),
            coupling,
        })
        .collect::<Vec<_>>();

    top.sort_unstable_by_key(|file| std::cmp::Reverse(file.coupling));
    top.truncate(10);

    DependencyAnalysis {
        total_dependencies: metrics.total_dependencies,
        connected_files: metrics.connected_files,
        highly_connected_files: metrics.highly_connected_files,
        resolved_dependencies: metrics
            .coupling
            .keys()
            .filter(|path| !path.contains('$'))
            .count(),
        top_coupled_files: top,
    }
}

pub fn build_cochange(pairs: &HashMap<(String, String), usize>) -> CochangeAnalysis {
    let mut pairs = pairs
        .iter()
        .map(|((file_a, file_b), &count)| CochangePair {
            file_a: file_a.clone(),
            file_b: file_b.clone(),
            count,
        })
        .collect::<Vec<_>>();

    pairs.sort_unstable_by_key(|pair| std::cmp::Reverse(pair.count));
    pairs.truncate(50);

    CochangeAnalysis {
        total_pairs: pairs.len(),
        pairs,
    }
}
