//! Prompt assets + assembly of playable system / PHI strings from a card.
//!
//! Meta-agent templates live under `prompts/character/` and are embedded via
//! `include_str!` so the library does not depend on process CWD.
//! Runtime packs for chat inject use [`assemble_prompt_pack`].

use super::card::{NovelAgentCharExt, TavernCardV2};

/// Embedded meta-agent system prompt (Phase 3 loop).
pub const META_SYSTEM: &str = include_str!("../../prompts/character/system_meta_agent.md");
/// User template for `create` (contains `{{concept}}`).
pub const USER_CREATE: &str = include_str!("../../prompts/character/user_create.md");
/// Critique rubric template (contains `{{card_json}}`).
pub const CRITIQUE_RUBRIC: &str = include_str!("../../prompts/character/critique_rubric.md");
/// Refine template (contains `{{card_json}}` / `{{critique_json}}`).
pub const REFINE: &str = include_str!("../../prompts/character/refine.md");

/// Default PHI when the card leaves `post_history_instructions` empty.
const DEFAULT_PHI: &str = "\
只以 {{char}} 的身份回应。禁止代写 {{user}} 的对话、动作或内心。\
禁止总结道德金句收尾。禁止透露 {{char}} 不可能知道的信息。\
情绪用行为与身体细节外化，勿堆砌情绪标签。";

/// Playable prompt surfaces derived from a V2 card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptPack {
    /// System / 扮演契约（已替换 `{{char}}` 若卡片用了占位符则保留约定见下方）。
    pub system: String,
    /// 尾部纠偏（PHI / UJB 位）。
    pub post_history_instructions: String,
    /// description + personality + scenario 拼成的角色上下文块。
    pub role_context: String,
}

/// Replace `{{char}}` with the card name. Leaves `{{user}}` for the host chat layer.
#[must_use]
pub fn apply_char_placeholder(text: &str, char_name: &str) -> String {
    text.replace("{{char}}", char_name)
}

/// Fill `{{concept}}` in the create user template.
#[must_use]
pub fn render_user_create(concept: &str) -> String {
    USER_CREATE.replace("{{concept}}", concept.trim())
}

/// Fill `{{card_json}}` in the critique template.
#[must_use]
pub fn render_critique_user(card_json: &str) -> String {
    CRITIQUE_RUBRIC.replace("{{card_json}}", card_json)
}

/// Fill card + critique JSON into the refine template.
#[must_use]
pub fn render_refine_user(card_json: &str, critique_json: &str) -> String {
    REFINE
        .replace("{{card_json}}", card_json)
        .replace("{{critique_json}}", critique_json)
}

/// Assemble system + PHI + role context for chat injection.
///
/// - Non-empty `system_prompt` / `post_history_instructions` are used after
///   `{{char}}` → name substitution.
/// - Empty `system_prompt` falls back to a synthesized Chinese contract from
///   core fields + `extensions.novelagent`.
/// - Empty PHI falls back to [`DEFAULT_PHI`] (with `{{char}}` replaced).
#[must_use]
pub fn assemble_prompt_pack(card: &TavernCardV2) -> PromptPack {
    let name = card.data.name.as_str();
    let system = if card.data.system_prompt.trim().is_empty() {
        synthesize_system(card)
    } else {
        apply_char_placeholder(card.data.system_prompt.trim(), name)
    };
    let post_history_instructions = if card.data.post_history_instructions.trim().is_empty() {
        apply_char_placeholder(DEFAULT_PHI, name)
    } else {
        apply_char_placeholder(card.data.post_history_instructions.trim(), name)
    };
    PromptPack {
        system,
        post_history_instructions,
        role_context: assemble_role_context(card),
    }
}

