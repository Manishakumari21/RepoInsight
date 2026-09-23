use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::dependencies::DependencyEdge;
use super::history::relevant_files;
use crate::github::commits::Commit;

pub const DEFAULT_FOLLOWUP_WINDOW_SECS: i64 = 7 * 24 * 60 * 60;
pub const DEFAULT_RECENT_WINDOW_SECS: i64 = 30 * 24 * 60 * 60;
pub const MAX_RIPPLE_DEPTH: usize = 5;
pub const MAX_CANDIDATES: usize = 10;
pub const DEFAULT_MIN_CONFIDENCE: f64 = 0.5;
pub const MAX_EXAMPLES_PER_EDGE: usize = 5;

#[derive(Debug, Clone, Copy)]
pub struct RippleWeights {
    pub dependency: f64,
    pub cochange: f64,
    pub temporal: f64,
    pub recent: f64,
}

impl Default for RippleWeights {
    fn default() -> Self {
        Self {
            dependency: 0.20,
            cochange: 0.30,
            temporal: 0.35,
            recent: 0.15,
        }
    }
}

impl RippleWeights {
    fn normalized(self) -> Self {
        let total = self.dependency + self.cochange + self.temporal + self.recent;
        if total <= 0.0 {
            return Self::default();
        }
        Self {
            dependency: self.dependency / total,
            cochange: self.cochange / total,
            temporal: self.temporal / total,
            recent: self.recent / total,
        }
    }

    pub fn without(self, group: &str) -> Self {
        match group {
            "structural" => Self {
                dependency: 0.0,
                ..self
            },
            "cochange" => Self {
                cochange: 0.0,
                ..self
            },
            "temporal" => Self {
                temporal: 0.0,
                ..self
            },
            "recent" => Self {
                recent: 0.0,
                ..self
            },
            _ => self,
        }
        .normalized_fallback()
    }

