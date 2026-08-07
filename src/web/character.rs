//! Topcoat procedures for character create / roleplay (Phase 5).

use std::time::{SystemTime, UNIX_EPOCH};

use topcoat::Result;
use topcoat::runtime::procedure;

fn utc_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Live concept → card JSON under `data/characters/`. Always `Ok(String)`.
#[procedure]
pub async fn character_create(concept: String) -> Result<String> {
    let trimmed = concept.trim();
    if trimmed.is_empty() {
        return Ok(format!("[{}] (empty concept)", utc_hms()));
    }
    match crate::app::character_create(trimmed).await {
        Ok(summary) => Ok(format!("[{}] {summary}", utc_hms())),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

/// One chat turn as a saved card (`slug` = name/slug). Always `Ok(String)`.
#[procedure]
pub async fn character_chat(slug: String, message: String) -> Result<String> {
    let slug = slug.trim();
    let message = message.trim();
    if slug.is_empty() {
        return Ok(format!("[{}] (empty slug)", utc_hms()));
    }
    if message.is_empty() {
        return Ok(format!("[{}] (empty message)", utc_hms()));
    }
    match crate::app::character_chat(slug, message).await {
        Ok(reply) => Ok(format!("[{}] {reply}", utc_hms())),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

/// Enumerate saved characters as a human-readable list. Always `Ok(String)`.
///
/// On error the string is a `[HH:MM:SS] (error: ...)` line; on success it
/// is a `已存 N 个角色` header followed by `• {name} (slug=..., rounds=N,
/// scores=p/c/v/t/c, mem=N, kg=N, lore=N)` lines. Empty list → header only.
/// Body formatting lives in [`crate::app::format_character_list_summary`].
#[procedure]
pub async fn character_list() -> Result<String> {
    match crate::app::character_list() {
        Ok(list) => Ok(format!(
            "[{}] {}",
            utc_hms(),
            crate::app::format_character_list_summary(&list)
        )),
        Err(err) => Ok(format!("[{}] (error: {err})", utc_hms())),
    }
}

#[cfg(test)]
mod tests {
    use crate::character::rubric::DimensionScores;
    use std::path::PathBuf;

    fn sample_with_scores() -> crate::character::CharacterSummary {
        crate::character::CharacterSummary {
            slug: "苏晚".to_owned(),
            name: "苏晚".to_owned(),
            concept: None,
            refine_rounds: Some(1),
            scores: Some(DimensionScores {
                premise: 4,
                character: 5,
                voice: 4,
                tom: 4,
                constraints: 5,
            }),
            memory_entries: Some(3),
            kg_edges: Some(2),
            lore_entries: Some(1),
            card_path: PathBuf::from("data/characters/苏晚_card.json"),
        }
    }

    fn sample_without_report() -> crate::character::CharacterSummary {
        crate::character::CharacterSummary {
            slug: "ghost".to_owned(),
            name: "ghost".to_owned(),
            concept: None,
            refine_rounds: None,
            scores: None,
            memory_entries: None,
            kg_edges: None,
            lore_entries: None,
            card_path: PathBuf::from("data/characters/ghost_card.json"),
        }
    }

    #[test]
    fn shared_formatter_empty_says_zero() {
        let out = crate::app::format_character_list_summary(&[]);
        assert!(out.contains("已存 0 个角色"));
        assert!(!out.contains('•'));
    }

    #[test]
    fn shared_formatter_with_score_row_includes_all_fields() {
        let out = crate::app::format_character_list_summary(&[sample_with_scores()]);
        assert!(out.contains("已存 1 个角色"));
        assert!(out.contains("• 苏晚"));
        assert!(out.contains("slug=苏晚"));
        assert!(out.contains("rounds=1"));
        assert!(out.contains("scores=4/5/4/4/5"));
        assert!(out.contains("mem=3"));
        assert!(out.contains("kg=2"));
        assert!(out.contains("lore=1"));
    }

    #[test]
    fn shared_formatter_without_report_uses_na_placeholders() {
        let out = crate::app::format_character_list_summary(&[sample_without_report()]);
        assert!(out.contains("ghost"));
        assert!(out.contains("rounds=n/a"));
        assert!(out.contains("scores=n/a"));
        assert!(out.contains("mem=n/a"));
    }

    #[test]
    fn shared_formatter_multi_rows_separated_by_newline() {
        let out = crate::app::format_character_list_summary(&[
            sample_with_scores(),
            sample_without_report(),
        ]);
        assert!(out.contains("已存 2 个角色"));
        assert!(out.contains("• 苏晚"));
        assert!(out.contains("• ghost"));
    }
}
