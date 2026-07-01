use eve_core::{Pattern, PatternTerm, Value};

/// Translate a natural language question into a Pattern query.
pub fn question_to_pattern(question: &str) -> Pattern {
    // Simple heuristic: "What does X have?" -> Pattern(X, has, Variable)
    // "What is X?" -> Pattern(X, is, Variable)
    // "What is Y?" -> Pattern(Variable, is, Y)
    
    let q = question.to_lowercase();
    
    // Try to parse "what is X" pattern
    if let Some(rest) = q.strip_prefix("what is ") {
        let entity = rest.trim_end_matches('?').trim();
        if !entity.is_empty() {
            return Pattern::new(
                PatternTerm::Bound(Value::Symbol(entity.to_string())),
                PatternTerm::Bound(Value::Symbol("is".into())),
                PatternTerm::Variable("X".into()),
            );
        }
    }
    
    // Try to parse "what does X have" pattern
    if let Some(rest) = q.strip_prefix("what does ") {
        if let Some(entity) = rest.strip_prefix("have") {
            // Pattern: X has ?
            return Pattern::new(
                PatternTerm::Bound(Value::Symbol(entity.to_string())),
                PatternTerm::Bound(Value::Symbol("has".into())),
                PatternTerm::Variable("X".into()),
            );
        }
    }
    
    // Try to parse "what has X" pattern
    if let Some(rest) = q.strip_prefix("what has ") {
        let entity = rest.trim_end_matches('?').trim();
        if !entity.is_empty() {
            return Pattern::new(
                PatternTerm::Variable("X".into()),
                PatternTerm::Bound(Value::Symbol("has".into())),
                PatternTerm::Bound(Value::Symbol(entity.to_string())),
            );
        }
    }
    
    // Default: try to extract any identifier and make a broad query
    for word in q.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric())
            .trim_end_matches('?');
        if !clean.is_empty() && clean.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ') {
            // Try both subject and object positions
            return Pattern::new(
                PatternTerm::Bound(Value::Symbol(clean.to_string())),
                PatternTerm::Variable("P".into()),
                PatternTerm::Variable("O".into()),
            );
        }
    }
    
    // Ultimate fallback: broad query
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
        assert!(matches!(pattern.subject, PatternTerm::Bound(Value::Symbol(ref s)) if s == "TypeScript"));
    }

    #[test]
    fn parses_what_does_have_question() {
        let pattern = question_to_pattern("What does TypeScript have?");
        assert!(matches!(pattern.subject, PatternTerm::Bound(Value::Symbol(ref s)) if s == "TypeScript"));
    }
}