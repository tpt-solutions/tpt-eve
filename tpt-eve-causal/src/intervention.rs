// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{graph::CausalGraph, scm::StructuralCausalModel};

/// A do-calculus intervention: set variable `variable` to `value`, cutting all incoming edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Intervention {
    pub variable: String,
    pub value: f64,
}

impl Intervention {
    pub fn new(variable: impl Into<String>, value: f64) -> Self {
        Intervention {
            variable: variable.into(),
            value,
        }
    }
}

/// The result of an intervention prediction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prediction {
    pub outcome_variable: String,
    pub expected_value: f64,
    /// Rough confidence: 1.0 if the path from intervention to outcome is direct;
    /// decreases by 0.05 per extra hop to reflect increasing uncertainty.
    pub confidence: f64,
}

/// Predict the expected value of `outcome` under the intervention `do(intervention.variable = v)`.
///
/// The do-operator works by:
/// 1. Fixing the intervened variable at `intervention.value` in the "known" map.
/// 2. Removing all incoming edges to the intervened variable (surgery on the graph), so its
///    parents no longer influence it.
/// 3. Propagating forward through the SCM from that fixed seed.
///
/// `baseline` provides the values of other independent root nodes; any root node not in
/// `baseline` falls back to the SCM's intercept for that node.
pub fn predict_intervention(
    graph: &CausalGraph,
    scm: &StructuralCausalModel,
    intervention: &Intervention,
    outcome: &str,
    baseline: &HashMap<String, f64>,
) -> Prediction {
    // Build a mutilated graph: remove incoming edges to the intervened variable
    let mut mutilated = graph.clone();
    mutilated.edges.retain(|e| e.to != intervention.variable);

    // Seed known values: baseline + the intervention itself
    let mut known = baseline.clone();
    known.insert(intervention.variable.clone(), intervention.value);

    // Propagate through the SCM
    let predicted = scm.predict(&mutilated, &known);
    let expected_value = predicted.get(outcome).copied().unwrap_or(0.0);

    // Confidence heuristic based on path length
    let path_len = graph
        .paths_between(&intervention.variable, outcome)
        .iter()
        .map(|p| p.len())
        .min()
        .unwrap_or(usize::MAX);
    let confidence = if path_len == usize::MAX {
        0.0
    } else {
        (1.0_f64 - 0.05 * (path_len.saturating_sub(2)) as f64).max(0.1)
    };

    Prediction {
        outcome_variable: outcome.to_string(),
        expected_value,
        confidence,
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
    use tpt_eve_core::CausalRelationKind;

    #[test]
    fn predict_study_hours_exam_score() {
        // studyHours -> examScore, roughly: examScore ≈ 10 * studyHours + 35
        let mut graph = CausalGraph::new();
        graph
            .add_edge(CausalEdge {
                from: "studyHours".into(),
                to: "examScore".into(),
                relation: CausalRelationKind::Causes,
                strength: 1.0,
            })
            .unwrap();

        let obs = vec![
            Observation::from_pairs([("studyHours", 3.0), ("examScore", 65.0)]),
            Observation::from_pairs([("studyHours", 5.0), ("examScore", 85.0)]),
            Observation::from_pairs([("studyHours", 7.0), ("examScore", 105.0)]),
        ];
        let scm = StructuralCausalModel::fit(&graph, &obs);
        let intervention = Intervention::new("studyHours", 10.0);
        let prediction =
            predict_intervention(&graph, &scm, &intervention, "examScore", &HashMap::new());

        // slope ≈ 10, intercept ≈ 35  →  expected ≈ 135
        assert!(
            (prediction.expected_value - 135.0).abs() < 1.0,
            "expected ~135, got {}",
            prediction.expected_value
        );
        assert!(prediction.confidence > 0.0);
    }
}