    fn normalized_fallback(self) -> Self {
        let total = self.dependency + self.cochange + self.temporal + self.recent;
        if total <= 0.0 {
            return Self {
                dependency: 0.0,
                cochange: 1.0,
                temporal: 0.0,
                recent: 0.0,
            };
        }
        Self {
            dependency: self.dependency / total,
            cochange: self.cochange / total,
            temporal: self.temporal / total,
            recent: self.recent / total,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RippleConfig {
    pub weights: RippleWeights,
    pub followup_window_secs: i64,
    pub recent_window_secs: i64,
    pub max_depth: usize,
    pub max_candidates: usize,
    pub min_confidence: f64,
}

impl Default for RippleConfig {
    fn default() -> Self {
        Self {
            weights: RippleWeights::default().normalized(),
            followup_window_secs: DEFAULT_FOLLOWUP_WINDOW_SECS,
            recent_window_secs: DEFAULT_RECENT_WINDOW_SECS,
            max_depth: MAX_RIPPLE_DEPTH,
            max_candidates: MAX_CANDIDATES,
            min_confidence: DEFAULT_MIN_CONFIDENCE,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RippleEdge {
    pub source_file: String,
    pub target_file: String,
    pub dependency_score: f64,
    pub cochange_count: usize,
    pub cochange_probability: f64,
    pub follow_count: usize,
    pub follow_probability: f64,
    pub median_follow_delay_seconds: Option<i64>,
    pub avg_follow_delay_seconds: Option<i64>,
    pub recent_follow_probability: f64,
    pub combined_score: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RippleExample {
    pub commit_sha: String,
    pub timestamp: Option<i64>,
    pub date: Option<String>,
    pub event_type: String,
    pub related_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RippleCandidate {
    pub file: String,
    pub probability: f64,
    pub rank: usize,
    pub reasons: Vec<String>,
    pub historical_examples: Vec<RippleExample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RipplePath {
    pub nodes: Vec<String>,
    pub edge_scores: Vec<f64>,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MissingImpact {
    pub file: String,
    pub probability: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RippleModelInfo {
    pub name: String,
    pub calibrated: bool,
    pub threshold: f64,
    pub weights: HashMap<String, f64>,
    pub training: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RippleResponse {
    pub source: String,
    pub candidates: Vec<RippleCandidate>,
    pub ripple_paths: Vec<RipplePath>,
    pub missing_impact: Vec<MissingImpact>,
    pub model: RippleModelInfo,
}

pub struct RippleInputs<'a> {
    pub commits: &'a [Commit],
    pub dependencies: &'a [DependencyEdge],
    pub cochange_pairs: &'a HashMap<(String, String), usize>,
    pub config: RippleConfig,
}

impl<'a> RippleInputs<'a> {
    pub fn from_prefix(
        commits: &'a [Commit],
        dependencies: &'a [DependencyEdge],
        cochange_pairs: &'a HashMap<(String, String), usize>,
        cutoff_ts: i64,
        config: RippleConfig,
    ) -> Self {
        let _ = cutoff_ts;

        Self {
            commits,
            dependencies,
            cochange_pairs,
            config,
        }
    }
}

fn canonical(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    }
}

fn cochange_count(pairs: &HashMap<(String, String), usize>, a: &str, b: &str) -> usize {
    pairs.get(&canonical(a, b)).copied().unwrap_or(0)
}

fn change_counts(commits: &[Commit]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for commit in commits {
        let mut seen = HashSet::new();
        for file in relevant_files(&commit.files) {
            if seen.insert(file.filename.clone()) {
                *counts.entry(file.filename).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn dependency_exists(dependencies: &[DependencyEdge], a: &str, b: &str) -> bool {
    dependencies.iter().any(|edge| {
        (edge.source == a && edge.target == b) || (edge.source == b && edge.target == a)
    })
}

fn sorted_commits(commits: &[Commit]) -> Vec<&Commit> {
    let mut ordered: Vec<(usize, &Commit)> = commits.iter().enumerate().collect();
    ordered.sort_by_key(|(index, commit)| (commit.timestamp.unwrap_or(0), *index));
    ordered.into_iter().map(|(_, commit)| commit).collect()
}

fn follow_delays(commits: &[Commit], source: &str, target: &str, window_secs: i64) -> Vec<i64> {
    let ordered = sorted_commits(commits);
    let sets: Vec<(Option<i64>, HashSet<String>)> = ordered
        .iter()
        .map(|commit| {
            (
                commit.timestamp,
                relevant_files(&commit.files)
                    .into_iter()
                    .map(|file| file.filename)
                    .collect(),
            )
        })
        .collect();
    let mut delays = Vec::new();
    for (i, (t0, files)) in sets.iter().enumerate() {
        let Some(t0) = *t0 else { continue };
        if !files.contains(source) {
            continue;
        }
        for (t1, later) in sets.iter().skip(i + 1) {
            let Some(t1) = *t1 else { continue };
            let delay = t1 - t0;
            if delay < 0 {
                continue;
            }
            if delay > window_secs {
                break;
            }
            if later.contains(target) {
                delays.push(delay);
            }
        }
    }
    delays.sort_unstable();
    delays
}

fn median(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    Some(values[values.len() / 2])
}

fn mean(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<i64>() / values.len() as i64)
}

fn recent_follow_delays(
    commits: &[Commit],
    source: &str,
    target: &str,
    window_secs: i64,
    recent_window_secs: i64,
) -> Vec<i64> {
    let max_ts = commits.iter().filter_map(|commit| commit.timestamp).max();
    let Some(max_ts) = max_ts else {
        return Vec::new();
    };
    follow_delays(commits, source, target, window_secs)
        .into_iter()
        .zip(follow_target_times(commits, source, target, window_secs))
        .filter(|(_, target_time)| max_ts - target_time <= recent_window_secs)
        .map(|(delay, _)| delay)
        .collect()
}

fn follow_target_times(
    commits: &[Commit],
    source: &str,
    target: &str,
    window_secs: i64,
) -> Vec<i64> {
    let ordered = sorted_commits(commits);
    let sets: Vec<(Option<i64>, HashSet<String>)> = ordered
        .iter()
        .map(|commit| {
            (
                commit.timestamp,
                relevant_files(&commit.files)
                    .into_iter()
                    .map(|file| file.filename)
                    .collect(),
            )
        })
        .collect();
    let mut times = Vec::new();
    for (i, (t0, files)) in sets.iter().enumerate() {
        let Some(t0) = *t0 else { continue };
        if !files.contains(source) {
            continue;
        }
        for (t1, later) in sets.iter().skip(i + 1) {
            let Some(t1) = *t1 else { continue };
            let delay = t1 - t0;
            if delay < 0 {
                continue;
            }
            if delay > window_secs {
                break;
            }
            if later.contains(target) {
                times.push(t1);
            }
        }
    }
    times.sort_unstable();
    times
}

fn candidate_files(
    commits: &[Commit],
    dependencies: &[DependencyEdge],
    source: &str,
) -> Vec<String> {
    let mut files = HashSet::new();
    for commit in commits {
        for file in relevant_files(&commit.files) {
            if file.filename != source {
                files.insert(file.filename);
            }
        }
    }
    for edge in dependencies {
        if edge.source == source && edge.target != source {
            files.insert(edge.target.clone());
        } else if edge.target == source && edge.source != source {
            files.insert(edge.source.clone());
        }
    }
    let mut out: Vec<String> = files.into_iter().collect();
    out.sort();
    out
}

fn evidence_for(
    source: &str,
    target: &str,
    has_dep: bool,
    co_count: usize,
    follow_count: usize,
    median_delay: Option<i64>,
    recent_prob: f64,
) -> Vec<String> {
    let mut evidence = Vec::new();
    if has_dep {
        evidence.push(format!(
            "Structural dependency relationship between {source} and {target}."
        ));
    }
    if co_count > 0 {
        evidence.push(format!(
            "Changed together in {co_count} historical commit{}.",
            if co_count == 1 { "" } else { "s" }
        ));
    }
    if follow_count > 0 {
        let delay = median_delay
            .map(|delay| format!(" Median historical delay: {}s.", delay))
            .unwrap_or_default();
        evidence.push(format!(
            "{target} followed {source} in {follow_count} comparable change{}.{delay}",
            if follow_count == 1 { "" } else { "s" }
        ));
    }
    if recent_prob > 0.0 {
        evidence.push("Recent coupling increased.".to_owned());
    }
    evidence
}

pub fn build_ripple_edges(
    source: &str,
    commits: &[Commit],
    dependencies: &[DependencyEdge],
    cochange_pairs: &HashMap<(String, String), usize>,
    config: &RippleConfig,
) -> Vec<RippleEdge> {
    let weights = config.weights.normalized();
    let counts = change_counts(commits);
    let source_changes = counts.get(source).copied().unwrap_or(0) as f64;
    let mut edges = Vec::new();
    for target in candidate_files(commits, dependencies, source) {
        let has_dep = dependency_exists(dependencies, source, &target);
        let dependency_score = if has_dep { 1.0 } else { 0.0 };
        let co_count = cochange_count(cochange_pairs, source, &target);
        let co_prob = if source_changes > 0.0 {
            co_count as f64 / source_changes
        } else {
            0.0
        };
        let delays = follow_delays(commits, source, &target, config.followup_window_secs);
        let follow_count = delays.len();
        let follow_prob = if source_changes > 0.0 {
            follow_count as f64 / source_changes
        } else {
            0.0
        };
        let recent = recent_follow_delays(
            commits,
            source,
            &target,
            config.followup_window_secs,
            config.recent_window_secs,
        );
        let recent_prob = if source_changes > 0.0 {
            recent.len() as f64 / source_changes
        } else {
            0.0
        };
        let combined = weights.dependency * dependency_score
            + weights.cochange * co_prob.clamp(0.0, 1.0)
            + weights.temporal * follow_prob.clamp(0.0, 1.0)
            + weights.recent * recent_prob.clamp(0.0, 1.0);
        edges.push(RippleEdge {
            source_file: source.to_owned(),
            target_file: target.clone(),
            dependency_score,
            cochange_count: co_count,
            cochange_probability: co_prob,
            follow_count,
            follow_probability: follow_prob,
            median_follow_delay_seconds: median(&delays),
            avg_follow_delay_seconds: mean(&delays),
            recent_follow_probability: recent_prob,
            combined_score: combined.clamp(0.0, 1.0),
            evidence: evidence_for(
                source,
                &target,
                has_dep,
                co_count,
                follow_count,
                median(&delays),
                recent_prob,
            ),
        });
    }
    edges.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.target_file.cmp(&b.target_file))
    });
    edges
}

fn historical_examples_for(
    commits: &[Commit],
    source: &str,
    target: &str,
    window_secs: i64,
) -> Vec<RippleExample> {
    let ordered = sorted_commits(commits);
    let mut examples = Vec::new();
    for commit in &ordered {
        let files: HashSet<String> = relevant_files(&commit.files)
            .into_iter()
            .map(|file| file.filename)
            .collect();
        if files.contains(source) && files.contains(target) {
            let related: Vec<String> = {
                let mut related: Vec<String> = files.iter().cloned().collect();
                related.sort();
                related
            };
            examples.push(RippleExample {
                commit_sha: commit.sha.clone(),
                timestamp: commit.timestamp,
                date: commit.date.clone(),
                event_type: "co_change".to_owned(),
                related_files: related,
            });
        }
    }

    let sets: Vec<(&Commit, HashSet<String>)> = ordered
        .iter()
        .map(|commit| {
            (
                *commit,
                relevant_files(&commit.files)
                    .into_iter()
                    .map(|file| file.filename)
                    .collect(),
            )
        })
        .collect();
    for (i, (commit, files)) in sets.iter().enumerate() {
        if !files.contains(source) || files.contains(target) {
            continue;
        }
        let Some(t0) = commit.timestamp else {
            continue;
        };
        for (later, later_files) in sets.iter().skip(i + 1) {
            let Some(t1) = later.timestamp else {
                continue;
            };
            if t1 - t0 > window_secs || t1 < t0 {
                break;
            }
            if later_files.contains(target) {
                examples.push(RippleExample {
                    commit_sha: later.sha.clone(),
                    timestamp: later.timestamp,
                    date: later.date.clone(),
                    event_type: "follow_up".to_owned(),
                    related_files: {
                        let mut related: Vec<String> = later_files.iter().cloned().collect();
                        related.sort();
                        related
                    },
                });
                break;
            }
        }
    }
    examples.sort_by(|a, b| {
        b.timestamp
            .unwrap_or(0)
            .cmp(&a.timestamp.unwrap_or(0))
            .then_with(|| a.commit_sha.cmp(&b.commit_sha))
            .then_with(|| a.event_type.cmp(&b.event_type))
    });
    examples.dedup_by(|a, b| a.commit_sha == b.commit_sha && a.event_type == b.event_type);
    examples.truncate(MAX_EXAMPLES_PER_EDGE);
    examples
}

fn reasons_for(edge: &RippleEdge) -> Vec<String> {
    let mut reasons = Vec::new();
    if edge.dependency_score > 0.0 {
        reasons.push("Strong dependency relationship.".to_owned());
    }
    if edge.cochange_count > 0 {
        reasons.push(format!(
            "Changed together in {} historical commit{}.",
            edge.cochange_count,
            if edge.cochange_count == 1 { "" } else { "s" }
        ));
    }
    if edge.follow_count > 0 {
        reasons.push(format!(
            "Followed in {} comparable change{}.",
            edge.follow_count,
            if edge.follow_count == 1 { "" } else { "s" }
        ));
    }
    if let Some(delay) = edge.median_follow_delay_seconds {
        reasons.push(format!("Median historical delay: {delay}s."));
    }
    if edge.recent_follow_probability > 0.0 {
        reasons.push("Recent coupling increased.".to_owned());
    }
    if reasons.is_empty() {
        reasons.push("Weak historical relationship.".to_owned());
    }
    reasons
}

pub fn rank_candidates(
    edges: &[RippleEdge],
    commits: &[Commit],
    config: &RippleConfig,
) -> Vec<RippleCandidate> {
    edges
        .iter()
        .filter(|edge| edge.combined_score >= config.min_confidence)
        .take(config.max_candidates.max(1))
        .enumerate()
        .map(|(index, edge)| RippleCandidate {
            file: edge.target_file.clone(),
            probability: edge.combined_score,
            rank: index + 1,
            reasons: reasons_for(edge),
            historical_examples: historical_examples_for(
                commits,
                &edge.source_file,
                &edge.target_file,
                config.followup_window_secs,
            ),
        })
        .collect()
}

pub fn build_ripple_path(
    source: &str,
    edges: &[RippleEdge],
    config: &RippleConfig,
) -> Vec<RipplePath> {
    let scores: HashMap<(&str, &str), &RippleEdge> = edges
        .iter()
        .map(|edge| ((edge.source_file.as_str(), edge.target_file.as_str()), edge))
        .collect();
    let mut nodes = vec![source.to_owned()];
    let mut visited = HashSet::from([source.to_owned()]);
    let mut edge_scores = Vec::new();
    let mut evidence = Vec::new();
    let mut current = source.to_owned();
    for _ in 0..config.max_depth.max(1) {
        let mut best: Option<&RippleEdge> = None;
        for edge in edges {
            if edge.source_file != current {
                continue;
            }
            if visited.contains(&edge.target_file) {
                continue;
            }
            if edge.target_file == edge.source_file {
                continue;
            }
            if edge.combined_score < config.min_confidence {
                continue;
            }
            if best.is_none_or(|current_best: &RippleEdge| {
                edge.combined_score > current_best.combined_score
            }) {
                best = Some(edge);
            }
        }
        let Some(next) = best else {
            break;
        };

        let _ = &scores;
        visited.insert(next.target_file.clone());
        edge_scores.push(next.combined_score);
        evidence.extend(next.evidence.clone());
        nodes.push(next.target_file.clone());
        current = next.target_file.clone();
    }
    if edge_scores.is_empty() {
        return Vec::new();
    }
    let confidence = edge_scores.iter().sum::<f64>() / edge_scores.len() as f64;
    evidence.sort();
    evidence.dedup();
    vec![RipplePath {
        nodes,
        edge_scores,
        confidence,
        evidence,
    }]
}

pub fn detect_missing_impact(
    candidates: &[RippleCandidate],
    changed_set: &HashSet<String>,
) -> Vec<MissingImpact> {
    candidates
        .iter()
        .filter(|candidate| !changed_set.contains(&candidate.file))
        .map(|candidate| MissingImpact {
            file: candidate.file.clone(),
            probability: candidate.probability,
            message: format!(
                "These files frequently changed in comparable historical changes. Consider reviewing {} (historical follow probability: {}%).",
                candidate.file,
                (candidate.probability * 100.0).round() as u64
            ),
        })
        .collect()
}

fn model_info(config: &RippleConfig) -> RippleModelInfo {
    let weights = HashMap::from([
        ("dependency".to_owned(), config.weights.dependency),
        ("cochange".to_owned(), config.weights.cochange),
        ("temporal".to_owned(), config.weights.temporal),
        ("recent".to_owned(), config.weights.recent),
    ]);
    RippleModelInfo {
        name: "ripple_baseline".to_owned(),
        calibrated: false,
        threshold: config.min_confidence,
        weights,
        training: "Deterministic baseline integrating structural, co-change, temporal and recent signals; no trained weights. See analysis/ripple.rs.".to_owned(),
    }
}

pub fn forecast(
    source: &str,
    changed_set: &HashSet<String>,
    commits: &[Commit],
    dependencies: &[DependencyEdge],
    cochange_pairs: &HashMap<(String, String), usize>,
    config: &RippleConfig,
) -> RippleResponse {
    let edges = build_ripple_edges(source, commits, dependencies, cochange_pairs, config);
    let candidates = rank_candidates(&edges, commits, config);
    let ripple_paths = build_ripple_path(source, &edges, config);
    let missing_impact = detect_missing_impact(&candidates, changed_set);
    RippleResponse {
        source: source.to_owned(),
        candidates,
        ripple_paths,
        missing_impact,
        model: model_info(config),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RippleEvaluation {
    pub precision_at_k: Option<f64>,
    pub recall_at_k: Option<f64>,
    pub mrr: Option<f64>,
    pub map: Option<f64>,
    pub evaluated_targets: usize,
    pub k: usize,
}

pub fn evaluate_ripple(
    commits: &[Commit],
    dependencies: &[DependencyEdge],
    cochange_pairs_full: &HashMap<(String, String), usize>,
    config: &RippleConfig,
    k: usize,
) -> RippleEvaluation {
    let k = k.max(1);
    let mut ordered: Vec<&Commit> = commits
        .iter()
        .filter(|commit| commit.timestamp.is_some())
        .collect();
    ordered.sort_by_key(|commit| (commit.timestamp.unwrap_or(0), commit.sha.clone()));
    if ordered.len() < 3 {
        return RippleEvaluation {
            precision_at_k: None,
            recall_at_k: None,
            mrr: None,
            map: None,
            evaluated_targets: 0,
            k,
        };
    }
    let split_at = ordered.len() * 7 / 10;
    let test = &ordered[split_at..];
    let mut precisions = Vec::new();
    let mut recalls = Vec::new();
    let mut reciprocal_ranks = Vec::new();
    let mut average_precisions = Vec::new();
    let mut evaluated = 0;
    for target in test {
        let Some(target_ts) = target.timestamp else {
            continue;
        };
        let mut files: Vec<String> = relevant_files(&target.files)
            .into_iter()
            .map(|file| file.filename)
            .collect();
        files.sort();
        files.dedup();
        if files.len() < 2 {
            continue;
        }
        let prefix: Vec<Commit> = ordered
            .iter()
            .filter(|commit| commit.timestamp.is_some_and(|ts| ts < target_ts))
            .map(|commit| (*commit).clone())
            .collect();
        if prefix.is_empty() {
            continue;
        }

        let (_, prefix_metrics) = super::history::analyze(&prefix);
        let source = files[0].clone();
        let ground: HashSet<String> = files[1..].iter().cloned().collect();
        let edges = build_ripple_edges(
            &source,
            &prefix,
            dependencies,
            &prefix_metrics.cochange_pairs,
            config,
        );
        let ranked: Vec<String> = edges
            .into_iter()
            .map(|edge| edge.target_file)
            .take(k)
            .collect();
        if ranked.is_empty() {
            continue;
        }
        evaluated += 1;
        let hits = ranked.iter().filter(|file| ground.contains(*file)).count();
        precisions.push(hits as f64 / ranked.len() as f64);
        recalls.push(hits as f64 / ground.len() as f64);
        let first_hit = ranked
            .iter()
            .position(|file| ground.contains(file))
            .map(|position| 1.0 / (position + 1) as f64)
            .unwrap_or(0.0);
        reciprocal_ranks.push(first_hit);
        let mut relevant_seen = 0;
        let mut precision_sum = 0.0;
        for (index, file) in ranked.iter().enumerate() {
            if ground.contains(file) {
                relevant_seen += 1;
                precision_sum += relevant_seen as f64 / (index + 1) as f64;
            }
        }
        average_precisions.push(if relevant_seen > 0 {
            precision_sum / ground.len() as f64
        } else {
            0.0
        });
    }
    let _ = cochange_pairs_full;
    let mean = |values: &[f64]| {
        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f64>() / values.len() as f64)
        }
    };
    RippleEvaluation {
        precision_at_k: mean(&precisions),
        recall_at_k: mean(&recalls),
        mrr: mean(&reciprocal_ranks),
        map: mean(&average_precisions),
        evaluated_targets: evaluated,
        k,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::commits::{ChangedFile, CommitStats, FileChangeStatus};

    fn file(name: &str) -> ChangedFile {
        ChangedFile {
            filename: name.to_owned(),
            additions: 1,
            deletions: 0,
            changes: 1,
            status: FileChangeStatus::Modified,
        }
    }

    fn commit(sha: &str, ts: i64, files: &[&str]) -> Commit {
        Commit {
            sha: sha.to_owned(),
            message: format!("commit {sha}"),
            author: None,
            timestamp: Some(ts),
            date: None,
            url: String::new(),
            stats: CommitStats::default(),
            files: files.iter().map(|name| file(name)).collect(),
        }
    }

    fn cochange(commits: &[Commit]) -> HashMap<(String, String), usize> {
        let (_, metrics) = super::super::history::analyze(commits);
        metrics.cochange_pairs
    }

    fn config() -> RippleConfig {
        RippleConfig {
            min_confidence: 0.0,
            ..RippleConfig::default()
        }
    }

    #[test]
    fn dependency_signal_fires_on_structural_edge() {
        let commits = vec![commit("c1", 100, &["a.rs"]), commit("c2", 200, &["b.rs"])];
        let deps = vec![DependencyEdge {
            source: "a.rs".to_owned(),
            target: "b.rs".to_owned(),
        }];
        let edges = build_ripple_edges("a.rs", &commits, &deps, &cochange(&commits), &config());
        let edge = edges
            .iter()
            .find(|edge| edge.target_file == "b.rs")
            .unwrap();
        assert_eq!(edge.dependency_score, 1.0);
        assert!(edge.evidence.iter().any(|line| line.contains("dependency")));
    }

    #[test]
    fn cochange_probability_is_directional() {
        let commits = vec![
            commit("c1", 100, &["a.rs", "b.rs"]),
            commit("c2", 200, &["a.rs"]),
            commit("c3", 300, &["a.rs"]),
        ];
        let edges = build_ripple_edges("a.rs", &commits, &[], &cochange(&commits), &config());
        let edge = edges
            .iter()
            .find(|edge| edge.target_file == "b.rs")
            .unwrap();
        assert_eq!(edge.cochange_count, 1);
        assert!((edge.cochange_probability - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn temporal_follow_excludes_same_commit() {
        let commits = vec![commit("c1", 100, &["a.rs", "b.rs"])];
        let edges = build_ripple_edges("a.rs", &commits, &[], &cochange(&commits), &config());
        let edge = edges
            .iter()
            .find(|edge| edge.target_file == "b.rs")
            .unwrap();
        assert_eq!(edge.follow_count, 0);
        assert_eq!(edge.median_follow_delay_seconds, None);

        assert_eq!(edge.cochange_count, 1);
    }

    #[test]
    fn combined_score_is_weighted_and_bounded() {
        let commits = vec![commit("c1", 100, &["a.rs", "b.rs"])];
        let deps = vec![DependencyEdge {
            source: "a.rs".to_owned(),
            target: "b.rs".to_owned(),
        }];
        let edges = build_ripple_edges("a.rs", &commits, &deps, &cochange(&commits), &config());
        let edge = edges
            .iter()
            .find(|edge| edge.target_file == "b.rs")
            .unwrap();
        assert!((0.0..=1.0).contains(&edge.combined_score));
        assert!(edge.combined_score > 0.0);
    }

    #[test]
    fn candidates_ranked_and_capped() {
        let commits = vec![
            commit("c1", 100, &["a.rs", "b.rs"]),
            commit("c2", 200, &["a.rs", "c.rs"]),
        ];
        let edges = build_ripple_edges("a.rs", &commits, &[], &cochange(&commits), &config());
        let candidates = rank_candidates(
            &edges,
            &commits,
            &RippleConfig {
                max_candidates: 1,
                min_confidence: 0.0,
                ..RippleConfig::default()
            },
        );
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rank, 1);
        assert!(!candidates[0].reasons.is_empty());
        assert!(!candidates[0].historical_examples.is_empty());
    }

    #[test]
    fn ripple_path_avoids_cycles_and_respects_depth() {
        let commits = vec![
            commit("c1", 100, &["a.rs", "b.rs"]),
            commit("c2", 200, &["b.rs", "c.rs"]),
            commit("c3", 300, &["c.rs", "a.rs"]),
        ];
        let edges = build_ripple_edges("a.rs", &commits, &[], &cochange(&commits), &config());

        let mut chain = edges;
        chain.push(RippleEdge {
            source_file: "b.rs".to_owned(),
            target_file: "c.rs".to_owned(),
            dependency_score: 0.0,
            cochange_count: 1,
            cochange_probability: 0.5,
            follow_count: 1,
            follow_probability: 0.5,
            median_follow_delay_seconds: Some(100),
            avg_follow_delay_seconds: Some(100),
            recent_follow_probability: 0.0,
            combined_score: 0.9,
            evidence: vec!["test".to_owned()],
        });
        let paths = build_ripple_path(
            "a.rs",
            &chain,
            &RippleConfig {
                max_depth: 2,
                min_confidence: 0.0,
                ..RippleConfig::default()
            },
        );
        assert_eq!(paths.len(), 1);
        assert!(paths[0].nodes.len() <= 3);
        let mut seen = HashSet::new();
        for node in &paths[0].nodes {
            assert!(seen.insert(node.clone()), "cycle in {node}");
        }
    }

    #[test]
    fn weak_evidence_produces_no_path() {
        let gap = DEFAULT_FOLLOWUP_WINDOW_SECS + 1;
        let commits = vec![
            commit("c1", 100, &["a.rs"]),
            commit("c2", 100 + gap, &["b.rs"]),
        ];
        let edges = build_ripple_edges(
            "a.rs",
            &commits,
            &[],
            &cochange(&commits),
            &RippleConfig::default(),
        );

        assert!(build_ripple_path("a.rs", &edges, &RippleConfig::default()).is_empty());
    }

    #[test]
    fn missing_impact_flags_uncovered_files_neutrally() {
        let commits = vec![commit("c1", 100, &["a.rs", "b.rs"])];
        let edges = build_ripple_edges("a.rs", &commits, &[], &cochange(&commits), &config());
        let candidates = rank_candidates(&edges, &commits, &config());
        let changed = HashSet::from(["a.rs".to_owned()]);
        let missing = detect_missing_impact(&candidates, &changed);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].message.contains("Consider reviewing"));
        assert!(!missing[0].message.to_ascii_lowercase().contains("mistake"));
        let covered = HashSet::from(["a.rs".to_owned(), "b.rs".to_owned()]);
        assert!(detect_missing_impact(&candidates, &covered).is_empty());
    }

    #[test]
    fn empty_and_single_file_histories_are_safe() {
        let empty: Vec<Commit> = Vec::new();
        assert!(build_ripple_edges("a.rs", &empty, &[], &HashMap::new(), &config()).is_empty());
        let single = vec![commit("c1", 100, &["only.rs"])];
        assert!(
            build_ripple_edges("only.rs", &single, &[], &cochange(&single), &config()).is_empty()
        );
    }

    #[test]
    fn unrelated_files_score_zero_without_fabrication() {
        let gap = DEFAULT_FOLLOWUP_WINDOW_SECS + 1;
        let commits = vec![
            commit("c1", 100, &["a.rs"]),
            commit("c2", 100 + gap, &["z.rs"]),
        ];
        let edges = build_ripple_edges("a.rs", &commits, &[], &cochange(&commits), &config());
        let edge = edges
            .iter()
            .find(|edge| edge.target_file == "z.rs")
            .unwrap();
        assert_eq!(edge.cochange_count, 0);
        assert_eq!(edge.follow_count, 0);
        assert_eq!(edge.median_follow_delay_seconds, None);
        assert_eq!(edge.combined_score, 0.0);
    }

    #[test]
    fn leakage_prefix_excludes_target_and_future() {
        let commits = vec![
            commit("c1", 100, &["a.rs", "b.rs"]),
            commit("c2", 200, &["a.rs", "b.rs"]),
            commit("c3", 300, &["a.rs", "c.rs"]),
        ];
        let prefix: Vec<Commit> = commits
            .iter()
            .filter(|commit| commit.timestamp.is_some_and(|ts| ts < 300))
            .cloned()
            .collect();
        let edges = build_ripple_edges("a.rs", &prefix, &[], &cochange(&prefix), &config());

        assert!(edges.iter().all(|edge| edge.target_file != "c.rs"));
        let b = edges
            .iter()
            .find(|edge| edge.target_file == "b.rs")
            .unwrap();
        assert_eq!(b.cochange_count, 2);
    }

    #[test]
    fn evaluation_is_chronological_and_bounded() {
        let commits = vec![
            commit("c1", 100, &["a.rs", "b.rs"]),
            commit("c2", 200, &["a.rs", "b.rs"]),
            commit("c3", 300, &["a.rs", "c.rs"]),
            commit("c4", 400, &["a.rs", "b.rs"]),
            commit("c5", 500, &["a.rs", "c.rs"]),
        ];
        let report = evaluate_ripple(&commits, &[], &cochange(&commits), &config(), 2);
        assert!(report.evaluated_targets >= 1);
        assert!(report.precision_at_k.is_some());
        assert!(report.recall_at_k.is_some());
    }
}
