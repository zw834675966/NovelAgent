//! Typed errors for model construction.

use thiserror::Error;

/// Result alias for model construction helpers.
pub type Result<T> = std::result::Result<T, ModelError>;

/// Errors raised while constructing the upstream model client.
#[derive(Debug, Error)]
pub enum ModelError {
    /// The required API key environment variable was not set.
    #[error("environment variable `{0}` is not set")]
    MissingApiKey(&'static str),

    /// The underlying HTTP client builder rejected the configuration.
    #[error("failed to build opencode go client: {0}")]
    ClientBuild(String),
}
