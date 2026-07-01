use eve_core::{Fact, Pattern, PatternTerm, Value};
use eve_dsl::{QueryDef, RuleDef, Term, Triple};

/// Convert a DSL Term to a core Value.
pub fn term_to_value(term: &Term) -> Value {
    match term {
        Term::Ident(s) => Value::Symbol(s.clone()),
        Term::Literal(s) => Value::Text(s.clone()),
        Term::Var(_) => panic!("Variables cannot be converted to Value directly"),
    }
}

/// Convert a DSL Triple to a core Fact (for asserted facts).
pub fn triple_to_fact(triple: &Triple) -> Fact {
    Fact::asserted(term_to_value(&triple.subject), term_to_value(&triple.predicate), term_to_value(&triple.object))
}

/// Convert a DSL Triple to a Pattern for querying.
pub fn triple_to_pattern(triple: &Triple) -> Pattern {
    Pattern::new(term_to_pattern_term(&triple.subject), term_to_pattern_term(&triple.predicate), term_to_pattern_term(&triple.object))
}

/// Convert a DSL Term to a PatternTerm.
pub fn term_to_pattern_term(term: &Term) -> PatternTerm {
    match term {
        Term::Ident(s) => PatternTerm::Bound(Value::Symbol(s.clone())),
        Term::Literal(s) => PatternTerm::Bound(Value::Text(s.clone())),
        Term::Var(s) => PatternTerm::Variable(s.clone()),
    }
}

/// Convert a DSL RuleDef to runtime representation (stores terms as-is for matching).
pub fn rule_to_runtime(rule: &RuleDef) -> (Vec<(PatternTerm, PatternTerm, PatternTerm)>, (PatternTerm, PatternTerm, PatternTerm)) {
    let conditions: Vec<(PatternTerm, PatternTerm, PatternTerm)> = rule
        .conditions
        .iter()
        .map(|t| (term_to_pattern_term(&t.subject), term_to_pattern_term(&t.predicate), term_to_pattern_term(&t.object)))
        .collect();
    let conclusion = (
        term_to_pattern_term(&rule.conclusion.subject),
        term_to_pattern_term(&rule.conclusion.predicate),
        term_to_pattern_term(&rule.conclusion.object),
    );
    (conditions, conclusion)
}

/// Convert a DSL QueryDef to a vector of Patterns.
pub fn query_to_patterns(query: &QueryDef) -> Vec<Pattern> {
    query.clauses.iter().map(triple_to_pattern).collect()
}