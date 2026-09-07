use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RepositoryAnalysis {
    pub repository: RepositoryInfo,
    pub source: SourceAnalysis,
    pub complexity: ComplexityAnalysis,
    pub history: HistoryAnalysis,
    pub dependencies: DependencyAnalysis,
    pub hotspots: Vec<Hotspot>,
    pub difficulty: DifficultyScore,
}

#[derive(Debug, Serialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub default_branch: String,
    pub total_files: usize,
}

#[derive(Debug, Serialize)]
pub struct SourceAnalysis {
    pub source_files: usize,
    pub total_lines: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ComplexityAnalysis {
    pub average_complexity: f64,
    pub max_complexity: usize,
    pub complex_files: usize,
}

#[derive(Debug, Serialize)]
pub struct HistoryAnalysis {
    pub total_commits: usize,
    pub active_contributors: usize,
    pub changed_files: usize,
}

#[derive(Debug, Serialize)]
pub struct DependencyAnalysis {
    pub total_dependencies: usize,
    pub highly_connected_files: usize,
}

#[derive(Debug, Serialize)]
pub struct Hotspot {
    pub path: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DifficultyScore {
    pub score: f64,
    pub level: String,
}
