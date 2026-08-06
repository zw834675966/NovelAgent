//! Typed errors for the character-card domain.

use thiserror::Error;

/// Result alias for character-card helpers.
pub type Result<T> = std::result::Result<T, CharacterError>;

/// Errors raised while validating or building character cards / embeddings.
#[derive(Debug, Error)]
pub enum CharacterError {
    /// Hard schema / constraint validation failed.
    #[error("character card validation failed: {0}")]
    Validation(String),

    /// Concept / user input empty after trim.
    #[error("concept must not be empty")]
    EmptyConcept,

    /// Required environment variable missing (e.g. Cohere key).
    #[error("environment variable `{0}` is not set")]
    MissingApiKey(&'static str),

    /// Upstream client construction failed.
    #[error("failed to build client: {0}")]
    ClientBuild(String),

    /// LLM completion failed (network / provider / scripted mock exhausted).
    #[error("llm call failed: {0}")]
    Llm(String),

    /// Failed to parse LLM output into card or critique JSON.
    #[error("failed to parse llm json: {0}")]
    Parse(String),

    /// JSON (de)serialization failed.
    #[error("character card json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Embedding API / model call failed.
    #[error("embedding failed: {0}")]
    Embed(String),

    /// Vector store (`LanceDB` / index) I/O or query failed.
    #[error("vector store error: {0}")]
    VectorStore(String),

    /// Filesystem I/O under `data/characters/` (or caller path) failed.
    #[error("io error: {0}")]
    Io(String),
}
