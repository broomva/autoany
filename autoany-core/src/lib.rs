//! # Autoany Core
//!
//! EGRI microkernel — Evaluator-Governed Recursive Improvement runtime.
//!
//! Provides the reusable loop substrate: trait abstractions for executors,
//! evaluators, selectors, and mutation proposers, plus the loop orchestrator,
//! ledger, budget enforcement, and promotion controller.
//!
//! ## Architecture
//!
//! ```text
//! Proposer → Executor → Evaluator → Selector → PromotionController
//!     ↑                                              ↓
//!     └──────────── Ledger ←─────────────────────────┘
//! ```

pub mod budget;
pub mod constraint;
pub mod dead_ends;
pub mod error;
pub mod evaluator;
pub mod executor;
pub mod inheritance;
pub mod ledger;
pub mod loop_engine;
pub mod promotion;
pub mod proposer;
pub mod selector;
pub mod spec;
pub mod stagnation;
pub mod strategy;
pub mod types;

pub use error::{EgriError, Result};
pub use loop_engine::EgriLoop;
pub use types::*;
