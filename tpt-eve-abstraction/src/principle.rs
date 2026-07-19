// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tpt_eve_core::Fact;

/// A recurring relational structure found across multiple example domains.
///
/// A principle is a predicate chain `[p1, p2, …]` where `p1`'s object is the subject of `p2`,
/// appearing in at least `support` of the example fact sets that were mined.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbstractPrinciple {
    /// Human-readable form, e.g. `"is → has"`.
    pub name: String,
    /// Ordered sequence of predicate names forming the structural chain.
    pub predicate_chain: Vec<String>,
    /// Number of distinct examples in which this chain was observed.
    pub support: usize,
    /// `support / total_examples` — fraction of examples exhibiting this pattern.
    pub confidence: f32,
}

/// Mines `AbstractPrinciple`s from a collection of example fact sets.
#[derive(Debug, Clone)]
pub struct PatternMiner {
    min_support: usize,
}

impl PatternMiner {
    pub fn new(min_support: usize) -> Self {
        PatternMiner { min_support }
    }

    /// Mine recurring length-2 predicate chains from `examples`.
    ///
    /// A chain `[p1, p2]` is extracted from an example whenever some fact with predicate `p1`
    /// has an object that matches the subject of another fact with predicate `p2` in the same
    /// example. A chain must appear in at least `min_support` examples to become a principle.
    pub fn mine(&self, examples: &[Vec<Fact>]) -> Vec<AbstractPrinciple> {
        let mut chain_counts: HashMap<Vec<String>, usize> = HashMap::new();

        for example in examples {
            // Collect unique chains present in this example (deduplicated per example).
            let chains = self.extract_chains(example);
            for chain in chains {
                *chain_counts.entry(chain).or_insert(0) += 1;
            }
        }

        let total = examples.len();
        let mut principles: Vec<AbstractPrinciple> = chain_counts
            .into_iter()
            .filter(|(_, count)| *count >= self.min_support)
            .map(|(chain, count)| AbstractPrinciple {
                name: chain.join(" \u{2192} "),
                predicate_chain: chain,
                support: count,
                confidence: count as f32 / total.max(1) as f32,
            })
            .collect();

        principles.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });
        principles
    }

    /// Extract all unique length-2 predicate chains from one example.
    fn extract_chains(&self, facts: &[Fact]) -> HashSet<Vec<String>> {
        let mut chains = HashSet::new();
        for f1 in facts {
            for f2 in facts {
                if f1.object.as_str() == f2.subject.as_str() {
                    chains.insert(vec![
                        f1.predicate.as_str().to_string(),
                        f2.predicate.as_str().to_string(),
                    ]);
                }
            }
        }
        chains
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_eve_core::{Fact, Value};

    fn fact(s: &str, p: &str, o: &str) -> Fact {
        Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        )
    }

    fn lang_example(name: &str) -> Vec<Fact> {
        vec![
            fact(name, "is", "programming_language"),
            fact("programming_language", "has", "syntax"),
        ]
    }

    #[test]
    fn mines_chain_present_in_all_examples() {
        let examples = vec![
            lang_example("TypeScript"),
            lang_example("Python"),
            lang_example("Java"),
        ];
        let miner = PatternMiner::new(2);
        let principles = miner.mine(&examples);
        assert!(!principles.is_empty(), "should find at least one principle");
        let names: Vec<&str> = principles.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("is") && n.contains("has")),
            "expected is → has principle, got {names:?}"
        );
    }

    #[test]
    fn min_support_filters_rare_chains() {
        let examples = vec![
            lang_example("TypeScript"),
            lang_example("Python"),
            vec![fact("Rust", "compiles_to", "machine_code")],
        ];
        let miner = PatternMiner::new(3);
        // Only chains appearing in all 3 examples should survive; none do for min_support=3
        let principles = miner.mine(&examples);
        // is → has only appears in 2 of 3 examples
        for p in &principles {
            assert!(
                p.support >= 3,
                "principle '{}' has support {} < 3",
                p.name,
                p.support
            );
        }
    }

    #[test]
    fn confidence_equals_support_over_total() {
        let examples = vec![lang_example("TypeScript"), lang_example("Python")];
        let miner = PatternMiner::new(1);
        let principles = miner.mine(&examples);
        for p in &principles {
            let expected = p.support as f32 / 2.0;
            assert!(
                (p.confidence - expected).abs() < 1e-6,
                "confidence mismatch for '{}'",
                p.name
            );
        }
    }

    #[test]
    fn empty_examples_returns_no_principles() {
        let miner = PatternMiner::new(1);
        assert!(miner.mine(&[]).is_empty());
    }
}
