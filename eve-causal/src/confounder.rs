use crate::graph::CausalGraph;

/// Identify confounders of (`treatment`, `outcome`).
///
/// A variable C is a confounder if it is an ancestor of both `treatment` and `outcome` via
/// directed paths in the graph. This is a structural identification; it does not require
/// observations, only the DAG topology.
///
/// Returns a sorted, deduplicated list of confounder variable names.
pub fn identify_confounders(graph: &CausalGraph, treatment: &str, outcome: &str) -> Vec<String> {
    let anc_treatment = graph.ancestors_of(treatment);
    let anc_outcome = graph.ancestors_of(outcome);

    let mut confounders: Vec<String> = anc_treatment
        .intersection(&anc_outcome)
        .filter(|&n| n != treatment && n != outcome)
        .cloned()
        .collect();
    confounders.sort();
    confounders
}

/// Estimate the total causal effect of `treatment` on `outcome` using the linear SCM.
///
/// For a linear DAG the total effect is the sum over all directed paths of the product of
/// edge coefficients along that path. This equals the partial derivative ∂E[outcome]/∂treatment
/// when the model is correctly specified.
///
/// Returns `None` if there is no directed path from `treatment` to `outcome`.
pub fn estimate_causal_effect(
    graph: &CausalGraph,
    scm: &crate::scm::StructuralCausalModel,
    treatment: &str,
    outcome: &str,
) -> Option<f64> {
    let paths = graph.paths_between(treatment, outcome);
    if paths.is_empty() {
        return None;
    }
    let total: f64 = paths.iter().map(|path| path_product(path, scm)).sum();
    Some(total)
}

fn path_product(path: &[String], scm: &crate::scm::StructuralCausalModel) -> f64 {
    path.windows(2)
        .map(|pair| scm.coefficient(&pair[0], &pair[1]))
        .product()
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

    fn sym_edge(from: &str, to: &str) -> CausalEdge {
        CausalEdge {
            from: from.into(),
            to: to.into(),
            relation: CausalRelationKind::Causes,
            strength: 1.0,
        }
    }

    #[test]
    fn finds_common_cause_confounder() {
        // Z -> X, Z -> Y, X -> Y
        // Z is a confounder of (X, Y) since it's an ancestor of both
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("Z", "X")).unwrap();
        g.add_edge(sym_edge("Z", "Y")).unwrap();
        g.add_edge(sym_edge("X", "Y")).unwrap();
        let confounders = identify_confounders(&g, "X", "Y");
        assert!(confounders.contains(&"Z".to_string()));
    }

    #[test]
    fn no_confounders_when_independent() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("X", "Y")).unwrap();
        let confounders = identify_confounders(&g, "X", "Y");
        assert!(confounders.is_empty());
    }

    #[test]
    fn causal_effect_direct_path() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("x", "y")).unwrap();
        let obs = vec![
            Observation::from_pairs([("x", 1.0), ("y", 2.0)]),
            Observation::from_pairs([("x", 2.0), ("y", 4.0)]),
            Observation::from_pairs([("x", 3.0), ("y", 6.0)]),
        ];
        let scm = StructuralCausalModel::fit(&g, &obs);
        let effect = estimate_causal_effect(&g, &scm, "x", "y").unwrap();
        assert!((effect - 2.0).abs() < 0.01, "expected 2.0, got {effect}");
    }

    #[test]
    fn no_causal_effect_when_no_path() {
        let mut g = CausalGraph::new();
        g.add_edge(sym_edge("x", "y")).unwrap();
        g.add_node("z");
        let obs: Vec<Observation> = vec![];
        let scm = StructuralCausalModel::fit(&g, &obs);
        assert!(estimate_causal_effect(&g, &scm, "z", "y").is_none());
    }
}
