//! Crate root for the `novelagent` workspace.
//!
//! Re-exports the public surface of [`app`], [`character`], [`model`], and
//! [`web`] so downstream callers (the binary in `main.rs`, future integration
//! tests) can write `use novelagent::...` without nesting.

pub mod app;
pub mod character;
pub mod model;
pub mod web;

pub use app::{
    bootstrap, character_chat, character_create, character_list, load_environment, prompt_message,
    run, run_readiness_check,
};
pub use character::{
    ArtifactPaths, COHERE_API_KEY_ENV, COHERE_EMBED_MODEL_ID, CardArtifacts, CharacterError,
    CharacterSummary, CreateCardOutcome, DEFAULT_CARD_LOCALE, DEFAULT_CHARACTERS_DIR,
    DEFAULT_LANCEDB_DIR, HybridHit, KnowledgeGraph, MemoryStream, PromptPack, TavernCardV2,
    assemble_prompt_pack, create_card, create_card_live, format_create_summary,
    index_memory_stream, list_characters, load_card_by_slug, search_memory_hybrid,
    seed_card_artifacts, validate_card, write_create_outcome,
};
pub use model::{
    DEEPSEEK_V4_FLASH_MODEL_ID, ModelError, OPENCODE_GO_API_KEY_ENV, OPENCODE_GO_BASE_URL, Result,
    build_agent_builder,
};
