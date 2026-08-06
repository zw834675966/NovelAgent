//! `SillyTavern` Character Card V2 types + `NovelAgent` extensions.
//!
//! Field shapes follow the community V2 spec
//! (`chara_card_v2` / `spec_version` `2.0`) and `SillyTavern`'s `v2CharData`.
//! `NovelAgent`-only fields live under `data.extensions.novelagent` so cards
//! remain importable elsewhere.

use serde::{Deserialize, Deserializer, Serialize};

/// V2 root wrapper (PNG/JSON export envelope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TavernCardV2 {
    /// Must be `"chara_card_v2"`.
    pub spec: String,
    /// Must be `"2.0"`.
    pub spec_version: String,
    /// Card payload.
    pub data: CharDataV2,
}

/// V2 character payload (permanent defs + control surfaces + lorebook).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharDataV2 {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub first_mes: String,
    #[serde(default)]
    pub mes_example: String,
    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub post_history_instructions: String,
    #[serde(default)]
    pub alternate_greetings: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub character_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_book: Option<CharacterBook>,
    #[serde(default)]
    pub extensions: CardExtensions,
}

/// Extension bag; unknown keys preserved via `other` is out of scope for v0 —
/// we only model the `novelagent` namespace we own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CardExtensions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub novelagent: Option<NovelAgentCharExt>,
}

/// NovelAgent-only character engineering fields (not ST-native).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NovelAgentCharExt {
    /// 外在欲望（可见目标）。
    #[serde(default)]
    pub desire: String,
    /// 内在需求（须克服的缺陷/错误认知）。
    #[serde(default)]
    pub need: String,
    /// 开局致命弱点。
    #[serde(default)]
    pub weakness: String,
    /// 道德/主题张力轴。
    #[serde(default)]
    pub moral_axis: String,
    /// 情绪弧：允许短标签字符串或 `{trigger, response}` 节拍对象（LLM 两种都常见）。
    #[serde(default, deserialize_with = "deserialize_emotion_arc")]
    pub emotion_arc: Vec<EmotionBeat>,
    /// 人物网络节点。
    #[serde(default)]
    pub relationships: Vec<RelationshipNode>,
    /// 声浪标记（用词/句式/禁忌）。
    #[serde(default)]
    pub voice_markers: Vec<String>,
    /// 启用的约束 ID（如 `C-TOM`）。
    #[serde(default)]
    pub constraints: Vec<String>,
    /// 所知边界说明。
    #[serde(default)]
    pub knowledge_bounds: String,
    /// 默认内容语言（BCP-47）；产品默认 `zh-CN`。
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "zh-CN".to_owned()
}

/// One beat on the emotional arc (tag and/or concrete trigger→response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EmotionBeat {
    /// Optional short tag (e.g. `hope` / `fear`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tag: String,
    /// Situation that fires the beat.
    #[serde(default)]
    pub trigger: String,
    /// Character response / behavior.
    #[serde(default)]
    pub response: String,
}

/// One node in the character network (defines the protagonist from an angle).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipNode {
    #[serde(default)]
    pub name: String,
    /// e.g. `ally` / `rival` / `mentor` / `foil`.
    ///
    /// Serialize as ST-style `"type"`; accept LLM aliases on deserialize.
    #[serde(
        rename = "type",
        alias = "relation_type",
        alias = "relation",
        alias = "role",
        default
    )]
    pub relation_type: String,
    /// How this character defines the protagonist.
    #[serde(
        default,
        alias = "defines_how",
        alias = "how",
        alias = "definition_of_self"
    )]
    pub defines_protagonist_how: String,
}

fn deserialize_emotion_arc<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<EmotionBeat>, D::Error>
where
    D: Deserializer<'de>,
{
    let values: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        match value {
            serde_json::Value::String(tag) => out.push(EmotionBeat {
                tag,
                ..EmotionBeat::default()
            }),
            other => {
                let beat: EmotionBeat =
                    serde_json::from_value(other).map_err(serde::de::Error::custom)?;
                out.push(beat);
            }
        }
    }
    Ok(out)
}

/// Embedded lorebook (ST `character_book`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CharacterBook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recursive_scanning: Option<bool>,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub entries: Vec<LoreEntry>,
}

