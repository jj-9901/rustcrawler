use crate::models::LinkEdge;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use std::collections::{HashMap, HashSet};
use std::fs;

pub fn build_graph(edges: &[LinkEdge]) -> DiGraph<String, ()> {
    let mut graph = DiGraph::new();
    let mut node_map: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();

    for edge in edges {
        let edge_key = (edge.from.clone(), edge.to.clone());
        if seen_edges.contains(&edge_key) {
            continue;
        }
        seen_edges.insert(edge_key);

        let from_idx = if let Some(&idx) = node_map.get(&edge.from) {
            idx
        } else {
            let idx = graph.add_node(edge.from.clone());
            node_map.insert(edge.from.clone(), idx);
            idx
        };

        let to_idx = if let Some(&idx) = node_map.get(&edge.to) {
            idx
        } else {
            let idx = graph.add_node(edge.to.clone());
            node_map.insert(edge.to.clone(), idx);
            idx
        };

        graph.add_edge(from_idx, to_idx, ());
    }

    graph
}

pub fn export_graph(graph: &DiGraph<String, ()>, path: &str) {
    let dot = format!("{:?}", Dot::with_config(graph, &[Config::EdgeNoLabel]));
    fs::write(path, dot).expect("Could not write graph file");
    println!("  Saved to: {}", path);
    println!("  View at : https://dreampuf.github.io/GraphvizOnline");
}