// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::fact::Value;
use serde::{Deserialize, Serialize};

/// One slot of a [`Pattern`]: either bound to a concrete value or an
/// unbound variable to be solved for during unification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternTerm {
    Bound(Value),
    Variable(String),
}

/// A triple pattern used for querying working memory. Any combination of
/// slots may be bound or variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    pub subject: PatternTerm,
    pub predicate: PatternTerm,
    pub object: PatternTerm,
}

impl Pattern {
    pub fn new(subject: PatternTerm, predicate: PatternTerm, object: PatternTerm) -> Self {
        Pattern {
            subject,
            predicate,
            object,
        }
    }
}
