//! Deterministic post-loop seeding: lorebook + memory stream + KG (Phase 4a).

use super::card::TavernCardV2;
use super::kg::{KnowledgeGraph, build_kg_from_card};
use super::lorebook::attach_lorebook;
use super::memory::{MemoryStream, seed_memory_from_card};

/// Sidecar artifacts produced alongside an enriched card.
#[derive(Debug, Clone, PartialEq)]
pub struct CardArtifacts {
    /// Memory stream metadata (no vectors in 4a).
    pub memory: MemoryStream,
    /// Ego knowledge graph.
    pub kg: KnowledgeGraph,
}

/// Attach lorebook to `card` and build memory + KG sidecars.
///
/// Pure / deterministic. Safe to call after hard validation.
#[must_use]
pub fn seed_card_artifacts(card: &mut TavernCardV2, ts: Option<u64>) -> CardArtifacts {
    attach_lorebook(card);
    let memory = seed_memory_from_card(card, ts);
    let kg = build_kg_from_card(card);
    CardArtifacts { memory, kg }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::character::MemoryKind;
    use crate::character::card::{
        CardExtensions, NovelAgentCharExt, RelationshipNode, TavernCardV2,
    };
    use crate::character::constraints::validate_card;

    fn rich_card() -> TavernCardV2 {
        let mut card = TavernCardV2::skeleton_zh("苏晚");
        card.data.description = "夜班店员".to_owned();
        card.data.personality = "克制".to_owned();
        card.data.scenario = "雨夜".to_owned();
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                desire: "离开".to_owned(),
                need: "被看见".to_owned(),
                weakness: "拒人千里".to_owned(),
                moral_axis: "独立 vs 依附".to_owned(),
                knowledge_bounds: "不知用户身份".to_owned(),
                relationships: vec![RelationshipNode {
                    name: "老周".to_owned(),
                    relation_type: "mentor".to_owned(),
                    defines_protagonist_how: "记得她".to_owned(),
                }],
                constraints: vec!["C-TOM".to_owned(), "C-NO-USER".to_owned()],
                ..NovelAgentCharExt::default()
            }),
        };
        card
    }

    #[test]
    fn seed_fills_book_mem_kg() {
        let mut card = rich_card();
        validate_card(&card).expect("valid before seed");
        let art = seed_card_artifacts(&mut card, Some(1_700_000_000));
        let book = card
            .data
            .character_book
            .as_ref()
            .expect("lorebook attached");
        assert!(book.entries.len() >= 3);
        assert!(art.memory.entries.len() >= 5);
        assert!(!art.kg.edges.is_empty());
        validate_card(&card).expect("still valid after seed");
    }

    /// The trio must be internally consistent: `character_name` matches
    /// across all three artifacts, and the seeded entries reference the
    /// same card content. This is the contract the sidecar files rely
    /// on for downstream consumers.
    #[test]
    fn produces_card_mem_kg_trio() {
        let mut card = rich_card();
        let art = seed_card_artifacts(&mut card, Some(1_700_000_000));

        // Lorebook attached onto card.
        let book = card
            .data
            .character_book
            .as_ref()
            .expect("lorebook attached on card");
        assert_eq!(book.name.as_deref(), Some("苏晚 的 lorebook"));

        // Memory stream bound to the same character; all seeds are Seed kind.
        assert_eq!(art.memory.character_name, "苏晚");
        assert!(art.memory.next_id > 0);
        assert!(!art.memory.entries.is_empty());
        assert!(
            art.memory
                .entries
                .iter()
                .all(|e| matches!(e.kind, MemoryKind::Seed))
        );
        // Description seed names the character.
        assert!(
            art.memory
                .entries
                .iter()
                .any(|e| e.text.starts_with("苏晚："))
        );

        // KG ego node uses the same name.
        assert_eq!(art.kg.character_name, "苏晚");
        assert!(
            art.kg
                .nodes
                .iter()
                .any(|n| n.node_type == "protagonist" && n.label == "苏晚")
        );
    }
}
