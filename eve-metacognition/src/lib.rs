// SPDX-License-Identifier: Apache-2.0
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
