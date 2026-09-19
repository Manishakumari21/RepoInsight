use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};

use crate::{
    analysis::{
        complexity::{self, ComplexityResult},
        dependencies, features, historical_features, history,
        models::{
            ComplexityAnalysis, RepositoryAnalysis, RepositoryInfo, build_cochange,
            build_dependency_analysis,
        },
        parsers::tree_sitter::{self, ParsedSource, TreeSitterAnalyzer},
        propagation_history::{self, DEFAULT_SEQUENCE_WINDOW_SECS, PropagationConfig},
        scoring::{self, FileAnalysis},
        source::{self, SourceFile},
        temporal_features,
    },
    dataset::{self, DatasetConfig, DatasetRow},
    github::{
        client::GithubClient, commits::Commit, files::RepositoryFile, repository::Repository,
    },
};

pub async fn analyze_repository(
    client: &GithubClient,
    owner: &str,
    repo: &str,
) -> Result<RepositoryAnalysis> {
    let repository = Repository::fetch(client, owner, repo)
        .await
        .with_context(|| format!("failed to fetch repository {owner}/{repo}"))?;

    let files = RepositoryFile::fetch_tree(client, owner, repo, &repository.default_branch)
        .await
        .with_context(|| format!("failed to fetch repository tree for {owner}/{repo}"))?;

    let commits = Commit::fetch(client, owner, repo)
        .await
        .with_context(|| format!("failed to fetch commits for {owner}/{repo}"))?;

    let source_files = source::collect_source_files(client, &files)
        .await
        .with_context(|| format!("failed to collect source files for {owner}/{repo}"))?;

    let source_analysis = source::build_source_analysis(&source_files);

    let file_paths: HashSet<String> = files.iter().map(|file| file.path.clone()).collect();

    let (file_complexity, file_edges) = analyze_source_files(&source_files, &file_paths);

    let (history_analysis, history_metrics) = history::analyze(&commits);

    let historical_features =
        historical_features::build_historical_features(&commits, &history_metrics);

    let temporal = history::temporal_analysis(&commits);

    let dependency_metrics = dependencies::analyze(&file_edges);

    let structural_features = file_complexity
        .iter()
        .filter_map(|file| {
            source_files
                .iter()
                .find(|source| source.path == file.path)
                .map(|source| {
                    features::build_structural_features(
                        source,
                        &file.complexity,
                        &dependency_metrics,
                    )
                })
        })
        .collect::<Vec<_>>();

    let dependency_analysis = build_dependency_analysis(&dependency_metrics);

    let cochange_analysis = build_cochange(&history_metrics.cochange_pairs);

    let propagation =
        history::build_propagation(&temporal, &history_metrics.cochange_pairs, &file_edges);

    let propagation_config = PropagationConfig::default();
    let timeline = propagation_history::build_timeline(&commits);
    let sequences = propagation_history::detect_sequences(&commits, &propagation_config);
    let followups = propagation_history::detect_followups(
        &commits,
        &file_edges,
        &history_metrics.cochange_pairs,
        &propagation_config,
    );
    let rework = propagation_history::detect_rework(
        &commits,
        &file_edges,
        &history_metrics.cochange_pairs,
        &propagation_config,
    );
    let mut followup_frequency = HashMap::new();
    for followup in &followups.followups {
        let files: HashSet<&String> = followup
            .source_files
            .iter()
            .chain(&followup.followup_files)
            .collect();
        for file in files {
            *followup_frequency.entry((*file).clone()).or_insert(0) += 1;
        }
    }

    let mut rework_frequency = HashMap::new();
    for event in &rework.events {
        *rework_frequency.entry(event.file.clone()).or_insert(0) += 1;
    }

    let mut file_times: HashMap<String, Vec<i64>> = HashMap::new();
    for commit in &commits {
        let Some(timestamp) = commit.timestamp else {
            continue;
        };
        for file in history::relevant_files(&commit.files) {
            file_times.entry(file.filename).or_default().push(timestamp);
        }
    }
    for stamps in file_times.values_mut() {
        stamps.sort_unstable();
    }
    let ref_ts = file_times.values().flatten().copied().max().unwrap_or(0);

    let temporal_features = temporal_features::build_temporal_features(
        &temporal,
        &propagation,
        &followup_frequency,
        &rework_frequency,
        &file_times,
        ref_ts,
        DEFAULT_SEQUENCE_WINDOW_SECS,
    );
    let examples = propagation_history::build_examples(
        &commits,
        &temporal,
        &sequences,
        &history_metrics.cochange_pairs,
        &file_edges,
        &propagation_config,
    );

    let complexity_analysis = build_complexity_analysis(&file_complexity);

    let scoring_files = file_complexity
        .iter()
        .map(|file| FileAnalysis {
            path: file.path.clone(),
            complexity: file.complexity,
        })
        .collect::<Vec<_>>();

    let hotspots =
        scoring::calculate_hotspots(&scoring_files, &history_metrics, &dependency_metrics);

    let difficulty =
        scoring::calculate_difficulty(&scoring_files, &history_metrics, &dependency_metrics);

    let total_files = files.iter().filter(|file| file.kind == "blob").count();

    Ok(RepositoryAnalysis {
        repository: RepositoryInfo {
            name: repository.name,
            default_branch: repository.default_branch,
            total_files,
        },
        source: source_analysis,
        structural_features,
        historical_features,
        temporal_features,
        complexity: complexity_analysis,
        history: history_analysis,
        dependencies: dependency_analysis,
        cochange: cochange_analysis,
        temporal,
        propagation,
        timeline,
        sequences,
        followups,
        rework,
        examples,
        hotspots,
        difficulty,
    })
}

