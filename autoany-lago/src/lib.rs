//! Lago persistence adapter for autoany EGRI loops.
//!
//! Defines the event convention for storing EGRI trial records
//! in Lago's append-only journal using `EventKind::Custom` with
//! the `"egri."` prefix (following Autonomic's pattern).

pub mod ledger;
pub mod replay;
