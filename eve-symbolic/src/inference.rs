use crate::rule::Rule;
use eve_core::Fact;
use serde::{Deserialize, Serialize};

/// An inference result tracking the rule used and derived fact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inference {
    pub rule_name: Option<String>,
    pub derived: Fact,
}

/// Forward-chaining inference engine.
pub struct InferenceEngine {
    pub max_iterations: usize,
}

impl InferenceEngine {
    pub fn new() -> Self {
        InferenceEngine {
            max_iterations: 100,
        }
    }

    pub fn with_max_iterations(max: usize) -> Self {
        InferenceEngine {
            max_iterations: max,
        }
    }

    /// Run forward chaining until fixpoint (no new facts derived) or iteration cap.
    pub fn forward_chain(
        &self,
        wm: &mut crate::working_memory::WorkingMemory,
        rules: &[Rule],
    ) -> Vec<Inference> {
        let mut inferences = Vec::new();
        let mut iterations = 0;

        loop {
            if iterations >= self.max_iterations {
                break;
            }
            iterations += 1;

            let mut new_facts = Vec::new();

            for rule in rules {
                let solutions = rule.check_conditions(wm.facts(), &rule.conditions);
                for bindings in solutions {
                    if let Some(derived) = rule.apply(&bindings) {
                        // Check if this fact already exists with higher or equal confidence
                        if !wm
                            .facts()
                            .iter()
                            .any(|f| f.same_triple(&derived) && f.confidence >= derived.confidence)
                        {
                            new_facts.push(derived);
                        }
                    }
                }
            }

            if new_facts.is_empty() {
                break; // Fixpoint reached
            }

            for fact in new_facts {
                let rule_name = match &fact.source {
                    eve_core::FactSource::Inferred { rule_name } => rule_name.clone(),
                    _ => None,
                };
                let inference = Inference {
                    rule_name,
                    derived: fact.clone(),
                };
                inferences.push(inference);
                wm.add_fact(fact);
            }
        }

        inferences
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::working_memory::WorkingMemory;
    use eve_core::{PatternTerm, Value};

    #[test]
    fn forward_chain_derives_transitive_fact() {
        let mut wm = WorkingMemory::new();
        // TypeScript is programming language
        wm.add_fact(Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("is".into()),
            Value::Symbol("programming language".into()),
        ));
        // programming language has syntax
        wm.add_fact(Fact::asserted(
            Value::Symbol("programming language".into()),
            Value::Symbol("has".into()),
            Value::Symbol("syntax".into()),
        ));

        // Rule: if X is Y, Y has Z then X has Z
        let rule = Rule::new(
            Some("transitive_has".into()),
            vec![
                (
                    PatternTerm::Variable("X".into()),
                    PatternTerm::Bound(Value::Symbol("is".into())),
                    PatternTerm::Variable("Y".into()),
                ),
                (
                    PatternTerm::Variable("Y".into()),
                    PatternTerm::Bound(Value::Symbol("has".into())),
                    PatternTerm::Variable("Z".into()),
                ),
            ],
            (
                PatternTerm::Variable("X".into()),
                PatternTerm::Bound(Value::Symbol("has".into())),
                PatternTerm::Variable("Z".into()),
            ),
        );

        let engine = InferenceEngine::new();
        let inferences = engine.forward_chain(&mut wm, &[rule]);

        assert_eq!(inferences.len(), 1);
        assert!(inferences[0].derived.subject == Value::Symbol("TypeScript".into()));
        assert!(inferences[0].derived.predicate == Value::Symbol("has".into()));
        assert!(inferences[0].derived.object == Value::Symbol("syntax".into()));
    }

    #[test]
    fn forward_chain_fixpoint() {
        let mut wm = WorkingMemory::new();
        wm.add_fact(Fact::asserted(
            Value::Symbol("A".into()),
            Value::Symbol("is".into()),
            Value::Symbol("B".into()),
        ));
        wm.add_fact(Fact::asserted(
            Value::Symbol("B".into()),
            Value::Symbol("is".into()),
            Value::Symbol("C".into()),
        ));

        // Rule: if X is Y then Y is X (symmetric)
        let rule = Rule::new(
            Some("symmetric".into()),
            vec![(
                PatternTerm::Variable("X".into()),
                PatternTerm::Bound(Value::Symbol("is".into())),
                PatternTerm::Variable("Y".into()),
            )],
            (
                PatternTerm::Variable("Y".into()),
                PatternTerm::Bound(Value::Symbol("is".into())),
                PatternTerm::Variable("X".into()),
            ),
        );

        let engine = InferenceEngine::new();
        let inferences = engine.forward_chain(&mut wm, &[rule]);

        // Should derive: B is A, C is B (then on next iteration: A is B exists, C is B derived, etc.)
        assert!(!inferences.is_empty());
    }
}
