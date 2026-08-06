//! Build ST `character_book` entries from card fields + `NovelAgent` extensions.
//!
//! Pure, deterministic seeding (no LLM). Target: 3–10 entries, thin content.

use super::card::{CharacterBook, LoreEntry, NovelAgentCharExt, TavernCardV2};

/// Default token budget for generated character books (C-BUDGET).
pub const DEFAULT_LORE_TOKEN_BUDGET: u32 = 512;

/// Default scan depth for keyword matching.
pub const DEFAULT_LORE_SCAN_DEPTH: u32 = 50;

/// Soft cap on generated entries (plan: 3–10).
pub const MAX_LORE_ENTRIES: usize = 10;

/// Build a lorebook from a validated card (name + static fields + extensions).
///
/// Returns `None` when there is nothing useful to seed (empty skeleton with no
/// extension substance). Callers that always want a book can still force an
/// empty `CharacterBook` themselves.
#[must_use]
pub fn build_lorebook(card: &TavernCardV2) -> Option<CharacterBook> {
    let entries = collect_entries(card);
    if entries.is_empty() {
        return None;
    }
    Some(CharacterBook {
        name: Some(format!("{} 的 lorebook", card.data.name.trim())),
        description: Some("由 NovelAgent 从人物卡字段种子生成（Phase 4a）".to_owned()),
        scan_depth: Some(DEFAULT_LORE_SCAN_DEPTH),
        token_budget: Some(DEFAULT_LORE_TOKEN_BUDGET),
        recursive_scanning: Some(false),
        extensions: serde_json::Map::new(),
        entries,
    })
}

/// Attach a generated lorebook onto `card.data.character_book` when seedable.
pub fn attach_lorebook(card: &mut TavernCardV2) {
    if let Some(book) = build_lorebook(card) {
        card.data.character_book = Some(book);
    }
}

fn collect_entries(card: &TavernCardV2) -> Vec<LoreEntry> {
    let name = card.data.name.trim();
    let mut entries = Vec::new();
    let mut order = 0_i32;

    push_if(
        &mut entries,
        &mut order,
        lore_entry(
            vec![name.to_owned(), "身份".to_owned(), "简介".to_owned()],
            non_empty(&card.data.description).or_else(|| non_empty(&card.data.personality)),
            Some("identity"),
            true,
        ),
    );

    push_if(
        &mut entries,
        &mut order,
        lore_entry(
            vec!["场景".to_owned(), "开局".to_owned(), name.to_owned()],
            non_empty(&card.data.scenario),
            Some("scenario"),
            false,
        ),
    );

    if let Some(ext) = card.data.extensions.novelagent.as_ref() {
        push_ext_entries(&mut entries, &mut order, name, ext);
    }

    entries.truncate(MAX_LORE_ENTRIES);
    entries
}

fn push_ext_entries(
    entries: &mut Vec<LoreEntry>,
    order: &mut i32,
    name: &str,
    ext: &NovelAgentCharExt,
) {
    // (keys, content field, comment, constant)
    let fields: [(&[&str], &str, &str, bool); 5] = [
        (&["欲望", "目标"], &ext.desire, "desire", false),
        (&["需求", "内在"], &ext.need, "need", false),
        (&["弱点", "软肋"], &ext.weakness, "weakness", false),
        (&["主题", "道德"], &ext.moral_axis, "moral_axis", false),
        (
            &["所知", "边界", "不知道"],
            &ext.knowledge_bounds,
            "knowledge_bounds",
            true,
        ),
    ];
    for (keys, content, comment, constant) in fields {
        let mut key_vec: Vec<String> = keys.iter().map(|k| (*k).to_owned()).collect();
        key_vec.push(name.to_owned());
        push_if(
            entries,
            order,
            lore_entry(key_vec, non_empty(content), Some(comment), constant),
        );
    }

    if !ext.voice_markers.is_empty() {
        let content = ext
            .voice_markers
            .iter()
            .map(String::as_str)
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join("；");
        push_if(
            entries,
            order,
            lore_entry(
                vec!["声浪".to_owned(), "口吻".to_owned(), name.to_owned()],
                non_empty(&content),
                Some("voice"),
                true,
            ),
        );
    }

    push_relationship_entries(entries, order, name, ext);
}

fn push_relationship_entries(
    entries: &mut Vec<LoreEntry>,
    order: &mut i32,
    name: &str,
    ext: &NovelAgentCharExt,
) {
    for rel in &ext.relationships {
        if entries.len() >= MAX_LORE_ENTRIES {
            break;
        }
        let rel_name = rel.name.trim();
        if rel_name.is_empty() {
            continue;
        }
        let rel_type = if rel.relation_type.trim().is_empty() {
            "未标注类型"
        } else {
            rel.relation_type.trim()
        };
        let how = rel.defines_protagonist_how.trim();
        let content = if how.is_empty() {
            format!("{name} 与 {rel_name} 的关系：{rel_type}")
        } else {
            format!("{name} 与 {rel_name} 的关系：{rel_type}。{how}")
        };
        let keys = [rel_name, rel.relation_type.trim(), name]
            .into_iter()
            .filter(|k| !k.is_empty())
            .map(str::to_owned)
            .collect();
        push_if(
            entries,
            order,
            lore_entry(keys, Some(content), Some("relationship"), false),
        );
    }
}

fn push_if(entries: &mut Vec<LoreEntry>, order: &mut i32, entry: Option<LoreEntry>) {
    if entries.len() >= MAX_LORE_ENTRIES {
        return;
    }
    if let Some(mut e) = entry {
        e.insertion_order = *order;
        e.id = Some(*order);
        *order += 1;
        entries.push(e);
    }
}

