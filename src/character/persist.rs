//! Persist create-card outcomes under `data/characters/` (Phase 5).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::agent::CreateCardOutcome;
use super::card::TavernCardV2;
use super::error::{CharacterError, Result};
use super::vector_store::character_slug;

/// Default on-disk root for card JSON + sidecars (gitignored).
pub const DEFAULT_CHARACTERS_DIR: &str = "data/characters";

/// Paths written by [`write_create_outcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPaths {
    /// Directory that holds the files.
    pub dir: PathBuf,
    /// Filesystem slug derived from the card name.
    pub slug: String,
    pub card: PathBuf,
    pub memory: PathBuf,
    pub kg: PathBuf,
    pub report: PathBuf,
}

/// Compact critique summary stored next to the card.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateReportFile {
    name: String,
    /// Original free-text concept the user fed into `create_card_live`.
    /// Empty when the report was written by a build that pre-dated the
    /// concept field — older reports still load fine via `Option<String>`
    /// via the `concept` field on [`CharacterSummary`] (None for legacy).
    #[serde(default)]
    concept: String,
    refine_rounds: u8,
    scores: super::rubric::DimensionScores,
    must_fix: Vec<String>,
    summary: String,
    memory_entries: usize,
    kg_edges: usize,
    lore_entries: usize,
}

/// Public per-card summary returned by [`list_characters`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterSummary {
    pub slug: String,
    pub name: String,
    /// Original concept, if the report file captured one. `None` for legacy
    /// reports (before the `concept` field was added) or for entries whose
    /// `*_report.json` is missing or unparseable.
    pub concept: Option<String>,
    pub refine_rounds: Option<u8>,
    pub scores: Option<super::rubric::DimensionScores>,
    pub memory_entries: Option<usize>,
    pub kg_edges: Option<usize>,
    pub lore_entries: Option<usize>,
    pub card_path: PathBuf,
}

/// Write card + memory + kg + critique report for a successful create.
///
/// The `concept` argument is the free-text prompt the user fed into
/// `create_card_live`; it is persisted in `*_report.json` so a later
/// "regenerate" pass can read it back without re-prompting.
///
/// # Errors
///
/// Directory create / write failures → [`CharacterError::Io`].
pub fn write_create_outcome(
    outcome: &CreateCardOutcome,
    concept: &str,
    dir: impl AsRef<Path>,
) -> Result<ArtifactPaths> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir).map_err(|err| CharacterError::Io(err.to_string()))?;

    let slug = character_slug(&outcome.card.data.name);
    let paths = ArtifactPaths {
        card: dir.join(format!("{slug}_card.json")),
        memory: dir.join(format!("{slug}_memory.json")),
        kg: dir.join(format!("{slug}_kg.json")),
        report: dir.join(format!("{slug}_report.json")),
        dir: dir.to_path_buf(),
        slug,
    };

    let card_json = serde_json::to_string_pretty(&outcome.card)?;
    write_text(&paths.card, &card_json)?;
    write_text(&paths.memory, &outcome.memory.to_json_pretty()?)?;
    write_text(&paths.kg, &outcome.kg.to_json_pretty()?)?;

    let lore_entries = outcome
        .card
        .data
        .character_book
        .as_ref()
        .map_or(0, |b| b.entries.len());
    let report = CreateReportFile {
        name: outcome.card.data.name.clone(),
        concept: concept.to_owned(),
        refine_rounds: outcome.refine_rounds,
        scores: outcome.critique.scores.clone(),
        must_fix: outcome.critique.must_fix.clone(),
        summary: outcome.critique.summary.clone(),
        memory_entries: outcome.memory.entries.len(),
        kg_edges: outcome.kg.edges.len(),
        lore_entries,
    };
    write_text(&paths.report, &serde_json::to_string_pretty(&report)?)?;

    Ok(paths)
}

/// Load a V2 card from `{dir}/{slug}_card.json`.
///
/// # Errors
///
/// Missing file / invalid JSON → [`CharacterError::Io`] or [`CharacterError::Json`].
pub fn load_card_by_slug(dir: impl AsRef<Path>, slug: &str) -> Result<TavernCardV2> {
    let slug = character_slug(slug);
    let path = dir.as_ref().join(format!("{slug}_card.json"));
    let text = fs::read_to_string(&path)
        .map_err(|err| CharacterError::Io(format!("read {}: {err}", path.display())))?;
    Ok(serde_json::from_str(&text)?)
}

