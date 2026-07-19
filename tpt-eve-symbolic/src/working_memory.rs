// SPDX-License-Identifier: MIT OR Apache-2.0
use std::collections::HashMap;
use tpt_eve_core::{Fact, Pattern, PatternTerm, Value};

/// Working memory holds facts and supports indexed lookup for pattern matching.
/// Implements a minimal "index + scan" Datalog pattern for Phase 1.
#[derive(Debug, Clone, Default)]
pub struct WorkingMemory {
    /// All facts held in memory.
    facts: Vec<Fact>,
    /// Index by subject for O(1) candidate narrowing.
    by_subject: HashMap<String, Vec<usize>>,
    /// Index by predicate.
    by_predicate: HashMap<String, Vec<usize>>,
    /// Index by object.
    by_object: HashMap<String, Vec<usize>>,
}

impl WorkingMemory {
    pub fn new() -> Self {
        WorkingMemory {
            facts: Vec::new(),
            by_subject: HashMap::new(),
            by_predicate: HashMap::new(),
            by_object: HashMap::new(),
        }
    }

    /// Add a fact to working memory. If a fact with the same triple already exists,
    /// keep the one with higher confidence.
    pub fn add_fact(&mut self, fact: Fact) {
        // Check for existing triple with lower confidence
        for existing in &self.facts {
            if existing.same_triple(&fact) && existing.confidence >= fact.confidence {
                return;
            }
        }

        // Remove any existing fact with the same triple before inserting the higher-confidence one
        self.facts.retain(|existing| !existing.same_triple(&fact));

        // Rebuild indexes
        self.by_subject.clear();
        self.by_predicate.clear();
        self.by_object.clear();

        self.facts.push(fact);

        for (i, f) in self.facts.iter().enumerate() {
            let subject_key = match &f.subject {
                Value::Symbol(s) => s.clone(),
                Value::Text(s) => s.clone(),
            };
            let predicate_key = match &f.predicate {
                Value::Symbol(s) => s.clone(),
                Value::Text(s) => s.clone(),
            };
            let object_key = match &f.object {
                Value::Symbol(s) => s.clone(),
                Value::Text(s) => s.clone(),
            };
            self.by_subject.entry(subject_key).or_default().push(i);
            self.by_predicate.entry(predicate_key).or_default().push(i);
            self.by_object.entry(object_key).or_default().push(i);
        }
    }

    /// Query working memory for all facts matching the given pattern.
    pub fn query(&self, pattern: &Pattern) -> Vec<Fact> {
        let candidates = self.find_candidates(pattern);
        candidates
            .into_iter()
            .filter(|fact| self.matches_pattern(fact, pattern))
            .collect()
    }

    /// Query with multiple conjunctive clauses (join across patterns).
    pub fn query_conjunctive(&self, patterns: &[Pattern]) -> Vec<HashMap<String, Value>> {
        if patterns.is_empty() {
            return vec![HashMap::new()];
        }

        let mut solutions: Vec<HashMap<String, Value>> = Vec::new();
        self.query_conjunctive_recursive(patterns, 0, &HashMap::new(), &mut solutions);
        solutions
    }

    fn query_conjunctive_recursive(
        &self,
        patterns: &[Pattern],
        idx: usize,
        bindings: &HashMap<String, Value>,
        solutions: &mut Vec<HashMap<String, Value>>,
    ) {
        if idx >= patterns.len() {
            solutions.push(bindings.clone());
            return;
        }

        for fact in self.query(&patterns[idx]) {
            if let Some(new_bindings) = self.unify_with_bindings(&fact, &patterns[idx], bindings) {
                self.query_conjunctive_recursive(patterns, idx + 1, &new_bindings, solutions);
            }
        }
    }

    fn unify_with_bindings(
        &self,
        fact: &Fact,
        pattern: &Pattern,
        bindings: &HashMap<String, Value>,
    ) -> Option<HashMap<String, Value>> {
        let mut result = bindings.clone();

        for (slot, value) in [
            (&pattern.subject, &fact.subject),
            (&pattern.predicate, &fact.predicate),
            (&pattern.object, &fact.object),
        ] {
            match slot {
                PatternTerm::Bound(bound) => {
                    if bound != value {
                        return None;
                    }
                }
                PatternTerm::Variable(var) => {
                    if let Some(existing) = result.get(var) {
                        if existing != value {
                            return None;
                        }
                    } else {
                        result.insert(var.clone(), value.clone());
                    }
                }
            }
        }

        Some(result)
    }

