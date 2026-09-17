use itertools::Itertools;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyMetrics {
    pub total_dependencies: usize,
    pub connected_files: usize,
    pub highly_connected_files: usize,
    #[allow(dead_code)]
    pub outgoing_dependencies: HashMap<String, usize>,
    #[allow(dead_code)]
    pub incoming_dependencies: HashMap<String, usize>,
    pub coupling: HashMap<String, usize>,
}

pub fn analyze(edges: &[DependencyEdge]) -> DependencyMetrics {
    let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
    let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();

    for edge in edges
        .iter()
        .unique()
        .filter(|edge| edge.source != edge.target)
    {
        outgoing
            .entry(edge.source.clone())
            .or_default()
            .insert(edge.target.clone());
        incoming
            .entry(edge.target.clone())
            .or_default()
            .insert(edge.source.clone());
    }

    let files: HashSet<&String> = outgoing.keys().chain(incoming.keys()).collect();

    let coupling: HashMap<String, usize> = files
        .iter()
        .map(|file| {
            let count = outgoing.get(*file).map_or(0, HashSet::len)
                + incoming.get(*file).map_or(0, HashSet::len);
            ((*file).clone(), count)
        })
        .collect();

    DependencyMetrics {
        total_dependencies: outgoing.values().map(HashSet::len).sum(),
        connected_files: files.len(),
        highly_connected_files: calculate_highly_connected_files(coupling.values().copied()),
        outgoing_dependencies: outgoing
            .into_iter()
            .map(|(key, value)| (key, value.len()))
            .collect(),
        incoming_dependencies: incoming
            .into_iter()
            .map(|(key, value)| (key, value.len()))
            .collect(),
        coupling,
    }
}

fn calculate_highly_connected_files(values: impl Iterator<Item = usize>) -> usize {
    let values: Vec<f64> = values.map(|value| value as f64).collect();
    if values.is_empty() {
        return 0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    let threshold = mean + variance.sqrt();

    values.iter().filter(|value| **value > threshold).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_edges_are_counted_once() {
        let edges = vec![
            DependencyEdge {
                source: "a.rs".into(),
                target: "b.rs".into(),
            },
            DependencyEdge {
                source: "a.rs".into(),
                target: "b.rs".into(),
            },
        ];

        let result = analyze(&edges);

        assert_eq!(result.total_dependencies, 1);
        assert_eq!(result.connected_files, 2);
    }

    #[test]
    fn self_dependencies_are_ignored() {
        let edges = vec![DependencyEdge {
            source: "a.rs".into(),
            target: "a.rs".into(),
        }];

        let result = analyze(&edges);

        assert_eq!(result.total_dependencies, 0);
        assert_eq!(result.connected_files, 0);
    }

    #[test]
    fn incoming_and_outgoing_coupling_are_calculated() {
        let edges = vec![
            DependencyEdge {
                source: "a".into(),
                target: "b".into(),
            },
            DependencyEdge {
                source: "c".into(),
                target: "b".into(),
            },
        ];

        let result = analyze(&edges);

        assert_eq!(result.outgoing_dependencies["a"], 1);
        assert_eq!(result.incoming_dependencies["b"], 2);
        assert_eq!(result.coupling["b"], 2);
    }

    #[test]
    fn empty_graph_is_supported() {
        let result = analyze(&[]);

        assert_eq!(result.total_dependencies, 0);
        assert_eq!(result.connected_files, 0);
        assert_eq!(result.highly_connected_files, 0);
    }
}
