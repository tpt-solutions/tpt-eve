// SPDX-License-Identifier: Apache-2.0
use std::collections::{HashMap, HashSet};

use eve_core::{CausalRelationKind, EveError};
use serde::{Deserialize, Serialize};

/// A directed edge in the causal DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalEdge {
    pub from: String,
    pub to: String,
    pub relation: CausalRelationKind,
    /// Estimated causal strength in [-1.0, 1.0]. Positive = amplifying, negative = inhibiting.
    pub strength: f64,
}

/// A causal graph as a directed acyclic graph (DAG).
///
/// Nodes are named variables; edges encode causal direction with optional strength estimates
/// fitted from observations by the SCM layer. Cycle detection uses iterative DFS so the
/// graph stays a true DAG — `add_edge` rejects any edge that would introduce a cycle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CausalGraph {
    pub nodes: HashSet<String>,
    pub edges: Vec<CausalEdge>,
}

impl CausalGraph {
    pub fn new() -> Self {
        CausalGraph::default()
    }

    pub fn add_node(&mut self, name: impl Into<String>) {
        self.nodes.insert(name.into());
    }

    /// Add a directed causal edge. Returns `Err(Causal)` if the edge would create a cycle.
    pub fn add_edge(&mut self, edge: CausalEdge) -> Result<(), EveError> {
        self.nodes.insert(edge.from.clone());
        self.nodes.insert(edge.to.clone());
        self.edges.push(edge);
        if self.has_cycle() {
            let bad = self.edges.pop().unwrap();
            return Err(EveError::Causal(format!(
                "adding edge '{}' → '{}' would create a cycle",
                bad.from, bad.to
            )));
        }
        Ok(())
    }

    /// True if the graph contains at least one directed cycle (should never be true after
    /// `add_edge` guards, but useful as an invariant check).
    pub fn has_cycle(&self) -> bool {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut in_stack: HashSet<&str> = HashSet::new();
        for node in &self.nodes {
            if self.dfs_cycle(node, &mut visited, &mut in_stack) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle<'a>(
        &'a self,
        node: &'a str,
        visited: &mut HashSet<&'a str>,
        in_stack: &mut HashSet<&'a str>,
    ) -> bool {
        if in_stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        visited.insert(node);
        in_stack.insert(node);
        for edge in self.edges.iter().filter(|e| e.from == node) {
            if self.dfs_cycle(&edge.to, visited, in_stack) {
                return true;
            }
        }
        in_stack.remove(node);
        false
    }

    /// Edges whose `to` field is `node` (its parents in the DAG).
    pub fn parents_of(&self, node: &str) -> Vec<&CausalEdge> {
        self.edges.iter().filter(|e| e.to == node).collect()
    }

    /// Edges whose `from` field is `node` (its children in the DAG).
    pub fn children_of(&self, node: &str) -> Vec<&CausalEdge> {
        self.edges.iter().filter(|e| e.from == node).collect()
    }

    /// All ancestors of `node` reachable by following edges backwards.
    pub fn ancestors_of(&self, node: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut stack: Vec<&str> = self
            .parents_of(node)
            .iter()
            .map(|e| e.from.as_str())
            .collect();
        while let Some(n) = stack.pop() {
            if result.insert(n.to_string()) {
                for e in self.parents_of(n) {
                    stack.push(&e.from);
                }
            }
        }
        result
    }

    /// All descendants of `node` reachable by following edges forwards.
    pub fn descendants_of(&self, node: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut stack: Vec<&str> = self
            .children_of(node)
            .iter()
            .map(|e| e.to.as_str())
            .collect();
        while let Some(n) = stack.pop() {
            if result.insert(n.to_string()) {
                for e in self.children_of(n) {
                    stack.push(&e.to);
                }
            }
        }
        result
    }

    /// All simple directed paths from `from` to `to`, each as an ordered list of node names.
    pub fn paths_between(&self, from: &str, to: &str) -> Vec<Vec<String>> {
        let mut results = Vec::new();
        let mut current_path = vec![from.to_string()];
        self.dfs_paths(from, to, &mut current_path, &mut results);
        results
    }

    fn dfs_paths(
        &self,
        current: &str,
        target: &str,
        path: &mut Vec<String>,
        results: &mut Vec<Vec<String>>,
    ) {
        if current == target {
            results.push(path.clone());
            return;
        }
        for edge in self.children_of(current) {
            if !path.contains(&edge.to) {
                path.push(edge.to.clone());
                self.dfs_paths(&edge.to, target, path, results);
                path.pop();
            }
        }
    }

    /// Build an adjacency map: node → list of direct children.
    pub fn adjacency(&self) -> HashMap<&str, Vec<&str>> {
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            map.entry(node.as_str()).or_default();
        }
        for edge in &self.edges {
            map.entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym_edge(from: &str, to: &str) -> CausalEdge {
        CausalEdge {
            from: from.into(),
            to: to.into(),
            relation: CausalRelationKind::Causes,
            strength: 1.0,
        }
    }

    #[test]
    fn add_nodes_and_edges() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("A", "B")).unwrap();
        g.add_edge(sym_edge("B", "C")).unwrap();
        assert!(g.nodes.contains("A"));
        assert!(g.nodes.contains("B"));
        assert!(g.nodes.contains("C"));
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn cycle_detection_rejects_back_edge() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("A", "B")).unwrap();
        g.add_edge(sym_edge("B", "C")).unwrap();
        assert!(g.add_edge(sym_edge("C", "A")).is_err());
        // graph remains cycle-free and the rejected edge was rolled back
        assert_eq!(g.edges.len(), 2);
        assert!(!g.has_cycle());
    }

    #[test]
    fn self_loop_is_rejected() {
        let mut g = CausalGraph::new();
        assert!(g.add_edge(sym_edge("A", "A")).is_err());
    }

    #[test]
    fn parents_and_children() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("A", "C")).unwrap();
        g.add_edge(sym_edge("B", "C")).unwrap();
        g.add_edge(sym_edge("C", "D")).unwrap();
        let parents: Vec<_> = g.parents_of("C").iter().map(|e| e.from.as_str()).collect();
        assert!(parents.contains(&"A"));
        assert!(parents.contains(&"B"));
        let children: Vec<_> = g.children_of("C").iter().map(|e| e.to.as_str()).collect();
        assert_eq!(children, vec!["D"]);
    }

    #[test]
    fn ancestors_and_descendants() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("A", "B")).unwrap();
        g.add_edge(sym_edge("B", "C")).unwrap();
        let anc = g.ancestors_of("C");
        assert!(anc.contains("A") && anc.contains("B"));
        let desc = g.descendants_of("A");
        assert!(desc.contains("B") && desc.contains("C"));
    }

    #[test]
    fn paths_between_finds_all() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("A", "B")).unwrap();
        g.add_edge(sym_edge("A", "C")).unwrap();
        g.add_edge(sym_edge("B", "D")).unwrap();
        g.add_edge(sym_edge("C", "D")).unwrap();
        let paths = g.paths_between("A", "D");
        assert_eq!(paths.len(), 2);
    }
}