    fn find_candidates(&self, pattern: &Pattern) -> Vec<Fact> {
        // Narrow candidates by any bound slots
        let candidate_indices: Vec<usize> =
            match (&pattern.subject, &pattern.predicate, &pattern.object) {
                (PatternTerm::Bound(Value::Symbol(s)), _, _) => {
                    self.by_subject.get(s).cloned().unwrap_or_default()
                }
                (_, PatternTerm::Bound(Value::Symbol(p)), _) => {
                    self.by_predicate.get(p).cloned().unwrap_or_default()
                }
                (_, _, PatternTerm::Bound(Value::Symbol(o))) => {
                    self.by_object.get(o).cloned().unwrap_or_default()
                }
                (PatternTerm::Bound(Value::Text(s)), _, _) => {
                    self.by_subject.get(s).cloned().unwrap_or_default()
                }
                (_, PatternTerm::Bound(Value::Text(p)), _) => {
                    self.by_predicate.get(p).cloned().unwrap_or_default()
                }
                (_, _, PatternTerm::Bound(Value::Text(o))) => {
                    self.by_object.get(o).cloned().unwrap_or_default()
                }
                _ => (0..self.facts.len()).collect(),
            };

        candidate_indices
            .into_iter()
            .map(|i| self.facts[i].clone())
            .collect()
    }

    fn matches_pattern(&self, fact: &Fact, pattern: &Pattern) -> bool {
        let subject_match = match &pattern.subject {
            PatternTerm::Bound(v) => v == &fact.subject,
            PatternTerm::Variable(_) => true,
        };
        let predicate_match = match &pattern.predicate {
            PatternTerm::Bound(v) => v == &fact.predicate,
            PatternTerm::Variable(_) => true,
        };
        let object_match = match &pattern.object {
            PatternTerm::Bound(v) => v == &fact.object,
            PatternTerm::Variable(_) => true,
        };
        subject_match && predicate_match && object_match
    }

    pub fn facts(&self) -> &[Fact] {
        &self.facts
    }

    pub fn len(&self) -> usize {
        self.facts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fact() -> Fact {
        Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("is".into()),
            Value::Text("programming language".into()),
        )
    }

    #[test]
    fn add_fact_inserts() {
        let mut wm = WorkingMemory::new();
        wm.add_fact(sample_fact());
        assert_eq!(wm.len(), 1);
    }

    #[test]
    fn add_fact_deduplicates_same_triple() {
        let mut wm = WorkingMemory::new();
        wm.add_fact(sample_fact());
        let mut duplicate = sample_fact();
        duplicate.confidence = 0.5;
        wm.add_fact(duplicate);
        assert_eq!(wm.len(), 1);
        assert_eq!(wm.facts()[0].confidence, 1.0);
    }

    #[test]
    fn add_fact_keeps_higher_confidence() {
        let mut wm = WorkingMemory::new();
        let mut low_conf = sample_fact();
        low_conf.confidence = 0.3;
        wm.add_fact(low_conf);
        wm.add_fact(sample_fact());
        assert_eq!(wm.facts()[0].confidence, 1.0);
    }

    #[test]
    fn query_finds_matching_facts() {
        let mut wm = WorkingMemory::new();
        wm.add_fact(sample_fact());
        let pattern = Pattern::new(
            PatternTerm::Bound(Value::Symbol("TypeScript".into())),
            PatternTerm::Bound(Value::Symbol("is".into())),
            PatternTerm::Variable("X".into()),
        );
        let results = wm.query(&pattern);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].object, Value::Text(_)));
    }

    #[test]
    fn query_conjunctive_joins_patterns() {
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

        let patterns = vec![
            Pattern::new(
                PatternTerm::Bound(Value::Symbol("TypeScript".into())),
                PatternTerm::Bound(Value::Symbol("is".into())),
                PatternTerm::Variable("Y".into()),
            ),
            Pattern::new(
                PatternTerm::Variable("Y".into()),
                PatternTerm::Bound(Value::Symbol("has".into())),
                PatternTerm::Variable("Z".into()),
            ),
        ];

        let solutions = wm.query_conjunctive(&patterns);
        assert_eq!(solutions.len(), 1);
        assert_eq!(
            solutions[0].get("Y"),
            Some(&Value::Symbol("programming language".into()))
        );
        assert_eq!(solutions[0].get("Z"), Some(&Value::Symbol("syntax".into())));
    }
}
