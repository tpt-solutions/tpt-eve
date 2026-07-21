// SPDX-License-Identifier: MIT OR Apache-2.0
/// Format a fact extraction prompt with the given text.
pub fn format_fact_prompt(text: &str) -> String {
    format!(
        r#"Extract all factual statements from the following text. For each fact, output a JSON object with "subject", "predicate", and "object" fields. Return a JSON array of facts.

Example input: "TypeScript is a programming language. It provides compile-time type checking."
Example output:
[
  {{"subject": "TypeScript", "predicate": "is", "object": "programming language"}},
  {{"subject": "TypeScript", "predicate": "provides", "object": "compile-time type checking"}}
]

Text to analyze:
{}"#,
        text
    )
}

/// Format a causal extraction prompt with the given text.
pub fn format_causal_prompt(text: &str) -> String {
    format!(
        r#"Extract causal relationships from the following text. For each relationship, output a JSON object with "cause", "effect", and "relation" (causes|precedes|enables|prevents) fields. Return a JSON array.

Example input: "When you study more, your exam score improves. Lack of sleep prevents good performance."
Example output:
[
  {{"cause": "study more", "effect": "exam score improves", "relation": "causes"}},
  {{"cause": "lack of sleep", "effect": "good performance", "relation": "prevents"}}
]

Text to analyze:
{}"#,
        text
    )
}
