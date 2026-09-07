use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyEdge {
    pub source: String,

    pub target: String,
}

#[derive(Debug, Clone)]
pub struct DependencyMetrics {
    pub total_dependencies: usize,

    pub connected_files: usize,

    pub highly_connected_files: usize,

    pub outgoing_dependencies: HashMap<String, usize>,

    pub incoming_dependencies: HashMap<String, usize>,

    pub coupling: HashMap<String, usize>,
}

pub fn analyze(edges: &[DependencyEdge]) -> DependencyMetrics {
    let unique_edges = edges.iter().collect::<HashSet<_>>();

    let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
    let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();

    for edge in unique_edges {
        if edge.source == edge.target {
            continue;
        }

        outgoing
            .entry(edge.source.clone())
            .or_default()
            .insert(edge.target.clone());

        incoming
            .entry(edge.target.clone())
            .or_default()
            .insert(edge.source.clone());
    }

    let mut coupling = HashMap::new();

    let files = outgoing
        .keys()
        .chain(incoming.keys())
        .collect::<HashSet<_>>();

    for file in &files {
        let outgoing_count = outgoing.get(*file).map_or(0, HashSet::len);

        let incoming_count = incoming.get(*file).map_or(0, HashSet::len);

        coupling.insert((*file).clone(), outgoing_count + incoming_count);
    }

    let highly_connected_files = calculate_highly_connected_files(coupling.values().copied());

    DependencyMetrics {
        total_dependencies: outgoing.values().map(HashSet::len).sum(),

        connected_files: files.len(),

        highly_connected_files,

        outgoing_dependencies: outgoing
            .into_iter()
            .map(|(file, dependencies)| (file, dependencies.len()))
            .collect(),

        incoming_dependencies: incoming
            .into_iter()
            .map(|(file, dependents)| (file, dependents.len()))
            .collect(),

        coupling,
    }
}

fn calculate_highly_connected_files(values: impl Iterator<Item = usize>) -> usize {
    let values = values.collect::<Vec<_>>();

    if values.is_empty() {
        return 0;
    }

    let mean = values.iter().map(|&value| value as f64).sum::<f64>() / values.len() as f64;

    let variance = values
        .iter()
        .map(|&value| {
            let difference = value as f64 - mean;
            difference * difference
        })
        .sum::<f64>()
        / values.len() as f64;

    let standard_deviation = variance.sqrt();
    let threshold = mean + standard_deviation;

    values
        .iter()
        .filter(|&&value| value as f64 > threshold)
        .count()
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
