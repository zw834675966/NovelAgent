//! Meta-agent loop: draft → critique → refine (Self-Refine ≤2).
//!
//! Live path uses [`RigLlm`] + `model::build_agent_builder`. Tests inject a
//! scripted [`LlmBackend`] so CI never needs the network.

use rig::completion::Prompt;
use serde::Serialize;
use serde_json::{Value, json};

use super::card::TavernCardV2;
use super::constraints::validate_card;
use super::error::{CharacterError, Result};
use super::kg::KnowledgeGraph;
use super::memory::MemoryStream;
use super::prompt_pack::{
    META_SYSTEM, render_critique_user, render_refine_user, render_user_create,
};
use super::rubric::{CritiqueReport, MAX_REFINE_ROUNDS};
use super::seed::seed_card_artifacts;
use crate::model::build_agent_builder;

/// Headroom for a full V2 card JSON (`DeepSeek` flash thinking can burn budget).
const CARD_MAX_TOKENS: u64 = 8192;

/// Injected completion surface (live rig or scripted mock).
pub trait LlmBackend: Send + Sync {
    /// Complete one turn with a system preamble and user message.
    fn complete(
        &self,
        system: &str,
        user: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

/// Default backend: `DeepSeek` V4 Flash via `OpenCode` Go (`model::build_agent_builder`).
#[derive(Debug, Default, Clone, Copy)]
pub struct RigLlm;

impl LlmBackend for RigLlm {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        // Structured JSON generation: disable thinking so budget goes to
        // `content` (DeepSeek-V4-flash may put long CoT in reasoning_content
        // and return empty content when max_tokens is tight).
        // Refs: Self-Refine needs usable FEEDBACK text; ST V2 needs valid JSON.
        let agent = build_agent_builder()
            .map_err(|err| match err {
                crate::model::ModelError::MissingApiKey(name) => {
                    CharacterError::MissingApiKey(name)
                }
                crate::model::ModelError::ClientBuild(msg) => CharacterError::ClientBuild(msg),
            })?
            .preamble(system)
            .max_tokens(CARD_MAX_TOKENS)
            .additional_params(json!({ "thinking": { "type": "disabled" } }))
            .build();

        let text = agent
            .prompt(user)
            .await
            .map_err(|err| CharacterError::Llm(err.to_string()))?;
        if text.trim().is_empty() {
            return Err(CharacterError::Llm(
                "model returned empty content (check max_tokens / thinking budget)".to_owned(),
            ));
        }
        Ok(text)
    }
}

/// Outcome of [`create_card`].
#[derive(Debug, Clone)]
pub struct CreateCardOutcome {
    /// Final validated card (with lorebook attached when seedable).
    pub card: TavernCardV2,
    /// Last critique after the final draft/refine.
    pub critique: CritiqueReport,
    /// How many refine calls ran (0..=[`MAX_REFINE_ROUNDS`]).
    pub refine_rounds: u8,
    /// Memory-stream metadata seeds (Phase 4a; no vectors).
    pub memory: MemoryStream,
    /// Ego knowledge-graph seeds (Phase 4a).
    pub kg: KnowledgeGraph,
}

/// Create a card from a free-text concept via Self-Refine.
///
/// # Errors
///
/// Empty concept, LLM failure, unparseable JSON (after one repair), or hard
/// validation failure after the loop.
pub async fn create_card(concept: &str, llm: &impl LlmBackend) -> Result<CreateCardOutcome> {
    let concept = concept.trim();
    if concept.is_empty() {
        return Err(CharacterError::EmptyConcept);
    }

    let draft_raw = llm
        .complete(META_SYSTEM, &render_user_create(concept))
        .await?;
    let mut card = parse_card_with_repair(llm, &draft_raw).await?;

    let mut critique = run_critique(llm, &card).await?;
    let mut refine_rounds = 0_u8;

    while critique.needs_refine() && refine_rounds < MAX_REFINE_ROUNDS {
        let card_json = to_compact_json(&card)?;
        let critique_json = to_compact_json(&critique)?;
        let refine_raw = llm
            .complete(META_SYSTEM, &render_refine_user(&card_json, &critique_json))
            .await?;
        card = parse_card_with_repair(llm, &refine_raw).await?;
        refine_rounds = refine_rounds.saturating_add(1);
        critique = run_critique(llm, &card).await?;
    }

    validate_card(&card)?;

    let artifacts = seed_card_artifacts(&mut card, None);
    // Lorebook attachment is schema-compatible; re-validate for safety.
    validate_card(&card)?;

    Ok(CreateCardOutcome {
        card,
        critique,
        refine_rounds,
        memory: artifacts.memory,
        kg: artifacts.kg,
    })
}

/// Live convenience: `create_card` with [`RigLlm`].
///
/// # Errors
///
/// Same as [`create_card`].
pub async fn create_card_live(concept: &str) -> Result<CreateCardOutcome> {
    create_card(concept, &RigLlm).await
}

async fn run_critique(llm: &impl LlmBackend, card: &TavernCardV2) -> Result<CritiqueReport> {
    let card_json = to_compact_json(card)?;
    let raw = llm
        .complete(META_SYSTEM, &render_critique_user(&card_json))
        .await?;
    let extracted = extract_json_object(&raw)
        .ok_or_else(|| CharacterError::Parse("critique reply has no json object".to_owned()))?;
    CritiqueReport::from_json_str(extracted)
}

async fn parse_card_with_repair(llm: &impl LlmBackend, text: &str) -> Result<TavernCardV2> {
    // Best-effort artifact for live debugging (gitignored `data/`).
    let _ = std::fs::create_dir_all("data/characters");
    let _ = std::fs::write("data/characters/last_llm_card_raw.txt", text);

    match parse_card_json(text) {
        Ok(card) => Ok(card),
        Err(first) => {
            let repair_user = format!(
                "将以下内容修复为合法的 SillyTavern 根 JSON。\
必须包含顶层字段: \"spec\":\"chara_card_v2\", \"spec_version\":\"2.0\", \"data\":{{...}}。\
relationships 每项用 {{\"name\",\"type\",\"defines_protagonist_how\"}}。\
仅输出 JSON，无 markdown 围栏、无解释：\n\n{text}"
            );
            let fixed = llm.complete(META_SYSTEM, &repair_user).await?;
            let _ = std::fs::write("data/characters/last_llm_card_repair_raw.txt", &fixed);
            parse_card_json(&fixed).map_err(|second| {
                CharacterError::Parse(format!(
                    "after repair: {second}; first: {first}; raw_len={}",
                    text.len()
                ))
            })
        }
    }
}

fn parse_card_json(text: &str) -> Result<TavernCardV2> {
    let extracted = extract_json_object(text).ok_or_else(|| {
        CharacterError::Parse(format!(
            "card reply has no json object (len={})",
            text.len()
        ))
    })?;
    if let Ok(card) = serde_json::from_str::<TavernCardV2>(extracted) {
        return Ok(card);
    }
    let value: Value = serde_json::from_str(extracted)
        .map_err(|err| CharacterError::Parse(format!("card json: {err}")))?;
    coerce_to_v2_card(value)
}

/// Accept common LLM shapes: missing `spec`, or flat `data` payload as root.
fn coerce_to_v2_card(value: Value) -> Result<TavernCardV2> {
    let obj = value
        .as_object()
        .ok_or_else(|| CharacterError::Parse("card json root must be an object".to_owned()))?;

    let wrapped = if obj.contains_key("data") {
        let mut v = value;
        if v.get("spec").is_none() {
            v["spec"] = json!(TavernCardV2::SPEC);
        }
        if v.get("spec_version").is_none() {
            v["spec_version"] = json!(TavernCardV2::SPEC_VERSION);
        }
        v
    } else if obj.contains_key("name") {
        // Flat CharData-shaped object
        json!({
            "spec": TavernCardV2::SPEC,
            "spec_version": TavernCardV2::SPEC_VERSION,
            "data": value,
        })
    } else {
        return Err(CharacterError::Parse(
            "card json missing `data` or top-level `name`".to_owned(),
        ));
    };

    serde_json::from_value(wrapped)
        .map_err(|err| CharacterError::Parse(format!("card json after coerce: {err}")))
}

/// Pull the outermost `{ ... }` object; strips common markdown fences first.
#[must_use]
pub fn extract_json_object(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    let unfenced = strip_markdown_fence(trimmed);
    let start = unfenced.find('{')?;
    let end = unfenced.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&unfenced[start..=end])
}

