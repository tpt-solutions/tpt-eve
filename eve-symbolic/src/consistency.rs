// SPDX-License-Identifier: Apache-2.0
use eve_core::{Fact, Value};
use std::collections::HashMap;

/// A contradiction detected in working memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contradiction {
    pub subject: Value,
    pub predicate: Value,
    pub conflicting_objects: Vec<Value>,
}

/// Checks working memory for contradictions. Uses structural checking
/// (same subject+predicate, differing objects for "exclusive" predicates).
pub struct ConsistencyChecker;

impl ConsistencyChecker {
    pub fn new() -> Self {
        ConsistencyChecker
    }

    /// Check for contradictions in the working memory.
    /// Returns a vector of detected contradictions.
    pub fn check(&self, facts: &[Fact]) -> Vec<Contradiction> {
        let mut subject_predicate_objects: HashMap<(String, String), Vec<Value>> = HashMap::new();

        for fact in facts {
            let subject_key = fact.subject.as_str().to_string();
            let predicate_key = fact.predicate.as_str().to_string();
            let object = fact.object.clone();
            subject_predicate_objects
                .entry((subject_key, predicate_key))
                .or_default()
                .push(object);
        }

        let mut contradictions = Vec::new();
        for ((subject, predicate), objects) in subject_predicate_objects {
            // Check if this is an "exclusive" predicate (is, was, has)
            if self.is_exclusive_predicate(&predicate) && objects.len() > 1 {
                contradictions.push(Contradiction {
                    subject: Value::Symbol(subject),
                    predicate: Value::Symbol(predicate),
                    conflicting_objects: objects,
                });
            }
        }

        contradictions
    }

    fn is_exclusive_predicate(&self, predicate: &str) -> bool {
        // "is"/"was" are exclusive (identity); "has" is not (a thing can have multiple attributes)
        matches!(predicate.to_lowercase().as_str(), "is" | "was")
    }
}

impl Default for ConsistencyChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_no_contradiction_for_unique_facts() {
        let wm = vec![Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("is".into()),
            Value::Text("programming language".into()),
        )];
        let checker = ConsistencyChecker::new();
        let contradictions = checker.check(&wm);
        assert!(contradictions.is_empty());
    }

    #[test]
    fn detects_contradiction_for_same_subject_predicate() {
        let wm = vec![
            Fact::asserted(
                Value::Symbol("TypeScript".into()),
                Value::Symbol("is".into()),
                Value::Text("programming language".into()),
            ),
            Fact::asserted(
                Value::Symbol("TypeScript".into()),
                Value::Symbol("is".into()),
                Value::Text("markup language".into()),
            ),
        ];
        let checker = ConsistencyChecker::new();
        let contradictions = checker.check(&wm);
        assert_eq!(contradictions.len(), 1);
        assert_eq!(contradictions[0].conflicting_objects.len(), 2);
    }

    #[test]
    fn no_contradiction_for_different_subjects() {
        let wm = vec![
            Fact::asserted(
                Value::Symbol("TypeScript".into()),
                Value::Symbol("has".into()),
                Value::Text("type system".into()),
            ),
            Fact::asserted(
                Value::Symbol("JavaScript".into()),
                Value::Symbol("has".into()),
                Value::Text("type system".into()),
            ),
        ];
        let checker = ConsistencyChecker::new();
        let contradictions = checker.check(&wm);
        assert!(contradictions.is_empty());
    }
}
