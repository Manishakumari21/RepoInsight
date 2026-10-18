use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::{
    auth::{OAuthTransactionStore, authorization_url},
    cache::AnalysisCache,
    github::{
        client::{GithubClient, GithubError},
        commits::Commit,
        files::RepositoryFile,
        repository::Repository,
    },
};

mod analysis;
mod auth;
mod cache;
mod config;
mod crypto;
mod csrf;
mod dataset;
mod db;
mod github;
mod local;
mod prediction;
mod session;
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

// Deployment configuration. Defaults preserve local-development behavior;
// the Docker setup overrides HOST (and optionally PORT) via environment.
fn server_addr() -> String {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
    format!("{host}:{port}")
}

// Deployment safeguard for the /api/local/* routes, which read arbitrary
// filesystem paths. Enabled by default (current behavior); set
// LOCAL_API_ENABLED=0/false/no to disable them on public deployments.
fn local_api_enabled() -> bool {
    std::env::var("LOCAL_API_ENABLED")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            normalized != "0" && normalized != "false" && normalized != "no"
        })
        .unwrap_or(true)
}

fn local_api_disabled() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorBody {
            error: "Local repository analysis is disabled on this deployment.".to_owned(),
        }),
    )
}

#[derive(serde::Deserialize, Default)]
struct AuthCallbackQuery {
    code: Vec<String>,
    state: Vec<String>,
}

#[derive(Clone)]
struct AppState {
    github: GithubClient,
    cache: Arc<AnalysisCache>,
    oauth_transactions: Arc<OAuthTransactionStore>,
    db_pool: Option<sqlx::PgPool>,
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("RepoInsight backend starting");
    eprintln!(
        "HOST={}",
        std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned())
    );
    eprintln!(
        "PORT={}",
        std::env::var("PORT").unwrap_or_else(|_| "3000".to_owned())
    );
    eprintln!(
        "LOCAL_API_ENABLED={}",
        std::env::var("LOCAL_API_ENABLED").unwrap_or_else(|_| "<unset>".to_owned())
    );

    let config = config::AppConfig::from_env().map_err(|error| anyhow::anyhow!("{error}"))?;

    let db_pool = if config.auth_persistence_enabled {
        match db::init_pool(std::env::var("DATABASE_URL").ok().as_deref()).await {
            Ok(pool) => pool,
            Err(error) => {
                eprintln!("Database initialization failed: {error}");
                return Err(anyhow::anyhow!("database initialization failed"));
            }
        }
    } else {
        None
    };

    let state = AppState {
        github: match GithubClient::new() {
            Ok(client) => {
                eprintln!("GitHub client initialized");
                client
            }
            Err(error) => {
                eprintln!("Failed to initialize GitHub client: {error:#}");
                return Err(error.into());
            }
        },
        cache: Arc::new(AnalysisCache::new()),
        oauth_transactions: Arc::new(OAuthTransactionStore::new(std::time::Duration::from_secs(
            600,
        ))),
        db_pool,
    };

    let allowed_origins = if config.allowed_origins.is_empty() {
        vec!["https://repo-insight-smoky.vercel.app".to_owned()]
    } else {
        config.allowed_origins.clone()
    };

    let mut cors = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-csrf-token"),
        ])
        .allow_credentials(true);

    for origin in allowed_origins {
        if let Ok(value) = HeaderValue::from_str(&origin) {
            cors = cors.allow_origin(value);
        }
    }

    let app = Router::new()
        .route("/auth/github", get(auth_github))
        .route("/auth/github/callback", get(auth_github_callback))
        .route("/auth/me", get(auth_me))
        .route("/auth/logout", post(auth_logout))
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

    let addr = server_addr();
    eprintln!("Binding listener on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("Listener bound on {addr}");

    println!("RepoInsight backend running on http://{addr}");

    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("Server error: {error:#}");
        return Err(error.into());
    }
    eprintln!("Server stopped");

    Ok(())
}

async fn health() -> &'static str {
    "RepoInsight backend is healthy"
}

#[debug_handler]
async fn auth_github(State(state): State<AppState>) -> Result<axum::response::Redirect, ApiError> {
    let config = crate::config::AppConfig::from_env().map_err(|error| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: error.to_string(),
            }),
        )
    })?;
    let url = authorization_url(&config, &state.oauth_transactions).map_err(|error| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: error.to_string(),
            }),
        )
    })?;
    Ok(axum::response::Redirect::temporary(&url))
}

