use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

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
    local::LoadedLocalRepo,
    timing::{AnalysisTimings, elapsed_ms},
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

    let total_files = files.iter().filter(|file| file.kind == "blob").count();
    let tree_paths: HashSet<String> = files.iter().map(|file| file.path.clone()).collect();

    Ok(analyze_loaded_data(
        repository.name,
        repository.default_branch,
        total_files,
        &tree_paths,
        &commits,
        &source_files,
    ))
}

pub async fn analyze_local_repository(raw_path: &str) -> Result<RepositoryAnalysis> {
    let loaded = crate::local::load_local_repo(raw_path).await?;
    run_analysis_blocking(loaded).await
}

pub struct TimedAnalysis {
    pub repository_name: String,
    pub repository_branch: String,
    pub analysis: RepositoryAnalysis,
    pub timings: AnalysisTimings,
}

pub async fn analyze_local_timed(raw_path: &str) -> Result<TimedAnalysis> {
    let total = Instant::now();
    let timed_load = crate::local::load_local_repo_timed(raw_path).await?;
    let mut timings = timed_load.timings;
    let loaded = timed_load.repo;

    let LoadedLocalRepo {
        name,
        default_branch,
        total_files,
        tree,
        commits,
        sources,
        ..
    } = loaded;
    let repository_name = name.clone();
    let repository_branch = default_branch.clone();
    let tree_paths: HashSet<String> = tree.iter().map(|file| file.path.clone()).collect();

    let (analysis, structural_ms, dataset_row_count, dataset_ms) =
        tokio::task::spawn_blocking(move || {
            let stage = Instant::now();
            let analysis = analyze_loaded_data(
                name,
                default_branch,
                total_files,
                &tree_paths,
                &commits,
                &sources,
            );
            let structural_ms = elapsed_ms(stage);
            let stage = Instant::now();
            let rows = build_dataset_from_loaded(&tree, &commits, &sources)?;
            Ok::<_, anyhow::Error>((analysis, structural_ms, rows.len(), elapsed_ms(stage)))
        })
        .await
        .map_err(|error| anyhow::anyhow!("local analysis task failed: {error}"))??;

    timings.structural_analysis_ms = Some(structural_ms);
    timings.dataset_construction_ms = Some(dataset_ms);
    timings.dataset_row_count = dataset_row_count;
    timings.total_ms = Some(elapsed_ms(total));

    Ok(TimedAnalysis {
        repository_name,
        repository_branch,
        analysis,
        timings,
    })
}

async fn run_analysis_blocking(loaded: LoadedLocalRepo) -> Result<RepositoryAnalysis> {
    let LoadedLocalRepo {
        name,
        default_branch,
        total_files,
        tree,
        commits,
        sources,
        ..
    } = loaded;
    let tree_paths: HashSet<String> = tree.iter().map(|file| file.path.clone()).collect();
    tokio::task::spawn_blocking(move || {
        analyze_loaded_data(
            name,
            default_branch,
            total_files,
            &tree_paths,
            &commits,
            &sources,
        )
    })
    .await
    .map_err(|error| anyhow::anyhow!("local analysis task failed: {error}"))
}