pub async fn build_ml_dataset(
    client: &GithubClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<DatasetRow>> {
    let repository = Repository::fetch(client, owner, repo)
        .await
        .with_context(|| format!("failed to fetch repository {owner}/{repo}"))?;

    let files = RepositoryFile::fetch_tree(client, owner, repo, &repository.default_branch)
        .await
        .with_context(|| format!("failed to fetch repository tree for {owner}/{repo}"))?;

    let commits = Commit::fetch(client, owner, repo)
        .await
        .with_context(|| format!("failed to fetch commits for {owner}/{repo}"))?;

    let source_files = source::collect_source_files(client, &files)
        .await
        .with_context(|| format!("failed to collect source files for {owner}/{repo}"))?;

    let file_paths: HashSet<String> = files.iter().map(|file| file.path.clone()).collect();

    let (file_complexity, file_edges) = analyze_source_files(&source_files, &file_paths);

    let dependency_metrics = dependencies::analyze(&file_edges);

    let structural: HashMap<String, features::StructuralFeatures> = file_complexity
        .iter()
        .filter_map(|file| {
            source_files
                .iter()
                .find(|source| source.path == file.path)
                .map(|source| {
                    (
                        file.path.clone(),
                        features::build_structural_features(
                            source,
                            &file.complexity,
                            &dependency_metrics,
                        ),
                    )
                })
        })
        .collect();

    let rows = dataset::build_dataset(
        &commits,
        &structural,
        &file_edges,
        &DatasetConfig::default(),
    );

    let report = dataset::validate_dataset(&rows, &commits);
    if !report.removed.is_empty() {
        let details: Vec<String> = report
            .removed
            .iter()
            .take(5)
            .map(|row| format!("row {}: {}", row.index, row.reason))
            .collect();
        anyhow::bail!(
            "generated dataset failed validation: {}/{} rows invalid ({})",
            report.removed.len(),
            report.total_rows,
            details.join("; ")
        );
    }

    Ok(rows)
}

#[derive(Debug)]
struct FileComplexity {
    path: String,
    complexity: ComplexityResult,
}

fn analyze_source_files(
    source_files: &[SourceFile],
    file_paths: &HashSet<String>,
) -> (Vec<FileComplexity>, Vec<dependencies::DependencyEdge>) {
    let mut complexity = Vec::new();
    let mut edges = Vec::new();

    for file in source_files {
        let Some(language) = tree_sitter::language_from_path(&file.path) else {
            continue;
        };
        let Ok(parsed) = tree_sitter::parse(&file.content, language) else {
            continue;
        };

        complexity.push(FileComplexity {
            path: file.path.clone(),
            complexity: analyze_complexity(&parsed),
        });

        let references = tree_sitter::extract_dependency_references(&file.content, &parsed);
        edges.extend(tree_sitter::dependency_edges(
            &file.path,
            &references,
            file_paths,
        ));
    }

    (complexity, edges)
}

fn analyze_complexity(parsed: &ParsedSource) -> ComplexityResult {
    let analyzer = TreeSitterAnalyzer::new(parsed.language);
    complexity::analyze(parsed.tree.root_node(), &analyzer)
}

fn build_complexity_analysis(results: &[FileComplexity]) -> ComplexityAnalysis {
    if results.is_empty() {
        return ComplexityAnalysis {
            average_complexity: 0.0,
            max_complexity: 0,
            complex_files: 0,
        };
    }

    let values: Vec<f64> = results
        .iter()
        .map(|result| result.complexity.cyclomatic as f64)
        .collect();

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    let threshold = mean + variance.sqrt();

    ComplexityAnalysis {
        average_complexity: mean,
        max_complexity: values.iter().fold(0, |max, value| max.max(*value as usize)),
        complex_files: values.iter().filter(|value| **value > threshold).count(),
    }
}
