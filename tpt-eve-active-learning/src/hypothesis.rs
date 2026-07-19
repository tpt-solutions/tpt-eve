// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tpt_eve_core::{Fact, FactSource, Value};

/// A testable prediction generated from observed facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Human-readable description of what is being hypothesised.
    pub description: String,
    /// The fact predicted to be true if the hypothesis holds.
    pub prediction: Fact,
    /// Current confidence that the hypothesis is correct (0.0–1.0).
    pub confidence: f32,
    /// The existing facts that motivated this hypothesis.
    pub supporting_facts: Vec<Fact>,
}

/// Generates `Hypothesis` candidates from a set of known facts using two strategies:
///
/// 1. **Chain completion**: if `A -p1-> B -p2-> C` exists but `A -p2-> C` does not,
///    the transitive link `(A, p2, C)` is a candidate hypothesis.
///
/// 2. **Category generalisation**: if `A -is-> C` and `B -is-> C`, and `A -has-> D`
///    but `B -has-> D` does not exist, suggest `(B, has, D)` as a hypothesis.
pub struct HypothesisGenerator;

impl HypothesisGenerator {
    pub fn new() -> Self {
        HypothesisGenerator
    }

    /// Generate all candidate hypotheses from `facts`.
    pub fn generate(&self, facts: &[Fact]) -> Vec<Hypothesis> {
        let mut seen: HashSet<(String, String, String)> = HashSet::new();
        let mut hypotheses = Vec::new();

        self.chain_completions(facts, &mut seen, &mut hypotheses);
        self.category_generalisations(facts, &mut seen, &mut hypotheses);

        hypotheses
    }

    // -----------------------------------------------------------------------

    fn chain_completions(
        &self,
        facts: &[Fact],
        seen: &mut HashSet<(String, String, String)>,
        out: &mut Vec<Hypothesis>,
    ) {
        for f1 in facts {
            for f2 in facts {
                if f1.object != f2.subject {
                    continue;
                }
                // f1: A -p1-> B,  f2: B -p2-> C
                let key = (
                    f1.subject.as_str().to_string(),
                    f2.predicate.as_str().to_string(),
                    f2.object.as_str().to_string(),
                );
                if seen.contains(&key) {
                    continue;
                }
                let predicted = Fact {
                    subject: f1.subject.clone(),
                    predicate: f2.predicate.clone(),
                    object: f2.object.clone(),
                    source: FactSource::Extracted { confidence: 0.6 },
                    confidence: 0.6,
                };
                // Don't re-generate facts that already exist.
                if facts.iter().any(|f| f.same_triple(&predicted)) {
                    seen.insert(key);
                    continue;
                }
                seen.insert(key);
                out.push(Hypothesis {
                    description: format!(
                        "chain completion: {} -[{}]-> {} (via {})",
                        f1.subject,
                        f2.predicate.as_str(),
                        f2.object,
                        f1.object
                    ),
                    prediction: predicted,
                    confidence: 0.6,
                    supporting_facts: vec![f1.clone(), f2.clone()],
                });
            }
        }
    }

    fn category_generalisations(
        &self,
        facts: &[Fact],
        seen: &mut HashSet<(String, String, String)>,
        out: &mut Vec<Hypothesis>,
    ) {
        // Collect all (member, category) pairs for "is" predicate.
        let members: Vec<(&Value, &Value)> = facts
            .iter()
            .filter(|f| f.predicate == Value::Symbol("is".into()))
            .map(|f| (&f.subject, &f.object))
            .collect();

        // For each property A -has-> D, look for category-mates B without that property.
        for prop_fact in facts
            .iter()
            .filter(|f| f.predicate == Value::Symbol("has".into()))
        {
            let a = &prop_fact.subject;
            let d = &prop_fact.object;

            // Find categories A belongs to.
            let a_categories: Vec<&Value> = members
                .iter()
                .filter(|(m, _)| *m == a)
                .map(|(_, c)| *c)
                .collect();

            for category in a_categories {
                // Find other members of the same category.
                for (b, cat) in &members {
                    if *cat != category || *b == a {
                        continue;
                    }
                    let key = (
                        b.as_str().to_string(),
                        "has".to_string(),
                        d.as_str().to_string(),
                    );
                    if seen.contains(&key) {
                        continue;
                    }
                    let predicted = Fact {
                        subject: (*b).clone(),
                        predicate: Value::Symbol("has".into()),
                        object: d.clone(),
                        source: FactSource::Extracted { confidence: 0.55 },
                        confidence: 0.55,
                    };
                    if facts.iter().any(|f| f.same_triple(&predicted)) {
                        seen.insert(key);
                        continue;
                    }
                    seen.insert(key);
                    out.push(Hypothesis {
                        description: format!(
                            "category generalisation: {} has {} (like {} in category {})",
                            b, d, a, category
                        ),
                        prediction: predicted,
                        confidence: 0.55,
                        supporting_facts: vec![prop_fact.clone()],
                    });
                }
            }
        }
    }
}

impl Default for HypothesisGenerator {
    fn default() -> Self {
        HypothesisGenerator::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_eve_core::Value;

    fn fact(s: &str, p: &str, o: &str) -> Fact {
        Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        )
    }

    #[test]
    fn chain_completion_generates_transitive_hypothesis() {
        // A -is-> B, B -has-> C  →  suggest A -has-> C
        let facts = vec![
            fact("TypeScript", "is", "programming_language"),
            fact("programming_language", "has", "syntax"),
        ];
        let gen = HypothesisGenerator::new();
        let hs = gen.generate(&facts);
        assert!(
            hs.iter().any(|h| {
                h.prediction.subject == Value::Symbol("TypeScript".into())
                    && h.prediction.predicate == Value::Symbol("has".into())
                    && h.prediction.object == Value::Symbol("syntax".into())
            }),
            "expected TypeScript-has-syntax chain hypothesis; got: {hs:?}"
        );
    }

    #[test]
    fn does_not_duplicate_existing_facts() {
        let facts = vec![
            fact("TypeScript", "is", "programming_language"),
            fact("programming_language", "has", "syntax"),
            fact("TypeScript", "has", "syntax"), // already exists
        ];
        let gen = HypothesisGenerator::new();
        let hs = gen.generate(&facts);
        assert!(
            !hs.iter().any(|h| {
                h.prediction.subject == Value::Symbol("TypeScript".into())
                    && h.prediction.predicate == Value::Symbol("has".into())
                    && h.prediction.object == Value::Symbol("syntax".into())
            }),
            "should not suggest a fact that already exists"
        );
    }

    #[test]
    fn category_generalisation_suggests_shared_property() {
        // Python and TypeScript are both programming_languages; TypeScript has syntax → suggest Python has syntax
        let facts = vec![
            fact("TypeScript", "is", "programming_language"),
            fact("Python", "is", "programming_language"),
            fact("TypeScript", "has", "syntax"),
        ];
        let gen = HypothesisGenerator::new();
        let hs = gen.generate(&facts);
        assert!(
            hs.iter().any(|h| {
                h.prediction.subject == Value::Symbol("Python".into())
                    && h.prediction.predicate == Value::Symbol("has".into())
                    && h.prediction.object == Value::Symbol("syntax".into())
            }),
            "expected Python-has-syntax category hypothesis; got: {hs:?}"
        );
    }

    #[test]
    fn empty_facts_produce_no_hypotheses() {
        let gen = HypothesisGenerator::new();
        assert!(gen.generate(&[]).is_empty());
    }
}
