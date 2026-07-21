// SPDX-License-Identifier: MIT OR Apache-2.0
//! Abstraction layer for TPT Eve.
//!
//! Cross-domain pattern extraction ([`PatternMiner`]), analogical mapping
//! ([`AnalogyMapper`]), and knowledge transfer between domains via
//! [`AbstractionEngine`].
mod analogy;
mod layer;
mod principle;
mod transfer;

pub use analogy::{Analogy, AnalogyMapper};
pub use layer::AbstractionEngine;
pub use principle::{AbstractPrinciple, PatternMiner};
pub use transfer::transfer_knowledge;
