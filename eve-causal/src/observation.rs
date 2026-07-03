// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A single observed data point: a mapping from variable names to numeric values.
///
/// For example, one row in a study-hours / exam-score dataset:
/// `Observation { variables: {"studyHours" -> 5.0, "examScore" -> 85.0} }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub variables: HashMap<String, f64>,
}

impl Observation {
    pub fn new(variables: HashMap<String, f64>) -> Self {
        Observation { variables }
    }

    /// Convenience constructor from an iterator of `(name, value)` pairs.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (impl Into<String>, f64)>) -> Self {
        Observation {
            variables: pairs.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }

    pub fn get(&self, var: &str) -> Option<f64> {
        self.variables.get(var).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pairs_roundtrip() {
        let obs = Observation::from_pairs([("x", 1.0), ("y", 2.0)]);
        assert_eq!(obs.get("x"), Some(1.0));
        assert_eq!(obs.get("y"), Some(2.0));
        assert_eq!(obs.get("z"), None);
    }
}
