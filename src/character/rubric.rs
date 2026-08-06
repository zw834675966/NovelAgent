//! Five-dimension critique report for character-card Self-Refine.

use serde::{Deserialize, Serialize};

use super::error::{CharacterError, Result};

/// Default: any score below this triggers refine (plan: score &lt; 3).
pub const SCORE_THRESHOLD: u8 = 3;

/// Max refine rounds after the initial draft (plan / DISTILL: ≤2).
pub const MAX_REFINE_ROUNDS: u8 = 2;

/// Per-dimension scores (1–5 integers).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionScores {
    pub premise: u8,
    pub character: u8,
    pub voice: u8,
    pub tom: u8,
    pub constraints: u8,
}

impl DimensionScores {
    /// Lowest dimension score.
    #[must_use]
    pub fn min(&self) -> u8 {
        [
            self.premise,
            self.character,
            self.voice,
            self.tom,
            self.constraints,
        ]
        .into_iter()
        .min()
        .unwrap_or(0)
    }

    /// True when any score is below [`SCORE_THRESHOLD`].
    #[must_use]
    pub fn below_threshold(&self) -> bool {
        self.min() < SCORE_THRESHOLD
    }
}

/// Boolean hard checks from the critique rubric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CritiqueFlags {
    pub schema_ok: bool,
    pub placeholders_ok: bool,
    pub locale_ok: bool,
}

impl CritiqueFlags {
    /// True when every hard flag is ok.
    #[must_use]
    pub fn all_ok(&self) -> bool {
        self.schema_ok && self.placeholders_ok && self.locale_ok
    }
}

/// Full critique payload (matches `prompts/character/critique_rubric.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CritiqueReport {
    pub scores: DimensionScores,
    pub flags: CritiqueFlags,
    #[serde(default)]
    pub issues: Vec<String>,
    #[serde(default)]
    pub must_fix: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

impl CritiqueReport {
    /// Parse critique JSON from an LLM reply (fence-tolerant via caller extract).
    ///
    /// # Errors
    ///
    /// Returns [`CharacterError::Parse`] when JSON is invalid or scores are out of 1–5.
    pub fn from_json_str(raw: &str) -> Result<Self> {
        let report: Self = serde_json::from_str(raw.trim())
            .map_err(|err| CharacterError::Parse(format!("critique json: {err}")))?;
        report.validate_ranges()?;
        Ok(report)
    }

    /// Whether Self-Refine should run another refine pass.
    ///
    /// Aligns with Self-Refine: **actionable** feedback drives rewrite — not only
    /// numeric scores. Non-empty `must_fix` always triggers refine.
    #[must_use]
    pub fn needs_refine(&self) -> bool {
        self.scores.below_threshold() || !self.flags.schema_ok || !self.must_fix.is_empty()
    }

