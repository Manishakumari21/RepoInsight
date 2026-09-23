use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
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
mod dataset;
mod github;
mod local;
mod prediction;
mod timing;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Deserialize)]
struct LocalRepoRequest {
    path: String,
}

type ApiError = (StatusCode, Json<ErrorBody>);

fn status_code_from(error: &anyhow::Error) -> StatusCode {
    let message = format!("{error:#}");
    if message.contains("Repository path does not exist.") {
        return StatusCode::NOT_FOUND;
    }
    if message.contains("not a Git repository.") {
        return StatusCode::UNPROCESSABLE_ENTITY;
    }
    if message.contains("not a directory.") || message.contains("Unable to read Git history") {
        return StatusCode::BAD_REQUEST;
    }
    if message.contains("cannot access this repository.") {
        return StatusCode::FORBIDDEN;
    }

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

    (
        status,
        Json(ErrorBody {
            error: error.to_string(),
        }),
    )
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
        .route(
            "/api/repositories/{owner}/{repo}/dataset",
            get(get_repository_dataset),
        )
        .route("/api/evaluation/rework", get(get_rework_evaluation))
        .route(
            "/api/repositories/{owner}/{repo}/predictions",
            get(get_repository_predictions),
        )
        .route(
            "/api/repositories/{owner}/{repo}/ripple",
            get(get_repository_ripple),
        )
        .route(
            "/api/repositories/{owner}/{repo}/impact-simulation",
            get(get_repository_impact),
        )
        .route("/api/local/analysis", post(post_local_analysis))
        .route("/api/local/dataset", post(post_local_dataset))
        .route("/api/local/predictions", post(post_local_predictions))
        .route("/api/local/ripple", post(post_local_ripple))
        .route("/api/local/impact-simulation", post(post_local_impact))
        .route("/api/local/timings", post(post_local_timings))
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
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        );
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

async fn get_rework_evaluation() -> Json<analysis::rework_evaluation::EvaluationReport> {
    Json(analysis::rework_evaluation::run_evaluation())
}

