//! Thin app-boundary commands for character create / roleplay chat (Phase 5).

use std::fmt::Write as _;

use anyhow::{Context, Result, bail};
use rig::completion::Prompt;

use crate::character::{
    CharacterSummary, DEFAULT_CHARACTERS_DIR, DeleteOutcome, assemble_prompt_pack,
    create_card_live, delete_character, format_create_summary, list_characters, load_card_by_slug,
    load_concept, write_create_outcome,
};
use crate::model::build_agent_builder;

/// Live create + persist under [`DEFAULT_CHARACTERS_DIR`].
///
/// Returns a one-line summary (paths + scores). Does not index `LanceDB`.
///
/// # Errors
///
/// Empty concept, LLM/validation failure, or filesystem write errors.
pub async fn character_create(concept: &str) -> Result<String> {
    let concept = concept.trim();
    if concept.is_empty() {
        bail!("concept must not be empty");
    }
    let outcome = create_card_live(concept)
        .await
        .context("character create failed")?;
    let paths = write_create_outcome(&outcome, concept, DEFAULT_CHARACTERS_DIR)
        .context("failed to write character artifacts")?;
    Ok(format_create_summary(&outcome, &paths))
}

/// Chat one turn as a saved card (`{slug}_card.json`), injecting system + role context.
///
/// # Errors
///
/// Missing card, empty message, builder failure, or model call failure.
pub async fn character_chat(slug: &str, message: &str) -> Result<String> {
    let message = message.trim();
    if message.is_empty() {
        bail!("message must not be empty");
    }
    let slug = slug.trim();
    if slug.is_empty() {
        bail!("character slug must not be empty");
    }

    let card = load_card_by_slug(DEFAULT_CHARACTERS_DIR, slug)
        .with_context(|| format!("load card slug={slug}"))?;
    let pack = assemble_prompt_pack(&card);
    let preamble = format!(
        "{}\n\n{}\n\n{}",
        pack.system, pack.role_context, pack.post_history_instructions
    );

    let agent = build_agent_builder()
        .context("failed to build agent for character chat")?
        .preamble(&preamble)
        .build();

    agent
        .prompt(message)
        .await
        .context("character chat model call failed")
}

/// Enumerate every saved card under [`DEFAULT_CHARACTERS_DIR`].
///
/// Thin pass-through to [`list_characters`] that pins the on-disk root so the
/// web layer never has to know the convention. Returned order is sorted by
/// slug for a stable UI.
///
/// # Errors
///
/// Filesystem errors other than "directory missing" are propagated.
pub fn character_list() -> Result<Vec<CharacterSummary>> {
    list_characters(DEFAULT_CHARACTERS_DIR)
        .with_context(|| format!("failed to list characters under {DEFAULT_CHARACTERS_DIR}"))
}

/// Delete every sidecar file for a saved character (card + memory + kg + report).
///
/// Thin pass-through to [`delete_character`] that pins the on-disk root.
/// Returns a one-line summary suitable for both CLI and Topcoat procedure use.
///
/// # Errors
///
/// Filesystem errors other than "file missing" are propagated. A slug with no
/// surviving files surfaces as an `Io` error.
pub fn character_delete(slug: &str) -> Result<String> {
    let slug = slug.trim();
    if slug.is_empty() {
        bail!("character slug must not be empty");
    }
    let outcome = delete_character(DEFAULT_CHARACTERS_DIR, slug)
        .with_context(|| format!("failed to delete character `{slug}`"))?;
    Ok(format_delete_summary(&outcome))
}

/// One-line summary for the delete command / procedure.
#[must_use]
pub fn format_delete_summary(outcome: &DeleteOutcome) -> String {
    format!(
        "deleted slug={} removed={} (card={} memory={} kg={} report={})",
        outcome.slug,
        outcome.removed_count(),
        outcome.card_removed,
        outcome.memory_removed,
        outcome.kg_removed,
        outcome.report_removed
    )
}

