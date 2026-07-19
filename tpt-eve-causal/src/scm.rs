// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{graph::CausalGraph, observation::Observation};

/// A simple linear Structural Causal Model.
///
/// For each node Y with parents X₁…Xₙ we store regression coefficients β₁…βₙ and a
/// per-node intercept β₀, estimated by multivariate OLS over the supplied observations.
/// This handles the common case where parents are correlated — univariate OLS per parent
/// would give wrong coefficients when parents are statistically dependent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuralCausalModel {
    /// (parent, child) → OLS coefficient
    pub coefficients: HashMap<(String, String), f64>,
    /// per-node intercepts (β₀)
    pub intercepts: HashMap<String, f64>,
}

impl StructuralCausalModel {
    /// Fit a linear SCM from `observations` using the parent structure in `graph`.
    ///
    /// For each node with parents, runs multivariate OLS across all parents jointly.
    /// Nodes with no parents get their mean value as intercept (constant root nodes).
    pub fn fit(graph: &CausalGraph, observations: &[Observation]) -> Self {
        let mut scm = StructuralCausalModel::default();

        for node in &graph.nodes {
            let parent_edges = graph.parents_of(node);
            if parent_edges.is_empty() {
                scm.intercepts
                    .insert(node.clone(), column_mean(observations, node));
            } else {
                let parent_names: Vec<&str> =
                    parent_edges.iter().map(|e| e.from.as_str()).collect();
                let (betas, intercept) = multivariate_ols(observations, &parent_names, node);
                scm.intercepts.insert(node.clone(), intercept);
                for (parent, beta) in parent_names.iter().zip(betas.iter()) {
                    scm.coefficients
                        .insert((parent.to_string(), node.clone()), *beta);
                }
            }
        }
        scm
    }

