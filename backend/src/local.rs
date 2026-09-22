use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use chrono::DateTime;
use tokio::process::Command;

use crate::analysis::source::SourceFile;
use crate::github::commits::{ChangedFile, Commit, CommitAuthor, CommitStats, FileChangeStatus};
use crate::github::files::RepositoryFile;
use crate::timing::{AnalysisTimings, elapsed_ms};

pub const MAX_LOCAL_COMMITS: usize = 1000;

#[derive(Debug, Clone, thiserror::Error)]
pub enum LocalRepoError {
    #[error("Repository path does not exist.")]
    NotFound,
    #[error("Path is not a directory.")]
    NotDirectory,
    #[error("The selected directory is not a Git repository.")]
    NotGit,
    #[error("Unable to read Git history from the repository.")]
    GitError,
    #[error("RepoInsight cannot access this repository.")]
    Permission,
}

pub struct LoadedLocalRepo {
    pub name: String,
    pub default_branch: String,
    #[allow(dead_code)]
    pub canonical_path: PathBuf,
    pub tree: Vec<RepositoryFile>,
    pub commits: Vec<Commit>,
    pub sources: Vec<SourceFile>,
    pub total_files: usize,
}

pub fn validate_local_path(raw: &str) -> Result<PathBuf, LocalRepoError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LocalRepoError::NotFound);
    }
    let candidate = PathBuf::from(trimmed);
    let metadata = std::fs::symlink_metadata(&candidate).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => LocalRepoError::NotFound,
        std::io::ErrorKind::PermissionDenied => LocalRepoError::Permission,
        _ => LocalRepoError::GitError,
    })?;
    if !metadata.is_dir() {
        return Err(LocalRepoError::NotDirectory);
    }
    let canonical = std::fs::canonicalize(&candidate).map_err(|error| match error.kind() {
        std::io::ErrorKind::PermissionDenied => LocalRepoError::Permission,
        _ => LocalRepoError::GitError,
    })?;
    if !canonical.is_dir() {
        return Err(LocalRepoError::NotDirectory);
    }
    Ok(canonical)
}

async fn run_git(path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .await
        .with_context(|| "Unable to read Git history from the repository.")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") || stderr.contains("not a git directory") {
            bail!(LocalRepoError::NotGit);
        }
        if stderr.contains("Permission denied") || stderr.contains("permission denied") {
            bail!(LocalRepoError::Permission);
        }
        bail!(LocalRepoError::GitError);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub async fn ensure_git_repo(path: &Path) -> Result<(), LocalRepoError> {
    match run_git(path, &["rev-parse", "--git-dir"]).await {
        Ok(_) => Ok(()),
        Err(error) => {
            if let Some(local) = error.downcast_ref::<LocalRepoError>() {
                return Err(local.clone());
            }
            Err(LocalRepoError::GitError)
        }
    }
}

pub async fn load_local_repo(raw_path: &str) -> Result<LoadedLocalRepo> {
    load_local_repo_timed(raw_path)
        .await
        .map(|timed| timed.repo)
}

/// Loaded repository plus honest per-stage timings (Phase 9
/// instrumentation). Stages that did not complete stay `None`.
pub struct TimedLocalRepo {
    pub repo: LoadedLocalRepo,
    pub timings: AnalysisTimings,
}

