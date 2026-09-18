//! Codex capability decisions with TypeSafe/Jev and cleanup of existing installations.

pub mod catalog;
#[cfg(not(target_arch = "wasm32"))]
pub mod cleanup;
#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
pub mod decision;
#[cfg(not(target_arch = "wasm32"))]
mod digest;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod handoff;
pub mod typesafe;

#[cfg(not(target_arch = "wasm32"))]
pub use cleanup::{LifecycleReport, status_repository, uninstall_repository};
pub use error::{Error, Result};

#[cfg(target_arch = "wasm32")]
mod web;
