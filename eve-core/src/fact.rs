// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};

/// A single term in a fact triple. `Symbol` is an identifier-like token
/// (e.g. `TypeScript`, `has`); `Text` is a free-form quoted phrase
/// (e.g. `"programming language"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Value {
    Symbol(String),
    Text(String),
}

impl Value {
    pub fn as_str(&self) -> &str {
        match self {
            Value::Symbol(s) => s,
            Value::Text(s) => s,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Symbol(s) => write!(f, "{s}"),
            Value::Text(s) => write!(f, "\"{s}\""),
        }
    }
}

/// Where a fact came from, for provenance and confidence tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FactSource {
    /// Hand-authored / loaded from a `.eve` source file.
    Asserted,
    /// Produced by a `NeuralPatternLayer::extract_facts` call.
    Extracted { confidence: f32 },
    /// Derived by the forward-chaining inference engine.
    Inferred { rule_name: Option<String> },
}

/// A subject-predicate-object triple held in working memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub subject: Value,
    pub predicate: Value,
    pub object: Value,
    pub source: FactSource,
    pub confidence: f32,
}

impl Fact {
    pub fn asserted(subject: Value, predicate: Value, object: Value) -> Self {
        Fact {
            subject,
            predicate,
            object,
            source: FactSource::Asserted,
            confidence: 1.0,
        }
    }

    /// Whether two facts share the same (subject, predicate, object) triple,
    /// ignoring source/confidence — used for working-memory deduplication.
    pub fn same_triple(&self, other: &Fact) -> bool {
        self.subject == other.subject
            && self.predicate == other.predicate
            && self.object == other.object
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_triple_ignores_source_and_confidence() {
        let a = Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("is".into()),
            Value::Text("programming language".into()),
        );
        let mut b = a.clone();
        b.source = FactSource::Extracted { confidence: 0.5 };
        b.confidence = 0.5;
        assert!(a.same_triple(&b));
    }

    #[test]
    fn different_object_is_not_same_triple() {
        let a = Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("is".into()),
            Value::Text("programming language".into()),
        );
        let b = Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("is".into()),
            Value::Text("markup language".into()),
        );
        assert!(!a.same_triple(&b));
    }

    #[test]
    fn serde_round_trip() {
        let fact = Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("has".into()),
            Value::Text("type system".into()),
        );
        let json = serde_json::to_string(&fact).unwrap();
        let back: Fact = serde_json::from_str(&json).unwrap();
        assert_eq!(fact, back);
    }

    #[test]
    fn value_display() {
        assert_eq!(Value::Symbol("TypeScript".into()).to_string(), "TypeScript");
        assert_eq!(
            Value::Text("type system".into()).to_string(),
            "\"type system\""
        );
    }
}
