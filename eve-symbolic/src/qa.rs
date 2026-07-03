// SPDX-License-Identifier: Apache-2.0
use eve_core::{Pattern, PatternTerm, Value};

/// Translate a natural language question into a Pattern query.
///
/// Supported forms (case-insensitive keywords, entity case preserved):
/// - "What is X?" → Pattern(X, is, ?)
/// - "What does X have?" → Pattern(X, has, ?)
/// - "What has X?" → Pattern(?, has, X)
pub fn question_to_pattern(question: &str) -> Pattern {
    let q_lower = question.to_lowercase();

    // "What is X?" → (X, is, ?)
    if q_lower.starts_with("what is ") {
        let entity = question["what is ".len()..].trim_end_matches('?').trim();
        if !entity.is_empty() {
            return Pattern::new(
                PatternTerm::Bound(Value::Symbol(entity.to_string())),
                PatternTerm::Bound(Value::Symbol("is".into())),
                PatternTerm::Variable("X".into()),
            );
        }
    }

    // "What does X have?" → (X, has, ?)
    if q_lower.starts_with("what does ") && q_lower.contains(" have") {
        let rest = &question["what does ".len()..];
        if let Some(have_pos) = rest.to_lowercase().find(" have") {
            let entity = rest[..have_pos].trim();
            if !entity.is_empty() {
                return Pattern::new(
                    PatternTerm::Bound(Value::Symbol(entity.to_string())),
                    PatternTerm::Bound(Value::Symbol("has".into())),
                    PatternTerm::Variable("X".into()),
                );
            }
        }
    }

    // "What has X?" → (?, has, X)
    if q_lower.starts_with("what has ") {
        let entity = question["what has ".len()..].trim_end_matches('?').trim();
        if !entity.is_empty() {
            return Pattern::new(
                PatternTerm::Variable("X".into()),
                PatternTerm::Bound(Value::Symbol("has".into())),
                PatternTerm::Bound(Value::Symbol(entity.to_string())),
            );
        }
    }

    // Fallback: broad query
    Pattern::new(
        PatternTerm::Variable("S".into()),
        PatternTerm::Variable("P".into()),
        PatternTerm::Variable("O".into()),
    )
}

/// Extract attribute value from question like "What does X have?" or "What is X?"
pub fn question_to_attribute_query(question: &str, attribute: &str) -> Pattern {
    let q = question.to_lowercase();

    // "What has X?" -> who/what has X?
    if let Some(rest) = q.strip_prefix("what has ") {
        let entity = rest.trim_end_matches('?').trim();
        if !entity.is_empty() {
            return Pattern::new(
                PatternTerm::Variable("S".into()),
                PatternTerm::Bound(Value::Symbol(attribute.into())),
                PatternTerm::Bound(Value::Symbol(entity.to_string())),
            );
        }
    }

    // "What is X?" -> X is Y
    if let Some(rest) = q.strip_prefix("what is ") {
        let entity = rest.trim_end_matches('?').trim();
        if !entity.is_empty() {
            return Pattern::new(
                PatternTerm::Bound(Value::Symbol(entity.to_string())),
                PatternTerm::Bound(Value::Symbol(attribute.into())),
                PatternTerm::Variable("O".into()),
            );
        }
    }

    // Fallback
    Pattern::new(
        PatternTerm::Variable("S".into()),
        PatternTerm::Bound(Value::Symbol(attribute.into())),
        PatternTerm::Variable("O".into()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_is_question() {
        let pattern = question_to_pattern("What is TypeScript?");
        assert!(
            matches!(pattern.subject, PatternTerm::Bound(Value::Symbol(ref s)) if s == "TypeScript")
        );
    }

    #[test]
    fn parses_what_does_have_question() {
        let pattern = question_to_pattern("What does TypeScript have?");
        assert!(
            matches!(pattern.subject, PatternTerm::Bound(Value::Symbol(ref s)) if s == "TypeScript")
        );
    }
}