fn strip_markdown_fence(text: &str) -> &str {
    let t = text.trim();
    let Some(after_open) = t.strip_prefix("```") else {
        return t;
    };
    // optional language tag on first line
    let body = after_open
        .strip_prefix("json")
        .or_else(|| after_open.strip_prefix("JSON"))
        .unwrap_or(after_open);
    let body = body.strip_prefix('\n').unwrap_or(body);
    body.strip_suffix("```").map_or(body, str::trim)
}

fn to_compact_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(CharacterError::from)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::character::card::{CardExtensions, NovelAgentCharExt, RelationshipNode};
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct ScriptedLlm {
        responses: Mutex<VecDeque<String>>,
    }

    impl ScriptedLlm {
        fn new(responses: impl IntoIterator<Item = String>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
            }
        }
    }

    impl LlmBackend for ScriptedLlm {
        async fn complete(&self, _system: &str, _user: &str) -> Result<String> {
            let mut q = self.responses.lock().expect("scripted lock");
            q.pop_front()
                .ok_or_else(|| CharacterError::Llm("scripted responses exhausted".to_owned()))
        }
    }

    fn valid_card_json(name: &str) -> String {
        let card = sample_card(name);
        serde_json::to_string(&card).expect("serialize sample")
    }

    fn sample_card(name: &str) -> TavernCardV2 {
        let mut card = TavernCardV2::skeleton_zh(name);
        card.data.description = "雨夜便利店夜班店员".to_owned();
        card.data.personality = "寡言".to_owned();
        card.data.scenario = "暴雨停电前".to_owned();
        card.data.first_mes = "……灯又闪了。".to_owned();
        card.data.mes_example = "<START>\n{{user}}: 你好\n{{char}}: 嗯。要热饭吗。\n".to_owned();
        card.data.system_prompt = "你是{{char}}。只扮演{{char}}。禁止代写{{user}}。".to_owned();
        card.data.post_history_instructions = "保持短句；禁止全知。".to_owned();
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                desire: "撑完这班".to_owned(),
                need: "承认害怕孤独".to_owned(),
                weakness: "不求人".to_owned(),
                moral_axis: "自保 vs 伸出援手".to_owned(),
                knowledge_bounds: "不知道用户真名".to_owned(),
                voice_markers: vec!["短句".to_owned()],
                constraints: vec![
                    "C-TOM".to_owned(),
                    "C-NO-USER".to_owned(),
                    "C-VOICE".to_owned(),
                    "C-DESIRE-NEED".to_owned(),
                    "C-SCHEMA".to_owned(),
                ],
                locale: "zh-CN".to_owned(),
                relationships: vec![
                    RelationshipNode {
                        name: "店长".to_owned(),
                        relation_type: "boss".to_owned(),
                        defines_protagonist_how: "压榨与秩序".to_owned(),
                    },
                    RelationshipNode {
                        name: "常客".to_owned(),
                        relation_type: "foil".to_owned(),
                        defines_protagonist_how: "对照冷漠".to_owned(),
                    },
                ],
                ..NovelAgentCharExt::default()
            }),
        };
        card
    }

    fn good_critique_json() -> String {
        r#"{
          "scores": {
            "premise": 4,
            "character": 4,
            "voice": 4,
            "tom": 4,
            "constraints": 4
          },
          "flags": {
            "schema_ok": true,
            "placeholders_ok": true,
            "locale_ok": true
          },
          "issues": [],
          "must_fix": [],
          "summary": "达标"
        }"#
        .to_owned()
    }

    fn weak_critique_json() -> String {
        r#"{
          "scores": {
            "premise": 2,
            "character": 3,
            "voice": 3,
            "tom": 3,
            "constraints": 3
          },
          "flags": {
            "schema_ok": true,
            "placeholders_ok": true,
            "locale_ok": true
          },
          "issues": ["前提冲突不够锐"],
          "must_fix": ["强化 desire/need 对立"],
          "summary": "需 refine"
        }"#
        .to_owned()
    }

    #[tokio::test]
    async fn empty_concept_rejected() {
        let llm = ScriptedLlm::new([]);
        let err = create_card("  \n", &llm).await.expect_err("empty");
        assert!(matches!(err, CharacterError::EmptyConcept));
    }

    #[tokio::test]
    async fn happy_path_no_refine() {
        let llm = ScriptedLlm::new([valid_card_json("林晚"), good_critique_json()]);
        let out = create_card("雨夜店员", &llm).await.expect("create");
        assert_eq!(out.card.data.name, "林晚");
        assert_eq!(out.refine_rounds, 0);
        assert!(!out.critique.needs_refine());
        // Phase 4a: lore + mem + kg seeds after loop
        let book = out
            .card
            .data
            .character_book
            .as_ref()
            .expect("lorebook attached");
        assert!(book.entries.len() >= 3);
        assert!(out.memory.entries.len() >= 3);
        assert!(!out.kg.edges.is_empty());
    }

    /// Checkpoint 4a offline: fixture card → card+mem+kg JSON triple under `data/characters/`.
    #[test]
    fn checkpoint_4a_writes_card_mem_kg_artifacts() {
        use crate::character::seed_card_artifacts;

        let mut card = sample_card("苏晚");
        validate_card(&card).expect("fixture valid");
        let art = seed_card_artifacts(&mut card, Some(1_700_000_000));
        validate_card(&card).expect("still valid");

        std::fs::create_dir_all("data/characters").expect("mkdir");
        let card_path = "data/characters/checkpoint_4a_card.json";
        let mem_path = "data/characters/checkpoint_4a_memory.json";
        let kg_path = "data/characters/checkpoint_4a_kg.json";
        std::fs::write(
            card_path,
            serde_json::to_string_pretty(&card).expect("card ser"),
        )
        .expect("write card");
        std::fs::write(mem_path, art.memory.to_json_pretty().expect("mem ser")).expect("write mem");
        std::fs::write(kg_path, art.kg.to_json_pretty().expect("kg ser")).expect("write kg");

        let book = card.data.character_book.expect("book");
        assert!((3..=10).contains(&book.entries.len()));
        assert!(art.memory.entries.len() >= 5);
        assert!(art.kg.nodes.iter().any(|n| n.node_type == "protagonist"));
        assert!(
            std::path::Path::new(card_path).is_file()
                && std::path::Path::new(mem_path).is_file()
                && std::path::Path::new(kg_path).is_file()
        );
    }

    #[tokio::test]
    async fn refine_once_when_critique_weak() {
        let llm = ScriptedLlm::new([
            valid_card_json("林晚"),
            weak_critique_json(),
            valid_card_json("林晚·改"),
            good_critique_json(),
        ]);
        let out = create_card("雨夜店员", &llm).await.expect("create");
        assert_eq!(out.refine_rounds, 1);
        assert_eq!(out.card.data.name, "林晚·改");
        assert!(!out.critique.needs_refine());
    }

    #[tokio::test]
    async fn fenced_card_json_parses() {
        let fenced = format!("```json\n{}\n```", valid_card_json("阿宁"));
        let llm = ScriptedLlm::new([fenced, good_critique_json()]);
        let out = create_card("概念", &llm).await.expect("create");
        assert_eq!(out.card.data.name, "阿宁");
    }

    #[tokio::test]
    async fn repair_once_on_invalid_then_ok() {
        let llm = ScriptedLlm::new([
            "not json at all".to_owned(),
            valid_card_json("修复后"),
            good_critique_json(),
        ]);
        let out = create_card("概念", &llm).await.expect("repaired");
        assert_eq!(out.card.data.name, "修复后");
    }

    #[test]
    fn extract_json_object_from_noise() {
        let raw = "here you go:\n{\"a\":1}\nthanks";
        assert_eq!(extract_json_object(raw), Some(r#"{"a":1}"#));
    }

    #[test]
    fn coerce_wraps_flat_data_with_name() {
        let raw = r#"{"name":"阿宁","description":"店员","extensions":{}}"#;
        let card = parse_card_json(raw).expect("coerce flat");
        assert_eq!(card.spec, TavernCardV2::SPEC);
        assert_eq!(card.data.name, "阿宁");
    }

    #[test]
    fn coerce_fills_missing_spec_on_data_wrapper() {
        let raw = r#"{"data":{"name":"阿宁","description":"x","extensions":{}}}"#;
        let card = parse_card_json(raw).expect("coerce wrapper");
        assert_eq!(card.spec_version, TavernCardV2::SPEC_VERSION);
        assert_eq!(card.data.name, "阿宁");
    }

    /// `extract_json_object` must return `None` for empty / brace-less
    /// input so the parser surfaces a clean `Parse` error instead of
    /// panicking on slice bounds.
    #[test]
    fn extract_json_object_returns_none_for_no_braces() {
        assert_eq!(extract_json_object(""), None);
        assert_eq!(extract_json_object("no json here"), None);
        assert_eq!(extract_json_object("{unclosed"), None);
    }

    /// Fence stripping must accept ```json and ```JSON, plus a bare
    /// ``` pair (no language tag).
    #[test]
    fn strip_fence_handles_json_and_bare() {
        let json_body = r#"{"a":1}"#;
        let cases = [
            format!("```json\n{json_body}\n```"),
            format!("```JSON\n{json_body}\n```"),
            format!("```\n{json_body}\n```"),
        ];
        for c in cases {
            assert_eq!(extract_json_object(&c), Some(json_body), "input: {c}");
        }
    }

    /// When the draft is already valid JSON, `parse_card_with_repair`
    /// must not consume a second scripted response — the repair LLM call
    /// is reserved for failures only.
    #[tokio::test]
    async fn valid_first_parse_skips_repair_call() {
        // Two responses are enough: draft + good critique. If the loop
        // erroneously called repair, the third call would panic with
        // "scripted responses exhausted".
        let llm = ScriptedLlm::new([valid_card_json("阿宁"), good_critique_json()]);
        let out = create_card("概念", &llm).await.expect("create");
        assert_eq!(out.card.data.name, "阿宁");
    }

    /// When the LLM produces unparseable noise AND the repair also fails,
    /// the loop must surface a `Parse` error rather than silently return
    /// a half-built card.
    #[tokio::test]
    async fn repair_failure_yields_parse_error() {
        let llm = ScriptedLlm::new([
            "totally not json".to_owned(),
            "still not json".to_owned(),
            good_critique_json(),
        ]);
        let err = create_card("概念", &llm).await.expect_err("should fail");
        assert!(
            matches!(err, CharacterError::Parse(_)),
            "expected Parse, got {err:?}"
        );
    }

    /// `needs_refine` keeps firing through `MAX_REFINE_ROUNDS` (2) — the
    /// third pass uses the post-loop critique, not a fourth refine.
    #[tokio::test]
    async fn refine_caps_at_max_rounds_then_accepts() {
        // Scripted queue: draft, weak, refined-1, weak, refined-2, weak.
        // The loop should consume 1 draft + 2 refines and exit with
        // refine_rounds == 2 even though the final critique still needs
        // refine (Self-Refine cap, not convergence).
        let llm = ScriptedLlm::new([
            valid_card_json("v0"),
            weak_critique_json(),
            valid_card_json("v1"),
            weak_critique_json(),
            valid_card_json("v2"),
            weak_critique_json(),
        ]);
        let out = create_card("概念", &llm)
            .await
            .expect("create with capped refine");
        assert_eq!(out.refine_rounds, MAX_REFINE_ROUNDS);
        assert_eq!(out.card.data.name, "v2");
    }

    /// LLM errors during critique must propagate; a single bad critique
    /// must not be silently treated as "ok" and skip the loop.
    #[tokio::test]
    async fn llm_error_during_critique_propagates() {
        let llm = ScriptedLlm::new([
            valid_card_json("x"),
            "this is not json for critique".to_owned(),
        ]);
        let err = create_card("概念", &llm).await.expect_err("llm err");
        assert!(matches!(err, CharacterError::Parse(_)));
    }

    /// Live: single short completion via `RigLlm` (isolates HTTP vs loop).
    #[tokio::test]
    #[ignore = "network + OPENCODE_GO_API_KEY"]
    async fn live_rig_short_complete() {
        let _ = dotenvy::dotenv();
        let text = RigLlm
            .complete("You are a concise assistant.", "Reply with exactly: ok")
            .await
            .expect("short complete");
        eprintln!("live_rig_short_complete len={} preview={text}", text.len());
        assert!(!text.trim().is_empty(), "model returned empty content");
    }

    /// Live Checkpoint C: real model, write artifacts under `data/characters/`.
    ///
    /// ```text
    /// cargo test live_create_card_checkpoint_c -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "network + OPENCODE_GO_API_KEY"]
    #[allow(clippy::too_many_lines)]
    async fn live_create_card_checkpoint_c() {
        let _ = dotenvy::dotenv();

        let concept =
            "雨夜便利店的夜班店员，寡言，怕孤独却绝不主动求人，对常客冷淡但对落魄流浪猫温柔";
        std::fs::create_dir_all("data/characters").expect("mkdir data/characters");
        let out = match create_card_live(concept).await {
            Ok(o) => o,
            Err(err) => {
                let fail_path = "data/characters/live_checkpoint_c_error.txt";
                std::fs::write(fail_path, format!("{err}")).expect("write error artifact");
                eprintln!("wrote {fail_path}: {err}");
                panic!("live create_card with OPENCODE_GO_API_KEY: {err}");
            }
        };

        validate_card(&out.card).expect("hard validation after live create");

        let pack = crate::character::assemble_prompt_pack(&out.card);
        let ext = out
            .card
            .data
            .extensions
            .novelagent
            .as_ref()
            .expect("novelagent extension required for product cards");

        let checks = serde_json::json!({
            "concept": concept,
            "refine_rounds": out.refine_rounds,
            "critique": {
                "scores": out.critique.scores,
                "flags": out.critique.flags,
                "summary": out.critique.summary,
                "must_fix": out.critique.must_fix,
                "issues": out.critique.issues,
                "needs_refine_still": out.critique.needs_refine(),
            },
            "quality_gates": {
                "name_nonempty": !out.card.data.name.trim().is_empty(),
                "description_nonempty": !out.card.data.description.trim().is_empty(),
                "personality_nonempty": !out.card.data.personality.trim().is_empty(),
                "scenario_nonempty": !out.card.data.scenario.trim().is_empty(),
                "first_mes_nonempty": !out.card.data.first_mes.trim().is_empty(),
                "mes_example_nonempty": !out.card.data.mes_example.trim().is_empty(),
                "system_prompt_nonempty": !out.card.data.system_prompt.trim().is_empty(),
                "phi_nonempty": !out.card.data.post_history_instructions.trim().is_empty(),
                "has_char_placeholder_or_name_in_system":
                    out.card.data.system_prompt.contains("{{char}}")
                    || out.card.data.system_prompt.contains(&out.card.data.name),
                "desire_nonempty": !ext.desire.trim().is_empty(),
                "need_nonempty": !ext.need.trim().is_empty(),
                "knowledge_bounds_nonempty": !ext.knowledge_bounds.trim().is_empty(),
                "relationships_ge_2": ext.relationships.len() >= 2,
                "voice_markers_nonempty": !ext.voice_markers.is_empty(),
                "has_c_tom": ext.constraints.iter().any(|c| c == "C-TOM"),
                "has_c_no_user": ext.constraints.iter().any(|c| c == "C-NO-USER"),
                "locale_zh": ext.locale.starts_with("zh"),
                "assembled_system_nonempty": !pack.system.is_empty(),
                "assembled_phi_nonempty": !pack.post_history_instructions.is_empty(),
                "has_character_book": out.card.data.character_book.as_ref().is_some_and(|b| !b.entries.is_empty()),
                "memory_seeds_ge_3": out.memory.entries.len() >= 3,
                "kg_edges_nonempty": !out.kg.edges.is_empty(),
            },
            "preview": {
                "name": out.card.data.name,
                "first_mes": out.card.data.first_mes,
                "system_prompt_head": out.card.data.system_prompt.chars().take(240).collect::<String>(),
                "mes_example_head": out.card.data.mes_example.chars().take(320).collect::<String>(),
                "desire": ext.desire,
                "need": ext.need,
                "voice_markers": ext.voice_markers,
            }
        });

        std::fs::create_dir_all("data/characters").expect("mkdir data/characters");
        let card_path = "data/characters/live_checkpoint_c_card.json";
        let report_path = "data/characters/live_checkpoint_c_report.json";
        let mem_path = "data/characters/live_checkpoint_c_memory.json";
        let kg_path = "data/characters/live_checkpoint_c_kg.json";
        let card_json = serde_json::to_string_pretty(&out.card).expect("serialize card");
        std::fs::write(card_path, card_json).expect("write card");
        std::fs::write(
            report_path,
            serde_json::to_string_pretty(&checks).expect("serialize report"),
        )
        .expect("write report");
        std::fs::write(mem_path, out.memory.to_json_pretty().expect("mem")).expect("write mem");
        std::fs::write(kg_path, out.kg.to_json_pretty().expect("kg")).expect("write kg");

        eprintln!("wrote {card_path}");
        eprintln!("wrote {report_path}");
        eprintln!("wrote {mem_path}");
        eprintln!("wrote {kg_path}");
        eprintln!(
            "name={} refine_rounds={} min_score={} needs_refine_still={}",
            out.card.data.name,
            out.refine_rounds,
            out.critique.scores.min(),
            out.critique.needs_refine()
        );

        assert!(
            !out.card.data.name.trim().is_empty(),
            "live card must have a name"
        );
        assert!(
            ext.relationships.len() >= 2,
            "product create prompt requires relationships >= 2"
        );
    }
}
