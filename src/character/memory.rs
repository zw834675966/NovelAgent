//! Memory-stream metadata (Generative Agents style).
//!
//! v0 path: JSON metadata + recency × importance ranking.
//! v1 path: same texts → Cohere embed → [`crate::character::vector_store`].

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::card::TavernCardV2;
use super::error::{CharacterError, Result};

/// Kind of a memory-stream entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// Seed fact distilled from the static card / extensions.
    Seed,
    /// Observed event or dialogue summary.
    Observation,
    /// Higher-order reflection over prior memories.
    Reflection,
}

/// One entry in the character memory stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Stable id within the stream (monotone counter as string).
    pub id: String,
    /// Unix timestamp (seconds).
    pub ts: u64,
    /// Entry kind.
    pub kind: MemoryKind,
    /// Human-readable memory text (Chinese by default).
    pub text: String,
    /// Importance 1..=10 (Generative Agents scale; clamped on insert).
    pub importance: u8,
}

/// In-memory stream + character binding (serializable sidecar JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryStream {
    /// Character display name at seed time (for humans; not a foreign key).
    #[serde(default)]
    pub character_name: String,
    /// Monotone id counter (next id = `next_id`).
    #[serde(default)]
    pub next_id: u64,
    /// Ordered append-only log (oldest first).
    #[serde(default)]
    pub entries: Vec<MemoryEntry>,
}

impl MemoryStream {
    /// Empty stream bound to a character name.
    #[must_use]
    pub fn new(character_name: impl Into<String>) -> Self {
        Self {
            character_name: character_name.into(),
            next_id: 0,
            entries: Vec::new(),
        }
    }

