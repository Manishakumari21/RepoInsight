use super::{
    complexity::ComplexityResult,
    dependencies::DependencyMetrics,
    history::{FileHistory, HistoryMetrics},
    models::{DifficultyScore, Hotspot},
};

#[derive(Debug, Clone)]
pub struct FileAnalysis {
    pub path: String,
    pub complexity: ComplexityResult,
}

#[derive(Debug, Clone, Copy)]
pub struct RiskWeights {
    pub complexity: f64,
    pub churn: f64,
    pub coupling: f64,
    pub nesting: f64,
}

impl Default for RiskWeights {
    fn default() -> Self {
        Self {
            complexity: 0.35,
            churn: 0.30,
            coupling: 0.20,
            nesting: 0.15,
        }
    }
}

impl RiskWeights {
    fn normalized(self) -> Self {
        let total = self.complexity + self.churn + self.coupling + self.nesting;

        if total == 0.0 {
            return Self::default();
        }

        Self {
            complexity: self.complexity / total,
            churn: self.churn / total,
            coupling: self.coupling / total,
            nesting: self.nesting / total,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct FileSignals {
    complexity: f64,
    churn: f64,
    coupling: f64,
    nesting: f64,
}

pub fn calculate_hotspots(
    files: &[FileAnalysis],
    history: &HistoryMetrics,
    dependencies: &DependencyMetrics,
) -> Vec<Hotspot> {
    if files.is_empty() {
        return Vec::new();
    }

    let weights = RiskWeights::default().normalized();

    let complexity_values = files
        .iter()
        .map(|file| file.complexity.cyclomatic as f64)
        .collect::<Vec<_>>();

    let nesting_values = files
        .iter()
        .map(|file| file.complexity.max_nesting_depth as f64)
        .collect::<Vec<_>>();

    let churn_values = files
        .iter()
        .map(|file| {
            history
                .file_changes
                .get(&file.path)
                .map(file_churn)
                .unwrap_or(0.0)
        })
        .collect::<Vec<_>>();

    let coupling_values = files
        .iter()
        .map(|file| dependencies.coupling.get(&file.path).copied().unwrap_or(0) as f64)
        .collect::<Vec<_>>();

    files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let signals = FileSignals {
                complexity: percentile_score(complexity_values[index], &complexity_values),
                churn: percentile_score(churn_values[index], &churn_values),
                coupling: percentile_score(coupling_values[index], &coupling_values),
                nesting: percentile_score(nesting_values[index], &nesting_values),
            };

            let score = weighted_score(signals, weights);

            Hotspot {
                path: file.path.clone(),
                score,
                reasons: build_reasons(signals),
            }
        })
        .filter(|hotspot| hotspot.score > 0.0)
        .collect::<Vec<_>>()
}

pub fn calculate_difficulty(
    files: &[FileAnalysis],
    history: &HistoryMetrics,
    dependencies: &DependencyMetrics,
) -> DifficultyScore {
    if files.is_empty() {
        return DifficultyScore {
            score: 0.0,
            level: "unknown".to_owned(),
        };
    }

    let hotspots = calculate_hotspots(files, history, dependencies);

    if hotspots.is_empty() {
        return DifficultyScore {
            score: 0.0,
            level: "low".to_owned(),
        };
    }

    let total = hotspots.iter().map(|hotspot| hotspot.score).sum::<f64>();
    let average = total / hotspots.len() as f64;

    let score = clamp_score(average);

    DifficultyScore {
        score,
        level: difficulty_level(score).to_owned(),
    }
}

fn file_churn(history: &FileHistory) -> f64 {
    history.churn as f64
}

fn weighted_score(signals: FileSignals, weights: RiskWeights) -> f64 {
    let score = signals.complexity * weights.complexity
        + signals.churn * weights.churn
        + signals.coupling * weights.coupling
        + signals.nesting * weights.nesting;

    clamp_score(score)
}

fn percentile_score(value: f64, values: &[f64]) -> f64 {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);

    if (max - min).abs() <= f64::EPSILON {
        return 0.0;
    }

    let less_or_equal = values
        .iter()
        .filter(|candidate| **candidate <= value)
        .count();

    let percentile = (less_or_equal.saturating_sub(1) as f64) / (values.len() - 1) as f64;

    percentile * 100.0
}

fn build_reasons(signals: FileSignals) -> Vec<String> {
    let mut reasons = Vec::new();

    let mut ranked = [
        ("high relative complexity", signals.complexity),
        ("high relative churn", signals.churn),
        ("high dependency coupling", signals.coupling),
        ("deep nesting", signals.nesting),
    ];

    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));

    for (reason, value) in ranked {
        if value >= 75.0 {
            reasons.push(reason.to_owned());
        }
    }

    if reasons.is_empty() && let Some((reason, _)) = ranked.first() {
        reasons.push(reason.to_string());
    }

    reasons
}

fn difficulty_level(score: f64) -> &'static str {
    match score {
        score if score >= 75.0 => "high",
        score if score >= 50.0 => "medium",
        score if score >= 25.0 => "moderate",
        _ => "low",
    }
}

fn clamp_score(score: f64) -> f64 {
    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_is_repository_relative() {
        let values = vec![1.0, 2.0, 3.0, 10.0];

        assert_eq!(percentile_score(10.0, &values), 100.0);
        assert_eq!(percentile_score(1.0, &values), 0.0);
    }

    #[test]
    fn zero_values_do_not_create_fake_risk() {
        let values = vec![0.0, 0.0, 0.0];

        assert_eq!(percentile_score(0.0, &values), 0.0);
    }

    #[test]
    fn score_stays_within_expected_range() {
        let signals = FileSignals {
            complexity: 100.0,
            churn: 100.0,
            coupling: 100.0,
            nesting: 100.0,
        };

        let score = weighted_score(signals, RiskWeights::default().normalized());

        assert!((0.0..=100.0).contains(&score));
    }

    #[test]
    fn empty_repository_has_zero_difficulty() {
        let history = HistoryMetrics::default();
        let dependencies = DependencyMetrics::default();

        let result = calculate_difficulty(&[], &history, &dependencies);

        assert_eq!(result.score, 0.0);
        assert_eq!(result.level, "unknown");
    }
}
