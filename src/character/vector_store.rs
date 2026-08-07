//! `LanceDB`-backed semantic memory index (Phase 4b).
//!
//! Pipeline: memory texts → Cohere `search_document` → `LanceDB` table under
//! `data/lancedb/{slug}/` → query via `search_query` + hybrid re-rank:
//! `score = α·recency + β·importance + γ·cosine_relevance`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use arrow_array::types::Float64Type;
use arrow_array::{
    ArrayRef, FixedSizeListArray, RecordBatch, StringArray, UInt8Array, UInt64Array,
};
use rig::embeddings::embed::{Embed, EmbedError, TextEmbedder};
use rig::embeddings::{Embedding, EmbeddingModel as _, EmbeddingsBuilder};
use rig::lancedb::{LanceDbVectorIndex, SearchParams, SearchType};
use rig::vector_store::VectorStoreIndex;
use rig::vector_store::request::VectorSearchRequest;
use serde::{Deserialize, Serialize};

use super::embed::{
    COHERE_EMBED_DIMS, build_document_embedding_model, build_query_embedding_model,
};
use super::error::{CharacterError, Result};
use super::memory::{MemoryEntry, MemoryKind, MemoryStream};

/// Default root for `LanceDB` databases (gitignored under `/data/`).
pub const DEFAULT_LANCEDB_DIR: &str = "data/lancedb";

/// Table name inside each character database.
pub const MEMORY_TABLE_NAME: &str = "memory";

/// Default hybrid weights (semantic-first).
pub const DEFAULT_ALPHA: f64 = 0.2;
pub const DEFAULT_BETA: f64 = 0.3;
pub const DEFAULT_GAMMA: f64 = 0.5;

/// One document row stored in `LanceDB` (also used for search deserialization).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryDoc {
    /// Same id as [`MemoryEntry::id`].
    pub id: String,
    /// Text embedded with Cohere document input type.
    pub text: String,
    /// Unix timestamp (seconds).
    pub ts: u64,
    /// Importance 1..=10.
    pub importance: u8,
    /// `seed` / `observation` / `reflection`.
    pub kind: String,
}

// Manual `Embed` — avoid `#[derive(Embed)]` clashing with this module's `Result` alias.
impl Embed for MemoryDoc {
    fn embed(&self, embedder: &mut TextEmbedder) -> std::result::Result<(), EmbedError> {
        embedder.embed(self.text.clone());
        Ok(())
    }
}

/// Hybrid-ranked hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridHit {
    /// Combined score (higher is better).
    pub score: f64,
    /// Cosine relevance in `0..=1` (1 = identical direction).
    pub cosine_relevance: f64,
    /// Recency component in `0..=1`.
    pub recency: f64,
    /// Importance / 10.
    pub importance_norm: f64,
    /// `LanceDB` distance (Cosine metric: lower is closer).
    pub distance: f64,
    /// Matched document.
    pub doc: MemoryDoc,
}

/// Weights + limits for hybrid search (keeps call sites short).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HybridSearchOpts {
    /// Max results after re-rank.
    pub k: usize,
    /// Weight on recency.
    pub alpha: f64,
    /// Weight on importance.
    pub beta: f64,
    /// Weight on cosine relevance.
    pub gamma: f64,
    /// Optional fixed "now" for recency (Unix seconds); `None` = wall clock.
    pub now: Option<u64>,
}

impl Default for HybridSearchOpts {
    fn default() -> Self {
        Self {
            k: 5,
            alpha: DEFAULT_ALPHA,
            beta: DEFAULT_BETA,
            gamma: DEFAULT_GAMMA,
            now: None,
        }
    }
}

/// Pure hybrid score: `α·recency + β·importance_norm + γ·cosine_relevance`.
#[must_use]
pub fn hybrid_score(
    alpha: f64,
    beta: f64,
    gamma: f64,
    recency: f64,
    importance_norm: f64,
    cosine_relevance: f64,
) -> f64 {
    alpha * recency + beta * importance_norm + gamma * cosine_relevance
}

