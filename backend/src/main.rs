use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router, debug_handler,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    cache::AnalysisCache,
    github::{
        client::{GithubClient, GithubError},
        commits::Commit,
        files::RepositoryFile,
        repository::Repository,
    },
};

mod analysis;
mod cache;
mod github;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

type ApiError = (StatusCode, Json<ErrorBody>);

fn status_code_from(error: &anyhow::Error) -> StatusCode {
    let Some(github_error) = error.root_cause().downcast_ref::<GithubError>() else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    match github_error {
        GithubError::Api { status, .. } if status.as_u16() == 404 => StatusCode::NOT_FOUND,
        GithubError::Api { status, .. } if status.as_u16() == 401 => StatusCode::UNAUTHORIZED,
        GithubError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
        GithubError::Api { .. } | GithubError::Network { .. } => StatusCode::BAD_GATEWAY,
        GithubError::Deserialize(_) | GithubError::Client(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn api_error(error: anyhow::Error) -> ApiError {
    let status = status_code_from(&error);

    eprintln!("API error: {error:#}");

    (status, Json(ErrorBody { error: error.to_string() }))
}

#[derive(Clone)]
struct AppState {
    github: GithubClient,
    cache: Arc<AnalysisCache>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let state = AppState {
        github: GithubClient::new()?,
        cache: Arc::new(AnalysisCache::new()),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

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
        .route(
            "/api/repositories/{owner}/{repo}/analysis",
            get(get_repository_analysis),
        )
        .layer(cors)
        .with_state(state);

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
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Repository>, ApiError> {
    Repository::fetch(&state.github, &owner, &repo)
        .await
        .map(Json)
        .map_err(api_error)
}

#[debug_handler]
async fn get_repository_files(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Vec<RepositoryFile>>, ApiError> {
    let repository = Repository::fetch(&state.github, &owner, &repo)
        .await
        .map_err(api_error)?;

    RepositoryFile::fetch_tree(&state.github, &owner, &repo, &repository.default_branch)
        .await
        .map(Json)
        .map_err(api_error)
}

#[debug_handler]
async fn get_repository_commits(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Vec<Commit>>, ApiError> {
    Commit::fetch(&state.github, &owner, &repo)
        .await
        .map(Json)
        .map_err(api_error)
}

#[debug_handler]
async fn get_repository_analysis(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let cache_key = format!("{owner}/{repo}");

    if let Some(body) = state.cache.get(&cache_key).await {
        let mut response = body.into_response();
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
        return Ok(response);
    }

    match analysis::analyzer::analyze_repository(&state.github, &owner, &repo).await {
        Ok(analysis) => {
            if let Ok(body) = serde_json::to_vec(&analysis) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(analysis).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}