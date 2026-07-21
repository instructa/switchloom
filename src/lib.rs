//! Standalone Switchloom policy compiler and repository lifecycle library.

pub mod cli;
pub mod config;
pub mod contracts;
pub mod error;
pub mod evidence;
pub mod hosts;
pub mod integrations;
pub mod lifecycle;
pub mod registry;
pub mod routing;

pub use config::*;
pub use contracts::*;
pub use error::{Error, Result};
pub use evidence::*;
pub use hosts::*;
pub use lifecycle::*;
pub use registry::*;
pub use routing::*;

#[cfg(test)]
#[path = "tests/architecture.rs"]
mod architecture_tests;
