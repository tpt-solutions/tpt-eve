use eve_core::{Fact, PatternTerm, Value};
use serde::{Deserialize, Serialize};

/// A rule for forward-chaining inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub name: Option<String>,
    pub conditions: Vec<(PatternTerm, PatternTerm, PatternTerm)>,
    pub conclusion: (PatternTerm, PatternTerm, PatternTerm),
}

impl Rule {
    pub fn new(
        name: Option<String>,
        conditions: Vec<(PatternTerm, PatternTerm, PatternTerm)>,
        conclusion: (PatternTerm, PatternTerm, PatternTerm),
    ) -> Self {
        Rule { name, conditions, conclusion }
    }

    /// Check if this rule's conditions are satisfied by the given bindings.
    pub fn check_conditions(
        &self,
        facts: &[Fact],
        patterns: &[(PatternTerm, PatternTerm, PatternTerm)],
    ) -> Vec<Vec<(String, Value)>> {
        if patterns.is_empty() {
            return vec![vec![]];
        }

        let mut solutions: Vec<Vec<(String, Value)>> = Vec::new();
        self.check_conditions_recursive(facts, patterns, 0, &mut vec![], &mut solutions);
        solutions
    }

    fn check_conditions_recursive(
        &self,
        facts: &[Fact],
        patterns: &[(PatternTerm, PatternTerm, PatternTerm)],
        idx: usize,
        bindings: &mut Vec<(String, Value)>,
        solutions: &mut Vec<Vec<(String, Value)>>,
    ) {
        if idx >= patterns.len() {
            solutions.push(bindings.clone());
            return;
        }

        let (subject_pt, predicate_pt, object_pt) = &patterns[idx];
        for fact in facts {
            if self.matches_fact_triple(fact, subject_pt, predicate_pt, object_pt, bindings) {
                self.check_conditions_recursive(facts, patterns, idx + 1, bindings, solutions);
            }
        }
    }

    fn matches_fact_triple(
        &self,
        fact: &Fact,
        subject_pt: &PatternTerm,
        predicate_pt: &PatternTerm,
        object_pt: &PatternTerm,
        bindings: &[(String, Value)],
    ) -> bool {
        fn matches_term(pt: &PatternTerm, fv: &Value, bindings: &[(String, Value)]) -> bool {
            match pt {
                PatternTerm::Bound(bound) => bound == fv,
                PatternTerm::Variable(var) => {
                    bindings
                        .iter()
                        .any(|(name, val)| name == var && val == fv)
                        || true // Variable can match anything (it binds)
                }
            }
        }
        matches_term(subject_pt, &fact.subject, bindings)
            && matches_term(predicate_pt, &fact.predicate, bindings)
            && matches_term(object_pt, &fact.object, bindings)
    }

    /// Apply the rule to generate inferred facts from bindings.
    pub fn apply(&self, bindings: &[(String, Value)]) -> Option<Fact> {
        let subject = self.resolve_term(&self.conclusion.0, bindings)?;
        let predicate = self.resolve_term(&self.conclusion.1, bindings)?;
        let object = self.resolve_term(&self.conclusion.2, bindings)?;
        
        Some(Fact {
            subject,
            predicate,
            object,
            source: eve_core::FactSource::Inferred { rule_name: self.name.clone() },
            confidence: 0.9, // Inferred facts get slightly lower confidence
        })
    }

    fn resolve_term(&self, pt: &PatternTerm, bindings: &[(String, Value)]) -> Option<Value> {
        match pt {
            PatternTerm::Bound(v) => Some(v.clone()),
            PatternTerm::Variable(var) => {
                bindings
                    .iter()
                    .find(|(name, _)| name == var)
                    .map(|(_, v)| v.clone())
            }
        }
    }
}