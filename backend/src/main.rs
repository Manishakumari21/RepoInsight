use anyhow::Result;
use axum::{
    Json, Router, debug_handler,
    extract::{Path, State},
    routing::get,
};

use crate::github::{
    client::GithubClient, commits::Commit, files::RepositoryFile, repository::Repository,
};

mod github;

#[tokio::main]
async fn main() -> Result<()> {
    let github_client = GithubClient::new()?;

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/repositories/{owner}/{repo}", get(get_repository))
        .route(
            "/api/repositories/{owner}/{repo}/files",
            get(get_repository_files),
        )
        .route(
            "/api/repositories/{owner}/{repo}/commits",
            get(get_repository_commits),
        )
        .with_state(github_client);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    println!("RepoInsight backend running on http://127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "RepoInsight backend is healthy"
}

#[debug_handler]
async fn get_repository(
    State(client): State<GithubClient>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Repository>, axum::http::StatusCode> {
    Repository::fetch(&client, &owner, &repo)
        .await
        .map(Json)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

#[debug_handler]
async fn get_repository_files(
    State(client): State<GithubClient>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Vec<RepositoryFile>>, axum::http::StatusCode> {
    let repository = Repository::fetch(&client, &owner, &repo)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    RepositoryFile::fetch_tree(&client, &owner, &repo, &repository.default_branch)
        .await
        .map(Json)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

#[debug_handler]
async fn get_repository_commits(
    State(client): State<GithubClient>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Vec<Commit>>, axum::http::StatusCode> {
    Commit::fetch(&client, &owner, &repo)
        .await
        .map(Json)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}