    /// Regression coefficient for (parent → child); 0.0 if unknown.
    pub fn coefficient(&self, parent: &str, child: &str) -> f64 {
        self.coefficients
            .get(&(parent.to_string(), child.to_string()))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn intercept(&self, node: &str) -> f64 {
        self.intercepts.get(node).copied().unwrap_or(0.0)
    }

    /// Predict all node values given a partial assignment, propagating in topological order.
    /// Nodes already present in `known` are not recomputed.
    pub fn predict(
        &self,
        graph: &CausalGraph,
        known: &HashMap<String, f64>,
    ) -> HashMap<String, f64> {
        let mut values = known.clone();
        for node in topo_sort(graph) {
            if values.contains_key(node.as_str()) {
                continue;
            }
            let parents = graph.parents_of(&node);
            if parents.is_empty() {
                values.insert(node.clone(), self.intercept(&node));
            } else {
                let mut predicted = self.intercept(&node);
                for edge in &parents {
                    if let Some(&pval) = values.get(&edge.from) {
                        predicted += self.coefficient(&edge.from, &node) * pval;
                    }
                }
                values.insert(node.clone(), predicted);
            }
        }
        values
    }
}

// ---------------------------------------------------------------------------
// OLS helpers
// ---------------------------------------------------------------------------

/// Multivariate OLS: regress `y_col` on all columns in `x_cols` jointly.
///
/// Returns `(betas, intercept)` where `betas[i]` corresponds to `x_cols[i]`.
/// Falls back to zeros when there are insufficient observations.
fn multivariate_ols(observations: &[Observation], x_cols: &[&str], y_col: &str) -> (Vec<f64>, f64) {
    let k = x_cols.len();

    // Collect rows where all columns are present
    let rows: Vec<(Vec<f64>, f64)> = observations
        .iter()
        .filter_map(|o| {
            let xs: Vec<f64> = x_cols.iter().filter_map(|&c| o.get(c)).collect();
            let y = o.get(y_col)?;
            if xs.len() == k {
                Some((xs, y))
            } else {
                None
            }
        })
        .collect();

    if rows.len() < k + 1 {
        // Not enough data: fall back to mean intercept, zero slopes
        let mean_y = if rows.is_empty() {
            0.0
        } else {
            rows.iter().map(|(_, y)| y).sum::<f64>() / rows.len() as f64
        };
        return (vec![0.0; k], mean_y);
    }

    // Build X'X (size (k+1)×(k+1)) and X'y (size k+1)
    // Column 0 is the intercept (constant 1).
    let dim = k + 1;
    let mut xtx = vec![vec![0.0_f64; dim]; dim];
    let mut xty = vec![0.0_f64; dim];

    for (xs, y) in &rows {
        let row: Vec<f64> = std::iter::once(1.0).chain(xs.iter().copied()).collect();
        for i in 0..dim {
            xty[i] += row[i] * y;
            for j in 0..dim {
                xtx[i][j] += row[i] * row[j];
            }
        }
    }

    let coeffs = gaussian_elimination(&mut xtx, &mut xty);
    let intercept = coeffs[0];
    let betas = coeffs[1..].to_vec();
    (betas, intercept)
}

/// Gaussian elimination with partial pivoting to solve Ax = b in-place.
fn gaussian_elimination(a: &mut [Vec<f64>], b: &mut [f64]) -> Vec<f64> {
    let n = b.len();

    for col in 0..n {
        // Partial pivot: find row with largest absolute value in this column
        let pivot = (col..n)
            .max_by(|&i, &j| {
                a[i][col]
                    .abs()
                    .partial_cmp(&a[j][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        a.swap(col, pivot);
        b.swap(col, pivot);

        let pv = a[col][col];
        if pv.abs() < 1e-12 {
            continue; // singular or near-singular — leave row as-is
        }

        for row in (col + 1)..n {
            let factor = a[row][col] / pv;
            // Clone the pivot row to avoid aliasing `a[col]` and `a[row]` simultaneously.
            let pivot_row: Vec<f64> = a[col].clone();
            for (offset, &pv_c) in pivot_row[col..].iter().enumerate() {
                a[row][col + offset] -= factor * pv_c;
            }
            b[row] -= factor * b[col];
        }
    }

    // Back substitution
    let mut x = vec![0.0_f64; n];
    for i in (0..n).rev() {
        x[i] = b[i];
        for j in (i + 1)..n {
            x[i] -= a[i][j] * x[j];
        }
        if a[i][i].abs() > 1e-12 {
            x[i] /= a[i][i];
        }
    }
    x
}

fn column_mean(observations: &[Observation], col: &str) -> f64 {
    let vals: Vec<f64> = observations.iter().filter_map(|o| o.get(col)).collect();
    if vals.is_empty() {
        return 0.0;
    }
    vals.iter().sum::<f64>() / vals.len() as f64
}

/// Topological sort of graph nodes (Kahn's algorithm). Exposed for counterfactual.rs.
pub(crate) fn topo_sort(graph: &CausalGraph) -> Vec<String> {
    let mut in_degree: HashMap<&str, usize> = graph.nodes.iter().map(|n| (n.as_str(), 0)).collect();
    for edge in &graph.edges {
        *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
    }
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&n, _)| n)
        .collect::<Vec<_>>()
        .into_iter()
        .collect();
    // Sort initial queue for deterministic output.
    let mut sorted_initial: Vec<&str> = queue.drain(..).collect();
    sorted_initial.sort();
    queue.extend(sorted_initial);
    let mut order = Vec::new();
    while let Some(node) = queue.pop_front() {
        order.push(node.to_string());
        let mut children: Vec<&str> = graph
            .children_of(node)
            .iter()
            .map(|e| e.to.as_str())
            .collect();
        children.sort();
        for child in children {
            let deg = in_degree.entry(child).or_insert(0);
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(child);
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        graph::{CausalEdge, CausalGraph},
        observation::Observation,
    };
    use tpt_eve_core::CausalRelationKind;

    fn make_edge(from: &str, to: &str) -> CausalEdge {
        CausalEdge {
            from: from.into(),
            to: to.into(),
            relation: CausalRelationKind::Causes,
            strength: 1.0,
        }
    }

    fn simple_graph() -> CausalGraph {
        let mut g = CausalGraph::new();
        g.add_edge(make_edge("x", "y")).unwrap();
        g
    }

    #[test]
    fn multivariate_ols_single_predictor() {
        // y = 2x + 1
        let obs = vec![
            Observation::from_pairs([("x", 1.0), ("y", 3.0)]),
            Observation::from_pairs([("x", 2.0), ("y", 5.0)]),
            Observation::from_pairs([("x", 3.0), ("y", 7.0)]),
        ];
        let (betas, intercept) = multivariate_ols(&obs, &["x"], "y");
        assert!((betas[0] - 2.0).abs() < 1e-9, "slope {}", betas[0]);
        assert!((intercept - 1.0).abs() < 1e-9, "intercept {intercept}");
    }

    #[test]
    fn multivariate_ols_two_predictors() {
        // y = 3*x1 + 2*x2 + 5
        // data rows satisfy this exactly: (1,2)→12, (2,3)→17, (3,1)→16, (4,4)→25
        let data: Vec<(f64, f64, f64)> = vec![
            (1.0, 2.0, 12.0),
            (2.0, 3.0, 17.0),
            (3.0, 1.0, 16.0),
            (4.0, 4.0, 25.0),
        ];
        let obs: Vec<Observation> = data
            .iter()
            .map(|&(x1, x2, y)| Observation::from_pairs([("x1", x1), ("x2", x2), ("y", y)]))
            .collect();
        let (betas, intercept) = multivariate_ols(&obs, &["x1", "x2"], "y");
        assert!((betas[0] - 3.0).abs() < 0.01, "beta_x1={}", betas[0]);
        assert!((betas[1] - 2.0).abs() < 0.01, "beta_x2={}", betas[1]);
        assert!((intercept - 5.0).abs() < 0.01, "intercept={intercept}");
    }

    #[test]
    fn fit_and_predict() {
        let g = simple_graph();
        let obs = vec![
            Observation::from_pairs([("x", 1.0), ("y", 2.0)]),
            Observation::from_pairs([("x", 2.0), ("y", 4.0)]),
            Observation::from_pairs([("x", 3.0), ("y", 6.0)]),
        ];
        let scm = StructuralCausalModel::fit(&g, &obs);
        assert!((scm.coefficient("x", "y") - 2.0).abs() < 0.01);

        let mut known = HashMap::new();
        known.insert("x".to_string(), 5.0);
        let predicted = scm.predict(&g, &known);
        assert!(
            (predicted["y"] - 10.0).abs() < 0.1,
            "got {}",
            predicted["y"]
        );
    }

    #[test]
    fn fit_multi_parent_node() {
        // studyHours + sleep → examScore (two parents, correlated in the data)
        let mut g = CausalGraph::new();
        g.add_edge(make_edge("studyHours", "examScore")).unwrap();
        g.add_edge(make_edge("sleep", "examScore")).unwrap();

        // examScore = 10*studyHours + 5*sleep + 0
        let data = vec![
            (3.0_f64, 6.0_f64, 60.0_f64),
            (5.0, 8.0, 90.0),
            (7.0, 7.0, 105.0),
            (4.0, 9.0, 85.0),
        ];
        let obs: Vec<Observation> = data
            .iter()
            .map(|&(h, s, score)| {
                Observation::from_pairs([("studyHours", h), ("sleep", s), ("examScore", score)])
            })
            .collect();
        let scm = StructuralCausalModel::fit(&g, &obs);

        // Coefficients should be close to 10 and 5
        let b_study = scm.coefficient("studyHours", "examScore");
        let b_sleep = scm.coefficient("sleep", "examScore");
        assert!((b_study - 10.0).abs() < 1.0, "b_study={b_study}");
        assert!((b_sleep - 5.0).abs() < 1.0, "b_sleep={b_sleep}");
    }
}
