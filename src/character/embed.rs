//! Cohere embed-api wiring for v1 semantic memory.
//!
//! Default model: `embed-multilingual-v3.0` (Chinese + multilingual, 1024-d).
//! API key: `COHERE_API_KEY` (see `.env.example`). Secrets never appear in cards.

use std::env;

use rig::client::ProviderClient;
use rig::providers::cohere;

use super::error::{CharacterError, Result};

/// Env var read by [`rig::providers::cohere::Client::from_env`].
pub const COHERE_API_KEY_ENV: &str = "COHERE_API_KEY";

/// Default embedding model for Chinese / multilingual memory (v1).
pub const COHERE_EMBED_MODEL_ID: &str = cohere::EMBED_MULTILINGUAL_V3;

/// Dimensions for [`COHERE_EMBED_MODEL_ID`].
pub const COHERE_EMBED_DIMS: usize = 1024;

/// Cohere v3 input type when indexing documents into `LanceDB`.
pub const COHERE_INPUT_DOCUMENT: &str = "search_document";

/// Cohere v3 input type when embedding a retrieval query.
pub const COHERE_INPUT_QUERY: &str = "search_query";

/// Product default locale for generated cards and prompts.
pub const DEFAULT_CARD_LOCALE: &str = "zh-CN";

/// Build a Cohere embedding model for **document** indexing.
///
/// # Errors
///
/// Returns [`CharacterError::MissingApiKey`] or [`CharacterError::ClientBuild`].
pub fn build_document_embedding_model() -> Result<cohere::EmbeddingModel> {
    build_embedding_model(COHERE_INPUT_DOCUMENT)
}

/// Build a Cohere embedding model for **query** retrieval.
///
/// # Errors
///
/// Returns [`CharacterError::MissingApiKey`] or [`CharacterError::ClientBuild`].
pub fn build_query_embedding_model() -> Result<cohere::EmbeddingModel> {
    build_embedding_model(COHERE_INPUT_QUERY)
}

fn build_embedding_model(input_type: &str) -> Result<cohere::EmbeddingModel> {
    if env::var(COHERE_API_KEY_ENV).is_err() {
        return Err(CharacterError::MissingApiKey(COHERE_API_KEY_ENV));
    }

    let client =
        cohere::Client::from_env().map_err(|err| CharacterError::ClientBuild(err.to_string()))?;

    Ok(client.embedding_model(COHERE_EMBED_MODEL_ID, input_type))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use rig::embeddings::EmbeddingModel as _;

    #[test]
    fn model_id_is_multilingual_v3() {
        assert_eq!(COHERE_EMBED_MODEL_ID, "embed-multilingual-v3.0");
        assert_eq!(COHERE_EMBED_DIMS, 1024);
        assert_eq!(DEFAULT_CARD_LOCALE, "zh-CN");
        assert_eq!(COHERE_INPUT_DOCUMENT, "search_document");
        assert_eq!(COHERE_INPUT_QUERY, "search_query");
    }

    /// Live smoke: requires network + `COHERE_API_KEY` (e.g. from `.env`).
    ///
    /// Run: `cargo test live_cohere_embed -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "network + COHERE_API_KEY"]
    async fn live_cohere_embed_multilingual_zh() {
        let _ = dotenvy::dotenv();

        let doc_model =
            build_document_embedding_model().expect("document embedding model from env");
        let query_model = build_query_embedding_model().expect("query embedding model from env");

        assert_eq!(doc_model.ndims(), COHERE_EMBED_DIMS);
        assert_eq!(query_model.ndims(), COHERE_EMBED_DIMS);

        let docs = doc_model
            .embed_texts([
                "角色记忆：她在雨夜的便利店遇见了故人。".to_owned(),
                "无关文本：今天股市大涨。".to_owned(),
            ])
            .await
            .expect("document embed call");

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].vec.len(), COHERE_EMBED_DIMS);
        assert_eq!(docs[1].vec.len(), COHERE_EMBED_DIMS);
        assert!(
            docs[0].vec.iter().any(|x| *x != 0.0),
            "document embedding should not be all zeros"
        );

        let queries = query_model
            .embed_texts(["雨夜便利店的故人".to_owned()])
            .await
            .expect("query embed call");

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].vec.len(), COHERE_EMBED_DIMS);

        let sim_related = cosine(&docs[0].vec, &queries[0].vec);
        let sim_unrelated = cosine(&docs[1].vec, &queries[0].vec);

        eprintln!("cosine(related)   = {sim_related:.4}");
        eprintln!("cosine(unrelated) = {sim_unrelated:.4}");
        eprintln!("model             = {COHERE_EMBED_MODEL_ID}");
        eprintln!("dims              = {COHERE_EMBED_DIMS}");

        assert!(
            sim_related > sim_unrelated,
            "related Chinese memory should rank above unrelated text \
             (related={sim_related}, unrelated={sim_unrelated})"
        );
    }

    fn cosine(a: &[f64], b: &[f64]) -> f64 {
        let mut dot = 0.0;
        let mut na = 0.0;
        let mut nb = 0.0;
        for (x, y) in a.iter().zip(b.iter()) {
            dot += x * y;
            na += x * x;
            nb += y * y;
        }
        dot / (na.sqrt() * nb.sqrt())
    }
}
