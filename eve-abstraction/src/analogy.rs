// SPDX-License-Identifier: Apache-2.0
use std::collections::{HashMap, HashSet};

use eve_core::Fact;
use serde::{Deserialize, Serialize};

/// A structural mapping between a source and a target domain.
///
/// `mapping` pairs source entity names with the target entity that best matches their
/// relational role (Jaccard similarity over shared predicates). `score` is the mean
/// Jaccard similarity across all mapped pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analogy {
    pub source_domain: String,
    pub target_domain: String,
    /// Source entity name → best-matching target entity name.
    pub mapping: HashMap<String, String>,
    /// Mean Jaccard similarity across all mapped pairs, in [0.0, 1.0].
    pub score: f64,
}

/// Computes the best structural analogy mapping from a source domain to a target domain.
#[derive(Debug, Clone, Default)]
pub struct AnalogyMapper;

impl AnalogyMapper {
    pub fn new() -> Self {
        AnalogyMapper
    }

    /// Map `source_facts` onto `target_facts` by aligning entities that share predicate roles.
    ///
    /// Uses a greedy bijection: source entities are matched (in lexicographic order for
    /// determinism) to the unmatched target entity with the highest Jaccard similarity of
    /// their predicate signature sets. Pairs with zero similarity are skipped.
    pub fn map(
        &self,
        source_domain: &str,
        source_facts: &[Fact],
        target_domain: &str,
        target_facts: &[Fact],
    ) -> Analogy {
        let source_sig = entity_signatures(source_facts);
        let target_sig = entity_signatures(target_facts);

        let mut mapping: HashMap<String, String> = HashMap::new();
        let mut used_targets: HashSet<String> = HashSet::new();

        let mut src_entities: Vec<&String> = source_sig.keys().collect();
        src_entities.sort();

        for src in src_entities {
            let src_preds = &source_sig[src];
            let mut best_tgt: Option<String> = None;
            let mut best_score = 0.0_f64;

            let mut tgt_entities: Vec<&String> = target_sig.keys().collect();
            tgt_entities.sort();

            for tgt in tgt_entities {
                if used_targets.contains(tgt.as_str()) {
                    continue;
                }
                let score = jaccard(src_preds, &target_sig[tgt]);
                if score > best_score {
                    best_score = score;
                    best_tgt = Some(tgt.clone());
                }
            }

            if let Some(tgt) = best_tgt {
                used_targets.insert(tgt.clone());
                mapping.insert(src.clone(), tgt);
            }
        }

        let score = if mapping.is_empty() {
            0.0
        } else {
            let total: f64 = mapping
                .iter()
                .map(|(src, tgt)| {
                    jaccard(source_sig.get(src).unwrap(), target_sig.get(tgt).unwrap())
                })
                .sum();
            total / mapping.len() as f64
        };

        Analogy {
            source_domain: source_domain.to_string(),
            target_domain: target_domain.to_string(),
            mapping,
            score,
        }
    }
}

/// Predicate signature of every entity: maps entity name → set of predicates it appears in.
///
/// An entity appears as a subject or object in a fact; both roles contribute predicates to its
/// signature, capturing the relational neighbourhood without directional bias.
fn entity_signatures(facts: &[Fact]) -> HashMap<String, HashSet<String>> {
    let mut sigs: HashMap<String, HashSet<String>> = HashMap::new();
    for fact in facts {
        let pred = fact.predicate.as_str().to_string();
        sigs.entry(fact.subject.as_str().to_string())
            .or_default()
            .insert(pred.clone());
        sigs.entry(fact.object.as_str().to_string())
            .or_default()
            .insert(pred);
    }
    sigs
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(b).count() as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use eve_core::{Fact, Value};

    fn fact(s: &str, p: &str, o: &str) -> Fact {
        Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        )
    }

    fn mammal_facts() -> Vec<Fact> {
        vec![
            fact("cat", "is", "mammal"),
            fact("cat", "has", "fur"),
            fact("dog", "is", "mammal"),
            fact("dog", "has", "fur"),
            fact("mammal", "is_a", "animal"),
        ]
    }

    fn bird_facts() -> Vec<Fact> {
        vec![
            fact("eagle", "is", "bird"),
            fact("eagle", "has", "feathers"),
            fact("hawk", "is", "bird"),
            fact("hawk", "has", "feathers"),
            fact("bird", "is_a", "animal"),
        ]
    }

    #[test]
    fn maps_shared_predicate_roles() {
        let mapper = AnalogyMapper::new();
        let analogy = mapper.map("mammals", &mammal_facts(), "birds", &bird_facts());
        assert_eq!(analogy.source_domain, "mammals");
        assert_eq!(analogy.target_domain, "birds");
        assert!(!analogy.mapping.is_empty(), "mapping must not be empty");
        // mammal → bird and animal → animal should be discovered
        assert_eq!(
            analogy.mapping.get("mammal").map(String::as_str),
            Some("bird"),
            "mammal should map to bird"
        );
        assert_eq!(
            analogy.mapping.get("animal").map(String::as_str),
            Some("animal"),
            "animal should map to animal"
        );
    }

    #[test]
    fn score_is_positive_for_related_domains() {
        let mapper = AnalogyMapper::new();
        let analogy = mapper.map("mammals", &mammal_facts(), "birds", &bird_facts());
        assert!(
            analogy.score > 0.0,
            "score should be positive for related domains"
        );
        assert!(analogy.score <= 1.0, "score cannot exceed 1.0");
    }

    #[test]
    fn empty_domains_yield_empty_mapping() {
        let mapper = AnalogyMapper::new();
        let analogy = mapper.map("empty", &[], "also_empty", &[]);
        assert!(analogy.mapping.is_empty());
        assert_eq!(analogy.score, 0.0);
    }
}
