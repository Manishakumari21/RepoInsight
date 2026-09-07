use anyhow::{Context, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use serde::Deserialize;

use crate::github::{client::GithubClient, files::RepositoryFile};

use super::models::SourceAnalysis;

const MAX_SOURCE_FILE_SIZE: u64 = 1_000_000;
const MAX_CONCURRENT_REQUESTS: usize = 8;

#[derive(Debug)]
pub struct SourceFile {
    pub path: String,
    pub content: String,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct GithubBlob {
    content: String,
    encoding: String,
}

pub async fn collect_source_files(
    client: &GithubClient,
    files: &[RepositoryFile],
) -> Result<Vec<SourceFile>> {
    let candidates = files
        .iter()
        .filter(|file| is_candidate(file))
        .collect::<Vec<_>>();

    stream::iter(candidates)
        .map(|file| async move { fetch_source_file(client, file).await })
        .buffer_unordered(MAX_CONCURRENT_REQUESTS)
        .try_filter_map(|file| async move { Ok(file) })
        .try_collect()
        .await
}

async fn fetch_source_file(
    client: &GithubClient,
    file: &RepositoryFile,
) -> Result<Option<SourceFile>> {
    let blob: GithubBlob = client
        .get_url(&file.url)
        .await
        .with_context(|| format!("failed to fetch source file: {}", file.path))?;

    if blob.encoding != "base64" {
        return Ok(None);
    }

    let content = decode_base64_content(&blob.content)?;

    let content = match String::from_utf8(content) {
        Ok(content) => content,
        Err(_) => return Ok(None),
    };

    Ok(Some(SourceFile {
        path: file.path.clone(),
        size_bytes: content.len() as u64,
        content,
    }))
}

fn is_candidate(file: &RepositoryFile) -> bool {
    file.kind == "blob"
        && file.size.unwrap_or(0) <= MAX_SOURCE_FILE_SIZE
        && !is_likely_non_source(&file.path)
}

fn is_likely_non_source(path: &str) -> bool {
    let path = path.to_ascii_lowercase();

    [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".svg", ".pdf", ".zip", ".gz", ".tar",
        ".mp3", ".mp4", ".mov", ".avi", ".wasm", ".lock", ".map",
    ]
    .iter()
    .any(|extension| path.ends_with(extension))
}

fn decode_base64_content(content: &str) -> Result<Vec<u8>> {
    let normalized = content.replace('\n', "");

    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, normalized)
        .context("failed to decode GitHub file content")
}

pub fn build_source_analysis(files: &[SourceFile]) -> SourceAnalysis {
    SourceAnalysis {
        source_files: files.len(),
        total_lines: files.iter().map(|file| file.content.lines().count()).sum(),
        total_size_bytes: files.iter().map(|file| file.size_bytes).sum(),
    }
}
