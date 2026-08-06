//! Runtime environment bootstrap for the binary.

use anyhow::{Context, Result};

use crate::model::OPENCODE_GO_API_KEY_ENV;

/// Load `.env` (best-effort) and fail fast if the API key is missing.
///
/// `.env` load is non-fatal: production deploys inject env vars through other
/// channels and the file is not always present. The API key check then
/// guarantees we never reach the agent layer with a half-configured process.
///
/// # Errors
///
/// Returns an error if `OPENCODE_GO_API_KEY` is not set after loading `.env`.
pub fn load_environment() -> Result<()> {
    let _ = dotenvy::dotenv();

    std::env::var(OPENCODE_GO_API_KEY_ENV).with_context(|| {
        format!("set {OPENCODE_GO_API_KEY_ENV} with your OpenCode Go API key before running")
    })?;

    Ok(())
}