    fn validate_ranges(&self) -> Result<()> {
        for (name, score) in [
            ("premise", self.scores.premise),
            ("character", self.scores.character),
            ("voice", self.scores.voice),
            ("tom", self.scores.tom),
            ("constraints", self.scores.constraints),
        ] {
            if !(1..=5).contains(&score) {
                return Err(CharacterError::Parse(format!(
                    "critique score `{name}` must be 1..=5, got {score}"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_critique() {
        let raw = r#"{
          "scores": {
            "premise": 4,
            "character": 3,
            "voice": 5,
            "tom": 4,
            "constraints": 3
          },
          "flags": {
            "schema_ok": true,
            "placeholders_ok": true,
            "locale_ok": true
          },
          "issues": ["mes_example 偏说明书"],
          "must_fix": [],
          "summary": "可用"
        }"#;
        let report = CritiqueReport::from_json_str(raw).expect("parse");
        assert_eq!(report.scores.min(), 3);
        assert!(!report.needs_refine());
    }

    #[test]
    fn must_fix_triggers_refine_even_if_scores_ok() {
        let raw = r#"{
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
          "must_fix": ["补 defines_protagonist_how"],
          "summary": "有硬债"
        }"#;
        let report = CritiqueReport::from_json_str(raw).expect("parse");
        assert!(report.needs_refine());
    }

    #[test]
    fn low_score_needs_refine() {
        let raw = r#"{
          "scores": {
            "premise": 2,
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
          "must_fix": ["补全 desire/need 冲突"],
          "summary": "前提弱"
        }"#;
        let report = CritiqueReport::from_json_str(raw).expect("parse");
        assert!(report.needs_refine());
    }

    #[test]
    fn schema_flag_false_needs_refine() {
        let raw = r#"{
          "scores": {
            "premise": 5,
            "character": 5,
            "voice": 5,
            "tom": 5,
            "constraints": 5
          },
          "flags": {
            "schema_ok": false,
            "placeholders_ok": true,
            "locale_ok": true
          },
          "issues": [],
          "must_fix": ["缺 name"],
          "summary": "schema 坏"
        }"#;
        let report = CritiqueReport::from_json_str(raw).expect("parse");
        assert!(report.needs_refine());
    }

    #[test]
    fn out_of_range_score_rejected() {
        let raw = r#"{
          "scores": {
            "premise": 0,
            "character": 3,
            "voice": 3,
            "tom": 3,
            "constraints": 3
          },
          "flags": {
            "schema_ok": true,
            "placeholders_ok": true,
            "locale_ok": true
          }
        }"#;
        assert!(CritiqueReport::from_json_str(raw).is_err());
    }

    /// A score above 5 is also out of range — the rubric is 1..=5.
    #[test]
    fn score_above_five_rejected() {
        let raw = r#"{
          "scores": {
            "premise": 6,
            "character": 3,
            "voice": 3,
            "tom": 3,
            "constraints": 3
          },
          "flags": {
            "schema_ok": true,
            "placeholders_ok": true,
            "locale_ok": true
          }
        }"#;
        let err = CritiqueReport::from_json_str(raw).expect_err("score > 5");
        assert!(err.to_string().contains("premise"));
    }

    /// All scores equal must yield `min() == score` and `below_threshold`
    /// only true when the score itself is below 3.
    #[test]
    fn all_equal_scores_min_is_the_score() {
        let raw = r#"{
          "scores": {
            "premise": 3,
            "character": 3,
            "voice": 3,
            "tom": 3,
            "constraints": 3
          },
          "flags": { "schema_ok": true, "placeholders_ok": true, "locale_ok": true }
        }"#;
        let report = CritiqueReport::from_json_str(raw).expect("parse");
        assert_eq!(report.scores.min(), 3);
        assert!(!report.scores.below_threshold());
        assert!(!report.needs_refine());
    }

    /// `issues` is informational; only `must_fix` or low scores trigger
    /// refine. An issues-only report must not loop.
    #[test]
    fn issues_alone_do_not_trigger_refine() {
        let raw = r#"{
          "scores": {
            "premise": 4,
            "character": 4,
            "voice": 4,
            "tom": 4,
            "constraints": 4
          },
          "flags": { "schema_ok": true, "placeholders_ok": true, "locale_ok": true },
          "issues": ["mes_example 偏说明书", "声浪可更锐"],
          "must_fix": [],
          "summary": "可用"
        }"#;
        let report = CritiqueReport::from_json_str(raw).expect("parse");
        assert!(!report.needs_refine());
    }

    /// `all_ok` is the conjunction of every flag; a single false is enough
    /// to make it false. (The refine trigger uses `schema_ok` directly, so
    /// this also pins the helper's contract.)
    #[test]
    fn flags_all_ok_requires_every_flag() {
        let ok = CritiqueFlags {
            schema_ok: true,
            placeholders_ok: true,
            locale_ok: true,
        };
        assert!(ok.all_ok());

        let bad = CritiqueFlags {
            schema_ok: true,
            placeholders_ok: false,
            locale_ok: true,
        };
        assert!(!bad.all_ok());
    }
}
