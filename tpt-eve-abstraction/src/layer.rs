// SPDX-License-Identifier: MIT OR Apache-2.0
use tpt_eve_core::Fact;

use crate::{
    analogy::{Analogy, AnalogyMapper},
    principle::{AbstractPrinciple, PatternMiner},
    transfer::transfer_knowledge,
};

/// High-level facade for the Phase 3 Abstraction layer.
///
/// Typical usage:
/// ```text
/// let mut engine = AbstractionEngine::new();
/// engine.add_example("typescript", typescript_facts);
/// engine.add_example("python", python_facts);
/// let principles = engine.mine_principles(2);
/// let analogy = engine.map_analogy("mammals", &mammal_facts, "birds", &bird_facts);
/// let transferred = engine.transfer(&analogy, &new_mammal_facts);
/// ```
#[derive(Debug, Default)]
pub struct AbstractionEngine {
    examples: Vec<(String, Vec<Fact>)>,
}

impl AbstractionEngine {
    pub fn new() -> Self {
        AbstractionEngine::default()
    }

    /// Register a named set of facts as one domain example.
    pub fn add_example(&mut self, name: impl Into<String>, facts: Vec<Fact>) {
        self.examples.push((name.into(), facts));
    }

    /// Mine abstract principles from all registered examples.
    ///
    /// `min_support` is the minimum number of examples a pattern must appear in to qualify as
    /// a principle. Returns principles sorted by confidence (descending).
    pub fn mine_principles(&self, min_support: usize) -> Vec<AbstractPrinciple> {
        let fact_sets: Vec<Vec<Fact>> = self.examples.iter().map(|(_, f)| f.clone()).collect();
        PatternMiner::new(min_support).mine(&fact_sets)
    }

    /// Compute the best structural analogy mapping from `source_facts` to `target_facts`.
    pub fn map_analogy(
        &self,
        source_domain: &str,
        source_facts: &[Fact],
        target_domain: &str,
        target_facts: &[Fact],
    ) -> Analogy {
        AnalogyMapper::new().map(source_domain, source_facts, target_domain, target_facts)
    }

    /// Transfer facts from a source domain into a target domain via an analogy mapping.
    pub fn transfer(&self, analogy: &Analogy, source_facts: &[Fact]) -> Vec<Fact> {
        transfer_knowledge(analogy, source_facts)
    }

    /// All registered example names.
    pub fn example_names(&self) -> Vec<&str> {
        self.examples.iter().map(|(n, _)| n.as_str()).collect()
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

    fn lang_facts(name: &str) -> Vec<Fact> {
        vec![
            fact(name, "is", "programming_language"),
            fact("programming_language", "has", "syntax"),
        ]
    }

    #[test]
    fn engine_mines_principles_from_examples() {
        let mut engine = AbstractionEngine::new();
        engine.add_example("typescript", lang_facts("TypeScript"));
        engine.add_example("python", lang_facts("Python"));
        engine.add_example("rust", lang_facts("Rust"));
        let principles = engine.mine_principles(2);
        assert!(!principles.is_empty());
        assert!(principles
            .iter()
            .any(|p| p.predicate_chain == ["is", "has"]));
    }

    #[test]
    fn engine_maps_and_transfers() {
        let engine = AbstractionEngine::new();
        let mammals = vec![fact("cat", "is", "mammal"), fact("mammal", "has", "fur")];
        let birds = vec![fact("eagle", "is", "bird"), fact("bird", "has", "feathers")];
        let analogy = engine.map_analogy("mammals", &mammals, "birds", &birds);
        assert!(analogy.score > 0.0);

        let new_facts = vec![fact("cat", "lives_in", "forest")];
        let transferred = engine.transfer(&analogy, &new_facts);
        assert_eq!(transferred.len(), 1);
    }

    #[test]
    fn example_names_are_tracked() {
        let mut engine = AbstractionEngine::new();
        engine.add_example("domain_a", vec![]);
        engine.add_example("domain_b", vec![]);
        assert_eq!(engine.example_names(), vec!["domain_a", "domain_b"]);
    }
}
