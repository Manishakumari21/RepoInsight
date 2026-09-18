use serde::Serialize;

use crate::analysis::{
    complexity::ComplexityResult, dependencies::DependencyMetrics, source::SourceFile,
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StructuralFeatures {
    pub file_path: String,
    pub file_size_bytes: u64,
    pub lines_of_code: usize,
    pub function_count: usize,
    pub cyclomatic_complexity: usize,
    pub max_nesting_depth: usize,
    pub incoming_dependencies: usize,
    pub outgoing_dependencies: usize,
    pub coupling: usize,
    pub num_dependents: usize,
}

pub fn build_structural_features(
    file: &SourceFile,
    complexity: &ComplexityResult,
    dependencies: &DependencyMetrics,
) -> StructuralFeatures {
    let incoming = dependencies
        .incoming_dependencies
        .get(&file.path)
        .copied()
        .unwrap_or(0);

    let outgoing = dependencies
        .outgoing_dependencies
        .get(&file.path)
        .copied()
        .unwrap_or(0);

    let coupling = dependencies.coupling.get(&file.path).copied().unwrap_or(0);

    StructuralFeatures {
        file_path: file.path.clone(),
        file_size_bytes: file.size_bytes,
        lines_of_code: file.content.lines().count(),
        function_count: complexity.functions,
        cyclomatic_complexity: complexity.cyclomatic,
        max_nesting_depth: complexity.max_nesting_depth,
        incoming_dependencies: incoming,
        outgoing_dependencies: outgoing,
        coupling,
        num_dependents: incoming,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_structural_features() {
        let file = SourceFile {
            path: "src/example.rs".to_owned(),
            content: "fn main() {}\nfn test() {}\n".to_owned(),
            size_bytes: 27,
        };

        let complexity = ComplexityResult::new(3, 2, 1);

        let dependencies = DependencyMetrics {
            outgoing_dependencies: [("src/example.rs".to_owned(), 2)].into_iter().collect(),
            incoming_dependencies: [("src/example.rs".to_owned(), 4)].into_iter().collect(),
            coupling: [("src/example.rs".to_owned(), 6)].into_iter().collect(),
            ..Default::default()
        };

        let result = build_structural_features(&file, &complexity, &dependencies);

        assert_eq!(result.file_path, "src/example.rs");
        assert_eq!(result.file_size_bytes, 27);
        assert_eq!(result.lines_of_code, 2);
        assert_eq!(result.function_count, 2);
        assert_eq!(result.cyclomatic_complexity, 3);
        assert_eq!(result.max_nesting_depth, 1);
        assert_eq!(result.incoming_dependencies, 4);
        assert_eq!(result.outgoing_dependencies, 2);
        assert_eq!(result.coupling, 6);
        assert_eq!(result.num_dependents, 4);
    }

    #[test]
    fn missing_dependency_data_defaults_to_zero() {
        let file = SourceFile {
            path: "src/standalone.rs".to_owned(),
            content: "fn main() {}\n".to_owned(),
            size_bytes: 13,
        };

        let complexity = ComplexityResult::new(1, 1, 0);
        let dependencies = DependencyMetrics::default();

        let result = build_structural_features(&file, &complexity, &dependencies);

        assert_eq!(result.incoming_dependencies, 0);
        assert_eq!(result.outgoing_dependencies, 0);
        assert_eq!(result.coupling, 0);
        assert_eq!(result.num_dependents, 0);
    }
}