fn lore_entry(
    keys: Vec<String>,
    content: Option<String>,
    comment: Option<&str>,
    constant: bool,
) -> Option<LoreEntry> {
    let content = content?;
    let keys: Vec<String> = keys
        .into_iter()
        .map(|k| k.trim().to_owned())
        .filter(|k| !k.is_empty())
        .collect();
    if keys.is_empty() {
        return None;
    }
    Some(LoreEntry {
        keys,
        content,
        enabled: true,
        insertion_order: 0,
        case_sensitive: Some(false),
        name: comment.map(str::to_owned),
        priority: None,
        id: None,
        comment: comment.map(str::to_owned),
        selective: Some(false),
        secondary_keys: None,
        constant: Some(constant),
        position: None,
        extensions: serde_json::Map::new(),
    })
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

    fn rich_card() -> TavernCardV2 {
        let mut card = TavernCardV2::skeleton_zh("苏晚");
        card.data.description = "雨夜便利店的夜班店员。".to_owned();
        card.data.scenario = "台风夜，店 bell 响起。".to_owned();
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                desire: "攒钱离开这座城".to_owned(),
                need: "承认自己需要被看见".to_owned(),
                weakness: "把关心当成施舍".to_owned(),
                moral_axis: "独立 vs 依附".to_owned(),
                knowledge_bounds: "不知道用户真实身份".to_owned(),
                voice_markers: vec!["短句".to_owned(), "少用感叹号".to_owned()],
                relationships: vec![RelationshipNode {
                    name: "老周".to_owned(),
                    relation_type: "mentor".to_owned(),
                    defines_protagonist_how: "提醒她还有人会记得她".to_owned(),
                }],
                locale: "zh-CN".to_owned(),
                ..NovelAgentCharExt::default()
            }),
        };
        card
    }

    #[test]
    fn builds_between_3_and_10_entries() {
        let book = build_lorebook(&rich_card()).expect("book");
        assert!(book.entries.len() >= 3);
        assert!(book.entries.len() <= MAX_LORE_ENTRIES);
        assert_eq!(book.token_budget, Some(DEFAULT_LORE_TOKEN_BUDGET));
    }

    #[test]
    fn empty_skeleton_yields_none() {
        let card = TavernCardV2::skeleton_zh("空");
        assert!(build_lorebook(&card).is_none());
    }

    #[test]
    fn attach_writes_character_book() {
        let mut card = rich_card();
        attach_lorebook(&mut card);
        let book = card.data.character_book.expect("attached");
        assert!(
            book.entries
                .iter()
                .any(|e| e.keys.iter().any(|k| k == "苏晚"))
        );
        assert!(
            book.entries
                .iter()
                .any(|e| e.comment.as_deref() == Some("desire"))
        );
    }

    #[test]
    fn relationship_entry_contains_other_name() {
        let book = build_lorebook(&rich_card()).expect("book");
        let rel = book
            .entries
            .iter()
            .find(|e| e.comment.as_deref() == Some("relationship"))
            .expect("rel entry");
        assert!(rel.content.contains("老周"));
        assert!(rel.keys.iter().any(|k| k == "老周"));
    }

    /// A card that yields 8–10 seedable entries must still produce a
    /// book at or below [`MAX_LORE_ENTRIES`]; the builder never overflows.
    #[test]
    fn attaches_lorebook_within_budget() {
        let book = build_lorebook(&rich_card()).expect("book");
        assert!(
            book.entries.len() <= MAX_LORE_ENTRIES,
            "got {} entries, cap = {MAX_LORE_ENTRIES}",
            book.entries.len()
        );
        assert!(book.entries.len() >= 3);
    }

    /// When the card has more seedable sources than the cap allows, the
    /// excess is dropped (not silently truncated mid-content). Identity
    /// and scenario are always first; relationships fill the tail.
    #[test]
    fn truncates_when_over_max() {
        let mut card = TavernCardV2::skeleton_zh("宁");
        card.data.description = "描述".to_owned();
        card.data.scenario = "场景".to_owned();
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                desire: "欲望".to_owned(),
                need: "需求".to_owned(),
                weakness: "弱点".to_owned(),
                moral_axis: "主题".to_owned(),
                knowledge_bounds: "边界".to_owned(),
                voice_markers: vec!["短句".to_owned()],
                relationships: (0..5)
                    .map(|i| RelationshipNode {
                        name: format!("人{i}"),
                        relation_type: "ally".to_owned(),
                        defines_protagonist_how: "对照".to_owned(),
                    })
                    .collect(),
                ..NovelAgentCharExt::default()
            }),
        };
        let book = build_lorebook(&card).expect("book");
        assert_eq!(book.entries.len(), MAX_LORE_ENTRIES);
    }

    /// `knowledge_bounds` and `voice` entries are emitted with
    /// `constant: Some(true)` so a downstream ST engine always includes
    /// them regardless of keyword match.
    #[test]
    fn constant_entries_always_active() {
        let book = build_lorebook(&rich_card()).expect("book");
        let kb = book
            .entries
            .iter()
            .find(|e| e.comment.as_deref() == Some("knowledge_bounds"))
            .expect("kb entry");
        assert_eq!(kb.constant, Some(true));
        let voice = book
            .entries
            .iter()
            .find(|e| e.comment.as_deref() == Some("voice"))
            .expect("voice entry");
        assert_eq!(voice.constant, Some(true));
    }
}