    /// Append one entry. `importance` is clamped to 1..=10.
    ///
    /// # Errors
    ///
    /// [`CharacterError::Validation`] when `text` is empty after trim.
    pub fn append(
        &mut self,
        kind: MemoryKind,
        text: impl Into<String>,
        importance: u8,
        ts: Option<u64>,
    ) -> Result<&MemoryEntry> {
        let text = text.into();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(CharacterError::Validation(
                "memory entry text must not be empty".to_owned(),
            ));
        }
        let importance = importance.clamp(1, 10);
        let ts = ts.unwrap_or_else(now_unix);
        let id = self.next_id.to_string();
        self.next_id = self.next_id.saturating_add(1);
        self.entries.push(MemoryEntry {
            id,
            ts,
            kind,
            text: trimmed.to_owned(),
            importance,
        });
        self.entries
            .last()
            .ok_or_else(|| CharacterError::Validation("memory stream append invariant".to_owned()))
    }

    /// Rank by `α·recency + β·importance` (no cosine yet). Higher is better.
    ///
    /// `now` is Unix seconds; if `None`, uses wall clock.
    #[must_use]
    pub fn rank_recency_importance(
        &self,
        alpha: f64,
        beta: f64,
        now: Option<u64>,
    ) -> Vec<(f64, &MemoryEntry)> {
        let now = now.unwrap_or_else(now_unix);
        let mut scored: Vec<(f64, &MemoryEntry)> = self
            .entries
            .iter()
            .map(|e| {
                // Age in hours; cap to u32 range so f64 conversion is lossless for ranking.
                let age_secs = now.saturating_sub(e.ts).min(u64::from(u32::MAX));
                let age_u32 = u32::try_from(age_secs).unwrap_or(u32::MAX);
                let age_hours = f64::from(age_u32) / 3600.0;
                // Generative Agents-style exponential decay (half-life ~24h soft).
                let recency = (-age_hours / 24.0).exp();
                let importance = f64::from(e.importance) / 10.0;
                let score = alpha * recency + beta * importance;
                (score, e)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Serialize to pretty JSON.
    ///
    /// # Errors
    ///
    /// Serde failures map to [`CharacterError::Json`].
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse from JSON text.
    ///
    /// # Errors
    ///
    /// Serde failures map to [`CharacterError::Json`].
    pub fn from_json_str(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// Iterate entries whose `kind` matches, in original append order.
    pub fn entries_of_kind(&self, kind: MemoryKind) -> impl Iterator<Item = &MemoryEntry> {
        self.entries.iter().filter(move |e| e.kind == kind)
    }
}

/// Seed a memory stream from a card (static fields → `MemoryKind::Seed`).
#[must_use]
pub fn seed_memory_from_card(card: &TavernCardV2, ts: Option<u64>) -> MemoryStream {
    let mut stream = MemoryStream::new(card.data.name.clone());
    let ts = ts.unwrap_or_else(now_unix);
    let name = card.data.name.trim();

    push_seed(
        &mut stream,
        format!("{name}：{}", card.data.description.trim()),
        7,
        ts,
    );
    push_seed(
        &mut stream,
        format!("性格：{}", card.data.personality.trim()),
        6,
        ts,
    );
    push_seed(
        &mut stream,
        format!("场景：{}", card.data.scenario.trim()),
        6,
        ts,
    );

    if let Some(ext) = card.data.extensions.novelagent.as_ref() {
        push_seed(&mut stream, format!("欲望：{}", ext.desire.trim()), 8, ts);
        push_seed(&mut stream, format!("内在需求：{}", ext.need.trim()), 8, ts);
        push_seed(&mut stream, format!("弱点：{}", ext.weakness.trim()), 7, ts);
        push_seed(
            &mut stream,
            format!("主题张力：{}", ext.moral_axis.trim()),
            6,
            ts,
        );
        push_seed(
            &mut stream,
            format!("所知边界：{}", ext.knowledge_bounds.trim()),
            7,
            ts,
        );
        for rel in &ext.relationships {
            let rel_name = rel.name.trim();
            if rel_name.is_empty() {
                continue;
            }
            let how = rel.defines_protagonist_how.trim();
            let text = if how.is_empty() {
                format!("关系：{name} —[{}]→ {rel_name}", rel.relation_type.trim())
            } else {
                format!(
                    "关系：{name} —[{}]→ {rel_name}；{how}",
                    rel.relation_type.trim()
                )
            };
            push_seed(&mut stream, text, 7, ts);
        }
    }

    stream
}

fn push_seed(stream: &mut MemoryStream, text: String, importance: u8, ts: u64) {
    if text
        .split('：')
        .nth(1)
        .is_some_and(|rest| rest.trim().is_empty())
    {
        // "标签：" with empty body — skip.
        return;
    }
    // Also skip pure "name：" when description empty already handled; for
    // "苏晚：" only (description empty) the format is `{name}：{desc}`.
    if text.ends_with('：') || text.chars().filter(|c| *c != '：').all(char::is_whitespace) {
        return;
    }
    let _ = stream.append(MemoryKind::Seed, text, importance, Some(ts));
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::character::card::{
        CardExtensions, NovelAgentCharExt, RelationshipNode, TavernCardV2,
    };

    fn sample_card() -> TavernCardV2 {
        let mut card = TavernCardV2::skeleton_zh("苏晚");
        card.data.description = "夜班店员".to_owned();
        card.data.personality = "克制".to_owned();
        card.data.scenario = "雨夜".to_owned();
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                desire: "离开".to_owned(),
                need: "被看见".to_owned(),
                weakness: "拒人".to_owned(),
                moral_axis: "独立/依附".to_owned(),
                knowledge_bounds: "不知用户身份".to_owned(),
                relationships: vec![RelationshipNode {
                    name: "老周".to_owned(),
                    relation_type: "mentor".to_owned(),
                    defines_protagonist_how: "唯一还记得她的人".to_owned(),
                }],
                ..NovelAgentCharExt::default()
            }),
        };
        card
    }

    #[test]
    fn seed_produces_multiple_seed_entries() {
        let stream = seed_memory_from_card(&sample_card(), Some(1_700_000_000));
        assert!(stream.entries.len() >= 5);
        assert!(stream.entries.iter().all(|e| e.kind == MemoryKind::Seed));
        assert_eq!(stream.character_name, "苏晚");
    }

    #[test]
    fn append_rejects_empty_text() {
        let mut s = MemoryStream::new("x");
        assert!(s.append(MemoryKind::Observation, "  ", 5, Some(1)).is_err());
    }

    #[test]
    fn append_clamps_importance() {
        let mut s = MemoryStream::new("x");
        s.append(MemoryKind::Observation, "看到了雨", 99, Some(10))
            .expect("ok");
        assert_eq!(s.entries[0].importance, 10);
    }

    #[test]
    fn rank_prefers_important_and_recent() {
        let mut s = MemoryStream::new("x");
        s.append(MemoryKind::Observation, "旧且低", 2, Some(100))
            .unwrap();
        s.append(MemoryKind::Observation, "新且高", 9, Some(10_000))
            .unwrap();
        let ranked = s.rank_recency_importance(1.0, 1.0, Some(10_000));
        assert_eq!(ranked[0].1.text, "新且高");
    }

    #[test]
    fn json_roundtrip() {
        let stream = seed_memory_from_card(&sample_card(), Some(42));
        let json = stream.to_json_pretty().expect("ser");
        let back = MemoryStream::from_json_str(&json).expect("de");
        assert_eq!(back.entries.len(), stream.entries.len());
        assert_eq!(back.entries[0].ts, 42);
    }

    /// Entries must keep insertion order regardless of `ts`; the stream
    /// is append-only and reads back FIFO.
    #[test]
    fn append_preserves_order() {
        let mut s = MemoryStream::new("x");
        s.append(MemoryKind::Observation, "一", 5, Some(300))
            .unwrap();
        s.append(MemoryKind::Observation, "二", 5, Some(100))
            .unwrap();
        s.append(MemoryKind::Observation, "三", 5, Some(200))
            .unwrap();
        assert_eq!(s.entries[0].text, "一");
        assert_eq!(s.entries[1].text, "二");
        assert_eq!(s.entries[2].text, "三");
        assert!(s.entries[0].ts > s.entries[1].ts);
    }

    /// `importance` clamps to the 1..=10 range; `0` becomes `1`.
    #[test]
    fn importance_clamps_zero_to_one() {
        let mut s = MemoryStream::new("x");
        s.append(MemoryKind::Observation, "极低", 0, Some(1))
            .unwrap();
        assert_eq!(s.entries[0].importance, 1);
    }

    /// `entries_of_kind` must return only entries matching the requested
    /// kind, in original order. Other kinds are excluded.
    #[test]
    fn filters_by_kind() {
        let mut s = MemoryStream::new("x");
        s.append(MemoryKind::Observation, "obs-1", 5, Some(1))
            .unwrap();
        s.append(MemoryKind::Reflection, "ref-1", 5, Some(2))
            .unwrap();
        s.append(MemoryKind::Observation, "obs-2", 5, Some(3))
            .unwrap();
        s.append(MemoryKind::Reflection, "ref-2", 5, Some(4))
            .unwrap();
        let reflections: Vec<&str> = s
            .entries_of_kind(MemoryKind::Reflection)
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(reflections, vec!["ref-1", "ref-2"]);
    }
}