/// Recency in `0..=1` with ~24h soft half-life (same curve as memory stream).
#[must_use]
pub fn recency_score(now: u64, ts: u64) -> f64 {
    let age_secs = now.saturating_sub(ts).min(u64::from(u32::MAX));
    let age_u32 = u32::try_from(age_secs).unwrap_or(u32::MAX);
    let age_hours = f64::from(age_u32) / 3600.0;
    (-age_hours / 24.0).exp()
}

/// Convert `LanceDB` Cosine **distance** to relevance in `0..=1`.
///
/// `LanceDB` Cosine distance is `1 - cosine_similarity` for unit vectors.
#[must_use]
pub fn cosine_relevance_from_distance(distance: f64) -> f64 {
    (1.0 - distance).clamp(0.0, 1.0)
}

impl MemoryDoc {
    /// Build from a stream entry.
    #[must_use]
    pub fn from_entry(entry: &MemoryEntry) -> Self {
        Self {
            id: entry.id.clone(),
            text: entry.text.clone(),
            ts: entry.ts,
            importance: entry.importance,
            kind: kind_str(entry.kind).to_owned(),
        }
    }
}

fn kind_str(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Seed => "seed",
        MemoryKind::Observation => "observation",
        MemoryKind::Reflection => "reflection",
    }
}

/// Sanitize a character name into a filesystem-safe database directory slug.
#[must_use]
pub fn character_slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '_'
            } else {
                // Keep CJK and other letters; drop path separators only.
                if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                    '_'
                } else {
                    c
                }
            }
        })
        .collect();
    let s = s.trim_matches('_');
    if s.is_empty() {
        "character".to_owned()
    } else {
        s.to_owned()
    }
}

/// Absolute (or relative) path for a character's `LanceDB` directory.
#[must_use]
pub fn db_path_for(root: impl AsRef<Path>, character_name: &str) -> PathBuf {
    root.as_ref().join(character_slug(character_name))
}

/// Convert embedded docs into a single Arrow [`RecordBatch`] for `LanceDB`.
///
/// # Errors
///
/// Arrow schema / array construction failures.
pub fn memory_docs_to_record_batch(
    records: Vec<(MemoryDoc, rig::OneOrMany<Embedding>)>,
    dims: usize,
) -> std::result::Result<RecordBatch, lancedb::arrow::arrow_schema::ArrowError> {
    let dims_i32 = i32::try_from(dims).map_err(|_| {
        lancedb::arrow::arrow_schema::ArrowError::InvalidArgumentError(format!(
            "embedding dims out of i32 range: {dims}"
        ))
    })?;

    let id = StringArray::from_iter_values(records.iter().map(|(d, _)| d.id.as_str()));
    let text = StringArray::from_iter_values(records.iter().map(|(d, _)| d.text.as_str()));
    let ts = UInt64Array::from_iter_values(records.iter().map(|(d, _)| d.ts));
    let importance = UInt8Array::from_iter_values(records.iter().map(|(d, _)| d.importance));
    let kind = StringArray::from_iter_values(records.iter().map(|(d, _)| d.kind.as_str()));

    let embedding = FixedSizeListArray::from_iter_primitive::<Float64Type, _, _>(
        records.into_iter().map(|(_, embeddings)| {
            Some(
                embeddings
                    .first()
                    .vec
                    .into_iter()
                    .map(Some)
                    .collect::<Vec<_>>(),
            )
        }),
        dims_i32,
    );

    RecordBatch::try_from_iter(vec![
        ("id", Arc::new(id) as ArrayRef),
        ("text", Arc::new(text) as ArrayRef),
        ("ts", Arc::new(ts) as ArrayRef),
        ("importance", Arc::new(importance) as ArrayRef),
        ("kind", Arc::new(kind) as ArrayRef),
        ("embedding", Arc::new(embedding) as ArrayRef),
    ])
}