/// One lorebook entry (keyword-triggered or constant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEntry {
    pub keys: Vec<String>,
    pub content: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub insertion_order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selective: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_keys: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl TavernCardV2 {
    /// Spec constant for V2 cards.
    pub const SPEC: &'static str = "chara_card_v2";
    /// Spec version constant.
    pub const SPEC_VERSION: &'static str = "2.0";

    /// Build an empty-but-valid Chinese skeleton card with a name.
    #[must_use]
    pub fn skeleton_zh(name: impl Into<String>) -> Self {
        Self {
            spec: Self::SPEC.to_owned(),
            spec_version: Self::SPEC_VERSION.to_owned(),
            data: CharDataV2 {
                name: name.into(),
                description: String::new(),
                personality: String::new(),
                scenario: String::new(),
                first_mes: String::new(),
                mes_example: String::new(),
                creator_notes: String::new(),
                system_prompt: String::new(),
                post_history_instructions: String::new(),
                alternate_greetings: Vec::new(),
                tags: vec!["zh-CN".to_owned()],
                creator: "NovelAgent".to_owned(),
                character_version: "0.1.0".to_owned(),
                character_book: None,
                extensions: CardExtensions {
                    novelagent: Some(NovelAgentCharExt {
                        locale: default_locale(),
                        ..NovelAgentCharExt::default()
                    }),
                },
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_roundtrips_json() {
        let card = TavernCardV2::skeleton_zh("林晚");
        let json = serde_json::to_string_pretty(&card).expect("serialize");
        let back: TavernCardV2 = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.spec, TavernCardV2::SPEC);
        assert_eq!(back.data.name, "林晚");
        let ext = back
            .data
            .extensions
            .novelagent
            .expect("novelagent ext present");
        assert_eq!(ext.locale, "zh-CN");
    }

    #[test]
    fn accepts_minimal_st_v2_fixture() {
        let raw = r#"{
          "spec": "chara_card_v2",
          "spec_version": "2.0",
          "data": {
            "name": "测试角色",
            "description": "简述",
            "personality": "冷静",
            "scenario": "雨夜便利店",
            "first_mes": "……又是你。",
            "mes_example": "",
            "system_prompt": "只扮演测试角色。",
            "post_history_instructions": "禁止代写用户言行。",
            "extensions": {
              "novelagent": {
                "desire": "查出真相",
                "need": "承认自己需要他人",
                "locale": "zh-CN"
              }
            }
          }
        }"#;
        let card: TavernCardV2 = serde_json::from_str(raw).expect("parse fixture");
        assert_eq!(card.data.name, "测试角色");
        assert_eq!(
            card.data
                .extensions
                .novelagent
                .as_ref()
                .map(|e| e.desire.as_str()),
            Some("查出真相")
        );
    }

    /// LLM cards often emit emotion arcs as bare strings; the deserializer
    /// must accept both a `tag` string and a `{trigger, response}` object.
    #[test]
    fn emotion_arc_accepts_mixed_strings_and_objects() {
        let raw = r#"{
          "spec": "chara_card_v2",
          "spec_version": "2.0",
          "data": {
            "name": "x",
            "extensions": {
              "novelagent": {
                "emotion_arc": [
                  "hope",
                  "fear",
                  { "trigger": "失去", "response": "沉默" },
                  { "tag": "resolve", "trigger": "决定", "response": "动身" }
                ]
              }
            }
          }
        }"#;
        let card: TavernCardV2 = serde_json::from_str(raw).expect("parse emotion_arc");
        let ext = card.data.extensions.novelagent.expect("ext");
        assert_eq!(ext.emotion_arc.len(), 4);
        assert_eq!(ext.emotion_arc[0].tag, "hope");
        assert!(ext.emotion_arc[0].trigger.is_empty());
        assert_eq!(ext.emotion_arc[2].trigger, "失去");
        assert_eq!(ext.emotion_arc[2].response, "沉默");
        assert_eq!(ext.emotion_arc[3].tag, "resolve");
    }

    /// LLMs frequently swap `relation_type` for `role` / `relation` and
    /// `defines_protagonist_how` for `how` / `definition_of_self`. The
    /// deserializer aliases all of these to keep the loop tolerant.
    #[test]
    fn relationship_node_accepts_llm_aliases() {
        let raw = r#"{
          "spec": "chara_card_v2",
          "spec_version": "2.0",
          "data": {
            "name": "x",
            "extensions": {
              "novelagent": {
                "relationships": [
                  { "name": "店长", "type": "boss", "defines_protagonist_how": "秩序" },
                  { "name": "常客", "role": "foil", "how": "冷漠" },
                  { "name": "旧友", "relation": "ally", "defines_how": "信任" },
                  { "name": "对手", "relation_type": "rival", "definition_of_self": "镜像" }
                ]
              }
            }
          }
        }"#;
        let card: TavernCardV2 = serde_json::from_str(raw).expect("parse relationships");
        let ext = card.data.extensions.novelagent.expect("ext");
        assert_eq!(ext.relationships.len(), 4);
        assert_eq!(ext.relationships[0].relation_type, "boss");
        assert_eq!(ext.relationships[0].defines_protagonist_how, "秩序");
        assert_eq!(ext.relationships[1].relation_type, "foil");
        assert_eq!(ext.relationships[1].defines_protagonist_how, "冷漠");
        assert_eq!(ext.relationships[2].relation_type, "ally");
        assert_eq!(ext.relationships[2].defines_protagonist_how, "信任");
        assert_eq!(ext.relationships[3].relation_type, "rival");
        assert_eq!(ext.relationships[3].defines_protagonist_how, "镜像");
    }

    /// Serde is permissive on unknown top-level keys by default; the spec
    /// invariant is enforced by `constraints::validate_card`, not by the
    /// deserializer. This pins the current behaviour.
    #[test]
    fn skeleton_ignores_unknown_extension_keys() {
        let raw = r#"{
          "spec": "chara_card_v2",
          "spec_version": "2.0",
          "data": {
            "name": "x",
            "extensions": {
              "novelagent": { "locale": "zh-CN" },
              "sillytavern_extra": { "ignored": true }
            }
          }
        }"#;
        let card: TavernCardV2 = serde_json::from_str(raw).expect("parse with extras");
        assert_eq!(card.data.name, "x");
        assert_eq!(
            card.data
                .extensions
                .novelagent
                .as_ref()
                .map(|e| e.locale.as_str()),
            Some("zh-CN")
        );
    }
}
