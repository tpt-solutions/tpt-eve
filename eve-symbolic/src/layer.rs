use crate::inference::{Inference, InferenceEngine};
use crate::qa::question_to_pattern;
use crate::rule::Rule;
use crate::working_memory::WorkingMemory;
use eve_core::{Fact, Pattern};

/// Facade providing the SymbolicReasoningLayer interface.
pub struct SymbolicReasoningLayer {
    working_memory: WorkingMemory,
    rules: Vec<Rule>,
    inference_engine: InferenceEngine,
}

impl SymbolicReasoningLayer {
    pub fn new() -> Self {
        SymbolicReasoningLayer {
            working_memory: WorkingMemory::new(),
            rules: Vec::new(),
            inference_engine: InferenceEngine::new(),
        }
    }

    /// Add a fact to working memory.
    pub fn add_fact(&mut self, fact: Fact) {
        self.working_memory.add_fact(fact);
    }

    /// Query working memory.
    pub fn query(&self, pattern: &Pattern) -> Vec<Fact> {
        self.working_memory.query(pattern)
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Run forward-chaining inference.
    pub fn infer(&mut self) -> Vec<Inference> {
        self.inference_engine
            .forward_chain(&mut self.working_memory, &self.rules)
    }

    /// Check for contradictions.
    pub fn check_consistency(&self) -> Vec<crate::consistency::Contradiction> {
        crate::consistency::ConsistencyChecker::new().check(self.working_memory.facts())
    }

    /// Answer a "what is" question.
    pub fn ask(&self, question: &str) -> Option<Fact> {
        let pattern = question_to_pattern(question);
        self.working_memory.query(&pattern).into_iter().next()
    }

    /// Get the underlying working memory.
    pub fn working_memory(&self) -> &WorkingMemory {
        &self.working_memory
    }

    /// Get facts.
    pub fn facts(&self) -> &[Fact] {
        self.working_memory.facts()
    }
}

impl Default for SymbolicReasoningLayer {
    fn default() -> Self {
        Self::new()
    }
}
