use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    graph::CausalGraph,
    scm::{topo_sort, StructuralCausalModel},
};

/// A counterfactual query: given what actually happened (`actual_world`), what would have
/// happened if some variables had been different (`counterfactual_changes`)?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Counterfactual {
    /// The complete actual-world observation (all observed variables).
    pub actual_world: HashMap<String, f64>,
    /// Variables to change in the counterfactual world (partial — only what differs).
    pub counterfactual_changes: HashMap<String, f64>,
}

impl Counterfactual {
    pub fn new(
        actual_world: HashMap<String, f64>,
        counterfactual_changes: HashMap<String, f64>,
    ) -> Self {
        Counterfactual {
            actual_world,
            counterfactual_changes,
        }
    }
}

/// The full variable assignment for the counterfactual world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Simulation {
    pub counterfactual_world: HashMap<String, f64>,
}

/// Simulate what the world would look like under `counterfactual`.
///
/// Pearl's three-step abduction-action-prediction procedure for linear SCMs:
///
/// 1. **Abduction** — for each observed variable, compute the noise term
///    `U = actual_value - SCM_prediction(actual_parent_values)`.
/// 2. **Action** — override counterfactual-changed variables at their new values;
///    keep root (parentless) node values from the actual world (they carry their own noise).
/// 3. **Prediction** — propagate forward through the SCM in topological order,
///    adding the abduced noise back to each endogenous node.
pub fn simulate_counterfactual(
    graph: &CausalGraph,
    scm: &StructuralCausalModel,
    counterfactual: &Counterfactual,
) -> Simulation {
    // --- Step 1: Abduction ---
    // For each observed variable, compute noise = actual - SCM_prediction(actual parents).
    let mut noise: HashMap<String, f64> = HashMap::new();
    for (var, &actual_val) in &counterfactual.actual_world {
        let parents = graph.parents_of(var);
        let predicted = if parents.is_empty() {
            scm.intercept(var)
        } else {
            let mut p = scm.intercept(var);
            for edge in &parents {
                if let Some(&pv) = counterfactual.actual_world.get(&edge.from) {
                    p += scm.coefficient(&edge.from, var) * pv;
                }
            }
            p
        };
        noise.insert(var.clone(), actual_val - predicted);
    }

    // --- Step 2: Action ---
    // Seed only root (parentless) nodes with their actual-world values (which already
    // incorporate noise), then apply the counterfactual changes on top.
    let mut cf_world: HashMap<String, f64> = HashMap::new();
    for node in &graph.nodes {
        if graph.parents_of(node).is_empty() {
            if let Some(&v) = counterfactual.actual_world.get(node.as_str()) {
                cf_world.insert(node.clone(), v);
            }
        }
    }
    for (var, &val) in &counterfactual.counterfactual_changes {
        cf_world.insert(var.clone(), val);
    }

    // --- Step 3: Prediction ---
    // Propagate in topological order; for each endogenous node not yet set,
    // compute SCM prediction from counterfactual parents, then add abduced noise.
    for node in topo_sort(graph) {
        if cf_world.contains_key(node.as_str()) {
            continue; // already set (root node or explicit change)
        }
        let parents = graph.parents_of(&node);
        let mut val = scm.intercept(&node);
        for edge in &parents {
            if let Some(&pv) = cf_world.get(&edge.from) {
                val += scm.coefficient(&edge.from, &node) * pv;
            }
        }
        val += noise.get(node.as_str()).copied().unwrap_or(0.0);
        cf_world.insert(node.clone(), val);
    }

    Simulation {
        counterfactual_world: cf_world,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        graph::{CausalEdge, CausalGraph},
        observation::Observation,
        scm::StructuralCausalModel,
    };
    use eve_core::CausalRelationKind;

    fn study_graph() -> CausalGraph {
        let mut g = CausalGraph::new();
        g.add_edge(CausalEdge {
            from: "studyHours".into(),
            to: "examScore".into(),
            relation: CausalRelationKind::Causes,
            strength: 1.0,
        })
        .unwrap();
        g
    }

    #[test]
    fn counterfactual_more_study_hours() {
        let graph = study_graph();
        let obs = vec![
            Observation::from_pairs([("studyHours", 3.0), ("examScore", 65.0)]),
            Observation::from_pairs([("studyHours", 5.0), ("examScore", 85.0)]),
            Observation::from_pairs([("studyHours", 7.0), ("examScore", 105.0)]),
        ];
        let scm = StructuralCausalModel::fit(&graph, &obs);
        // slope ≈ 10, intercept ≈ 35

        // Actual world: studied 5h, scored 85
        let actual = HashMap::from([
            ("studyHours".to_string(), 5.0_f64),
            ("examScore".to_string(), 85.0_f64),
        ]);
        // Counterfactual: what if we had studied 7h?
        let changes = HashMap::from([("studyHours".to_string(), 7.0_f64)]);

        let sim = simulate_counterfactual(&graph, &scm, &Counterfactual::new(actual, changes));
        let cf_score = sim.counterfactual_world["examScore"];
        // SCM: examScore = 10*studyHours + 35 + noise
        // actual noise = 85 - (10*5 + 35) = 85 - 85 = 0
        // CF prediction = 10*7 + 35 + 0 = 105
        assert!(
            (cf_score - 105.0).abs() < 2.0,
            "expected ~105, got {cf_score}"
        );
    }

    #[test]
    fn counterfactual_preserves_individual_noise() {
        // If actual examScore was higher than predicted (lucky individual),
        // counterfactual should preserve that advantage.
        let graph = study_graph();
        let obs = vec![
            Observation::from_pairs([("studyHours", 2.0), ("examScore", 40.0)]),
            Observation::from_pairs([("studyHours", 4.0), ("examScore", 60.0)]),
            Observation::from_pairs([("studyHours", 6.0), ("examScore", 80.0)]),
        ];
        let scm = StructuralCausalModel::fit(&graph, &obs);
        // slope ≈ 10, intercept ≈ 20

        // "Lucky" individual: studied 4h, scored 70 (10 points above prediction)
        let actual = HashMap::from([
            ("studyHours".to_string(), 4.0_f64),
            ("examScore".to_string(), 70.0_f64), // predicted 60, noise = +10
        ]);
        let changes = HashMap::from([("studyHours".to_string(), 6.0_f64)]);
        let sim = simulate_counterfactual(&graph, &scm, &Counterfactual::new(actual, changes));
        let cf_score = sim.counterfactual_world["examScore"];
        // CF = 10*6 + 20 + 10 (noise) = 90
        assert!(
            (cf_score - 90.0).abs() < 2.0,
            "expected ~90, got {cf_score}"
        );
    }
}
