// SPDX-License-Identifier: MIT OR Apache-2.0
//! Meta-cognition layer for TPT Eve.
//!
//! Confidence tracking across fact/inference/belief types, knowledge-gap
//! detection, strategic learning-priority decisions, understanding-level
//! monitoring, and clarification-request generation — all behind
//! [`MetaCognitionEngine`].
mod confidence;
mod knowledge_gap;
mod layer;
mod learning_priority;
mod monitoring;

pub use confidence::{ConfidenceRecord, ConfidenceTracker};
pub use knowledge_gap::{GapType, KnowledgeGap, KnowledgeGapDetector};
pub use layer::MetaCognitionEngine;
pub use learning_priority::{LearningPrioritizer, LearningPriority};
pub use monitoring::{ClarificationRequest, UnderstandingLevel, UnderstandingMonitor};
