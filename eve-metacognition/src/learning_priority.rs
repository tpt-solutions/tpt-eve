// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};

use crate::knowledge_gap::KnowledgeGap;

/// A ranked learning target derived from a `KnowledgeGap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPriority {
    /// 1-based rank (1 = highest priority).
    pub rank: usize,
    /// The gap this priority addresses.
    pub gap: KnowledgeGap,
    /// Human-readable explanation of why this gap is ranked here.
    pub rationale: String,
}

/// Ranks a list of `KnowledgeGap`s into an ordered learning agenda.
///
/// Ranking heuristic (highest priority first):
/// 1. `LowConfidenceFact` gaps (we *know* something but poorly — clarifying
///    is cheaper than discovering something new).
/// 2. `MissingAttribute` gaps (structured hole in a known entity).
/// 3. `UnexplainedEntity` gaps (entity exists but is opaque).
/// 4. Within each tier, sort by `gap.priority` descending.
pub struct LearningPrioritizer;

impl LearningPrioritizer {
    pub fn new() -> Self {
        LearningPrioritizer
    }

    pub fn prioritize(&self, gaps: &[KnowledgeGap]) -> Vec<LearningPriority> {
        let mut sorted = gaps.to_vec();
        sorted.sort_by(|a, b| {
            let tier_a = Self::tier(a);
            let tier_b = Self::tier(b);
            tier_a.cmp(&tier_b).then_with(|| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        sorted
            .into_iter()
            .enumerate()
            .map(|(i, gap)| {
                let rationale = Self::rationale(&gap, i + 1);
                LearningPriority {
                    rank: i + 1,
                    gap,
                    rationale,
                }
            })
            .collect()
    }

    fn tier(gap: &KnowledgeGap) -> u8 {
        use crate::knowledge_gap::GapType;
        match gap.gap_type {
            GapType::LowConfidenceFact => 1,
            GapType::MissingAttribute => 2,
            GapType::IncompletePeerCoverage => 3,
            GapType::UnexplainedEntity => 4,
        }
    }

    fn rationale(gap: &KnowledgeGap, rank: usize) -> String {
        use crate::knowledge_gap::GapType;
        let base = match gap.gap_type {
            GapType::LowConfidenceFact => {
                "Clarifying a low-confidence fact yields the fastest confidence gain"
            }
            GapType::MissingAttribute => {
                "Filling a missing attribute completes a known entity's profile"
            }
            GapType::IncompletePeerCoverage => {
                "Completing peer coverage reduces inconsistency across similar entities"
            }
            GapType::UnexplainedEntity => "Explaining an opaque entity opens new reasoning paths",
        };
        format!("Rank {rank}: {base} (gap priority {:.2})", gap.priority)
    }
}

impl Default for LearningPrioritizer {
    fn default() -> Self {
        LearningPrioritizer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_gap::{GapType, KnowledgeGap};
    use eve_core::Value;

    fn gap(gap_type: GapType, priority: f32) -> KnowledgeGap {
        KnowledgeGap {
            gap_type,
            description: "test gap".into(),
            subject: Value::Symbol("X".into()),
            missing_predicate: None,
            priority,
        }
    }

    #[test]
    fn low_confidence_gap_ranked_first() {
        let gaps = vec![
            gap(GapType::UnexplainedEntity, 0.9),
            gap(GapType::LowConfidenceFact, 0.5),
        ];
        let priorities = LearningPrioritizer::new().prioritize(&gaps);
        assert_eq!(priorities[0].gap.gap_type, GapType::LowConfidenceFact);
    }

    #[test]
    fn missing_attribute_ranked_before_unexplained_entity() {
        let gaps = vec![
            gap(GapType::UnexplainedEntity, 0.9),
            gap(GapType::MissingAttribute, 0.3),
        ];
        let priorities = LearningPrioritizer::new().prioritize(&gaps);
        assert_eq!(priorities[0].gap.gap_type, GapType::MissingAttribute);
    }

    #[test]
    fn within_tier_higher_gap_priority_ranked_first() {
        let gaps = vec![
            gap(GapType::MissingAttribute, 0.3),
            gap(GapType::MissingAttribute, 0.8),
        ];
        let priorities = LearningPrioritizer::new().prioritize(&gaps);
        assert!(
            priorities[0].gap.priority > priorities[1].gap.priority,
            "higher-priority gap should be ranked first"
        );
    }

    #[test]
    fn ranks_are_one_based_and_sequential() {
        let gaps = vec![
            gap(GapType::LowConfidenceFact, 0.7),
            gap(GapType::MissingAttribute, 0.5),
            gap(GapType::UnexplainedEntity, 0.3),
        ];
        let priorities = LearningPrioritizer::new().prioritize(&gaps);
        for (i, p) in priorities.iter().enumerate() {
            assert_eq!(p.rank, i + 1);
        }
    }

    #[test]
    fn empty_gaps_produces_empty_priorities() {
        assert!(LearningPrioritizer::new().prioritize(&[]).is_empty());
    }
}