/// Load the original free-text concept stored in `{dir}/{slug}_report.json`.
///
/// Returns [`None`] when the report file is missing, the report was written
/// by a build that pre-dated the `concept` field, or the JSON fails to parse.
///
/// # Errors
///
/// Filesystem errors other than "file missing" are propagated.
pub fn load_concept(dir: impl AsRef<Path>, slug: &str) -> Result<Option<String>> {
    let slug = character_slug(slug);
    let path = dir.as_ref().join(format!("{slug}_report.json"));
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    let Ok(report) = serde_json::from_str::<CreateReportFile>(&text) else {
        return Ok(None);
    };
    Ok((!report.concept.is_empty()).then_some(report.concept))
}

/// One-line human summary for CLI / procedure responses.
#[must_use]
pub fn format_create_summary(outcome: &CreateCardOutcome, paths: &ArtifactPaths) -> String {
    let s = &outcome.critique.scores;
    let lore = outcome
        .card
        .data
        .character_book
        .as_ref()
        .map_or(0, |b| b.entries.len());
    format!(
        "created name={} refine_rounds={} scores={{premise:{},character:{},voice:{},tom:{},constraints:{}}} \
         lore={} mem={} kg_edges={} card={}",
        outcome.card.data.name,
        outcome.refine_rounds,
        s.premise,
        s.character,
        s.voice,
        s.tom,
        s.constraints,
        lore,
        outcome.memory.entries.len(),
        outcome.kg.edges.len(),
        paths.card.display()
    )
}

/// Scan a character directory for saved cards.
///
/// Reads every `*_card.json` and pairs it with the adjacent `*_report.json`
/// when present. Missing directory → empty list. Files whose JSON fails to
/// parse as a V2 card are silently skipped — the list is a "what is here"
/// view, not a validator; corrupt files should not hide the rest. Filesystem
/// errors on the directory itself are propagated. Result is sorted by slug
/// for stable UI ordering.
///
/// # Errors
///
/// Filesystem errors other than "directory missing" or per-file parse
/// failures are propagated.
pub fn list_characters(dir: impl AsRef<Path>) -> Result<Vec<CharacterSummary>> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut summaries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| CharacterError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| CharacterError::Io(err.to_string()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !file_name.ends_with("_card.json") {
            continue;
        }
        let slug = file_name.trim_end_matches("_card.json").to_owned();
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(card) = serde_json::from_str::<TavernCardV2>(&text) else {
            continue;
        };

        let report_path = dir.join(format!("{slug}_report.json"));
        let (concept, refine_rounds, scores, memory_entries, kg_edges, lore_entries) =
            match fs::read_to_string(&report_path) {
                Ok(rpt_text) => match serde_json::from_str::<CreateReportFile>(&rpt_text) {
                    Ok(rpt) => (
                        (!rpt.concept.is_empty()).then_some(rpt.concept),
                        Some(rpt.refine_rounds),
                        Some(rpt.scores),
                        Some(rpt.memory_entries),
                        Some(rpt.kg_edges),
                        Some(rpt.lore_entries),
                    ),
                    Err(_) => (None, None, None, None, None, None),
                },
                Err(_) => (None, None, None, None, None, None),
            };

        summaries.push(CharacterSummary {
            slug: slug.clone(),
            name: card.data.name,
            concept,
            refine_rounds,
            scores,
            memory_entries,
            kg_edges,
            lore_entries,
            card_path: path,
        });
    }
    summaries.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(summaries)
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    fs::write(path, text)
        .map_err(|err| CharacterError::Io(format!("write {}: {err}", path.display())))
}

/// What was actually removed by [`delete_character`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct DeleteOutcome {
    pub slug: String,
    pub card_removed: bool,
    pub memory_removed: bool,
    pub kg_removed: bool,
    pub report_removed: bool,
}

impl DeleteOutcome {
    /// Number of files actually deleted.
    #[must_use]
    pub fn removed_count(&self) -> usize {
        [
            self.card_removed,
            self.memory_removed,
            self.kg_removed,
            self.report_removed,
        ]
        .iter()
        .filter(|b| **b)
        .count()
    }
}