#[debug_handler]
async fn auth_github_callback(
    State(state): State<AppState>,
    Query(query): Query<AuthCallbackQuery>,
) -> Result<Response, ApiError> {
    let params =
        crate::auth::validate_query_values(&query.code, &query.state).map_err(|error| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: error.to_string(),
                }),
            )
        })?;
    let transaction = crate::auth::consume_transaction(&state.oauth_transactions, &params)
        .map_err(|error| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: error.to_string(),
                }),
            )
        })?;
    let config = crate::config::AppConfig::from_env().map_err(|error| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: error.to_string(),
            }),
        )
    })?;
    let token = crate::auth::exchange_code(&config, &params.code, &transaction.pkce.verifier)
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorBody {
                    error: error.to_string(),
                }),
            )
        })?;

    if !config.auth_persistence_enabled {
        return Ok(axum::response::Redirect::temporary("/").into_response());
    }

    let Some(pool) = &state.db_pool else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "persistence is not available".to_owned(),
            }),
        ));
    };

    let key = std::env::var("TOKEN_ENCRYPTION_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorBody {
                    error: "token encryption is not configured".to_owned(),
                }),
            )
        })?;
    let key = key.as_bytes();
    if key.len() != 32 {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "token encryption key is invalid".to_owned(),
            }),
        ));
    }

    // Fetch the authenticated GitHub user's stable ID.
    let client = reqwest::Client::builder()
        .user_agent("RepoInsight")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorBody {
                    error: "github request failed".to_owned(),
                }),
            )
        })?;
    let user_response = client
        .get("https://api.github.com/user")
        .bearer_auth(&token.access_token)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorBody {
                    error: "github request failed".to_owned(),
                }),
            )
        })?;
    if !user_response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorBody {
                error: "github user lookup failed".to_owned(),
            }),
        ));
    }
    #[derive(serde::Deserialize)]
    struct GitHubUser {
        id: i64,
    }
    let user: GitHubUser = user_response.json().await.map_err(|_| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ErrorBody {
                error: "github user lookup failed".to_owned(),
            }),
        )
    })?;
    let github_user_id = user.id.to_string();

    let raw_session_id = crate::session::persist_authenticated_session(
        pool,
        &github_user_id,
        &token.access_token,
        token.refresh_token.as_deref(),
        token.expires_in,
        token.refresh_token_expires_in,
        key,
    )
    .await
    .map_err(|error| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: error.to_string(),
            }),
        )
    })?;

    let session_hash = crate::session::hash_session_id(&raw_session_id);
    let csrf_secret = std::env::var("CSRF_SECRET").unwrap_or_default();
    let csrf_token = crate::csrf::issue_token(&session_hash, &csrf_secret);

    let session_cookie = format!(
        "__Host-session={}; HttpOnly; Path=/; Secure; SameSite=None; Max-Age=28800",
        raw_session_id
    );
    let csrf_cookie = format!(
        "csrf_token={}; Path=/; Secure; SameSite=None; Max-Age=28800",
        csrf_token.as_str()
    );

    let mut response = axum::response::Redirect::temporary("/").into_response();
    if let Ok(value) = axum::http::HeaderValue::from_str(&session_cookie) {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, value);
    }
    if let Ok(value) = axum::http::HeaderValue::from_str(&csrf_cookie) {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, value);
    }
    Ok(response)
}

#[derive(serde::Serialize)]
struct AuthMeResponse {
    authenticated: bool,
    github_user_id: Option<String>,
}

#[debug_handler]
async fn auth_me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<AuthMeResponse>, ApiError> {
    let Some(pool) = &state.db_pool else {
        return Ok(Json(AuthMeResponse {
            authenticated: false,
            github_user_id: None,
        }));
    };
    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok());
    let now = chrono::Utc::now();
    match crate::session::authenticated_session(pool, cookie_header, now).await {
        Ok(session) => Ok(Json(AuthMeResponse {
            authenticated: true,
            github_user_id: Some(session.github_user_id),
        })),
        Err(_) => Ok(Json(AuthMeResponse {
            authenticated: false,
            github_user_id: None,
        })),
    }
}

#[debug_handler]
async fn auth_logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let allowed_origins = crate::config::AppConfig::from_env()
        .map(|config| {
            if config.allowed_origins.is_empty() {
                vec!["https://repo-insight-smoky.vercel.app".to_owned()]
            } else {
                config.allowed_origins
            }
        })
        .unwrap_or_else(|_| vec!["https://repo-insight-smoky.vercel.app".to_owned()]);
    if !crate::csrf::origin_allowed(origin, &allowed_origins) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "origin not allowed".to_owned(),
            }),
        ));
    }

    if let Some(pool) = &state.db_pool {
        if let Some(raw_id) = headers
            .get(axum::http::header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(crate::session::extract_session_cookie)
        {
            let now = chrono::Utc::now();
            if crate::session::lookup_session(pool, &raw_id, now)
                .await
                .is_ok()
            {
                let csrf_header = headers
                    .get("x-csrf-token")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("");
                let secret = std::env::var("CSRF_SECRET").unwrap_or_default();
                let session_hash = crate::session::hash_session_id(&raw_id);
                if !crate::csrf::validate_token(&session_hash, &secret, csrf_header) {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ErrorBody {
                            error: "invalid csrf token".to_owned(),
                        }),
                    ));
                }
                let _ = crate::session::revoke_session(pool, &raw_id).await;
            }
        }
    }

    let mut response = axum::response::Redirect::temporary("/").into_response();
    let session_cookie = "__Host-session=; HttpOnly; Path=/; Secure; SameSite=None; Max-Age=0";
    let csrf_cookie = "csrf_token=; Path=/; Secure; SameSite=None; Max-Age=0";
    if let Ok(value) = axum::http::HeaderValue::from_str(session_cookie) {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, value);
    }
    if let Ok(value) = axum::http::HeaderValue::from_str(csrf_cookie) {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, value);
    }
    Ok(response)
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
    if !local_api_enabled() {
        return Err(local_api_disabled());
    }
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
    if !local_api_enabled() {
        return Err(local_api_disabled());
    }
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
    if !local_api_enabled() {
        return Err(local_api_disabled());
    }
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
    if !local_api_enabled() {
        return Err(local_api_disabled());
    }
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
    if !local_api_enabled() {
        return Err(local_api_disabled());
    }
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
    if !local_api_enabled() {
        return Err(local_api_disabled());
    }
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
