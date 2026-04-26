// src/integration/mod.rs
//! Host integration helpers.
//!
//! This module groups adapters intended for application-level wiring
//! (for example focus managers, page routers, or orchestrators).

#[cfg(feature = "crossterm")]
pub mod crossterm_input;

pub mod focus_handoff;
