// SPDX-License-Identifier: Apache-2.0
use std::collections::{HashMap, HashSet};

use eve_core::{Fact, Value};
use serde::{Deserialize, Serialize};

/// The category of a detected knowledge gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapType {
    /// A known entity is missing an attribute that peer entities possess.
    MissingAttribute,
    /// A fact exists but has confidence below the acceptable threshold.
    LowConfidenceFact,
    /// Two entities share a category but one lacks predicates the other has.
    IncompletePeerCoverage,
    /// An entity appears only as an object, never as a subject — its own
    /// properties are unknown.
    UnexplainedEntity,
}

/// A single detected gap in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub gap_type: GapType,
    pub description: String,
    pub subject: Value,
    pub missing_predicate: Option<Value>,
    /// Relative importance of filling this gap; higher = more urgent (0.0–1.0).
    pub priority: f32,
}

/// Scans a fact set and surfaces knowledge gaps.
pub struct KnowledgeGapDetector;

impl KnowledgeGapDetector {
    pub fn new() -> Self {
        KnowledgeGapDetector
    }

    /// Detect all gaps in `facts`.  Facts below `confidence_threshold` are
    /// also flagged as low-confidence gaps.
    pub fn detect(&self, facts: &[Fact], confidence_threshold: f32) -> Vec<KnowledgeGap> {
        let mut gaps = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        self.low_confidence(facts, confidence_threshold, &mut seen, &mut gaps);
        self.missing_attributes(facts, &mut seen, &mut gaps);
        self.unexplained_entities(facts, &mut seen, &mut gaps);

        gaps
    }

    // -----------------------------------------------------------------------

    fn low_confidence(
        &self,
        facts: &[Fact],
        threshold: f32,
        seen: &mut HashSet<String>,
        out: &mut Vec<KnowledgeGap>,
    ) {
        for f in facts {
            if f.confidence < threshold {
                let key = format!("lc:{}:{}:{}", f.subject, f.predicate, f.object);
                if seen.insert(key) {
                    out.push(KnowledgeGap {
                        gap_type: GapType::LowConfidenceFact,
                        description: format!(
                            "fact ({} {} {}) has low confidence {:.2}",
                            f.subject, f.predicate, f.object, f.confidence
                        ),
                        subject: f.subject.clone(),
                        missing_predicate: Some(f.predicate.clone()),
                        priority: 1.0 - f.confidence,
                    });
                }
            }
        }
    }

