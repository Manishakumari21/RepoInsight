use tree_sitter::Node;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexityResult {
    pub cyclomatic: usize,

    pub functions: usize,

    pub max_nesting_depth: usize,
}

impl ComplexityResult {
    pub fn new(cyclomatic: usize, functions: usize, max_nesting_depth: usize) -> Self {
        Self {
            cyclomatic,
            functions,
            max_nesting_depth,
        }
    }
}

pub trait SyntaxAnalyzer {
    fn is_function(&self, node: Node<'_>) -> bool;

    fn is_decision(&self, node: Node<'_>) -> bool;

    fn is_nesting(&self, node: Node<'_>) -> bool;
}

pub fn analyze<A: SyntaxAnalyzer>(root: Node<'_>, adapter: &A) -> ComplexityResult {
    let mut metrics = Metrics::default();

    visit(root, adapter, 0, &mut metrics);

    ComplexityResult::new(
        metrics.decision_points.saturating_add(1),
        metrics.functions,
        metrics.max_nesting_depth,
    )
}

#[derive(Debug, Default)]
struct Metrics {
    decision_points: usize,
    functions: usize,
    max_nesting_depth: usize,
}

fn visit<A: SyntaxAnalyzer>(node: Node<'_>, analyzer: &A, depth: usize, metrics: &mut Metrics) {
    let current_depth = if analyzer.is_nesting(node) {
        depth.saturating_add(1)
    } else {
        depth
    };

    metrics.max_nesting_depth = metrics.max_nesting_depth.max(current_depth);

    if analyzer.is_function(node) {
        metrics.functions = metrics.functions.saturating_add(1);
    }

    if analyzer.is_decision(node) {
        metrics.decision_points = metrics.decision_points.saturating_add(1);
    }

    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        visit(child, analyzer, current_depth, metrics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAnalyzer;

    impl SyntaxAnalyzer for TestAnalyzer {
        fn is_function(&self, node: Node<'_>) -> bool {
            node.kind() == "test_function"
        }

        fn is_decision(&self, node: Node<'_>) -> bool {
            node.kind() == "test_decision"
        }

        fn is_nesting(&self, node: Node<'_>) -> bool {
            node.kind() == "test_block"
        }
    }

    #[test]
    fn empty_tree_has_base_complexity() {
        assert_eq!(
            ComplexityResult::new(1, 0, 0),
            ComplexityResult {
                cyclomatic: 1,
                functions: 0,
                max_nesting_depth: 0,
            }
        );
    }

    #[test]
    fn complexity_result_is_constructed_correctly() {
        let result = ComplexityResult::new(4, 3, 2);

        assert_eq!(result.cyclomatic, 4);
        assert_eq!(result.functions, 3);
        assert_eq!(result.max_nesting_depth, 2);
    }
}
