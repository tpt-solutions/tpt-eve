// SPDX-License-Identifier: MIT OR Apache-2.0
use tpt_eve_core::EveError;

/// Stub for Phase 1 backtracking support. Real implementation
/// belongs in Phase 2 with causal intervention/counterfactual machinery.
pub struct BacktrackStub;

impl BacktrackStub {
    pub fn new() -> Self {
        BacktrackStub
    }

    /// Attempt to backtrack on an assumption - not supported in Phase 1.
    pub fn backtrack(&self, _assumption: &str) -> Result<(), EveError> {
        Err(EveError::NotSupported("backtrack"))
    }
}

impl Default for BacktrackStub {
    fn default() -> Self {
        Self::new()
    }
}