// SPDX-License-Identifier: MIT OR Apache-2.0
use serde::{Deserialize, Serialize};
use tpt_eve_core::{Fact, Value};

use crate::knowledge_gap::KnowledgeGap;

/// Coarse-grained assessment of how well the system understands a domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnderstandingLevel {
    /// Mean confidence ≥ 0.8 and fewer than 3 gaps.
    High,
    /// Mean confidence ≥ 0.5 or moderate number of gaps.
    Medium,
    /// Mean confidence < 0.5 or many gaps.
    Low,
}

impl std::fmt::Display for UnderstandingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnderstandingLevel::High => write!(f, "High"),
            UnderstandingLevel::Medium => write!(f, "Medium"),
            UnderstandingLevel::Low => write!(f, "Low"),
        }
    }
}

/// A question the system should seek to answer to close a knowledge gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    /// The entity the clarification is about.
    pub subject: Value,
    /// The natural-language question to ask.
    pub question: String,
    /// How urgently this clarification is needed (0.0–1.0).
    pub priority: f32,
}

/// Monitors the overall understanding level of a domain and generates
/// clarification requests for the most pressing gaps.
pub struct UnderstandingMonitor;

impl UnderstandingMonitor {
    pub fn new() -> Self {
        UnderstandingMonitor
    }

    /// Classify the understanding level given the current fact set and gaps.
    pub fn assess(&self, facts: &[Fact], gaps: &[KnowledgeGap]) -> UnderstandingLevel {
        if facts.is_empty() {
            return UnderstandingLevel::Low;
        }
        let mean_conf: f32 = facts.iter().map(|f| f.confidence).sum::<f32>() / facts.len() as f32;
        let gap_count = gaps.len();

        if mean_conf >= 0.8 && gap_count < 3 {
            UnderstandingLevel::High
        } else if mean_conf >= 0.5 || gap_count < 6 {
            UnderstandingLevel::Medium
        } else {
            UnderstandingLevel::Low
        }
    }

    /// Generate clarification questions for the top `max_requests` gaps,
    /// ordered by gap priority descending.
    pub fn clarifications(
        &self,
        gaps: &[KnowledgeGap],
        max_requests: usize,
    ) -> Vec<ClarificationRequest> {
        let mut sorted = gaps.to_vec();
        sorted.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        sorted
            .into_iter()
            .take(max_requests)
            .map(|g| ClarificationRequest {
                question: Self::question_for(&g),
                priority: g.priority,
                subject: g.subject,
            })
            .collect()
    }

    fn question_for(gap: &KnowledgeGap) -> String {
        use crate::knowledge_gap::GapType;
        match &gap.gap_type {
            GapType::LowConfidenceFact => {
                if let Some(pred) = &gap.missing_predicate {
                    format!(
                        "Can you confirm whether '{}' {} something?",
                        gap.subject,
                        pred.as_str()
                    )
                } else {
                    format!("Can you provide more evidence about '{}'?", gap.subject)
                }
            }
            GapType::MissingAttribute => {
                if let Some(pred) = &gap.missing_predicate {
                    format!(
                        "Does '{}' {} anything? Peers in the same category do.",
                        gap.subject,
                        pred.as_str()
                    )
                } else {
                    format!("What attributes does '{}' have?", gap.subject)
                }
            }
            GapType::IncompletePeerCoverage => {
                format!(
                    "What do we know about '{}' compared to its peers?",
                    gap.subject
                )
            }
            GapType::UnexplainedEntity => {
                format!(
                    "What is '{}' and what properties does it have?",
                    gap.subject
                )
            }
        }
    }
}

impl Default for UnderstandingMonitor {
    fn default() -> Self {
        UnderstandingMonitor::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_gap::{GapType, KnowledgeGap};
    use tpt_eve_core::Value;

    fn fact_with_conf(conf: f32) -> Fact {
        let mut f = Fact::asserted(
            Value::Symbol("A".into()),
            Value::Symbol("is".into()),
            Value::Symbol("B".into()),
        );
        f.confidence = conf;
        f
    }

    fn gap(priority: f32) -> KnowledgeGap {
        KnowledgeGap {
            gap_type: GapType::UnexplainedEntity,
            description: "gap".into(),
            subject: Value::Symbol("X".into()),
            missing_predicate: None,
            priority,
        }
    }

    #[test]
    fn high_confidence_few_gaps_is_high() {
        let facts = vec![fact_with_conf(0.9), fact_with_conf(0.85)];
        let level = UnderstandingMonitor::new().assess(&facts, &[gap(0.3)]);
        assert_eq!(level, UnderstandingLevel::High);
    }

    #[test]
    fn low_confidence_is_low() {
        let facts = vec![fact_with_conf(0.2), fact_with_conf(0.3)];
        let gaps: Vec<KnowledgeGap> = (0..10).map(|_| gap(0.5)).collect();
        let level = UnderstandingMonitor::new().assess(&facts, &gaps);
        assert_eq!(level, UnderstandingLevel::Low);
    }

    #[test]
    fn empty_facts_is_low() {
        let level = UnderstandingMonitor::new().assess(&[], &[]);
        assert_eq!(level, UnderstandingLevel::Low);
    }

    #[test]
    fn clarifications_limited_to_max_requests() {
        let gaps: Vec<KnowledgeGap> = (0..10).map(|i| gap(i as f32 / 10.0)).collect();
        let requests = UnderstandingMonitor::new().clarifications(&gaps, 3);
        assert_eq!(requests.len(), 3);
    }

    #[test]
    fn clarifications_ordered_by_priority_descending() {
        let gaps = vec![gap(0.3), gap(0.9), gap(0.6)];
        let requests = UnderstandingMonitor::new().clarifications(&gaps, 3);
        assert!(requests[0].priority >= requests[1].priority);
        assert!(requests[1].priority >= requests[2].priority);
    }

    #[test]
    fn clarification_question_nonempty() {
        let gaps = vec![gap(0.8)];
        let requests = UnderstandingMonitor::new().clarifications(&gaps, 1);
        assert!(!requests[0].question.is_empty());
    }
}
