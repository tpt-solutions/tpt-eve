// SPDX-License-Identifier: Apache-2.0
use eve_core::{Fact, FactSource, Value};

use crate::analogy::Analogy;

/// Project `source_facts` into the target domain described by `analogy`.
///
/// Each fact's subject and object are substituted via `analogy.mapping`; values absent from
/// the mapping are kept verbatim. The confidence of each projected fact is scaled by
/// `analogy.score` to reflect the analogical uncertainty, and provenance is tagged as
/// `Inferred` with the analogy's source domain name.
pub fn transfer_knowledge(analogy: &Analogy, source_facts: &[Fact]) -> Vec<Fact> {
    source_facts
        .iter()
        .map(|f| Fact {
            subject: map_value(&f.subject, analogy),
            predicate: f.predicate.clone(),
            object: map_value(&f.object, analogy),
            source: FactSource::Inferred {
                rule_name: Some(format!("analogy:{}", analogy.source_domain)),
            },
            confidence: f.confidence * analogy.score as f32,
        })
        .collect()
}

fn map_value(val: &Value, analogy: &Analogy) -> Value {
    match analogy.mapping.get(val.as_str()) {
        Some(mapped) => match val {
            Value::Symbol(_) => Value::Symbol(mapped.clone()),
            Value::Text(_) => Value::Text(mapped.clone()),
        },
        None => val.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analogy::{Analogy, AnalogyMapper};

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
        ]
    }

    fn bird_facts() -> Vec<Fact> {
        vec![
            fact("eagle", "is", "bird"),
            fact("eagle", "has", "feathers"),
            fact("hawk", "is", "bird"),
        ]
    }

    fn build_analogy() -> Analogy {
        AnalogyMapper::new().map("mammals", &mammal_facts(), "birds", &bird_facts())
    }

    #[test]
    fn transferred_facts_use_target_entities() {
        let analogy = build_analogy();
        let new_mammal_facts = vec![fact("cat", "lives_in", "forest")];
        let transferred = transfer_knowledge(&analogy, &new_mammal_facts);
        assert_eq!(transferred.len(), 1);
        // "cat" should be mapped to its analogue in the bird domain
        let mapped_subject = transferred[0].subject.as_str().to_string();
        assert!(
            analogy.mapping.get("cat").map(String::as_str) == Some(mapped_subject.as_str()),
            "subject should be remapped via analogy"
        );
    }

    #[test]
    fn predicate_is_preserved() {
        let analogy = build_analogy();
        let source = vec![fact("cat", "eats", "fish")];
        let result = transfer_knowledge(&analogy, &source);
        assert_eq!(result[0].predicate, Value::Symbol("eats".into()));
    }

    #[test]
    fn confidence_is_scaled_by_analogy_score() {
        let analogy = build_analogy();
        let source = vec![Fact {
            subject: Value::Symbol("cat".into()),
            predicate: Value::Symbol("is".into()),
            object: Value::Symbol("mammal".into()),
            source: eve_core::FactSource::Asserted,
            confidence: 1.0,
        }];
        let result = transfer_knowledge(&analogy, &source);
        let expected = analogy.score as f32;
        assert!(
            (result[0].confidence - expected).abs() < 1e-5,
            "confidence should equal analogy.score when source confidence is 1.0"
        );
    }

    #[test]
    fn unmapped_values_are_kept_verbatim() {
        let analogy = Analogy {
            source_domain: "A".into(),
            target_domain: "B".into(),
            mapping: std::collections::HashMap::new(),
            score: 0.5,
        };
        let source = vec![fact("unknown_entity", "rel", "unknown_target")];
        let result = transfer_knowledge(&analogy, &source);
        assert_eq!(result[0].subject, Value::Symbol("unknown_entity".into()));
        assert_eq!(result[0].object, Value::Symbol("unknown_target".into()));
    }
}
