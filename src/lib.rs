//! Codex capability decisions with TypeSafe/Jev and cleanup of existing installations.

pub mod catalog;
pub mod cleanup;
pub mod cli;
pub mod decision;
mod digest;
pub mod error;
pub mod handoff;
pub mod typesafe;

pub use cleanup::{LifecycleReport, status_repository, uninstall_repository};
pub use error::{Error, Result};