/// Index all entries of a memory stream into `LanceDB` (create or replace table).
///
/// Uses Cohere document embeddings. Requires `COHERE_API_KEY`.
///
/// # Errors
///
/// Missing key, embed failure, or `LanceDB` I/O.
pub async fn index_memory_stream(
    stream: &MemoryStream,
    db_root: impl AsRef<Path>,
) -> Result<usize> {
    if stream.entries.is_empty() {
        return Ok(0);
    }

    let docs: Vec<MemoryDoc> = stream.entries.iter().map(MemoryDoc::from_entry).collect();
    let model = build_document_embedding_model()?;
    let dims = model.ndims();
    if dims != COHERE_EMBED_DIMS {
        return Err(CharacterError::Embed(format!(
            "unexpected embedding dims {dims}, expected {COHERE_EMBED_DIMS}"
        )));
    }

    let embeddings = EmbeddingsBuilder::new(model)
        .documents(docs)
        .map_err(|e| CharacterError::Embed(e.to_string()))?
        .build()
        .await
        .map_err(|e| CharacterError::Embed(e.to_string()))?;

    let batch = memory_docs_to_record_batch(embeddings, dims)
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;

    let path = db_path_for(db_root, &stream.character_name);
    std::fs::create_dir_all(&path).map_err(|e| {
        CharacterError::VectorStore(format!("create lancedb dir {}: {e}", path.display()))
    })?;

    let uri = path
        .to_str()
        .ok_or_else(|| CharacterError::VectorStore("lancedb path is not valid UTF-8".to_owned()))?;

    let db = lancedb::connect(uri)
        .execute()
        .await
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;

    // Replace table so re-index is idempotent for v1.
    let names = db
        .table_names()
        .execute()
        .await
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;
    if names.iter().any(|n| n == MEMORY_TABLE_NAME) {
        db.drop_table(MEMORY_TABLE_NAME, &[])
            .await
            .map_err(|e| CharacterError::VectorStore(e.to_string()))?;
    }

    let n = batch.num_rows();
    db.create_table(MEMORY_TABLE_NAME, vec![batch])
        .execute()
        .await
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;

    Ok(n)
}