#[allow(clippy::too_many_lines)]
fn analyze_loaded_data(
    repo_name: String,
    default_branch: String,
    total_files: usize,
    tree_paths: &HashSet<String>,
    commits: &[Commit],
    source_files: &[SourceFile],
) -> RepositoryAnalysis {
    let source_analysis = source::build_source_analysis(source_files);

    let file_paths: HashSet<String> = tree_paths.clone();

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
    for commit in commits {
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

    RepositoryAnalysis {
        repository: RepositoryInfo {
            name: repo_name,
            default_branch,
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
    }
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

    build_dataset_from_loaded(&files, &commits, &source_files)
}

pub async fn build_ml_dataset_local(raw_path: &str) -> Result<Vec<DatasetRow>> {
    let loaded = crate::local::load_local_repo(raw_path).await?;
    let LoadedLocalRepo {
        tree,
        commits,
        sources,
        ..
    } = loaded;

    tokio::task::spawn_blocking(move || build_dataset_from_loaded(&tree, &commits, &sources))
        .await
        .map_err(|error| anyhow::anyhow!("local dataset task failed: {error}"))?
}

pub struct PredictionInputs {
    pub rows: Vec<DatasetRow>,
    pub commits: Vec<Commit>,
}

pub async fn prediction_inputs(
    client: &GithubClient,
    owner: &str,
    repo: &str,
) -> Result<PredictionInputs> {
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

    Ok(PredictionInputs {
        rows: build_dataset_from_loaded(&files, &commits, &source_files)?,
        commits,
    })
}

pub async fn prediction_inputs_local(raw_path: &str) -> Result<PredictionInputs> {
    let loaded = crate::local::load_local_repo(raw_path).await?;
    let LoadedLocalRepo {
        tree,
        commits,
        sources,
        ..
    } = loaded;

    let (rows, commits) = tokio::task::spawn_blocking(move || {
        let rows = build_dataset_from_loaded(&tree, &commits, &sources)?;
        Ok::<_, anyhow::Error>((rows, commits))
    })
    .await
    .map_err(|error| anyhow::anyhow!("local prediction-input task failed: {error}"))??;

    Ok(PredictionInputs { rows, commits })
}

pub struct RippleInputs {
    pub commits: Vec<Commit>,
    pub edges: Vec<crate::analysis::dependencies::DependencyEdge>,
    pub cochange_pairs: HashMap<(String, String), usize>,
}

fn ripple_inputs_from_loaded(
    commits: &[Commit],
    files: &[RepositoryFile],
    source_files: &[SourceFile],
) -> RippleInputs {
    let file_paths: HashSet<String> = files.iter().map(|file| file.path.clone()).collect();
    let (_, file_edges) = analyze_source_files(source_files, &file_paths);
    let (_, metrics) = history::analyze(commits);
    RippleInputs {
        commits: commits.to_vec(),
        edges: file_edges,
        cochange_pairs: metrics.cochange_pairs,
    }
}

pub async fn ripple_inputs(client: &GithubClient, owner: &str, repo: &str) -> Result<RippleInputs> {
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
    Ok(ripple_inputs_from_loaded(&commits, &files, &source_files))
}

pub async fn ripple_inputs_local(raw_path: &str) -> Result<RippleInputs> {
    let loaded = crate::local::load_local_repo(raw_path).await?;
    let LoadedLocalRepo {
        tree,
        commits,
        sources,
        ..
    } = loaded;
    tokio::task::spawn_blocking(move || ripple_inputs_from_loaded(&commits, &tree, &sources))
        .await
        .map_err(|error| anyhow::anyhow!("local ripple-input task failed: {error}"))
}

pub struct ImpactInputs {
    pub commits: Vec<Commit>,
    pub edges: Vec<crate::analysis::dependencies::DependencyEdge>,
    pub cochange_pairs: HashMap<(String, String), usize>,
    pub file_paths: Vec<String>,
    pub contents: HashMap<String, String>,
}

fn impact_inputs_from_loaded(
    commits: &[Commit],
    files: &[RepositoryFile],
    source_files: &[SourceFile],
) -> ImpactInputs {
    let file_paths: HashSet<String> = files.iter().map(|file| file.path.clone()).collect();
    let (_, file_edges) = analyze_source_files(source_files, &file_paths);
    let (_, metrics) = history::analyze(commits);
    let mut paths: Vec<String> = files
        .iter()
        .filter(|file| file.kind == "blob")
        .map(|file| file.path.clone())
        .collect();
    paths.sort();
    paths.dedup();
    let contents: HashMap<String, String> = source_files
        .iter()
        .map(|source| (source.path.clone(), source.content.clone()))
        .collect();
    ImpactInputs {
        commits: commits.to_vec(),
        edges: file_edges,
        cochange_pairs: metrics.cochange_pairs,
        file_paths: paths,
        contents,
    }
}

pub async fn impact_inputs(client: &GithubClient, owner: &str, repo: &str) -> Result<ImpactInputs> {
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
    Ok(impact_inputs_from_loaded(&commits, &files, &source_files))
}

pub async fn impact_inputs_local(raw_path: &str) -> Result<ImpactInputs> {
    let loaded = crate::local::load_local_repo(raw_path).await?;
    let LoadedLocalRepo {
        tree,
        commits,
        sources,
        ..
    } = loaded;
    tokio::task::spawn_blocking(move || impact_inputs_from_loaded(&commits, &tree, &sources))
        .await
        .map_err(|error| anyhow::anyhow!("local impact-input task failed: {error}"))
}

fn build_dataset_from_loaded(
    files: &[RepositoryFile],
    commits: &[Commit],
    source_files: &[SourceFile],
) -> Result<Vec<DatasetRow>> {
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

    let mean = {
        use statrs::statistics::Statistics;
        values.clone().mean()
    };
    let variance = {
        use statrs::statistics::Statistics;
        values.clone().population_variance()
    };
    let threshold = mean + variance.sqrt();

    ComplexityAnalysis {
        average_complexity: mean,
        max_complexity: values.iter().fold(0, |max, value| max.max(*value as usize)),
        complex_files: values.iter().filter(|value| **value > threshold).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn create(files: &[(&str, &str)], message: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "repoinsight-analyzer-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&dir).expect("create temp repo dir");
            git(&dir, &["init"]);
            git(&dir, &["config", "user.name", "RepoInsight Test"]);
            git(&dir, &["config", "user.email", "test@repoinsight.local"]);
            Self::write(&dir, files, message);
            Self { path: dir }
        }

        fn write(dir: &std::path::Path, files: &[(&str, &str)], message: &str) {
            for (rel, content) in files {
                let absolute = dir.join(rel);
                if let Some(parent) = absolute.parent() {
                    std::fs::create_dir_all(parent).expect("create parent dirs");
                }
                std::fs::write(&absolute, content).expect("write fixture file");
            }
            git(dir, &["add", "."]);
            git(
                dir,
                &[
                    "commit",
                    "-m",
                    message,
                    "--author=RepoInsight Test <test@repoinsight.local>",
                ],
            );
        }

        fn commit(&self, files: &[(&str, &str)], message: &str) {
            Self::write(&self.path, files, message);
        }

        fn path_str(&self) -> String {
            self.path.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.path).ok();
        }
    }

    fn git(dir: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git command runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
    }

    #[test]
    fn timed_pipeline_reports_honest_counts_and_timings() {
        let repo = TempRepo::create(&[("src/main.rs", "fn main() {}\n")], "initial");
        repo.commit(&[("src/lib.rs", "pub fn f() {}\n")], "second");
        let timed = runtime()
            .block_on(analyze_local_timed(&repo.path_str()))
            .expect("timed analysis succeeds");
        let timings = &timed.timings;
        assert_eq!(timings.commit_count, 2);
        assert!(timings.file_count >= 2, "files: {}", timings.file_count);
        assert!(timings.source_file_count >= 2);
        for stage in [
            timings.repository_validation_ms,
            timings.tree_loading_ms,
            timings.commit_loading_ms,
            timings.source_loading_ms,
            timings.structural_analysis_ms,
            timings.dataset_construction_ms,
            timings.total_ms,
        ] {
            assert!(stage.is_some(), "every stage timed: {timings:?}");
        }
        assert!(
            timed.analysis.history.total_commits >= 2,
            "analysis agrees on commits"
        );
    }

    #[test]
    fn timed_pipeline_handles_sparse_repo_without_panicking() {
        let repo = TempRepo::create(&[("notes.bin", "")], "binary only");
        std::fs::write(repo.path.join("notes.bin"), [0xff, 0xfe, 0x00])
            .expect("write binary fixture");
        git(&repo.path, &["add", "."]);
        git(&repo.path, &["commit", "--amend", "--no-edit"]);
        let timed = runtime()
            .block_on(analyze_local_timed(&repo.path_str()))
            .expect("sparse repo still analyzes");
        assert_eq!(timed.timings.source_file_count, 0);
        assert_eq!(timed.timings.commit_count, 1);
        assert!(timed.timings.total_ms.is_some());
    }

    #[test]
    fn timed_pipeline_error_path_fabricates_nothing() {
        let missing = std::env::temp_dir().join("repoinsight-analyzer-definitely-missing-xyz");
        let result = runtime().block_on(analyze_local_timed(&missing.to_string_lossy()));

        assert!(result.is_err(), "missing path must fail");
    }
}
