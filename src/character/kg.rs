//! Minimal ego knowledge graph seed from card relationships (Phase 4a).
//!
//! Nodes/edges JSON only — no `GraphRAG` community summaries.

use serde::{Deserialize, Serialize};

use super::card::TavernCardV2;
use super::error::Result;

/// A graph node (character ego + relation targets + optional fact tags).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgNode {
    /// Stable id (`char:{name}` or `rel:{name}`).
    pub id: String,
    /// Display label.
    pub label: String,
    /// `protagonist` | `character` | `concept`.
    #[serde(rename = "type")]
    pub node_type: String,
}

/// A directed edge between nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgEdge {
    pub from: String,
    pub to: String,
    /// Relation label (ally / rival / …).
    pub relation: String,
    /// Optional free-text detail (`defines_protagonist_how`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
}

/// Ego graph for one character card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    /// Protagonist character name at seed time.
    #[serde(default)]
    pub character_name: String,
    #[serde(default)]
    pub nodes: Vec<KgNode>,
    #[serde(default)]
    pub edges: Vec<KgEdge>,
}

impl KnowledgeGraph {
    /// Pretty JSON for sidecar files under `data/characters/`.
    ///
    /// # Errors
    ///
    /// Serde failures map to [`CharacterError::Json`](super::error::CharacterError::Json).
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse from JSON text.
    ///
    /// # Errors
    ///
    /// Serde failures map to [`CharacterError::Json`](super::error::CharacterError::Json).
    pub fn from_json_str(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// Merge reciprocal edges: A→B and B→A collapse into a single
    /// edge A↔B. The surviving edge keeps the first non-empty `detail`
    /// and the `relation` of the original A→B. Pure / in-place.
    pub fn dedupe_reciprocal_edges(&mut self) {
        let mut keep: Vec<KgEdge> = Vec::with_capacity(self.edges.len());
        for edge in std::mem::take(&mut self.edges) {
            let reciprocal = keep
                .iter()
                .position(|existing| existing.from == edge.to && existing.to == edge.from);
            match reciprocal {
                Some(idx) => {
                    let existing = &mut keep[idx];
                    if existing.detail.is_empty() && !edge.detail.is_empty() {
                        existing.detail = edge.detail;
                    }
                }
                None => keep.push(edge),
            }
        }
        self.edges = keep;
    }
}

/// Build an ego KG from `extensions.novelagent.relationships` (+ optional `moral_axis` node).
#[must_use]
pub fn build_kg_from_card(card: &TavernCardV2) -> KnowledgeGraph {
    let name = card.data.name.trim();
    let ego_id = node_id("char", name);
    let mut nodes = vec![KgNode {
        id: ego_id.clone(),
        label: name.to_owned(),
        node_type: "protagonist".to_owned(),
    }];
    let mut edges = Vec::new();

    if let Some(ext) = card.data.extensions.novelagent.as_ref() {
        if let Some(axis) = non_empty(&ext.moral_axis) {
            let concept_id = node_id("concept", "moral_axis");
            nodes.push(KgNode {
                id: concept_id.clone(),
                label: axis,
                node_type: "concept".to_owned(),
            });
            edges.push(KgEdge {
                from: ego_id.clone(),
                to: concept_id,
                relation: "struggles_with".to_owned(),
                detail: String::new(),
            });
        }

        for rel in &ext.relationships {
            let other = rel.name.trim();
            if other.is_empty() {
                continue;
            }
            let other_id = node_id("char", other);
            if !nodes.iter().any(|n| n.id == other_id) {
                nodes.push(KgNode {
                    id: other_id.clone(),
                    label: other.to_owned(),
                    node_type: "character".to_owned(),
                });
            }
            let relation = if rel.relation_type.trim().is_empty() {
                "related_to".to_owned()
            } else {
                rel.relation_type.trim().to_owned()
            };
            edges.push(KgEdge {
                from: ego_id.clone(),
                to: other_id,
                relation,
                detail: rel.defines_protagonist_how.trim().to_owned(),
            });
        }
    }

    KnowledgeGraph {
        character_name: name.to_owned(),
        nodes,
        edges,
    }
}

fn node_id(prefix: &str, label: &str) -> String {
    let slug: String = label
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    format!("{prefix}:{slug}")
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::character::card::{
        CardExtensions, NovelAgentCharExt, RelationshipNode, TavernCardV2,
    };

    fn sample_card() -> TavernCardV2 {
        let mut card = TavernCardV2::skeleton_zh("苏晚");
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                moral_axis: "独立 vs 依附".to_owned(),
                relationships: vec![
                    RelationshipNode {
                        name: "老周".to_owned(),
                        relation_type: "mentor".to_owned(),
                        defines_protagonist_how: "提醒她被记得".to_owned(),
                    },
                    RelationshipNode {
                        name: "阿凯".to_owned(),
                        relation_type: "rival".to_owned(),
                        defines_protagonist_how: "镜像她的逃避".to_owned(),
                    },
                ],
                ..NovelAgentCharExt::default()
            }),
        };
        card
    }

    #[test]
    fn builds_ego_and_edges() {
        let kg = build_kg_from_card(&sample_card());
        assert_eq!(kg.character_name, "苏晚");
        assert!(kg.nodes.iter().any(|n| n.node_type == "protagonist"));
        assert_eq!(kg.edges.len(), 3); // moral_axis + 2 rels
        assert!(
            kg.edges
                .iter()
                .any(|e| e.relation == "mentor" && e.to == "char:老周")
        );
    }

    #[test]
    fn json_roundtrip() {
        let kg = build_kg_from_card(&sample_card());
        let json = kg.to_json_pretty().expect("ser");
        let back = KnowledgeGraph::from_json_str(&json).expect("de");
        assert_eq!(back, kg);
    }

    #[test]
    fn skeleton_has_only_ego() {
        let kg = build_kg_from_card(&TavernCardV2::skeleton_zh("空"));
        assert_eq!(kg.nodes.len(), 1);
        assert!(kg.edges.is_empty());
    }

    /// When the graph holds both A→B and B→A, `dedupe_reciprocal_edges`
    /// must collapse them into a single edge; the surviving edge keeps
    /// the first non-empty detail and the original relation.
    #[test]
    fn dedupes_reciprocal_edges() {
        let mut kg = KnowledgeGraph::default();
        kg.edges.push(KgEdge {
            from: "char:A".to_owned(),
            to: "char:B".to_owned(),
            relation: "mentor".to_owned(),
            detail: "first".to_owned(),
        });
        kg.edges.push(KgEdge {
            from: "char:B".to_owned(),
            to: "char:A".to_owned(),
            relation: "mentor".to_owned(),
            detail: "second".to_owned(),
        });
        kg.edges.push(KgEdge {
            from: "char:A".to_owned(),
            to: "char:C".to_owned(),
            relation: "ally".to_owned(),
            detail: String::new(),
        });
        kg.dedupe_reciprocal_edges();
        assert_eq!(kg.edges.len(), 2);
        let ab = kg
            .edges
            .iter()
            .find(|e| {
                (e.from == "char:A" && e.to == "char:B") || (e.from == "char:B" && e.to == "char:A")
            })
            .expect("A-B edge");
        assert_eq!(ab.detail, "first");
    }

    /// Round-trip must preserve nodes, edges, ego name, and every field
    /// — a strict equality check would be too brittle, so assert the
    /// load-bearing fields explicitly.
    #[test]
    fn serializes_round_trip_preserves_fields() {
        let kg = build_kg_from_card(&sample_card());
        let json = kg.to_json_pretty().expect("ser");
        let back = KnowledgeGraph::from_json_str(&json).expect("de");
        assert_eq!(back.character_name, kg.character_name);
        assert_eq!(back.nodes.len(), kg.nodes.len());
        assert_eq!(back.edges.len(), kg.edges.len());
        for (a, b) in back.edges.iter().zip(kg.edges.iter()) {
            assert_eq!(a.from, b.from);
            assert_eq!(a.to, b.to);
            assert_eq!(a.relation, b.relation);
            assert_eq!(a.detail, b.detail);
        }
    }
}