/// Search indexed memories with hybrid re-ranking.
///
/// 1. Embed query with Cohere `search_query`.
/// 2. `LanceDB` ENN (flat) top-k by cosine.
/// 3. Re-rank with [`hybrid_score`].
///
/// # Errors
///
/// Missing key, embed failure, or `LanceDB` I/O.
pub async fn search_memory_hybrid(
    character_name: &str,
    query: &str,
    db_root: impl AsRef<Path>,
    opts: HybridSearchOpts,
) -> Result<Vec<HybridHit>> {
    let query = query.trim();
    if query.is_empty() {
        return Err(CharacterError::Validation(
            "memory search query must not be empty".to_owned(),
        ));
    }
    if opts.k == 0 {
        return Ok(Vec::new());
    }

    let path = db_path_for(db_root, character_name);
    let uri = path
        .to_str()
        .ok_or_else(|| CharacterError::VectorStore("lancedb path is not valid UTF-8".to_owned()))?;

    let db = lancedb::connect(uri)
        .execute()
        .await
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;

    let table = db
        .open_table(MEMORY_TABLE_NAME)
        .execute()
        .await
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;

    let model = build_query_embedding_model()?;
    // Flat (ENN) — correct for small seed tables; no IVF-PQ needed.
    let search_params = SearchParams::default()
        .distance_type(lancedb::DistanceType::Cosine)
        .search_type(SearchType::Flat)
        .column("embedding");

    let index = LanceDbVectorIndex::new(table, model, "id", search_params)
        .await
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;

    // Over-fetch a bit so hybrid re-rank can reorder within a small pool.
    let samples =
        u64::try_from(opts.k.saturating_mul(2).max(opts.k)).unwrap_or(u64::from(u32::MAX));
    let req = VectorSearchRequest::builder()
        .query(query)
        .samples(samples)
        .build();

    let raw = index
        .top_n::<MemoryDoc>(req)
        .await
        .map_err(|e| CharacterError::VectorStore(e.to_string()))?;

    let now = opts.now.unwrap_or_else(now_unix);
    let mut hits: Vec<HybridHit> = raw
        .into_iter()
        .map(|(distance, _id, doc)| {
            let cosine_relevance = cosine_relevance_from_distance(distance);
            let recency = recency_score(now, doc.ts);
            let importance_norm = f64::from(doc.importance) / 10.0;
            let score = hybrid_score(
                opts.alpha,
                opts.beta,
                opts.gamma,
                recency,
                importance_norm,
                cosine_relevance,
            );
            HybridHit {
                score,
                cosine_relevance,
                recency,
                importance_norm,
                distance,
                doc,
            }
        })
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(opts.k);
    Ok(hits)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_prefers_high_cosine_when_gamma_dominates() {
        let low_cos = hybrid_score(0.2, 0.3, 0.5, 1.0, 1.0, 0.1);
        let high_cos = hybrid_score(0.2, 0.3, 0.5, 0.5, 0.5, 0.95);
        assert!(high_cos > low_cos);
    }

    #[test]
    fn hybrid_prefers_recent_when_alpha_dominates() {
        let old = hybrid_score(1.0, 0.0, 0.0, 0.1, 1.0, 1.0);
        let new = hybrid_score(1.0, 0.0, 0.0, 0.9, 0.0, 0.0);
        assert!(new > old);
    }

    #[test]
    fn hybrid_prefers_important_when_beta_dominates() {
        let low = hybrid_score(0.0, 1.0, 0.0, 1.0, 0.2, 1.0);
        let high = hybrid_score(0.0, 1.0, 0.0, 0.0, 0.9, 0.0);
        assert!(high > low);
    }

    #[test]
    fn cosine_relevance_from_distance_bounds() {
        assert_eq!(cosine_relevance_from_distance(0.0), 1.0);
        assert_eq!(cosine_relevance_from_distance(1.0), 0.0);
        assert_eq!(cosine_relevance_from_distance(0.25), 0.75);
        assert_eq!(cosine_relevance_from_distance(-0.1), 1.0);
        assert_eq!(cosine_relevance_from_distance(2.0), 0.0);
    }

    #[test]
    fn recency_decreases_with_age() {
        let now = 1_000_000_u64;
        let fresh = recency_score(now, now);
        let day_old = recency_score(now, now - 86_400);
        assert!((fresh - 1.0).abs() < 1e-9);
        assert!(day_old < fresh);
        assert!(day_old > 0.3); // ~0.5 at 24h half-life soft
    }

    #[test]
    fn character_slug_strips_path_chars() {
        assert_eq!(character_slug("苏晚"), "苏晚");
        assert_eq!(character_slug("a/b:c"), "a_b_c");
        assert_eq!(character_slug("   "), "character");
    }

    #[test]
    fn db_path_joins_slug() {
        let p = db_path_for("data/lancedb", "苏晚");
        assert!(p.ends_with("苏晚"));
    }

    #[test]
    fn memory_doc_from_entry() {
        let e = MemoryEntry {
            id: "0".into(),
            ts: 42,
            kind: MemoryKind::Seed,
            text: "雨夜便利店".into(),
            importance: 8,
        };
        let d = MemoryDoc::from_entry(&e);
        assert_eq!(d.id, "0");
        assert_eq!(d.kind, "seed");
        assert_eq!(d.importance, 8);
    }

    #[test]
    fn memory_doc_kind_maps_all_variants() {
        for (kind, expected) in [
            (MemoryKind::Seed, "seed"),
            (MemoryKind::Observation, "observation"),
            (MemoryKind::Reflection, "reflection"),
        ] {
            let e = MemoryEntry {
                id: "0".into(),
                ts: 1,
                kind,
                text: "t".into(),
                importance: 1,
            };
            assert_eq!(MemoryDoc::from_entry(&e).kind, expected);
        }
    }

    #[test]
    fn memory_docs_to_record_batch_roundtrip() {
        let records = vec![
            (
                MemoryDoc {
                    id: "0".into(),
                    text: "雨夜便利店".into(),
                    ts: 100,
                    importance: 8,
                    kind: "seed".into(),
                },
                rig::OneOrMany::many(vec![Embedding {
                    document: "雨夜便利店".into(),
                    vec: vec![1.0, 0.0, 0.0],
                }])
                .expect("one embedding"),
            ),
            (
                MemoryDoc {
                    id: "1".into(),
                    text: "故人".into(),
                    ts: 200,
                    importance: 5,
                    kind: "observation".into(),
                },
                rig::OneOrMany::many(vec![Embedding {
                    document: "故人".into(),
                    vec: vec![0.0, 1.0, 0.0],
                }])
                .expect("one embedding"),
            ),
        ];
        let batch = memory_docs_to_record_batch(records, 3).expect("record batch");
        assert_eq!(batch.num_rows(), 2);
        for name in ["id", "text", "ts", "importance", "kind", "embedding"] {
            assert!(
                batch.schema().field_with_name(name).is_ok(),
                "missing column {name}"
            );
        }
    }

    #[test]
    fn memory_docs_to_record_batch_rejects_huge_dims() {
        let err = memory_docs_to_record_batch(Vec::new(), usize::MAX).expect_err("dims overflow");
        assert!(err.to_string().contains("out of i32 range"));
    }

    #[tokio::test]
    async fn index_memory_stream_empty_returns_zero_without_key() {
        let stream = MemoryStream::new("empty_test");
        let n = index_memory_stream(&stream, "data/lancedb")
            .await
            .expect("empty stream indexes to zero");
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn search_rejects_empty_query_without_key() {
        let err = search_memory_hybrid("x", "   ", "data/lancedb", HybridSearchOpts::default())
            .await
            .expect_err("blank query rejected");
        assert!(matches!(err, CharacterError::Validation(_)));
    }

    #[tokio::test]
    async fn search_k_zero_returns_empty_without_key() {
        let opts = HybridSearchOpts {
            k: 0,
            ..HybridSearchOpts::default()
        };
        let hits = search_memory_hybrid("x", "雨夜", "data/lancedb", opts)
            .await
            .expect("k=0 short-circuits");
        assert!(hits.is_empty());
    }

    /// Live: write ≥3 Chinese memories → query hits related text.
    ///
    /// ```text
    /// cargo test live_lancedb_memory_zh -- --ignored --nocapture
    /// ```
    /// Requires `COHERE_API_KEY` and network. Uses a unique subdir under `data/lancedb/`.
    #[tokio::test]
    #[ignore = "network + COHERE_API_KEY + LanceDB"]
    async fn live_lancedb_memory_zh_checkpoint_4b() {
        let _ = dotenvy::dotenv();

        let mut stream = MemoryStream::new("checkpoint_4b_苏晚");
        let now = 1_700_000_000_u64;
        stream
            .append(
                MemoryKind::Seed,
                "角色记忆：她在雨夜的便利店遇见了故人。",
                9,
                Some(now),
            )
            .unwrap();
        stream
            .append(
                MemoryKind::Observation,
                "无关文本：今天股市大涨，指数创新高。",
                3,
                Some(now - 100),
            )
            .unwrap();
        stream
            .append(
                MemoryKind::Seed,
                "她在便利店值夜班，雨声敲着玻璃。",
                8,
                Some(now),
            )
            .unwrap();
        stream
            .append(
                MemoryKind::Observation,
                "厨房里烤了一盘曲奇饼干。",
                4,
                Some(now - 50),
            )
            .unwrap();

        assert!(stream.entries.len() >= 3);

        let root = PathBuf::from("data/lancedb");
        let n = index_memory_stream(&stream, &root)
            .await
            .expect("index memory into LanceDB");
        assert_eq!(n, stream.entries.len());

        let opts = HybridSearchOpts {
            k: 3,
            now: Some(now),
            ..HybridSearchOpts::default()
        };
        let hits = search_memory_hybrid(&stream.character_name, "雨夜便利店的故人", &root, opts)
            .await
            .expect("hybrid search");

        assert!(!hits.is_empty(), "expected at least one hit");
        eprintln!("top hits:");
        for (i, h) in hits.iter().enumerate() {
            eprintln!(
                "  [{i}] score={:.4} cos={:.4} dist={:.4} text={}",
                h.score, h.cosine_relevance, h.distance, h.doc.text
            );
        }

        let top_text = &hits[0].doc.text;
        assert!(
            top_text.contains("雨夜") || top_text.contains("便利店") || top_text.contains("故人"),
            "top hit should be rain/convenience-store related, got: {top_text}"
        );

        // Unrelated market text should not rank first under a rain-store query.
        assert!(
            !hits[0].doc.text.contains("股市"),
            "unrelated market memory should not be top-1"
        );
    }
}