/// Delete the card + memory + kg + report files for a given slug.
///
/// Slug is re-normalised through [`character_slug`] so callers can pass either
/// the human name or the on-disk stem. Missing files are silently skipped
/// (an entry that exists only as `_card.json` is treated as a successful
/// partial delete). A completely missing slug returns
/// [`CharacterError::Io`].
///
/// # Errors
///
/// Filesystem errors other than "file missing" are propagated.
pub fn delete_character(dir: impl AsRef<Path>, slug: &str) -> Result<DeleteOutcome> {
    let dir = dir.as_ref();
    let slug = character_slug(slug);
    let mut outcome = DeleteOutcome {
        slug: slug.clone(),
        card_removed: false,
        memory_removed: false,
        kg_removed: false,
        report_removed: false,
    };

    for (suffix, flag) in [
        ("_card.json", &mut outcome.card_removed),
        ("_memory.json", &mut outcome.memory_removed),
        ("_kg.json", &mut outcome.kg_removed),
        ("_report.json", &mut outcome.report_removed),
    ] {
        let path = dir.join(format!("{slug}{suffix}"));
        if !path.exists() {
            continue;
        }
        fs::remove_file(&path)
            .map_err(|err| CharacterError::Io(format!("remove {}: {err}", path.display())))?;
        *flag = true;
    }

    if outcome.removed_count() == 0 {
        return Err(CharacterError::Io(format!(
            "no character files found for slug `{slug}` under {}",
            dir.display()
        )));
    }

    Ok(outcome)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::character::card::{
        CardExtensions, NovelAgentCharExt, RelationshipNode, TavernCardV2,
    };
    use crate::character::rubric::{CritiqueFlags, CritiqueReport, DimensionScores};
    use crate::character::seed::seed_card_artifacts;

    fn sample_outcome() -> CreateCardOutcome {
        let mut card = TavernCardV2::skeleton_zh("苏晚");
        card.data.description = "夜班店员".to_owned();
        card.data.personality = "克制".to_owned();
        card.data.scenario = "雨夜".to_owned();
        card.data.system_prompt = "你是苏晚。".to_owned();
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
        let art = seed_card_artifacts(&mut card, Some(1_700_000_000));
        CreateCardOutcome {
            card,
            critique: CritiqueReport {
                scores: DimensionScores {
                    premise: 4,
                    character: 4,
                    voice: 4,
                    tom: 4,
                    constraints: 5,
                },
                flags: CritiqueFlags {
                    schema_ok: true,
                    placeholders_ok: true,
                    locale_ok: true,
                },
                issues: Vec::new(),
                must_fix: Vec::new(),
                summary: "ok".to_owned(),
            },
            refine_rounds: 1,
            memory: art.memory,
            kg: art.kg,
        }
    }

    #[test]
    fn write_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("novelagent_persist_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let outcome = sample_outcome();
        let paths = write_create_outcome(&outcome, "test concept", &dir).expect("write");
        assert!(paths.card.is_file());
        assert!(paths.memory.is_file());
        assert!(paths.kg.is_file());
        assert!(paths.report.is_file());
        assert_eq!(paths.slug, "苏晚");

        let loaded = load_card_by_slug(&dir, "苏晚").expect("load");
        assert_eq!(loaded.data.name, "苏晚");
        assert!(
            loaded
                .data
                .character_book
                .as_ref()
                .is_some_and(|b| !b.entries.is_empty())
        );

        let summary = format_create_summary(&outcome, &paths);
        assert!(summary.contains("苏晚"));
        assert!(summary.contains("card="));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_slug_is_io_error() {
        let dir = std::env::temp_dir().join("novelagent_persist_missing");
        let _ = fs::create_dir_all(&dir);
        let err = load_card_by_slug(&dir, "no_such_card").expect_err("missing");
        assert!(matches!(err, CharacterError::Io(_)));
    }

    fn unique_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("novelagent_list_{}_{}", std::process::id(), tag))
    }

    fn write_sample_pair(dir: &Path, slug: &str, name: &str, rounds: u8) {
        let mut card = TavernCardV2::skeleton_zh(name);
        card.data.description = "夜班店员".to_owned();
        card.data.personality = "克制".to_owned();
        card.data.scenario = "雨夜".to_owned();
        card.data.system_prompt = format!("你是{name}。");
        let card_path = dir.join(format!("{slug}_card.json"));
        let report_path = dir.join(format!("{slug}_report.json"));
        fs::write(&card_path, serde_json::to_string_pretty(&card).unwrap()).unwrap();
        let report = CreateReportFile {
            name: name.to_owned(),
            concept: String::new(),
            refine_rounds: rounds,
            scores: DimensionScores {
                premise: 4,
                character: 5,
                voice: 4,
                tom: 4,
                constraints: 5,
            },
            must_fix: vec![],
            summary: "ok".to_owned(),
            memory_entries: 3,
            kg_edges: 2,
            lore_entries: 1,
        };
        fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    #[test]
    fn list_characters_missing_dir_returns_empty() {
        let dir = unique_dir("missing");
        let _ = fs::remove_dir_all(&dir);
        let list = list_characters(&dir).expect("missing dir should be empty, not error");
        assert!(list.is_empty());
    }

    #[test]
    fn list_characters_empty_dir_returns_empty() {
        let dir = unique_dir("empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let list = list_characters(&dir).expect("list");
        assert!(list.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_characters_returns_summaries_for_each_card() {
        let dir = unique_dir("two");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        write_sample_pair(&dir, "苏晚", "苏晚", 1);
        write_sample_pair(&dir, "老周", "老周", 2);

        let list = list_characters(&dir).expect("list");
        assert_eq!(list.len(), 2);
        let suwan = list.iter().find(|s| s.slug == "苏晚").expect("苏晚");
        assert_eq!(suwan.name, "苏晚");
        assert_eq!(suwan.refine_rounds, Some(1));
        let scores = suwan.scores.as_ref().expect("scores");
        assert_eq!(scores.character, 5);
        assert_eq!(suwan.memory_entries, Some(3));
        assert_eq!(suwan.kg_edges, Some(2));
        assert_eq!(suwan.lore_entries, Some(1));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_characters_handles_missing_report() {
        let dir = unique_dir("no_report");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        write_sample_pair(&dir, "ghost", "ghost", 1);
        // remove just the report; card file remains
        fs::remove_file(dir.join("ghost_report.json")).unwrap();

        let list = list_characters(&dir).expect("list");
        assert_eq!(list.len(), 1);
        let ghost = &list[0];
        assert_eq!(ghost.slug, "ghost");
        assert_eq!(ghost.name, "ghost");
        assert!(ghost.scores.is_none());
        assert!(ghost.refine_rounds.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_characters_ignores_non_card_files() {
        let dir = unique_dir("mixed");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        write_sample_pair(&dir, "苏晚", "苏晚", 1);
        // noise files
        fs::write(dir.join("readme.txt"), b"hi").unwrap();
        fs::write(dir.join("random.json"), b"{}").unwrap();
        fs::write(dir.join("苏晚_memory.json"), b"{}").unwrap();
        fs::write(dir.join("苏晚_kg.json"), b"{}").unwrap();
        fs::write(dir.join("苏晚_report.json.bak"), b"{}").unwrap();

        let list = list_characters(&dir).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].slug, "苏晚");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_characters_skips_unparseable_card_files() {
        let dir = unique_dir("corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        write_sample_pair(&dir, "苏晚", "苏晚", 1);
        // a *_card.json whose contents are not a V2 card (missing `name`)
        fs::write(
            dir.join("junk_card.json"),
            br#"{"data": {"description": "no name field"}}"#,
        )
        .unwrap();
        // valid card-shaped JSON but with wrong types
        fs::write(dir.join("garbage_card.json"), b"this is not json at all").unwrap();

        let list = list_characters(&dir).expect("list");
        assert_eq!(list.len(), 1, "only the parseable card surfaces");
        assert_eq!(list[0].slug, "苏晚");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_character_removes_all_four_files() {
        let dir = unique_dir("delete_all");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // write all four sidecar files explicitly; write_sample_pair only
        // writes card + report.
        fs::write(dir.join("苏晚_card.json"), b"{}").unwrap();
        fs::write(dir.join("苏晚_memory.json"), b"{}").unwrap();
        fs::write(dir.join("苏晚_kg.json"), b"{}").unwrap();
        fs::write(dir.join("苏晚_report.json"), b"{}").unwrap();

        let outcome = delete_character(&dir, "苏晚").expect("delete");
        assert_eq!(outcome.slug, "苏晚");
        assert!(outcome.card_removed);
        assert!(outcome.memory_removed);
        assert!(outcome.kg_removed);
        assert!(outcome.report_removed);
        assert_eq!(outcome.removed_count(), 4);
        assert!(!dir.join("苏晚_card.json").exists());
        assert!(!dir.join("苏晚_memory.json").exists());
        assert!(!dir.join("苏晚_kg.json").exists());
        assert!(!dir.join("苏晚_report.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_character_partial_when_only_card_exists() {
        let dir = unique_dir("delete_partial");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("ghost_card.json"), b"{}").unwrap();

        let outcome = delete_character(&dir, "ghost").expect("partial delete");
        assert_eq!(outcome.slug, "ghost");
        assert!(outcome.card_removed);
        assert!(!outcome.memory_removed);
        assert!(!outcome.kg_removed);
        assert!(!outcome.report_removed);
        assert_eq!(outcome.removed_count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_character_missing_slug_errors() {
        let dir = unique_dir("delete_missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let err = delete_character(&dir, "never_existed").expect_err("missing");
        assert!(matches!(err, CharacterError::Io(_)));
        assert!(err.to_string().contains("never_existed"));
        let _ = fs::remove_dir_all(&dir);
    }
}
