// SPDX-License-Identifier: MIT OR Apache-2.0
use tpt_eve_core::{CausalRelationDraft, CausalRelationKind};
use tracing::warn;

use crate::{
    graph::{CausalEdge, CausalGraph},
    observation::Observation,
    scm::StructuralCausalModel,
};

/// Incrementally collects `CausalRelationDraft` outputs from the neural layer and optional
/// observations, then builds a validated `CausalGraph` (and optionally fits an SCM).
#[derive(Debug, Default)]
pub struct CausalGraphBuilder {
    relations: Vec<CausalRelationDraft>,
    observations: Vec<Observation>,
}

impl CausalGraphBuilder {
    pub fn new() -> Self {
        CausalGraphBuilder::default()
    }

    pub fn add_relation(&mut self, draft: CausalRelationDraft) -> &mut Self {
        self.relations.push(draft);
        self
    }

    pub fn add_observation(&mut self, obs: Observation) -> &mut Self {
        self.observations.push(obs);
        self
    }

    /// Build a `CausalGraph` from the accumulated relations (skipping any edge that would
    /// introduce a cycle), and optionally fit a `StructuralCausalModel` from the observations.
    ///
    /// Returns `(graph, Some(scm))` when observations were provided, `(graph, None)` otherwise.
    pub fn build(self) -> (CausalGraph, Option<StructuralCausalModel>) {
        let mut graph = CausalGraph::new();

        for draft in &self.relations {
            let strength = relation_default_strength(&draft.relation);
            let edge = CausalEdge {
                from: draft.cause.clone(),
                to: draft.effect.clone(),
                relation: draft.relation.clone(),
                strength,
            };
            // Skip edges that would introduce a cycle rather than panicking.
            if let Err(e) = graph.add_edge(edge) {
                warn!("Skipped edge {} -> {}: {e}", draft.cause, draft.effect);
            }
        }

        let scm = if self.observations.is_empty() {
            None
        } else {
            Some(StructuralCausalModel::fit(&graph, &self.observations))
        };

        (graph, scm)
    }
}

fn relation_default_strength(kind: &CausalRelationKind) -> f64 {
    match kind {
        CausalRelationKind::Causes => 1.0,
        CausalRelationKind::Precedes => 0.8,
        CausalRelationKind::Enables => 0.6,
        CausalRelationKind::Prevents => -1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_creates_graph_from_relations() {
        let mut builder = CausalGraphBuilder::new();
        builder.add_relation(CausalRelationDraft {
            cause: "studyHours".into(),
            effect: "examScore".into(),
            relation: CausalRelationKind::Causes,
        });
        let (graph, scm) = builder.build();
        assert!(graph.nodes.contains("studyHours"));
        assert!(graph.nodes.contains("examScore"));
        assert_eq!(graph.edges.len(), 1);
        assert!(scm.is_none());
    }

    #[test]
    fn builder_fits_scm_when_observations_given() {
        let mut builder = CausalGraphBuilder::new();
        builder.add_relation(CausalRelationDraft {
            cause: "x".into(),
            effect: "y".into(),
            relation: CausalRelationKind::Causes,
        });
        builder.add_observation(Observation::from_pairs([("x", 1.0), ("y", 2.0)]));
        builder.add_observation(Observation::from_pairs([("x", 2.0), ("y", 4.0)]));
        builder.add_observation(Observation::from_pairs([("x", 3.0), ("y", 6.0)]));
        let (graph, scm) = builder.build();
        assert!(scm.is_some());
        // Coefficient should be close to 2.0 (y = 2x)
        let coeff = scm.unwrap().coefficient("x", "y");
        assert!((coeff - 2.0).abs() < 0.01, "expected ~2.0, got {coeff}");
        assert!(graph.edges[0].strength != 0.0);
    }
}
