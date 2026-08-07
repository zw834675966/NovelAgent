//! Character-card domain: `SillyTavern` V2 schema, constraints, prompts, Cohere embed-api.
//!
//! Phase 1–5: types, validation, prompt pack, meta-agent loop, lore/memory/KG,
//! `LanceDB` hybrid search, and on-disk export under `data/characters/`.

pub mod agent;
pub mod card;
pub mod constraints;
pub mod embed;
pub mod error;
pub mod kg;
pub mod lorebook;
pub mod memory;
pub mod persist;
pub mod prompt_pack;
pub mod rubric;
pub mod seed;
pub mod vector_store;

pub use agent::{CreateCardOutcome, LlmBackend, RigLlm, create_card, create_card_live};
pub use card::{
    CardExtensions, CharDataV2, CharacterBook, EmotionBeat, LoreEntry, NovelAgentCharExt,
    RelationshipNode, TavernCardV2,
};
pub use constraints::{KNOWN_CONSTRAINT_IDS, validate_card};
pub use embed::{
    COHERE_API_KEY_ENV, COHERE_EMBED_DIMS, COHERE_EMBED_MODEL_ID, COHERE_INPUT_DOCUMENT,
    COHERE_INPUT_QUERY, DEFAULT_CARD_LOCALE, build_document_embedding_model,
    build_query_embedding_model,
};
pub use error::{CharacterError, Result};
pub use kg::{KgEdge, KgNode, KnowledgeGraph, build_kg_from_card};
pub use lorebook::{
    DEFAULT_LORE_SCAN_DEPTH, DEFAULT_LORE_TOKEN_BUDGET, MAX_LORE_ENTRIES, attach_lorebook,
    build_lorebook,
};
pub use memory::{MemoryEntry, MemoryKind, MemoryStream, seed_memory_from_card};
pub use persist::{
    ArtifactPaths, CharacterSummary, DEFAULT_CHARACTERS_DIR, DeleteOutcome, delete_character,
    format_create_summary, list_characters, load_card_by_slug, load_concept, write_create_outcome,
};
pub use prompt_pack::{
    CRITIQUE_RUBRIC, META_SYSTEM, PromptPack, REFINE, USER_CREATE, assemble_prompt_pack,
    render_critique_user, render_refine_user, render_user_create,
};
pub use rubric::{
    CritiqueFlags, CritiqueReport, DimensionScores, MAX_REFINE_ROUNDS, SCORE_THRESHOLD,
};
pub use seed::{CardArtifacts, seed_card_artifacts};
pub use vector_store::{
    DEFAULT_ALPHA, DEFAULT_BETA, DEFAULT_GAMMA, DEFAULT_LANCEDB_DIR, HybridHit, HybridSearchOpts,
    MEMORY_TABLE_NAME, MemoryDoc, character_slug, cosine_relevance_from_distance, db_path_for,
    hybrid_score, index_memory_stream, recency_score, search_memory_hybrid,
};
