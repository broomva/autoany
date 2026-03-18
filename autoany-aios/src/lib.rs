//! Arcan runtime adapter for autoany EGRI loops.
//!
//! Connects EGRI loops to the Arcan agent runtime for execution,
//! replacing local command execution with Arcan HTTP session API calls.

pub mod budget;
pub mod executor;
