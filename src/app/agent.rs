//! Agent invocation: build the configured agent and send a prompt.

use anyhow::{Context, Result, bail};
use rig::completion::Prompt;

use crate::app::constants::{ASSISTANT_PREAMBLE, READINESS_PROMPT};
use crate::model::build_agent_builder;

/// Reject blank / whitespace-only user messages before they hit the model.
///
/// # Errors
///
/// Returns an error when `message` is empty after trimming.
pub fn validate_user_message(message: &str) -> Result<&str> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        bail!("message must not be empty");
    }
    Ok(trimmed)
}

/// Build the configured agent and send an arbitrary user prompt.
///
/// # Errors
///
/// Returns an error if the message is empty, the agent builder fails, or the
/// upstream model call fails at runtime.
pub async fn prompt_message(message: &str) -> Result<String> {
    let message = validate_user_message(message)?;
    let agent = build_agent_builder()
        .context("failed to build deepseek v4 flash agent builder")?
        .preamble(ASSISTANT_PREAMBLE)
        .build();

    agent
        .prompt(message)
        .await
        .context("deepseek v4 flash call via opencode go failed")
}

/// Build the configured agent and send the readiness prompt.
///
/// # Errors
///
/// Returns an error if the agent builder fails (missing key, bad config) or
/// if the upstream model call fails at runtime.
pub async fn run_readiness_check() -> Result<String> {
    prompt_message(READINESS_PROMPT).await
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn validate_user_message_rejects_blank() {
        assert!(validate_user_message("").is_err());
        assert!(validate_user_message("   \n\t  ").is_err());
    }

    #[test]
    fn validate_user_message_trims_content() {
        let ok = validate_user_message("  hello  ").expect("non-empty");
        assert_eq!(ok, "hello");
    }
}