pub async fn load_local_repo_timed(raw_path: &str) -> Result<TimedLocalRepo> {
    let mut timings = AnalysisTimings::default();

    let stage = Instant::now();
    let canonical = validate_local_path(raw_path).map_err(|error| anyhow!(error.to_string()))?;
    timings.repository_validation_ms = Some(elapsed_ms(stage));

    ensure_git_repo(&canonical)
        .await
        .map_err(|error| anyhow!(error.to_string()))?;

    let name = canonical
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| canonical.to_string_lossy().into_owned());

    let default_branch = run_git(&canonical, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map(|branch| branch.trim().to_owned())
        .map(|branch| {
            if branch.is_empty() || branch == "HEAD" {
                "HEAD".to_owned()
            } else {
                branch
            }
        })
        .unwrap_or_else(|_| "HEAD".to_owned());

    let stage = Instant::now();
    let tree = load_tree(&canonical, &default_branch).await?;
    timings.tree_loading_ms = Some(elapsed_ms(stage));
    let total_files = tree.iter().filter(|file| file.kind == "blob").count();

    let stage = Instant::now();
    let commits = load_commits(&canonical).await?;
    timings.commit_loading_ms = Some(elapsed_ms(stage));

    // Blocking file I/O over the whole working tree: isolate it on the
    // blocking pool so async workers stay responsive to e.g. /health.
    let stage = Instant::now();
    let sources = tokio::task::spawn_blocking({
        let canonical = canonical.clone();
        let tree = tree.clone();
        move || load_sources(&canonical, &tree)
    })
    .await
    .map_err(|error| anyhow!("source loading task failed: {error}"))??;
    timings.source_loading_ms = Some(elapsed_ms(stage));

    timings.file_count = total_files;
    timings.commit_count = commits.len();
    timings.source_file_count = sources.len();

    Ok(TimedLocalRepo {
        repo: LoadedLocalRepo {
            name,
            default_branch,
            canonical_path: canonical,
            tree,
            commits,
            sources,
            total_files,
        },
        timings,
    })
}