    /// For every category (object of an "is" predicate), find attributes that
    /// some members have but others lack.
    fn missing_attributes(
        &self,
        facts: &[Fact],
        seen: &mut HashSet<String>,
        out: &mut Vec<KnowledgeGap>,
    ) {
        // category → members
        let mut category_members: HashMap<String, Vec<&Value>> = HashMap::new();
        for f in facts {
            if f.predicate == Value::Symbol("is".into()) {
                category_members
                    .entry(f.object.as_str().to_string())
                    .or_default()
                    .push(&f.subject);
            }
        }

        // For each category, find predicates (other than "is") held by some members.
        for members in category_members.values() {
            if members.len() < 2 {
                continue;
            }
            // predicate → subjects that have it
            let mut pred_subjects: HashMap<String, HashSet<String>> = HashMap::new();
            for f in facts {
                if f.predicate == Value::Symbol("is".into()) {
                    continue;
                }
                if members.contains(&&f.subject) {
                    pred_subjects
                        .entry(f.predicate.as_str().to_string())
                        .or_default()
                        .insert(f.subject.as_str().to_string());
                }
            }

            for (pred, have) in &pred_subjects {
                for member in members {
                    if !have.contains(member.as_str()) {
                        let key = format!("ma:{}:{}", member, pred);
                        if seen.insert(key) {
                            out.push(KnowledgeGap {
                                gap_type: GapType::MissingAttribute,
                                description: format!(
                                    "entity '{}' may be missing attribute '{pred}' (peers have it)",
                                    member
                                ),
                                subject: (*member).clone(),
                                missing_predicate: Some(Value::Symbol(pred.clone())),
                                priority: 0.6,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Entities that appear only as objects are "unexplained" — we know
    /// something about them but they have no outgoing facts of their own.
    fn unexplained_entities(
        &self,
        facts: &[Fact],
        seen: &mut HashSet<String>,
        out: &mut Vec<KnowledgeGap>,
    ) {
        let subjects: HashSet<String> = facts
            .iter()
            .map(|f| f.subject.as_str().to_string())
            .collect();
        let objects: HashSet<String> = facts
            .iter()
            .map(|f| f.object.as_str().to_string())
            .collect();

        for obj_str in objects.difference(&subjects) {
            let key = format!("ue:{obj_str}");
            if seen.insert(key) {
                out.push(KnowledgeGap {
                    gap_type: GapType::UnexplainedEntity,
                    description: format!(
                        "entity '{obj_str}' appears as an object but has no outgoing facts"
                    ),
                    subject: Value::Symbol(obj_str.clone()),
                    missing_predicate: None,
                    priority: 0.4,
                });
            }
        }
    }
}

impl Default for KnowledgeGapDetector {
    fn default() -> Self {
        KnowledgeGapDetector::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eve_core::Value;

    fn fact(s: &str, p: &str, o: &str) -> Fact {
        Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        )
    }

    fn low_conf_fact(s: &str, p: &str, o: &str, conf: f32) -> Fact {
        let mut f = fact(s, p, o);
        f.confidence = conf;
        f
    }

    #[test]
    fn detects_low_confidence_facts() {
        let facts = vec![low_conf_fact("A", "is", "B", 0.3)];
        let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
        assert!(
            gaps.iter()
                .any(|g| g.gap_type == GapType::LowConfidenceFact),
            "should detect low-confidence fact"
        );
    }

    #[test]
    fn detects_missing_attribute_in_same_category() {
        // TypeScript has syntax; Python also is a programming_language but lacks syntax.
        let facts = vec![
            fact("TypeScript", "is", "programming_language"),
            fact("Python", "is", "programming_language"),
            fact("TypeScript", "has", "syntax"),
        ];
        let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
        let missing = gaps.iter().find(|g| {
            g.gap_type == GapType::MissingAttribute && g.subject == Value::Symbol("Python".into())
        });
        assert!(
            missing.is_some(),
            "should detect Python missing 'has' attribute; got: {gaps:?}"
        );
    }

    #[test]
    fn detects_unexplained_entity() {
        // "syntax" only appears as object, never as subject.
        let facts = vec![fact("TypeScript", "has", "syntax")];
        let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
        assert!(
            gaps.iter().any(|g| g.gap_type == GapType::UnexplainedEntity
                && g.subject == Value::Symbol("syntax".into())),
            "should flag 'syntax' as unexplained; got: {gaps:?}"
        );
    }

    #[test]
    fn no_gaps_in_fully_described_facts() {
        // All entities appear as subjects; everything above threshold.
        let facts = vec![
            fact("TypeScript", "is", "language"),
            fact("language", "has", "syntax"),
            fact("syntax", "describes", "structure"),
        ];
        let gaps = KnowledgeGapDetector::new().detect(&facts, 0.1);
        let low_conf: Vec<_> = gaps
            .iter()
            .filter(|g| g.gap_type == GapType::LowConfidenceFact)
            .collect();
        assert!(low_conf.is_empty(), "no low-confidence gaps expected");
    }

    #[test]
    fn priority_of_low_confidence_gap_equals_one_minus_confidence() {
        let facts = vec![low_conf_fact("A", "is", "B", 0.2)];
        let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
        let lc = gaps
            .iter()
            .find(|g| g.gap_type == GapType::LowConfidenceFact)
            .unwrap();
        assert!(
            (lc.priority - 0.8).abs() < 1e-5,
            "priority should be 1.0 - 0.2 = 0.8; got {}",
            lc.priority
        );
    }
}