#[debug_handler]
async fn get_repository_predictions(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let cache_key = format!("pred:{owner}/{repo}");

    if let Some(body) = state.cache.get(&cache_key).await {
        return cached_json(body);
    }

    match analysis::analyzer::prediction_inputs(&state.github, &owner, &repo).await {
        Ok(inputs) => {
            let response = prediction::predict_files(&inputs.rows, &inputs.commits);
            if let Ok(body) = serde_json::to_vec(&response) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(response).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}

fn cached_json(body: Vec<u8>) -> Result<Response, ApiError> {
    let mut response = body.into_response();
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    Ok(response)
}

#[derive(Deserialize)]
struct RippleQuery {
    source: Option<String>,
    file: Option<String>,
    changed: Option<String>,
    max_depth: Option<usize>,
    min_confidence: Option<f64>,
}

#[derive(Deserialize)]
struct LocalRippleRequest {
    path: String,
    source: Option<String>,
    files: Option<Vec<String>>,
    changed: Option<Vec<String>>,
    max_depth: Option<usize>,
    min_confidence: Option<f64>,
}

fn ripple_config_from(
    max_depth: Option<usize>,
    min_confidence: Option<f64>,
) -> crate::analysis::ripple::RippleConfig {
    let mut config = crate::analysis::ripple::RippleConfig::default();
    if let Some(depth) = max_depth {
        config.max_depth = depth.clamp(1, crate::analysis::ripple::MAX_RIPPLE_DEPTH);
    }
    if let Some(confidence) = min_confidence {
        config.min_confidence = confidence.clamp(0.0, 1.0);
    }
    config
}

fn ripple_response_for(
    source: &str,
    changed: &std::collections::HashSet<String>,
    inputs: &crate::analysis::analyzer::RippleInputs,
    config: &crate::analysis::ripple::RippleConfig,
) -> crate::analysis::ripple::RippleResponse {
    crate::analysis::ripple::forecast(
        source,
        changed,
        &inputs.commits,
        &inputs.edges,
        &inputs.cochange_pairs,
        config,
    )
}

#[debug_handler]
async fn get_repository_ripple(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
    Query(query): Query<RippleQuery>,
) -> Result<Response, ApiError> {
    let source = query
        .source
        .or(query.file)
        .filter(|source| !source.trim().is_empty())
        .ok_or_else(|| api_error(anyhow::anyhow!("missing required query parameter: source")))?;
    let config = ripple_config_from(query.max_depth, query.min_confidence);
    let use_cache =
        query.max_depth.is_none() && query.min_confidence.is_none() && query.changed.is_none();
    let changed: std::collections::HashSet<String> = query
        .changed
        .map(|changed| {
            changed
                .split(',')
                .map(|file| file.trim().to_owned())
                .filter(|file| !file.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let cache_key = format!("ripple:{owner}/{repo}:{source}");
    if use_cache {
        if let Some(body) = state.cache.get(&cache_key).await {
            return cached_json(body);
        }
    }
    match analysis::analyzer::ripple_inputs(&state.github, &owner, &repo).await {
        Ok(inputs) => {
            let changed = {
                let mut set = changed;
                set.insert(source.clone());
                set
            };
            let response = tokio::task::spawn_blocking(move || {
                ripple_response_for(&source, &changed, &inputs, &config)
            })
            .await
            .map_err(|error| api_error(anyhow::anyhow!("ripple task failed: {error}")))?;
            if let Ok(body) = serde_json::to_vec(&response) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(response).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}

#[debug_handler]
async fn post_local_ripple(
    State(state): State<AppState>,
    Json(request): Json<LocalRippleRequest>,
) -> Result<Response, ApiError> {
    let source = request
        .source
        .or_else(|| request.files.and_then(|files| files.into_iter().next()))
        .filter(|source| !source.trim().is_empty())
        .ok_or_else(|| api_error(anyhow::anyhow!("missing required field: source or files")))?;
    let config = ripple_config_from(request.max_depth, request.min_confidence);
    let changed: std::collections::HashSet<String> =
        request.changed.unwrap_or_default().into_iter().collect();
    let canonical = crate::local::validate_local_path(&request.path)
        .map_err(|error| api_error(anyhow::anyhow!(error.to_string())))?;
    let cache_key = format!("ripple:local:{}:{source}", canonical.to_string_lossy());
    if let Some(body) = state.cache.get(&cache_key).await {
        return cached_json(body);
    }
    match analysis::analyzer::ripple_inputs_local(&request.path).await {
        Ok(inputs) => {
            let changed = {
                let mut set = changed;
                set.insert(source.clone());
                set
            };
            let response = tokio::task::spawn_blocking(move || {
                ripple_response_for(&source, &changed, &inputs, &config)
            })
            .await
            .map_err(|error| api_error(anyhow::anyhow!("local ripple task failed: {error}")))?;
            if let Ok(body) = serde_json::to_vec(&response) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(response).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}

#[derive(Deserialize)]
struct ImpactQuery {
    change_description: Option<String>,
    max_results: Option<usize>,
    include_low_confidence: Option<bool>,
}

#[derive(Deserialize)]
struct LocalImpactRequest {
    path: String,
    change_description: Option<String>,
    max_results: Option<usize>,
    include_low_confidence: Option<bool>,
}

fn impact_config_from(
    max_results: Option<usize>,
    include_low_confidence: Option<bool>,
) -> crate::analysis::impact::ImpactConfig {
    let mut config = crate::analysis::impact::ImpactConfig::default();
    if let Some(max) = max_results {
        config.max_results = max.clamp(1, 100);
    }
    if let Some(include) = include_low_confidence {
        config.include_low_confidence = include;
    }
    config
}

fn impact_response_for(
    description: &str,
    inputs: &crate::analysis::analyzer::ImpactInputs,
    config: &crate::analysis::impact::ImpactConfig,
) -> crate::analysis::impact::ImpactResponse {
    let contents: HashMap<&str, &str> = inputs
        .contents
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();
    let impact_inputs = crate::analysis::impact::ImpactInputs {
        commits: &inputs.commits,
        edges: &inputs.edges,
        cochange_pairs: &inputs.cochange_pairs,
        file_paths: inputs.file_paths.clone(),
        contents,
    };
    crate::analysis::impact::simulate(description, &impact_inputs, config)
}

#[debug_handler]
async fn get_repository_impact(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
    Query(query): Query<ImpactQuery>,
) -> Result<Response, ApiError> {
    let description = query
        .change_description
        .filter(|description| !description.trim().is_empty())
        .ok_or_else(|| {
            api_error(anyhow::anyhow!(
                "missing required query parameter: change_description"
            ))
        })?;
    let config = impact_config_from(query.max_results, query.include_low_confidence);
    let cache_key = format!("impact:{owner}/{repo}:{description}");
    if query.max_results.is_none() && query.include_low_confidence.is_none() {
        if let Some(body) = state.cache.get(&cache_key).await {
            return cached_json(body);
        }
    }
    match analysis::analyzer::impact_inputs(&state.github, &owner, &repo).await {
        Ok(inputs) => {
            let response = tokio::task::spawn_blocking(move || {
                impact_response_for(&description, &inputs, &config)
            })
            .await
            .map_err(|error| api_error(anyhow::anyhow!("impact task failed: {error}")))?;
            if let Ok(body) = serde_json::to_vec(&response) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(response).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}

#[debug_handler]
async fn post_local_impact(
    State(state): State<AppState>,
    Json(request): Json<LocalImpactRequest>,
) -> Result<Response, ApiError> {
    let description = request
        .change_description
        .filter(|description| !description.trim().is_empty())
        .ok_or_else(|| {
            api_error(anyhow::anyhow!(
                "missing required field: change_description"
            ))
        })?;
    let config = impact_config_from(request.max_results, request.include_low_confidence);
    let canonical = crate::local::validate_local_path(&request.path)
        .map_err(|error| api_error(anyhow::anyhow!(error.to_string())))?;
    let cache_key = format!("impact:local:{}:{description}", canonical.to_string_lossy());
    if let Some(body) = state.cache.get(&cache_key).await {
        return cached_json(body);
    }
    match analysis::analyzer::impact_inputs_local(&request.path).await {
        Ok(inputs) => {
            let response = tokio::task::spawn_blocking(move || {
                impact_response_for(&description, &inputs, &config)
            })
            .await
            .map_err(|error| api_error(anyhow::anyhow!("local impact task failed: {error}")))?;
            if let Ok(body) = serde_json::to_vec(&response) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(response).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}

#[debug_handler]
async fn get_repository_dataset(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let rows = analysis::analyzer::build_ml_dataset(&state.github, &owner, &repo)
        .await
        .map_err(api_error)?;

    let mut body = String::new();
    for row in &rows {
        let line = serde_json::to_string(row).map_err(|error| {
            api_error(anyhow::anyhow!("failed to serialize dataset row: {error}"))
        })?;
        body.push_str(&line);
        body.push('\n');
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from(body))
        .map_err(|error| api_error(anyhow::anyhow!("failed to build dataset response: {error}")))
}

async fn post_local_analysis(
    State(state): State<AppState>,
    Json(request): Json<LocalRepoRequest>,
) -> Result<Response, ApiError> {
    let canonical = crate::local::validate_local_path(&request.path)
        .map_err(|error| api_error(anyhow::anyhow!(error.to_string())))?;
    let cache_key = format!("local:{}", canonical.to_string_lossy());

    if let Some(body) = state.cache.get(&cache_key).await {
        let mut response = body.into_response();
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        );
        return Ok(response);
    }

    match analysis::analyzer::analyze_local_repository(&request.path).await {
        Ok(analysis) => {
            if let Ok(body) = serde_json::to_vec(&analysis) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(analysis).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}

async fn post_local_dataset(Json(request): Json<LocalRepoRequest>) -> Result<Response, ApiError> {
    let rows = analysis::analyzer::build_ml_dataset_local(&request.path)
        .await
        .map_err(api_error)?;

    let mut body = String::new();
    for row in &rows {
        let line = serde_json::to_string(row).map_err(|error| {
            api_error(anyhow::anyhow!("failed to serialize dataset row: {error}"))
        })?;
        body.push_str(&line);
        body.push('\n');
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from(body))
        .map_err(|error| api_error(anyhow::anyhow!("failed to build dataset response: {error}")))
}

async fn post_local_predictions(
    State(state): State<AppState>,
    Json(request): Json<LocalRepoRequest>,
) -> Result<Response, ApiError> {
    let canonical = crate::local::validate_local_path(&request.path)
        .map_err(|error| api_error(anyhow::anyhow!(error.to_string())))?;
    let cache_key = format!("pred:local:{}", canonical.to_string_lossy());

    if let Some(body) = state.cache.get(&cache_key).await {
        return cached_json(body);
    }

    match analysis::analyzer::prediction_inputs_local(&request.path).await {
        Ok(inputs) => {
            let response = tokio::task::spawn_blocking(move || {
                prediction::predict_files(&inputs.rows, &inputs.commits)
            })
            .await
            .map_err(|error| api_error(anyhow::anyhow!("local prediction task failed: {error}")))?;
            if let Ok(body) = serde_json::to_vec(&response) {
                state.cache.insert(&cache_key, body).await;
            }
            Ok(Json(response).into_response())
        }
        Err(error) => Err(api_error(error)),
    }
}

#[derive(Serialize)]
struct LocalTimingsResponse {
    repository: LocalTimingsRepo,
    timings: crate::timing::AnalysisTimings,
}

#[derive(Serialize)]
struct LocalTimingsRepo {
    name: String,
    branch: String,
}

#[debug_handler]
async fn post_local_timings(
    Json(request): Json<LocalRepoRequest>,
) -> Result<Json<LocalTimingsResponse>, ApiError> {
    match analysis::analyzer::analyze_local_timed(&request.path).await {
        Ok(timed) => Ok(Json(LocalTimingsResponse {
            repository: LocalTimingsRepo {
                name: timed.repository_name,
                branch: timed.repository_branch,
            },
            timings: timed.timings,
        })),
        Err(error) => Err(api_error(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode as ReqwestStatus;

    fn github_api_error(status: u16) -> anyhow::Error {
        anyhow::anyhow!(GithubError::Api {
            status: ReqwestStatus::from_u16(status).expect("valid status"),
            body: "upstream failure".to_owned(),
        })
    }

    #[test]
    fn local_path_errors_map_to_client_statuses() {
        let cases = [
            ("Repository path does not exist.", StatusCode::NOT_FOUND),
            ("Path is not a directory.", StatusCode::BAD_REQUEST),
            (
                "The selected directory is not a Git repository.",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "Unable to read Git history from the repository.",
                StatusCode::BAD_REQUEST,
            ),
            (
                "RepoInsight cannot access this repository.",
                StatusCode::FORBIDDEN,
            ),
        ];
        for (message, expected) in cases {
            assert_eq!(
                status_code_from(&anyhow::anyhow!(message)),
                expected,
                "message: {message}"
            );
        }
    }

    #[test]
    fn github_api_errors_map_to_gateway_statuses() {
        assert_eq!(
            status_code_from(&github_api_error(404)),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status_code_from(&github_api_error(401)),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_code_from(&github_api_error(403)),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            status_code_from(&github_api_error(500)),
            StatusCode::BAD_GATEWAY
        );
    }

    #[test]
    fn github_rate_limit_maps_to_429() {
        let error = anyhow::anyhow!(GithubError::RateLimit {
            message: "throttled".to_owned(),
        });
        assert_eq!(status_code_from(&error), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn unknown_errors_map_to_500_without_panicking() {
        assert_eq!(
            status_code_from(&anyhow::anyhow!("something unexpected")),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