async fn load_tree(path: &Path, branch: &str) -> Result<Vec<RepositoryFile>> {
    let revision = if branch.is_empty() { "HEAD" } else { branch };
    let output = match run_git(path, &["ls-tree", "-r", "-l", "-z", revision]).await {
        Ok(output) => output,
        Err(_) => run_git(path, &["ls-tree", "-r", "-l", "-z", "HEAD"]).await?,
    };

    let mut files = Vec::new();
    for entry in output.split('\0') {
        if entry.trim().is_empty() {
            continue;
        }
        // Format: "<mode> <type> <sha>\t<size>\t<path>"
        // Size may be "-" for blobs; fallback to filesystem size.
        let Some((meta, rel)) = entry.split_once('\t') else {
            continue;
        };
        let meta_parts: Vec<&str> = meta.split_whitespace().collect();
        if meta_parts.len() < 3 {
            continue;
        }
        let kind = meta_parts[1].to_owned();
        let sha = meta_parts[2].to_owned();
        let mut segments = rel.split('\t');
        let size_token = segments.next().unwrap_or("-");
        let rel_path = segments.next().unwrap_or(size_token);
        let rel_path = rel_path.trim();
        if rel_path.is_empty() || kind != "blob" {
            continue;
        }
        let size = size_token.parse::<u64>().ok().or_else(|| {
            std::fs::metadata(path.join(rel_path))
                .ok()
                .filter(|metadata| metadata.is_file())
                .map(|metadata| metadata.len())
        });
        files.push(RepositoryFile {
            path: rel_path.to_owned(),
            kind: "blob".to_owned(),
            size,
            sha,
            url: String::new(),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

async fn load_commits(path: &Path) -> Result<Vec<Commit>> {
    // Single git log invocation emitting both numstat (additions/deletions)
    // and name-status (A/M/D/R/...) per commit. Bounded to MAX_LOCAL_COMMITS.
    let output = run_git(
        path,
        &[
            "log",
            "--reverse",
            "--date=iso-strict",
            "--pretty=format:__COMMIT__%H%x1f%P%x1f%an%x1f%ae%x1f%cI%x1f%s",
            "--numstat",
            "--name-status",
            "-M",
            &format!("--max-count={}", MAX_LOCAL_COMMITS),
        ],
    )
    .await?;

    if output.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut commits = Vec::new();
    let mut current: Option<CommitBuilder> = None;

    for line in output.lines() {
        if let Some(header) = line.strip_prefix("__COMMIT__") {
            if let Some(builder) = current.take() {
                commits.push(builder.build());
            }
            current = Some(CommitBuilder::parse_header(header));
        } else if let Some(builder) = current.as_mut() {
            builder.parse_file_line(line);
        }
    }
    if let Some(builder) = current.take() {
        commits.push(builder.build());
    }

    commits.sort_by_key(|commit| commit.timestamp.unwrap_or(0));
    Ok(commits)
}

struct CommitBuilder {
    sha: String,
    author_name: String,
    author_email: String,
    date: String,
    message: String,
    numstat: std::collections::HashMap<String, (u64, u64)>,
    status: std::collections::HashMap<String, FileChangeStatus>,
    order: Vec<String>,
}

impl CommitBuilder {
    fn parse_header(header: &str) -> Self {
        let parts: Vec<&str> = header.split('\x1f').collect();
        Self {
            sha: parts.first().unwrap_or(&"").to_string(),
            author_name: parts.get(2).unwrap_or(&"").to_string(),
            author_email: parts.get(3).unwrap_or(&"").to_string(),
            date: parts.get(4).unwrap_or(&"").to_string(),
            message: parts.get(5).unwrap_or(&"").to_string(),
            numstat: std::collections::HashMap::new(),
            status: std::collections::HashMap::new(),
            order: Vec::new(),
        }
    }

    fn parse_file_line(&mut self, line: &str) {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            return;
        }
        // numstat lines: "<adds>\t<dels>\t<path>"
        let tab_parts: Vec<&str> = trimmed.split('\t').collect();
        if tab_parts.len() == 3
            && tab_parts[0]
                .trim()
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-')
            && tab_parts[1]
                .trim()
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-')
        {
            let additions = tab_parts[0].trim().parse::<u64>().unwrap_or(0);
            let deletions = tab_parts[1].trim().parse::<u64>().unwrap_or(0);
            let path = normalize_git_path(tab_parts[2]);
            self.register_path(&path);
            self.numstat.insert(path, (additions, deletions));
            return;
        }
        // name-status lines: "<STATUS>\t<path>" or "R100\t<old>\t<new>"
        if tab_parts.len() >= 2 {
            let status_token = tab_parts[0].trim();
            let status_char = status_token.chars().next().unwrap_or('M');
            if matches!(status_char, 'A' | 'M' | 'D' | 'R' | 'C' | 'T' | 'U') {
                let raw_path = if matches!(status_char, 'R' | 'C') && tab_parts.len() >= 3 {
                    tab_parts[2]
                } else {
                    tab_parts[1]
                };
                let path = normalize_git_path(raw_path);
                self.register_path(&path);
                self.status.insert(path, status_from(status_char));
                return;
            }
        }
    }

    fn register_path(&mut self, path: &str) {
        if !self.order.iter().any(|existing| existing == path) {
            self.order.push(path.to_owned());
        }
    }

    fn build(self) -> Commit {
        let timestamp = if self.date.is_empty() {
            None
        } else {
            DateTime::parse_from_rfc3339(&self.date)
                .ok()
                .map(|dt| dt.timestamp())
        };
        let mut files = Vec::new();
        let mut additions_total = 0_u64;
        let mut deletions_total = 0_u64;
        for path in &self.order {
            let (additions, deletions) = self.numstat.get(path).copied().unwrap_or((0, 0));
            let status = self
                .status
                .get(path)
                .cloned()
                .unwrap_or(FileChangeStatus::Modified);
            additions_total += additions;
            deletions_total += deletions;
            files.push(ChangedFile {
                filename: path.clone(),
                additions,
                deletions,
                changes: additions + deletions,
                status,
            });
        }
        let author = if self.author_name.is_empty() && self.author_email.is_empty() {
            None
        } else {
            Some(CommitAuthor {
                name: self.author_name.clone(),
                email: if self.author_email.is_empty() {
                    None
                } else {
                    Some(self.author_email.clone())
                },
                date: if self.date.is_empty() {
                    None
                } else {
                    Some(self.date.clone())
                },
            })
        };
        Commit {
            sha: self.sha,
            message: self.message,
            author,
            timestamp,
            date: if self.date.is_empty() {
                None
            } else {
                Some(self.date)
            },
            url: String::new(),
            stats: CommitStats {
                additions: additions_total,
                deletions: deletions_total,
                total: additions_total + deletions_total,
            },
            files,
        }
    }
}

fn normalize_git_path(raw: &str) -> String {
    let mut path = raw.trim().trim_matches('"').to_owned();
    // numstat rename format: "{old => new}" or "old => new"
    if path.contains("=>") {
        if let Some(braced) = path
            .strip_prefix('{')
            .and_then(|rest| rest.split_once(" => "))
        {
            let (old_part, new_part) = braced;
            let new_part = new_part.trim_end_matches('}').trim();
            if new_part.contains('/') {
                path = new_part.to_owned();
            } else if let Some((dir, _)) = old_part.rsplit_once('/') {
                path = format!("{dir}/{new_part}");
            } else {
                path = new_part.to_owned();
            }
        } else if let Some((_, new_part)) = path.split_once("=>") {
            path = new_part.trim().to_owned();
        }
    }
    path
}

fn status_from(token: char) -> FileChangeStatus {
    match token {
        'A' => FileChangeStatus::Added,
        'D' => FileChangeStatus::Removed,
        'R' => FileChangeStatus::Renamed,
        'C' => FileChangeStatus::Copied,
        _ => FileChangeStatus::Modified,
    }
}

fn is_source_candidate(path: &str, size: u64) -> bool {
    const MAX_SOURCE_FILE_SIZE: u64 = 1_000_000;
    if size > MAX_SOURCE_FILE_SIZE {
        return false;
    }
    let lower = path.to_ascii_lowercase();
    if lower
        .split('/')
        .any(|part| matches!(part, ".git" | "target" | "node_modules" | "dist" | "build"))
    {
        return false;
    }
    const IGNORED: &[&str] = &[
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".svg", ".pdf", ".zip", ".gz", ".tar",
        ".mp3", ".mp4", ".mov", ".avi", ".wasm", ".lock", ".map",
    ];
    if IGNORED.iter().any(|ext| lower.ends_with(ext)) {
        return false;
    }
    true
}

fn load_sources(canonical: &Path, tree: &[RepositoryFile]) -> Result<Vec<SourceFile>> {
    let mut sources = Vec::new();
    for file in tree {
        let size = file.size.or_else(|| {
            std::fs::metadata(canonical.join(&file.path))
                .ok()
                .filter(|metadata| metadata.is_file())
                .map(|metadata| metadata.len())
        });
        let size = match size {
            Some(size) => size,
            None => continue,
        };
        if !is_source_candidate(&file.path, size) {
            continue;
        }
        let absolute = canonical.join(&file.path);
        // Containment check: never escape the selected repository.
        if !absolute.starts_with(canonical) {
            continue;
        }
        let content = match std::fs::read(&absolute) {
            Ok(bytes) => bytes,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::PermissionDenied {
                    bail!(LocalRepoError::Permission);
                }
                eprintln!("Warning: skipping source file {}: {error}", file.path);
                continue;
            }
        };
        let content = match String::from_utf8(content) {
            Ok(content) => content,
            Err(_) => continue,
        };
        sources.push(SourceFile {
            path: file.path.clone(),
            size_bytes: content.len() as u64,
            content,
        });
    }
    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn create(files: &[(&str, &str)], message: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "repoinsight-local-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&dir).expect("create temp repo dir");
            git(&dir, &["init"]);
            git(&dir, &["config", "user.name", "RepoInsight Test"]);
            git(&dir, &["config", "user.email", "test@repoinsight.local"]);
            for (rel, content) in files {
                let absolute = dir.join(rel);
                if let Some(parent) = absolute.parent() {
                    std::fs::create_dir_all(parent).expect("create parent dirs");
                }
                std::fs::write(&absolute, content).expect("write fixture file");
            }
            git(&dir, &["add", "."]);
            git(
                &dir,
                &[
                    "commit",
                    "-m",
                    message,
                    "--author=RepoInsight Test <test@repoinsight.local>",
                ],
            );
            Self { path: dir }
        }

        fn commit(&self, files: &[(&str, &str)], message: &str) {
            for (rel, content) in files {
                let absolute = self.path.join(rel);
                if let Some(parent) = absolute.parent() {
                    std::fs::create_dir_all(parent).expect("create parent dirs");
                }
                std::fs::write(&absolute, content).expect("write fixture file");
            }
            git(&self.path, &["add", "."]);
            git(
                &self.path,
                &[
                    "commit",
                    "-m",
                    message,
                    "--author=RepoInsight Test <test@repoinsight.local>",
                ],
            );
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

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = StdCommand::new("git")
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
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
    }

    #[test]
    fn valid_local_repo_loads() {
        let repo = TempRepo::create(&[("src/main.rs", "fn main() {}\n")], "initial");
        let loaded = runtime()
            .block_on(load_local_repo(&repo.path_str()))
            .expect("valid repo loads");
        assert_eq!(loaded.commits.len(), 1);
        assert!(loaded.tree.iter().any(|file| file.path == "src/main.rs"));
        assert!(loaded.sources.iter().any(|file| file.path == "src/main.rs"));
    }

    #[test]
    fn nonexistent_path_rejected() {
        let missing = std::env::temp_dir().join("repoinsight-local-definitely-missing-xyz");
        let error = validate_local_path(&missing.to_string_lossy()).unwrap_err();
        assert_eq!(error.to_string(), "Repository path does not exist.");
    }

    #[test]
    fn non_directory_path_rejected() {
        let dir = std::env::temp_dir();
        let file = dir.join(format!("repoinsight-local-file-{}", std::process::id()));
        std::fs::write(&file, "data").expect("write temp file");
        let error = validate_local_path(&file.to_string_lossy()).unwrap_err();
        std::fs::remove_file(&file).ok();
        assert_eq!(error.to_string(), "Path is not a directory.");
    }

    #[test]
    fn non_git_directory_rejected() {
        let dir = std::env::temp_dir().join(format!(
            "repoinsight-local-nogit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create dir");
        let result = runtime().block_on(ensure_git_repo(&dir));
        std::fs::remove_dir_all(&dir).ok();
        assert!(result.is_err());
    }

    #[test]
    fn repo_with_commits_extracts_history() {
        let repo = TempRepo::create(&[("a.txt", "one\n")], "first");
        repo.commit(&[("a.txt", "one\ntwo\n"), ("b.txt", "new\n")], "second");
        let loaded = runtime()
            .block_on(load_local_repo(&repo.path_str()))
            .expect("repo loads");
        assert_eq!(loaded.commits.len(), 2);
        let second = loaded
            .commits
            .iter()
            .max_by_key(|c| c.timestamp.unwrap_or(0))
            .unwrap();
        let names: Vec<&str> = second.files.iter().map(|f| f.filename.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
        assert!(second.timestamp.is_some());
        assert!(second.author.is_some());
    }

    #[test]
    fn multiple_changed_files_tracked() {
        let repo = TempRepo::create(&[("a.txt", "a\n")], "first");
        repo.commit(
            &[
                ("a.txt", "a2\n"),
                ("b.py", "x = 1\n"),
                ("c.rs", "fn f() {}\n"),
            ],
            "multi",
        );
        let loaded = runtime()
            .block_on(load_local_repo(&repo.path_str()))
            .expect("repo loads");
        let latest = loaded
            .commits
            .iter()
            .max_by_key(|c| c.timestamp.unwrap_or(0))
            .unwrap();
        assert!(latest.files.len() >= 3);
    }

    #[test]
    fn history_extraction_preserves_order_and_stats() {
        let repo = TempRepo::create(&[("a.txt", "a\n")], "first");
        repo.commit(&[("a.txt", "a\nb\n")], "second");
        let loaded = runtime()
            .block_on(load_local_repo(&repo.path_str()))
            .expect("repo loads");
        let stamps: Vec<i64> = loaded.commits.iter().filter_map(|c| c.timestamp).collect();
        let mut sorted = stamps.clone();
        sorted.sort_unstable();
        assert_eq!(stamps, sorted);
        assert_eq!(stamps.len(), 2);
        assert!(loaded.commits.iter().all(|c| !c.sha.is_empty()));
    }

    #[test]
    fn common_representation_shapes_match_github() {
        let repo = TempRepo::create(&[("src/main.rs", "fn main() {}\n")], "initial");
        let loaded = runtime()
            .block_on(load_local_repo(&repo.path_str()))
            .expect("repo loads");
        // Same structs the GitHub pipeline uses.
        let _: &Vec<RepositoryFile> = &loaded.tree;
        let _: &Vec<Commit> = &loaded.commits;
        let _: &Vec<SourceFile> = &loaded.sources;
        assert!(!loaded.name.is_empty());
        assert!(!loaded.default_branch.is_empty());
    }

    #[test]
    fn local_source_needs_no_github_token() {
        // SAFETY: tests run single-threaded per test binary here; no concurrent
        // env access in this test.
        unsafe {
            std::env::remove_var("GITHUB_TOKEN");
        }
        let repo = TempRepo::create(&[("a.txt", "a\n")], "initial");
        let loaded = runtime()
            .block_on(load_local_repo(&repo.path_str()))
            .expect("no token required");
        assert_eq!(loaded.commits.len(), 1);
    }
}