/// Regenerate a saved character using its stored concept.
///
/// Loads the original concept from `{slug}_report.json`, runs the Self-Refine
/// loop again, and overwrites the existing sidecars. The slug is preserved
/// (the new card uses the same name to derive the slug), so chat-as-this-slug
/// keeps working after a regenerate.
///
/// # Errors
///
/// Missing report file, empty stored concept, LLM failure, or filesystem
/// write errors.
pub async fn character_regenerate(slug: &str) -> Result<String> {
    let slug = slug.trim();
    if slug.is_empty() {
        bail!("character slug must not be empty");
    }
    let concept = load_concept(DEFAULT_CHARACTERS_DIR, slug)
        .with_context(|| format!("failed to load concept for slug `{slug}`"))?
        .ok_or_else(|| {
            anyhow::anyhow!("no stored concept for slug `{slug}` (legacy report or report missing)")
        })?;
    if concept.trim().is_empty() {
        bail!("stored concept for slug `{slug}` is empty");
    }
    let outcome = create_card_live(&concept)
        .await
        .context("character regenerate failed")?;
    let paths = write_create_outcome(&outcome, &concept, DEFAULT_CHARACTERS_DIR)
        .context("failed to write regenerated character artifacts")?;
    Ok(format_create_summary(&outcome, &paths))
}

/// Render a character list as a one-shot human-readable block.
///
/// Shared by CLI `character-list` and the Topcoat `character_list` procedure.
/// Empty list → `已存 0 个角色`. Non-empty → header +
/// `• {name} (slug=..., rounds=N, scores=p/c/v/t/c, mem=N, kg=N, lore=N)`.
/// Missing report fields → `n/a`. No timestamp (caller adds framing).
#[must_use]
pub fn format_character_list_summary(list: &[CharacterSummary]) -> String {
    if list.is_empty() {
        return "已存 0 个角色".to_owned();
    }
    let mut out = format!("已存 {} 个角色:", list.len());
    for s in list {
        let scores = s.scores.as_ref().map_or_else(
            || "n/a".to_owned(),
            |sc| {
                format!(
                    "{}/{}/{}/{}/{}",
                    sc.premise, sc.character, sc.voice, sc.tom, sc.constraints
                )
            },
        );
        let rounds = s
            .refine_rounds
            .map_or_else(|| "n/a".to_owned(), |r| r.to_string());
        let mem = s
            .memory_entries
            .map_or_else(|| "n/a".to_owned(), |n| n.to_string());
        let kg = s
            .kg_edges
            .map_or_else(|| "n/a".to_owned(), |n| n.to_string());
        let lore = s
            .lore_entries
            .map_or_else(|| "n/a".to_owned(), |n| n.to_string());
        let _ = write!(
            out,
            "\n• {} (slug={}, rounds={}, scores={}, mem={}, kg={}, lore={})",
            s.name, s.slug, rounds, scores, mem, kg, lore
        );
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn character_create_rejects_blank() {
        let err = character_create("   ").await.expect_err("blank");
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn character_chat_rejects_blank_message() {
        let err = character_chat("苏晚", "  ").await.expect_err("blank");
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn character_chat_rejects_blank_slug() {
        let err = character_chat("   ", "你好").await.expect_err("blank slug");
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn format_character_list_summary_empty() {
        assert_eq!(format_character_list_summary(&[]), "已存 0 个角色");
    }

    #[test]
    fn format_character_list_summary_one_row() {
        use crate::character::rubric::DimensionScores;
        use std::path::PathBuf;
        let row = CharacterSummary {
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
        };
        let out = format_character_list_summary(&[row]);
        assert!(out.contains("已存 1 个角色"));
        assert!(out.contains("• 苏晚"));
        assert!(out.contains("scores=4/5/4/4/5"));
    }

    #[test]
    fn character_delete_rejects_blank() {
        let err = character_delete("   ").expect_err("blank");
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn character_regenerate_rejects_blank() {
        let err = character_regenerate("   ").await.expect_err("blank");
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn format_delete_summary_lists_each_flag() {
        let outcome = DeleteOutcome {
            slug: "苏晚".to_owned(),
            card_removed: true,
            memory_removed: true,
            kg_removed: true,
            report_removed: true,
        };
        let s = format_delete_summary(&outcome);
        assert!(s.contains("slug=苏晚"));
        assert!(s.contains("removed=4"));
        assert!(s.contains("card=true"));
        assert!(s.contains("report=true"));
    }
}
