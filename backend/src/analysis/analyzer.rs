use anyhow::{Context, Result};
use std::collections::HashSet;

use crate::{
    analysis::{
        complexity::{self, ComplexityResult},
        dependencies,
        history,
        models::{
            build_cochange, build_dependency_analysis, ComplexityAnalysis, RepositoryAnalysis,
            RepositoryInfo,
        },
        parsers::tree_sitter::{self, ParsedSource, TreeSitterAnalyzer},
        propagation_history::{self, PropagationConfig},
        scoring::{self, FileAnalysis},
        source::{self, SourceFile},
    },
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

    let file_paths: HashSet<String> =
        files.iter().map(|file| file.path.clone()).collect();

    let (file_complexity, file_edges) = analyze_source_files(&source_files, &file_paths);

    let (history_analysis, history_metrics) = history::analyze(&commits);

    let temporal = history::temporal_analysis(&commits);

    let dependency_metrics = dependencies::analyze(&file_edges);

    let dependency_analysis = build_dependency_analysis(&dependency_metrics);

    let cochange_analysis = build_cochange(&history_metrics.cochange_pairs);

    let propagation = history::build_propagation(
    &temporal,
    &history_metrics.cochange_pairs,
    &file_edges,
);

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

    let hotspots = scoring::calculate_hotspots(&scoring_files, &history_metrics, &dependency_metrics);

    let difficulty = scoring::calculate_difficulty(&scoring_files, &history_metrics, &dependency_metrics);

    let total_files = files.iter().filter(|file| file.kind == "blob").count();

Ok(RepositoryAnalysis {
    repository: RepositoryInfo {
        name: repository.name,
        default_branch: repository.default_branch,
        total_files,
    },
    source: source_analysis,
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
        complex_files: values
            .iter()
            .filter(|value| **value > threshold)
            .count(),
    }
}
