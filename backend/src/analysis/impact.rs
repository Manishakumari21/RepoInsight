use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::dependencies::DependencyEdge;
use super::history::relevant_files;
use crate::github::commits::Commit;

pub const DEFAULT_MAX_RESULTS: usize = 20;
pub const HIGH_THRESHOLD: f64 = 0.5;
pub const MEDIUM_THRESHOLD: f64 = 0.15;
pub const MIN_SCORE: f64 = 0.01;

pub const DIRECT_MATCH_FLOOR: f64 = 0.6;

#[derive(Debug, Clone, Copy)]
pub struct ImpactWeights {
    pub semantic: f64,
    pub dependency: f64,
    pub historical: f64,
    pub kind: f64,
}

impl Default for ImpactWeights {
    fn default() -> Self {
        Self {
            semantic: 0.35,
            dependency: 0.20,
            historical: 0.30,
            kind: 0.15,
        }
    }
}

impl ImpactWeights {
    fn normalized(self) -> Self {
        let total = self.semantic + self.dependency + self.historical + self.kind;
        if total <= 0.0 {
            return Self::default();
        }
        Self {
            semantic: self.semantic / total,
            dependency: self.dependency / total,
            historical: self.historical / total,
            kind: self.kind / total,
        }
    }

    pub fn without(self, group: &str) -> Self {
        let masked = match group {
            "semantic" => Self {
                semantic: 0.0,
                ..self
            },
            "dependency" => Self {
                dependency: 0.0,
                ..self
            },
            "historical" => Self {
                historical: 0.0,
                ..self
            },
            "kind" => Self { kind: 0.0, ..self },
            _ => self,
        };
        let total = masked.semantic + masked.dependency + masked.historical + masked.kind;
        if total <= 0.0 {
            return Self {
                semantic: 1.0,
                dependency: 0.0,
                historical: 0.0,
                kind: 0.0,
            };
        }
        Self {
            semantic: masked.semantic / total,
            dependency: masked.dependency / total,
            historical: masked.historical / total,
            kind: masked.kind / total,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImpactConfig {
    pub weights: ImpactWeights,
    pub max_results: usize,
    pub include_low_confidence: bool,
}

impl Default for ImpactConfig {
    fn default() -> Self {
        Self {
            weights: ImpactWeights::default().normalized(),
            max_results: DEFAULT_MAX_RESULTS,
            include_low_confidence: false,
        }
    }
}

const OPERATION_KEYWORDS: &[(&str, &str)] = &[
    ("replace", "replace"),
    ("replaces", "replace"),
    ("migrate", "replace"),
    ("migration", "replace"),
    ("switch", "replace"),
    ("convert", "replace"),
    ("move from", "replace"),
    ("instead of", "replace"),
    ("add", "add"),
    ("adds", "add"),
    ("introduce", "add"),
    ("implement", "add"),
    ("support", "add"),
    ("create", "add"),
    ("remove", "remove"),
    ("removes", "remove"),
    ("delete", "remove"),
    ("drop", "remove"),
    ("deprecate", "remove"),
    ("refactor", "refactor"),
    ("restructure", "refactor"),
    ("clean up", "refactor"),
    ("cleanup", "refactor"),
    ("upgrade", "upgrade"),
    ("update", "upgrade"),
    ("modernize", "upgrade"),
    ("bump", "upgrade"),
    ("fix", "fix"),
    ("cache", "add"),
];

const DOMAIN_ALIASES: &[(&str, &[&str])] = &[
    (
        "authentication",
        &[
            "auth",
            "authentication",
            "login",
            "logout",
            "session",
            "token",
            "jwt",
            "oauth",
            "oauth2",
            "password",
            "sso",
            "credentials",
            "signup",
            "signin",
        ],
    ),
    (
        "authorization",
        &[
            "authorization",
            "authorize",
            "permissions",
            "roles",
            "rbac",
            "acl",
            "policy",
            "policies",
            "guard",
        ],
    ),
    (
        "payments",
        &[
            "payment",
            "payments",
            "stripe",
            "razorpay",
            "billing",
            "invoice",
            "checkout",
            "subscription",
            "pay",
        ],
    ),
    (
        "database",
        &[
            "database",
            "postgres",
            "postgresql",
            "mysql",
            "sqlite",
            "mongo",
            "mongodb",
            "orm",
            "migration",
            "migrations",
            "schema",
            "query",
            "queries",
            "repository",
            "repositories",
        ],
    ),
    (
        "api",
        &[
            "api",
            "rest",
            "graphql",
            "endpoint",
            "endpoints",
            "route",
            "routes",
            "router",
            "controller",
            "controllers",
            "handler",
            "handlers",
            "middleware",
            "response",
            "responses",
            "request",
        ],
    ),
    (
        "caching",
        &[
            "cache",
            "caching",
            "redis",
            "memcached",
            "memoize",
            "invalidate",
        ],
    ),
    (
        "frontend",
        &[
            "frontend",
            "ui",
            "login",
            "page",
            "pages",
            "component",
            "components",
            "view",
            "views",
            "redux",
            "zustand",
            "store",
            "theme",
            "style",
            "styles",
            "css",
        ],
    ),
    (
        "messaging",
        &[
            "queue", "kafka", "rabbitmq", "message", "messages", "event", "events", "pubsub",
            "worker", "workers", "job", "jobs",
        ],
    ),
    (
        "storage",
        &[
            "storage", "upload", "uploads", "file", "files", "s3", "bucket", "blob", "asset",
            "assets",
        ],
    ),
    (
        "search",
        &[
            "search",
            "elasticsearch",
            "index",
            "indexing",
            "query",
            "filter",
            "filters",
        ],
    ),
    (
        "notifications",
        &[
            "notification",
            "notifications",
            "email",
            "emails",
            "sms",
            "push",
            "alert",
            "alerts",
            "webhook",
            "webhooks",
        ],
    ),
    (
        "logging",
        &[
            "log",
            "logs",
            "logging",
            "tracing",
            "metrics",
            "monitor",
            "monitoring",
            "observability",
        ],
    ),
    (
        "users",
        &[
            "user", "users", "profile", "profiles", "account", "accounts",
        ],
    ),
    (
        "configuration",
        &[
            "config",
            "configuration",
            "settings",
            "env",
            "environment",
            "dotenv",
            "yaml",
            "yml",
            "toml",
        ],
    ),
];

const TECHNOLOGIES: &[&str] = &[
    "jwt",
    "oauth",
    "oauth2",
    "sso",
    "postgres",
    "postgresql",
    "mysql",
    "sqlite",
    "mongodb",
    "mongo",
    "redis",
    "memcached",
    "stripe",
    "razorpay",
    "redux",
    "zustand",
    "rest",
    "graphql",
    "docker",
    "kubernetes",
    "kafka",
    "rabbitmq",
    "elasticsearch",
    "jest",
    "vitest",
    "pytest",
    "axios",
    "express",
    "axum",
    "tokio",
    "react",
    "vue",
    "angular",
];

const STOPWORDS: &[&str] = &[
    "the", "a", "an", "to", "of", "in", "on", "for", "with", "and", "or", "from", "by", "as", "is",
    "are", "be", "our", "we", "it", "its", "this", "that", "these", "those", "into", "over",
    "under", "all", "new", "old", "current", "existing", "please",
];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChangeIntent {
    pub operation: String,
    pub domains: Vec<String>,
    pub terms: Vec<String>,
    pub current_technology: Option<String>,
    pub target_technology: Option<String>,
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| token.len() > 2 && !STOPWORDS.contains(token))
        .map(str::to_owned)
        .collect()
}

fn detect_operation(lower: &str) -> String {
    for (keyword, operation) in OPERATION_KEYWORDS {
        if lower.contains(keyword) {
            return (*operation).to_owned();
        }
    }
    "general".to_owned()
}

pub fn extract_intent(description: &str) -> ChangeIntent {
    let lower = description.to_ascii_lowercase();
    let operation = detect_operation(&lower);
    let tokens = tokenize(description);
    let token_set: HashSet<&str> = tokens.iter().map(String::as_str).collect();

    let mut domains = Vec::new();
    let mut domain_terms: HashSet<String> = HashSet::new();
    for (domain, aliases) in DOMAIN_ALIASES {
        let mut matched = false;
        for alias in *aliases {
            if token_set.contains(alias) || lower.contains(alias) {
                domain_terms.insert((*alias).to_owned());
                matched = true;
            }
        }
        if matched {
            domains.push((*domain).to_owned());
        }
    }
    domains.sort();
    domains.dedup();

    let mut techs: Vec<String> = TECHNOLOGIES
        .iter()
        .filter(|tech| token_set.contains(**tech))
        .map(|tech| (*tech).to_owned())
        .collect();
    techs.sort();
    techs.dedup();

    let mut current_technology = None;
    let mut target_technology = None;
    if operation == "replace" && techs.len() >= 2 {
        let positions: Vec<(usize, &String)> = techs
            .iter()
            .filter_map(|tech| lower.find(tech).map(|position| (position, tech)))
            .collect();
        if positions.len() >= 2 {
            let mut ordered = positions;
            ordered.sort_by_key(|(position, _)| *position);
            current_technology = Some(ordered[0].1.clone());
            target_technology = Some(ordered[1].1.clone());
        }
    } else {
        current_technology = techs.first().cloned();
        if techs.len() >= 2 {
            target_technology = Some(techs[1].clone());
        }
    }

    let mut terms: HashSet<String> = tokens.into_iter().collect();
    terms.extend(domain_terms);
    terms.extend(techs);
    let mut terms: Vec<String> = terms.into_iter().collect();
    terms.sort();

    ChangeIntent {
        operation,
        domains,
        terms,
        current_technology,
        target_technology,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Capability {
    pub name: String,
    pub files: Vec<String>,
}

pub fn build_capability_map(file_paths: &[String]) -> Vec<Capability> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for path in file_paths {
        let lower = path.to_ascii_lowercase();
        let mut best: Option<(&str, usize)> = None;
        for (domain, aliases) in DOMAIN_ALIASES {
            let hits = aliases
                .iter()
                .filter(|alias| lower.contains(**alias))
                .count();
            if hits > 0 && best.is_none_or(|(_, best_hits)| hits > best_hits) {
                best = Some((domain, hits));
            }
        }
        let name = match best {
            Some((domain, _)) => (*domain).to_owned(),
            None => fallback_capability_name(path),
        };
        groups.entry(name).or_default().push(path.clone());
    }
    let mut capabilities: Vec<Capability> = groups
        .into_iter()
        .map(|(name, mut files)| {
            files.sort();
            Capability { name, files }
        })
        .collect();
    capabilities.sort_by(|a, b| {
        b.files
            .len()
            .cmp(&a.files.len())
            .then_with(|| a.name.cmp(&b.name))
    });
    capabilities
}

fn fallback_capability_name(path: &str) -> String {
    let top = path.split('/').next().unwrap_or(path);
    let stem = top
        .split('.')
        .next()
        .unwrap_or(top)
        .trim_start_matches(['_', '.']);
    if stem.is_empty() {
        return "general".to_owned();
    }
    let mut name = String::new();
    for (index, word) in stem.split(['-', '_']).enumerate() {
        if word.is_empty() {
            continue;
        }
        if index > 0 {
            name.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }
    if name.is_empty() {
        "general".to_owned()
    } else {
        name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathKind {
    Source,
    Test,
    Config,
    Docs,
}

fn classify_path(path: &str) -> PathKind {
    let lower = path.to_ascii_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or(&lower);
    if lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.starts_with("tests/")
        || lower.starts_with("test/")
        || lower.contains("__tests__")
        || file_name.contains(".test.")
        || file_name.contains("_test.")
        || file_name.starts_with("test_")
        || file_name.starts_with("conftest")
    {
        return PathKind::Test;
    }
    if lower.contains("config")
        || lower.contains("settings")
        || file_name.starts_with(".env")
        || file_name.ends_with(".env.example")
        || file_name.ends_with(".yml")
        || file_name.ends_with(".yaml")
        || file_name.ends_with(".toml")
        || file_name.ends_with(".ini")
    {
        return PathKind::Config;
    }
    if lower.starts_with("docs/")
        || lower.contains("/docs/")
        || file_name.ends_with(".md")
        || file_name.ends_with(".rst")
    {
        return PathKind::Docs;
    }
    PathKind::Source
}

fn path_tokens(path: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    for segment in path.to_ascii_lowercase().split('/') {
        let stem = segment.split('.').next().unwrap_or(segment);
        for part in stem.split(|c: char| !c.is_ascii_alphanumeric()) {
            if part.len() > 1 {
                tokens.insert(part.to_owned());
            }
        }

        let mut current = String::new();
        for c in stem.chars() {
            if c.is_ascii_uppercase() && !current.is_empty() {
                if current.len() > 1 {
                    tokens.insert(current.to_ascii_lowercase());
                }
                current = String::new();
            }
            if c.is_ascii_alphanumeric() {
                current.push(c);
            }
        }
        if current.len() > 1 {
            tokens.insert(current.to_ascii_lowercase());
        }
    }
    tokens
}

fn path_score(path: &str, terms: &[String]) -> (f64, Vec<String>) {
    if terms.is_empty() {
        return (0.0, Vec::new());
    }
    let tokens = path_tokens(path);
    let matched: Vec<String> = terms
        .iter()
        .filter(|term| {
            tokens.contains(term.as_str())
                || tokens.iter().any(|token| {
                    (token.len() > 3 && term.len() > 3)
                        && (token.contains(term.as_str()) || term.contains(token))
                })
        })
        .cloned()
        .collect();
    let denominator = terms.len().clamp(1, 4) as f64;
    (matched.len() as f64 / denominator, matched)
}

const MAX_CONTENT_SCAN_BYTES: usize = 20_000;

fn content_score(content: &str, terms: &[String]) -> (f64, Vec<String>) {
    if terms.is_empty() {
        return (0.0, Vec::new());
    }
    let end = content
        .char_indices()
        .take_while(|(index, _)| *index < MAX_CONTENT_SCAN_BYTES)
        .last()
        .map(|(index, c)| index + c.len_utf8())
        .unwrap_or(0);
    let lower = content[..end.min(content.len())].to_ascii_lowercase();
    let matched: Vec<String> = terms
        .iter()
        .filter(|term| term.len() > 2 && lower.contains(term.as_str()))
        .cloned()
        .collect();
    let denominator = terms.len().clamp(1, 4) as f64;
    (matched.len() as f64 / denominator, matched)
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

type ScoredFile = (String, f64, f64, f64, f64, Vec<String>, Vec<String>);

#[derive(Debug, Clone, Serialize)]
pub struct ImpactItem {
    pub path: String,
    pub level: String,
    pub score: f64,
    pub categories: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChecklistItem {
    pub label: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactGraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactGraphEdge {
    pub from: String,
    pub to: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactGraph {
    pub nodes: Vec<ImpactGraphNode>,
    pub edges: Vec<ImpactGraphEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactModelInfo {
    pub name: String,
    pub calibrated: bool,
    pub threshold: f64,
    pub weights: HashMap<String, f64>,
    pub training: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactResponse {
    pub change_description: String,
    pub intent: ChangeIntent,
    pub impact: Vec<ImpactItem>,
    pub capabilities: Vec<Capability>,
    pub checklist: Vec<ChecklistItem>,
    pub graph: ImpactGraph,
    pub model: ImpactModelInfo,
}

fn impact_level(score: f64) -> &'static str {
    if score >= HIGH_THRESHOLD {
        "high"
    } else if score >= MEDIUM_THRESHOLD {
        "medium"
    } else {
        "low"
    }
}

pub struct ImpactInputs<'a> {
    pub commits: &'a [Commit],
    pub edges: &'a [DependencyEdge],
    pub cochange_pairs: &'a HashMap<(String, String), usize>,
    pub file_paths: Vec<String>,
    pub contents: HashMap<&'a str, &'a str>,
}

pub fn simulate(
    description: &str,
    inputs: &ImpactInputs<'_>,
    config: &ImpactConfig,
) -> ImpactResponse {
    let weights = config.weights.normalized();
    let intent = extract_intent(description);

    let counts = change_counts(inputs.commits);
    let adjacency: HashSet<(&str, &str)> = inputs
        .edges
        .iter()
        .flat_map(|edge| {
            [
                (edge.source.as_str(), edge.target.as_str()),
                (edge.target.as_str(), edge.source.as_str()),
            ]
        })
        .collect();

    let mut direct: HashSet<&str> = HashSet::new();
    let mut scored: Vec<ScoredFile> = Vec::new();

    for path in &inputs.file_paths {
        let (path_hit, path_terms) = path_score(path, &intent.terms);
        let content = inputs.contents.get(path.as_str()).copied().unwrap_or("");
        let (content_hit, content_terms) = content_score(content, &intent.terms);

        let semantic = path_hit.max(content_hit).clamp(0.0, 1.0);
        if semantic >= 0.5 {
            direct.insert(path.as_str());
        }

        let kind = classify_path(path);
        let kind_score = if kind != PathKind::Source && semantic > 0.0 {
            1.0
        } else {
            0.0
        };

        let mut evidence = Vec::new();
        if !path_terms.is_empty() {
            let mut terms = path_terms.clone();
            terms.sort();
            terms.dedup();
            evidence.push(format!(
                "Path matches change concept: {}.",
                terms.join(", ")
            ));
        }
        if !content_terms.is_empty() {
            let mut terms = content_terms.clone();

            terms.sort();
            terms.dedup();
            let shown: Vec<String> = terms.into_iter().take(6).collect();
            evidence.push(format!(
                "Source references change concept: {}.",
                shown.join(", ")
            ));
        }
        scored.push((
            path.clone(),
            semantic,
            kind_score,
            path_hit,
            content_hit,
            evidence,
            Vec::new(),
        ));
    }

    let mut items = Vec::new();
    for (path, semantic, kind_score, _, _, mut evidence, _) in scored {
        let mut dep_evidence: Vec<String> = Vec::new();
        for hit in &direct {
            if adjacency.contains(&(path.as_str(), *hit))
                || adjacency.contains(&(*hit, path.as_str()))
            {
                dep_evidence.push((*hit).to_owned());
            }
        }
        dep_evidence.sort();
        dep_evidence.dedup();
        let dependency = if dep_evidence.is_empty() { 0.0 } else { 1.0 };
        if !dep_evidence.is_empty() {
            let shown: Vec<String> = dep_evidence.iter().take(3).cloned().collect();
            evidence.push(format!(
                "Dependency relationship with {}.",
                shown.join(", ")
            ));
        }

        let mut best_count = 0;
        let mut best_prob = 0.0;
        let mut best_peer = None;
        for hit in &direct {
            if *hit == path.as_str() {
                continue;
            }
            let count = cochange_count(inputs.cochange_pairs, &path, hit);
            let changes = counts.get(*hit).copied().unwrap_or(0) as f64;
            let prob = if changes > 0.0 {
                count as f64 / changes
            } else {
                0.0
            };
            if prob > best_prob {
                best_prob = prob;
                best_count = count;
                best_peer = Some((*hit).to_owned());
            }
        }
        let historical = best_prob.clamp(0.0, 1.0);
        if best_count > 0 {
            evidence.push(format!(
                "Changed with {} in {best_count} historical commit{}.",
                best_peer.unwrap_or_default(),
                if best_count == 1 { "" } else { "s" }
            ));
        }

        let combined = (weights.semantic * semantic
            + weights.dependency * dependency
            + weights.historical * historical
            + weights.kind * kind_score)
            .clamp(0.0, 1.0);

        let combined = if semantic >= 0.5 {
            combined.max(DIRECT_MATCH_FLOOR)
        } else {
            combined
        };

        let mut categories = Vec::new();
        if semantic >= 0.5 {
            categories.push("direct".to_owned());
        }
        if dependency > 0.0 {
            categories.push("dependent".to_owned());
        }
        if historical >= 0.3 {
            categories.push("historically_coupled".to_owned());
        }
        match classify_path(&path) {
            PathKind::Test if kind_score > 0.0 => categories.push("test".to_owned()),
            PathKind::Config if kind_score > 0.0 => categories.push("configuration".to_owned()),
            PathKind::Docs if kind_score > 0.0 => categories.push("documentation".to_owned()),
            _ => {}
        }
        if categories.is_empty() {
            categories.push("indirect".to_owned());
        }

        items.push(ImpactItem {
            path,
            level: impact_level(combined).to_owned(),
            score: combined,
            categories,
            evidence,
        });
    }

    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
    items.retain(|item| item.score >= MIN_SCORE);

    if !config.include_low_confidence {
        items.retain(|item| item.level != "low");
    }
    items.truncate(config.max_results.max(1));

    let capabilities = build_capability_map(&inputs.file_paths);
    let checklist = build_checklist(&items, &capabilities);
    let graph = build_graph(description, &items, &capabilities);

    let model = ImpactModelInfo {
        name: "impact_baseline".to_owned(),
        calibrated: false,
        threshold: MEDIUM_THRESHOLD,
        weights: HashMap::from([
            ("semantic".to_owned(), weights.semantic),
            ("dependency".to_owned(), weights.dependency),
            ("historical".to_owned(), weights.historical),
            ("kind".to_owned(), weights.kind),
        ]),
        training: "Deterministic baseline over repository structure, history and path-kind signals; no trained weights. See analysis/impact.rs.".to_owned(),
    };

    ImpactResponse {
        change_description: description.to_owned(),
        intent,
        impact: items,
        capabilities,
        checklist,
        graph,
        model,
    }
}

fn build_checklist(items: &[ImpactItem], capabilities: &[Capability]) -> Vec<ChecklistItem> {
    let mut checklist = Vec::new();
    let impacted: HashSet<&str> = items.iter().map(|item| item.path.as_str()).collect();
    for capability in capabilities {
        let files: Vec<String> = capability
            .files
            .iter()
            .filter(|file| impacted.contains(file.as_str()))
            .cloned()
            .collect();
        if files.is_empty() {
            continue;
        }

        checklist.push(ChecklistItem {
            label: format!("Review {}", capability.name.to_ascii_lowercase()),
            files,
        });
    }
    for (label, category) in [
        ("Review tests", "test"),
        ("Review configuration", "configuration"),
        ("Review documentation", "documentation"),
    ] {
        let files: Vec<String> = items
            .iter()
            .filter(|item| item.categories.iter().any(|c| c == category))
            .map(|item| item.path.clone())
            .collect();
        if !files.is_empty() {
            checklist.push(ChecklistItem {
                label: label.to_owned(),
                files,
            });
        }
    }
    checklist
}

fn build_graph(
    description: &str,
    items: &[ImpactItem],
    capabilities: &[Capability],
) -> ImpactGraph {
    let mut nodes = vec![ImpactGraphNode {
        id: "change".to_owned(),
        kind: "change".to_owned(),
        label: description.to_owned(),
        level: None,
    }];
    let mut edges = Vec::new();
    let impacted: HashSet<&str> = items.iter().map(|item| item.path.as_str()).collect();
    let by_path: HashMap<&str, &ImpactItem> = items
        .iter()
        .map(|item| (item.path.as_str(), item))
        .collect();

    for capability in capabilities {
        let files: Vec<&str> = capability
            .files
            .iter()
            .filter(|file| impacted.contains(file.as_str()))
            .map(String::as_str)
            .collect();
        if files.is_empty() {
            continue;
        }
        let id = format!("capability:{}", capability.name);

        let level = files
            .iter()
            .filter_map(|file| by_path.get(file).map(|item| item.level.as_str()))
            .min_by_key(|level| match *level {
                "high" => 0,
                "medium" => 1,
                _ => 2,
            })
            .unwrap_or("low")
            .to_owned();
        nodes.push(ImpactGraphNode {
            id: id.clone(),
            kind: "capability".to_owned(),
            label: capability.name.clone(),
            level: Some(level),
        });
        edges.push(ImpactGraphEdge {
            from: "change".to_owned(),
            to: id.clone(),
            evidence: format!(
                "{} impacted file{} in {}.",
                files.len(),
                if files.len() == 1 { "" } else { "s" },
                capability.name
            ),
        });
        for file in files {
            let item = by_path[file];
            nodes.push(ImpactGraphNode {
                id: file.to_owned(),
                kind: "file".to_owned(),
                label: file.to_owned(),
                level: Some(item.level.clone()),
            });
            edges.push(ImpactGraphEdge {
                from: id.clone(),
                to: file.to_owned(),
                evidence: item
                    .evidence
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Repository relationship.".to_owned()),
            });
        }
    }
    ImpactGraph { nodes, edges }
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactEvalMetrics {
    pub precision_at_k: Option<f64>,
    pub recall_at_k: Option<f64>,
    pub f1_at_k: Option<f64>,
    pub directory_accuracy: Option<f64>,
    pub test_discovery_rate: Option<f64>,
    pub config_discovery_rate: Option<f64>,
    pub evaluated_targets: usize,
    pub k: usize,
    pub baseline: String,
}

fn parent_dir(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(dir, _)| dir.to_owned())
        .unwrap_or_default()
}

pub fn evaluate_impact(
    commits: &[Commit],
    edges: &[DependencyEdge],
    cochange_pairs: &HashMap<(String, String), usize>,
    file_paths: &[String],
    contents: &HashMap<String, String>,
    baseline: &str,
    k: usize,
) -> ImpactEvalMetrics {
    let k = k.max(1);
    let weights = match baseline {
        "keyword-only" => ImpactWeights::default()
            .without("dependency")
            .without("historical")
            .without("kind"),
        "dependency-only" => ImpactWeights {
            semantic: 0.0,
            dependency: 1.0,
            historical: 0.0,
            kind: 0.0,
        },
        "cochange-only" => ImpactWeights {
            semantic: 0.0,
            dependency: 0.0,
            historical: 1.0,
            kind: 0.0,
        },
        _ => ImpactWeights::default().normalized(),
    };
    let config = ImpactConfig {
        weights,
        max_results: k,
        include_low_confidence: true,
    };

    let mut ordered: Vec<&Commit> = commits
        .iter()
        .filter(|commit| commit.timestamp.is_some() && !commit.message.trim().is_empty())
        .collect();
    ordered.sort_by_key(|commit| (commit.timestamp.unwrap_or(0), commit.sha.clone()));
    if ordered.len() < 5 {
        return ImpactEvalMetrics {
            precision_at_k: None,
            recall_at_k: None,
            f1_at_k: None,
            directory_accuracy: None,
            test_discovery_rate: None,
            config_discovery_rate: None,
            evaluated_targets: 0,
            k,
            baseline: baseline.to_owned(),
        };
    }
    let split_at = ordered.len() * 4 / 5;
    let mut precisions = Vec::new();
    let mut recalls = Vec::new();
    let mut dir_hits = Vec::new();
    let mut test_rates = Vec::new();
    let mut config_rates = Vec::new();
    let mut evaluated = 0;

    for target in &ordered[split_at..] {
        let Some(target_ts) = target.timestamp else {
            continue;
        };
        let mut actual: Vec<String> = relevant_files(&target.files)
            .into_iter()
            .map(|file| file.filename)
            .collect();
        actual.sort();
        actual.dedup();
        if actual.is_empty() || actual.len() > 15 {
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
        let owned_contents: HashMap<&str, &str> = contents
            .iter()
            .map(|(path, content)| (path.as_str(), content.as_str()))
            .collect();
        let inputs = ImpactInputs {
            commits: &prefix,
            edges,
            cochange_pairs: &prefix_metrics.cochange_pairs,
            file_paths: file_paths.to_vec(),
            contents: owned_contents,
        };
        let response = simulate(&target.message, &inputs, &config);
        let ranked: Vec<&str> = response
            .impact
            .iter()
            .map(|item| item.path.as_str())
            .take(k)
            .collect();
        if ranked.is_empty() {
            continue;
        }
        evaluated += 1;
        let actual_set: HashSet<&str> = actual.iter().map(String::as_str).collect();
        let hits = ranked
            .iter()
            .filter(|path| actual_set.contains(**path))
            .count();
        let precision = hits as f64 / ranked.len() as f64;
        let recall = hits as f64 / actual.len() as f64;
        precisions.push(precision);
        recalls.push(recall);

        let predicted_dirs: HashSet<String> = ranked.iter().map(|path| parent_dir(path)).collect();
        let actual_dirs: HashSet<String> = actual.iter().map(|path| parent_dir(path)).collect();
        let dir_hit = actual_dirs
            .iter()
            .filter(|dir| predicted_dirs.contains(*dir))
            .count();
        dir_hits.push(if actual_dirs.is_empty() {
            1.0
        } else {
            dir_hit as f64 / actual_dirs.len() as f64
        });

        for (rates, kind) in [
            (&mut test_rates, PathKind::Test),
            (&mut config_rates, PathKind::Config),
        ] {
            let wanted: Vec<&String> = actual
                .iter()
                .filter(|path| classify_path(path) == kind)
                .collect();
            if wanted.is_empty() {
                continue;
            }
            let found = wanted
                .iter()
                .filter(|path| ranked.contains(&path.as_str()))
                .count();
            rates.push(found as f64 / wanted.len() as f64);
        }
    }

    let _ = cochange_pairs;
    let mean = |values: &[f64]| {
        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f64>() / values.len() as f64)
        }
    };
    let precision = mean(&precisions);
    let recall = mean(&recalls);
    let f1 = match (precision, recall) {
        (Some(p), Some(r)) if p + r > 0.0 => Some(2.0 * p * r / (p + r)),
        _ => None,
    };
    ImpactEvalMetrics {
        precision_at_k: precision,
        recall_at_k: recall,
        f1_at_k: f1,
        directory_accuracy: mean(&dir_hits),
        test_discovery_rate: mean(&test_rates),
        config_discovery_rate: mean(&config_rates),
        evaluated_targets: evaluated,
        k,
        baseline: baseline.to_owned(),
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

    fn commit(sha: &str, ts: i64, message: &str, files: &[&str]) -> Commit {
        Commit {
            sha: sha.to_owned(),
            message: message.to_owned(),
            author: None,
            timestamp: Some(ts),
            date: None,
            url: String::new(),
            stats: CommitStats::default(),
            files: files.iter().map(|name| file(name)).collect(),
        }
    }

    fn inputs<'a>(
        commits: &'a [Commit],
        edges: &'a [DependencyEdge],
        cochange: &'a HashMap<(String, String), usize>,
        paths: Vec<String>,
    ) -> ImpactInputs<'a> {
        ImpactInputs {
            commits,
            edges,
            cochange_pairs: cochange,
            file_paths: paths,
            contents: HashMap::new(),
        }
    }

    fn cochange_of(commits: &[Commit]) -> HashMap<(String, String), usize> {
        let (_, metrics) = super::super::history::analyze(commits);
        metrics.cochange_pairs
    }

    #[test]
    fn intent_extraction_parses_replace_operation() {
        let intent = extract_intent("Replace JWT authentication with OAuth2");
        assert_eq!(intent.operation, "replace");
        assert!(intent.domains.contains(&"authentication".to_owned()));
        assert_eq!(intent.current_technology.as_deref(), Some("jwt"));
        assert_eq!(intent.target_technology.as_deref(), Some("oauth2"));
    }

    #[test]
    fn intent_extraction_handles_add_and_unknown() {
        let add = extract_intent("Add role-based authorization");
        assert_eq!(add.operation, "add");
        assert!(add.domains.contains(&"authorization".to_owned()));

        let unknown = extract_intent("zxqv blorp");
        assert_eq!(unknown.operation, "general");
        assert!(unknown.domains.is_empty());
        assert!(unknown.current_technology.is_none());
    }

    #[test]
    fn capability_map_groups_by_domain_without_manual_input() {
        let paths = vec![
            "src/auth/login.ts".to_owned(),
            "src/auth/session.ts".to_owned(),
            "src/payments/stripe.ts".to_owned(),
            "README.md".to_owned(),
        ];
        let capabilities = build_capability_map(&paths);
        let auth = capabilities
            .iter()
            .find(|capability| capability.name == "authentication")
            .expect("auth capability");
        assert!(auth.files.contains(&"src/auth/login.ts".to_owned()));
        let payments = capabilities
            .iter()
            .find(|capability| capability.name == "payments")
            .expect("payments capability");
        assert!(
            payments
                .files
                .contains(&"src/payments/stripe.ts".to_owned())
        );
    }

    #[test]
    fn semantic_matching_finds_auth_files() {
        let commits = vec![commit("c1", 100, "m", &["src/auth/login.ts"])];
        let cochange = cochange_of(&commits);
        let paths = vec![
            "src/auth/login.ts".to_owned(),
            "src/payments/stripe.ts".to_owned(),
        ];
        let response = simulate(
            "Replace JWT authentication with OAuth2",
            &inputs(&commits, &[], &cochange, paths),
            &ImpactConfig::default(),
        );
        assert!(
            response
                .impact
                .iter()
                .any(|item| item.path == "src/auth/login.ts"
                    && item.categories.contains(&"direct".to_owned()))
        );
        assert!(
            !response
                .impact
                .iter()
                .any(|item| item.path == "src/payments/stripe.ts")
        );
    }

    #[test]
    fn dependency_impact_marks_dependents() {
        let commits = vec![commit("c1", 100, "m", &["src/auth/login.ts"])];
        let cochange = cochange_of(&commits);
        let edges = vec![DependencyEdge {
            source: "src/api/users.ts".to_owned(),
            target: "src/auth/login.ts".to_owned(),
        }];
        let paths = vec![
            "src/auth/login.ts".to_owned(),
            "src/api/users.ts".to_owned(),
        ];
        let response = simulate(
            "Refactor the authentication module",
            &inputs(&commits, &edges, &cochange, paths),
            &ImpactConfig::default(),
        );
        let api = response
            .impact
            .iter()
            .find(|item| item.path == "src/api/users.ts")
            .expect("dependent file");
        assert!(api.categories.contains(&"dependent".to_owned()));
    }

    #[test]
    fn historical_impact_uses_cochange() {
        let commits = vec![
            commit("c1", 100, "m", &["src/auth/login.ts", "src/api/users.ts"]),
            commit("c2", 200, "m", &["src/auth/login.ts", "src/api/users.ts"]),
            commit("c3", 300, "m", &["src/auth/login.ts"]),
        ];
        let cochange = cochange_of(&commits);
        let paths = vec![
            "src/auth/login.ts".to_owned(),
            "src/api/users.ts".to_owned(),
        ];
        let response = simulate(
            "Replace JWT authentication with OAuth2",
            &inputs(&commits, &[], &cochange, paths),
            &ImpactConfig::default(),
        );
        let api = response
            .impact
            .iter()
            .find(|item| item.path == "src/api/users.ts")
            .expect("coupled file");
        assert!(api.categories.contains(&"historically_coupled".to_owned()));
        assert!(api.evidence.iter().any(|line| line.contains("historical")));
    }

    #[test]
    fn test_config_docs_discovery_by_kind() {
        let commits = vec![commit("c1", 100, "m", &["src/auth/login.ts"])];
        let cochange = cochange_of(&commits);
        let paths = vec![
            "src/auth/login.ts".to_owned(),
            "tests/auth.test.ts".to_owned(),
            "config/auth.yml".to_owned(),
            "docs/auth.md".to_owned(),
        ];
        let config = ImpactConfig {
            include_low_confidence: true,
            ..ImpactConfig::default()
        };
        let response = simulate(
            "Refactor authentication",
            &inputs(&commits, &[], &cochange, paths),
            &config,
        );
        let categories: HashSet<String> = response
            .impact
            .iter()
            .flat_map(|item| item.categories.clone())
            .collect();
        assert!(categories.contains("test"));
        assert!(categories.contains("configuration"));
        assert!(categories.contains("documentation"));
    }

    #[test]
    fn scores_are_normalized_and_levels_ordered() {
        let commits = vec![commit("c1", 100, "m", &["src/auth/login.ts"])];
        let cochange = cochange_of(&commits);
        let paths = vec![
            "src/auth/login.ts".to_owned(),
            "src/auth/session.ts".to_owned(),
        ];
        let response = simulate(
            "Replace JWT authentication with OAuth2",
            &inputs(&commits, &[], &cochange, paths),
            &ImpactConfig::default(),
        );
        for item in &response.impact {
            assert!((0.0..=1.0).contains(&item.score));
            assert!(["high", "medium", "low"].contains(&item.level.as_str()));
        }
        let scores: Vec<f64> = response.impact.iter().map(|item| item.score).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert_eq!(scores, sorted);
    }

    #[test]
    fn empty_repository_and_unknown_description_are_safe() {
        let commits: Vec<Commit> = Vec::new();
        let response = simulate(
            "Replace JWT authentication with OAuth2",
            &inputs(&commits, &[], &HashMap::new(), Vec::new()),
            &ImpactConfig::default(),
        );
        assert!(response.impact.is_empty());
        assert!(response.checklist.is_empty());

        let commits = [commit("c1", 100, "m", &["src/main.rs"])];
        let cochange = cochange_of(&commits);
        let unrelated = simulate(
            "zxqv blorp",
            &inputs(&commits, &[], &cochange, vec!["src/main.rs".to_owned()]),
            &ImpactConfig::default(),
        );
        assert!(unrelated.impact.is_empty());
    }

    #[test]
    fn leakage_prefix_excludes_target_and_future() {
        let commits = [
            commit(
                "c1",
                100,
                "auth setup",
                &["src/auth/login.ts", "src/api/a.ts"],
            ),
            commit(
                "c2",
                200,
                "auth tweak",
                &["src/auth/login.ts", "src/api/a.ts"],
            ),
            commit(
                "c3",
                300,
                "auth overhaul",
                &["src/auth/login.ts", "src/api/b.ts"],
            ),
        ];
        let prefix: Vec<Commit> = commits
            .iter()
            .filter(|commit| commit.timestamp.is_some_and(|ts| ts < 300))
            .cloned()
            .collect();
        let (_, metrics) = super::super::history::analyze(&prefix);
        let paths = vec![
            "src/auth/login.ts".to_owned(),
            "src/api/a.ts".to_owned(),
            "src/api/b.ts".to_owned(),
        ];
        let borrowed: HashMap<&str, &str> = HashMap::new();
        let response = simulate(
            "auth overhaul",
            &ImpactInputs {
                commits: &prefix,
                edges: &[],
                cochange_pairs: &metrics.cochange_pairs,
                file_paths: paths,
                contents: borrowed,
            },
            &ImpactConfig::default(),
        );

        assert!(
            response
                .impact
                .iter()
                .all(|item| item.path != "src/api/b.ts")
        );
        assert!(
            response
                .impact
                .iter()
                .any(|item| item.path == "src/api/a.ts")
        );
    }

    #[test]
    fn evaluation_is_chronological_with_baselines() {
        let commits = vec![
            commit("c1", 100, "auth setup", &["src/auth/a.ts", "src/api/a.ts"]),
            commit("c2", 200, "auth tweak", &["src/auth/a.ts", "src/api/a.ts"]),
            commit("c3", 300, "auth fix", &["src/auth/a.ts", "src/api/a.ts"]),
            commit("c4", 400, "auth fix", &["src/auth/a.ts", "src/api/a.ts"]),
            commit("c5", 500, "auth fix", &["src/auth/a.ts", "src/api/a.ts"]),
            commit("c6", 600, "auth fix", &["src/auth/a.ts", "src/api/a.ts"]),
        ];
        let cochange = cochange_of(&commits);
        let paths = vec!["src/auth/a.ts".to_owned(), "src/api/a.ts".to_owned()];
        let contents: HashMap<String, String> = HashMap::new();
        let edges = vec![DependencyEdge {
            source: "src/api/a.ts".to_owned(),
            target: "src/auth/a.ts".to_owned(),
        }];
        for baseline in [
            "combined",
            "keyword-only",
            "dependency-only",
            "cochange-only",
        ] {
            let report =
                evaluate_impact(&commits, &edges, &cochange, &paths, &contents, baseline, 2);
            assert_eq!(report.baseline, baseline);
            assert!(report.evaluated_targets >= 1, "baseline {baseline}");
        }
    }
}
