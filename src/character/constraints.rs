//! Hard validation for character cards (schema-level, no LLM).

use super::card::TavernCardV2;
use super::error::{CharacterError, Result};

/// Known constraint IDs used in `extensions.novelagent.constraints`.
pub const KNOWN_CONSTRAINT_IDS: &[&str] = &[
    "C-VOICE",
    "C-SUBTEXT",
    "C-DESIRE-NEED",
    "C-NETWORK",
    "C-TOM",
    "C-NO-USER",
    "C-NO-BUTTON",
    "C-EMOTION",
    "C-BUDGET",
    "C-SCHEMA",
];

/// Validate a card for export / storage.
///
/// # Errors
///
/// Returns [`CharacterError::Validation`] when hard rules fail.
pub fn validate_card(card: &TavernCardV2) -> Result<()> {
    if card.spec != TavernCardV2::SPEC {
        return Err(CharacterError::Validation(format!(
            "spec must be `{}`, got `{}`",
            TavernCardV2::SPEC,
            card.spec
        )));
    }
    if card.spec_version != TavernCardV2::SPEC_VERSION {
        return Err(CharacterError::Validation(format!(
            "spec_version must be `{}`, got `{}`",
            TavernCardV2::SPEC_VERSION,
            card.spec_version
        )));
    }
    if card.data.name.trim().is_empty() {
        return Err(CharacterError::Validation(
            "data.name must be non-empty".to_owned(),
        ));
    }
    if let Some(ext) = &card.data.extensions.novelagent {
        for id in &ext.constraints {
            if !KNOWN_CONSTRAINT_IDS.contains(&id.as_str()) {
                return Err(CharacterError::Validation(format!(
                    "unknown constraint id `{id}`"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::character::card::{CardExtensions, NovelAgentCharExt};

    #[test]
    fn accepts_skeleton() {
        let card = TavernCardV2::skeleton_zh("阿宁");
        validate_card(&card).expect("skeleton ok");
    }

    #[test]
    fn rejects_empty_name() {
        let mut card = TavernCardV2::skeleton_zh("x");
        card.data.name = "   ".to_owned();
        assert!(validate_card(&card).is_err());
    }

    #[test]
    fn rejects_unknown_constraint() {
        let mut card = TavernCardV2::skeleton_zh("阿宁");
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                constraints: vec!["C-FAKE".to_owned()],
                locale: "zh-CN".to_owned(),
                ..NovelAgentCharExt::default()
            }),
        };
        assert!(validate_card(&card).is_err());
    }

    #[test]
    fn accepts_known_constraint() {
        let mut card = TavernCardV2::skeleton_zh("阿宁");
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                constraints: vec!["C-TOM".to_owned(), "C-NO-USER".to_owned()],
                locale: "zh-CN".to_owned(),
                ..NovelAgentCharExt::default()
            }),
        };
        validate_card(&card).expect("known constraints ok");
    }

    /// Reject any spec other than `chara_card_v2`, even a near-miss like v3.
    #[test]
    fn rejects_wrong_spec() {
        let mut card = TavernCardV2::skeleton_zh("x");
        card.spec = "chara_card_v3".to_owned();
        let err = validate_card(&card).expect_err("wrong spec");
        let msg = err.to_string();
        assert!(msg.contains("spec must be"), "msg: {msg}");
    }

    /// Reject any `spec_version` other than `"2.0"`.
    #[test]
    fn rejects_wrong_spec_version() {
        let mut card = TavernCardV2::skeleton_zh("x");
        card.spec_version = "1.5".to_owned();
        let err = validate_card(&card).expect_err("wrong version");
        let msg = err.to_string();
        assert!(msg.contains("spec_version must be"), "msg: {msg}");
    }

    /// Trim whitespace before checking non-empty so a name of `"   "` is
    /// not accepted just because it is non-empty bytes.
    #[test]
    fn rejects_whitespace_only_name() {
        let mut card = TavernCardV2::skeleton_zh("x");
        card.data.name = "\t \n".to_owned();
        assert!(validate_card(&card).is_err());
    }

    /// A single-character name must be accepted; the rule is `non-empty`,
    /// not `length >= N`.
    #[test]
    fn accepts_single_char_name() {
        let card = TavernCardV2::skeleton_zh("宁");
        validate_card(&card).expect("single char ok");
    }

    /// When multiple constraint IDs are unknown, the first one is reported
    /// with its literal name (helps the LLM refine loop fix the right row).
    #[test]
    fn reports_first_unknown_constraint_id() {
        let mut card = TavernCardV2::skeleton_zh("x");
        card.data.extensions = CardExtensions {
            novelagent: Some(NovelAgentCharExt {
                constraints: vec!["C-TOM".to_owned(), "C-INVENTED".to_owned()],
                locale: "zh-CN".to_owned(),
                ..NovelAgentCharExt::default()
            }),
        };
        let err = validate_card(&card).expect_err("unknown id");
        assert!(err.to_string().contains("C-INVENTED"));
    }

    /// An empty `constraints: []` is the default and must validate.
    #[test]
    fn accepts_empty_constraints() {
        let card = TavernCardV2::skeleton_zh("x");
        validate_card(&card).expect("no constraints is fine");
    }
}