fn assemble_role_context(card: &TavernCardV2) -> String {
    let d = &card.data;
    let mut parts: Vec<String> = Vec::new();
    if !d.description.trim().is_empty() {
        parts.push(format!("【设定】{}", d.description.trim()));
    }
    if !d.personality.trim().is_empty() {
        parts.push(format!("【性格】{}", d.personality.trim()));
    }
    if !d.scenario.trim().is_empty() {
        parts.push(format!("【场景】{}", d.scenario.trim()));
    }
    if let Some(ext) = &d.extensions.novelagent {
        append_ext_context(&mut parts, ext);
    }
    parts.join("\n")
}

fn append_ext_context(parts: &mut Vec<String>, ext: &NovelAgentCharExt) {
    if !ext.desire.trim().is_empty() {
        parts.push(format!("【欲望】{}", ext.desire.trim()));
    }
    if !ext.need.trim().is_empty() {
        parts.push(format!("【需求】{}", ext.need.trim()));
    }
    if !ext.knowledge_bounds.trim().is_empty() {
        parts.push(format!("【所知边界】{}", ext.knowledge_bounds.trim()));
    }
    if !ext.voice_markers.is_empty() {
        parts.push(format!("【声浪】{}", ext.voice_markers.join("；")));
    }
}

fn synthesize_system(card: &TavernCardV2) -> String {
    let name = card.data.name.as_str();
    let mut lines = vec![
        format!("你是{name}。只扮演{name}，绝不扮演用户或其他角色。"),
        "对白与动作使用第二人称互动对象为用户；用中文书写（除非角色设定要求其他语言）。".to_owned(),
        "禁止代写用户的对话、动作或内心。禁止用道德总结金句收尾。".to_owned(),
        "禁止写出角色不可能知道的信息。情绪通过行为与身体细节外化。".to_owned(),
    ];
    if let Some(ext) = &card.data.extensions.novelagent {
        if !ext.desire.trim().is_empty() {
            lines.push(format!("外在目标：{}", ext.desire.trim()));
        }
        if !ext.need.trim().is_empty() {
            lines.push(format!("内在需求：{}", ext.need.trim()));
        }
        if !ext.knowledge_bounds.trim().is_empty() {
            lines.push(format!("所知边界：{}", ext.knowledge_bounds.trim()));
        }
        if !ext.voice_markers.is_empty() {
            lines.push(format!("声浪标记：{}", ext.voice_markers.join("；")));
        }
        if !ext.constraints.is_empty() {
            lines.push(format!("启用约束：{}", ext.constraints.join(", ")));
        }
    }
    if !card.data.personality.trim().is_empty() {
        lines.push(format!("性格要点：{}", card.data.personality.trim()));
    }
    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::character::card::{CardExtensions, NovelAgentCharExt, RelationshipNode};

    fn fixture_card() -> TavernCardV2 {
        let mut card = TavernCardV2::skeleton_zh("林晚");
        card.data.description = "雨夜便利店的夜班店员".to_owned();
        card.data.personality = "寡言，观察力强".to_owned();
        card.data.scenario = "暴雨停电前的十分钟".to_owned();
        card.data.system_prompt = "你是{{char}}。只扮演{{char}}。禁止代写{{user}}。".to_owned();
        card.data.post_history_instructions = "保持{{char}}的短句习惯。".to_owned();
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                desire: "撑完这班".to_owned(),
                need: "承认自己害怕孤独".to_owned(),
                knowledge_bounds: "不知道用户真实姓名".to_owned(),
                voice_markers: vec!["短句".to_owned(), "少形容词".to_owned()],
                constraints: vec!["C-TOM".to_owned(), "C-NO-USER".to_owned()],
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
                        defines_protagonist_how: "对照其冷漠".to_owned(),
                    },
                ],
                ..NovelAgentCharExt::default()
            }),
        };
        card
    }

    #[test]
    fn meta_templates_are_nonempty_and_mention_constraints() {
        assert!(META_SYSTEM.contains("C-TOM"));
        assert!(META_SYSTEM.contains("chara_card_v2"));
        assert!(USER_CREATE.contains("{{concept}}"));
        assert!(CRITIQUE_RUBRIC.contains("premise"));
        assert!(REFINE.contains("{{critique_json}}"));
    }

    #[test]
    fn apply_char_placeholder_replaces_char_keeps_user() {
        let out = apply_char_placeholder("{{char}}对{{user}}点头", "林晚");
        assert_eq!(out, "林晚对{{user}}点头");
    }

    #[test]
    fn assemble_uses_card_system_and_replaces_char() {
        let pack = assemble_prompt_pack(&fixture_card());
        assert!(pack.system.contains("林晚"));
        assert!(!pack.system.contains("{{char}}"));
        assert!(pack.system.contains("{{user}}"));
        assert!(pack.post_history_instructions.contains("林晚"));
        assert!(pack.role_context.contains("雨夜便利店"));
        assert!(pack.role_context.contains("撑完这班"));
        assert!(pack.role_context.contains("不知道用户真实姓名"));
        assert!(pack.role_context.contains("短句"));
    }

    #[test]
    fn empty_system_falls_back_to_synthesized() {
        let mut card = fixture_card();
        card.data.system_prompt.clear();
        card.data.post_history_instructions.clear();
        let pack = assemble_prompt_pack(&card);
        assert!(pack.system.contains("林晚"));
        assert!(pack.system.contains("撑完这班"));
        assert!(pack.system.contains("C-TOM"));
        assert!(pack.post_history_instructions.contains("林晚"));
        assert!(pack.post_history_instructions.contains("禁止代写"));
    }

    #[test]
    fn render_user_create_injects_concept() {
        let s = render_user_create("  赛博僧侣  ");
        assert!(s.contains("赛博僧侣"));
        assert!(!s.contains("{{concept}}"));
    }

    /// The critique user template embeds the full V2 card JSON; the
    /// rendered output must include the literal JSON and no leftover
    /// `{{card_json}}` token.
    #[test]
    fn render_critique_user_injects_card_json() {
        let json = r#"{"spec":"chara_card_v2","spec_version":"2.0","data":{"name":"x"}}"#;
        let out = render_critique_user(json);
        assert!(out.contains(json));
        assert!(!out.contains("{{card_json}}"));
    }

    /// The refine user template embeds both the current card JSON and
    /// the latest critique JSON. Both must appear, both placeholders gone.
    #[test]
    fn render_refine_user_injects_card_and_critique() {
        let card = r#"{"data":{"name":"x"}}"#;
        let crit = r#"{"scores":{"premise":3}}"#;
        let out = render_refine_user(card, crit);
        assert!(out.contains(card));
        assert!(out.contains(crit));
        assert!(!out.contains("{{card_json}}"));
        assert!(!out.contains("{{critique_json}}"));
    }

    /// `role_context` only emits a block when the underlying field is
    /// non-empty — a sparse card must not produce blank 【设定】 lines.
    #[test]
    fn role_context_omits_empty_fields() {
        let card = TavernCardV2::skeleton_zh("x");
        let pack = assemble_prompt_pack(&card);
        assert!(pack.role_context.is_empty());
    }

    /// When `extensions.novelagent` is set, `synthesize_system` must
    /// surface `constraints` so the LLM keeps them enabled after refine.
    #[test]
    fn synthesize_system_includes_constraints_and_voice() {
        let mut card = TavernCardV2::skeleton_zh("阿宁");
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                voice_markers: vec!["短句".to_owned()],
                constraints: vec!["C-VOICE".to_owned()],
                ..NovelAgentCharExt::default()
            }),
        };
        let pack = assemble_prompt_pack(&card);
        assert!(pack.system.contains("声浪标记"));
        assert!(pack.system.contains("启用约束"));
        assert!(pack.system.contains("C-VOICE"));
    }

    /// `apply_char_placeholder` must replace every `{{char}}` occurrence
    /// and never touch `{{user}}` or any other placeholder.
    #[test]
    fn apply_char_placeholder_replaces_all_occurrences() {
        let out = apply_char_placeholder("{{char}}→{{user}}·{{char}}·{{char_name}}", "宁");
        assert_eq!(out, "宁→{{user}}·宁·{{char_name}}");
    }
}
