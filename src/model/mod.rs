//! Wiring for the upstream `deepseek-v4-flash` model.
//!
//! Re-exports the public surface of [`client`], [`constants`], and [`error`]
//! so callers can write `use novelagent::model::...` without nesting.

pub mod client;
pub mod constants;
pub mod error;

pub use client::build_agent_builder;
pub use constants::{DEEPSEEK_V4_FLASH_MODEL_ID, OPENCODE_GO_API_KEY_ENV, OPENCODE_GO_BASE_URL};
pub use error::{ModelError, Result};
