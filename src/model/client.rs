//! Wiring for the `deepseek-v4-flash` model served by the
//! [`OpenCode`](https://opencode.ai) Go plan.
//!
//! `OpenCode` Go exposes an `OpenAI`-compatible `Chat` Completions API at
//! `https://opencode.ai/zen/go/v1`; we authenticate with the Go API key and
//! reuse Rig's `openai::CompletionsClient` with a custom `base_url`. The
//! `DeepSeek`-specific provider module in Rig is not used here because it
//! targets `DeepSeek`'s own API, not the `OpenCode` gateway.
//!
//! This module exposes a single public builder. Internal steps (read key,
//! build HTTP client, attach model) are inlined — no helper-of-helper layers
//! (see `AGENTS.md` §12).

use std::env;

use rig::agent::AgentBuilder;
use rig::client::{AgentModelExt, CompletionClient};
use rig::providers::openai;

use super::constants::{DEEPSEEK_V4_FLASH_MODEL_ID, OPENCODE_GO_API_KEY_ENV, OPENCODE_GO_BASE_URL};
use super::error::{ModelError, Result};

/// Build an [`AgentBuilder`] preconfigured for `DeepSeek` V4 `Flash` on
/// [`OpenCode`](https://opencode.ai) Go.
///
/// # Errors
///
/// Returns [`ModelError::MissingApiKey`] when `OPENCODE_GO_API_KEY` is not
/// set, or [`ModelError::ClientBuild`] when the HTTP client builder rejects
/// the configuration.
pub fn build_agent_builder() -> Result<AgentBuilder<openai::CompletionModel>> {
    let api_key = env::var(OPENCODE_GO_API_KEY_ENV)
        .map_err(|_| ModelError::MissingApiKey(OPENCODE_GO_API_KEY_ENV))?;

    let client = openai::CompletionsClient::builder()
        .api_key(api_key)
        .base_url(OPENCODE_GO_BASE_URL)
        .build()
        .map_err(|err| ModelError::ClientBuild(err.to_string()))?;

    Ok(client
        .completion_model(DEEPSEEK_V4_FLASH_MODEL_ID)
        .into_agent_builder())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unsafe_derive_deserialize, unsafe_code)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Placeholder test key — not a real secret; env var is always stubbed
    /// in this test module.
    const TEST_API_KEY: &str = "test-key-not-a-real-secret";

    /// Serialises mutations to `OPENCODE_GO_API_KEY` across cargo test's
    /// parallel worker threads. `env::set_var` is `unsafe` in edition 2024
    /// because it may invalidate invariants in other code; holding this lock
    /// while mutating the env keeps the mutation single-threaded within the
    /// test process.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Recover from a poisoned mutex by taking ownership of the inner value.
    /// A poison on this lock is benign for env-var bookkeeping; failing the
    /// test would mask the real assertion.
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[allow(clippy::expect_used)]
    fn with_api_key(body: fn()) {
        let _guard = lock();
        let previous = env::var(OPENCODE_GO_API_KEY_ENV).ok();
        // SAFETY: `ENV_LOCK` above serialises every env mutation in this
        // module, so no other thread observes a partial / clobbered value.
        unsafe {
            env::set_var(OPENCODE_GO_API_KEY_ENV, TEST_API_KEY);
        }
        body();
        match previous {
            Some(value) => unsafe {
                env::set_var(OPENCODE_GO_API_KEY_ENV, value);
            },
            None => unsafe {
                env::remove_var(OPENCODE_GO_API_KEY_ENV);
            },
        }
    }

    #[test]
    fn base_url_constant_points_at_opencode_go() {
        assert_eq!(OPENCODE_GO_BASE_URL, "https://opencode.ai/zen/go/v1");
    }

    #[test]
    fn agent_builder_builds_with_api_key() {
        with_api_key(|| {
            let builder = build_agent_builder().expect("builder should build");
            let _ = builder;
        });
    }

    #[test]
    fn missing_api_key_yields_typed_error() {
        let _guard = lock();
        let previous = env::var(OPENCODE_GO_API_KEY_ENV).ok();
        // SAFETY: see `ENV_LOCK` rationale above.
        unsafe {
            env::remove_var(OPENCODE_GO_API_KEY_ENV);
        }
        // `expect_err` / `assert_eq!` on the `Ok` variant need `Debug` for
        // `AgentBuilder`; we don't have that. Match into a boolean instead
        // and assert with a static message.
        let matches_expected = matches!(
            build_agent_builder(),
            Err(ModelError::MissingApiKey(name)) if name == OPENCODE_GO_API_KEY_ENV,
        );
        if let Some(value) = previous {
            // SAFETY: see `ENV_LOCK` rationale above.
            unsafe {
                env::set_var(OPENCODE_GO_API_KEY_ENV, value);
            }
        }
        assert!(
            matches_expected,
            "expected MissingApiKey for {OPENCODE_GO_API_KEY_ENV}"
        );
    }
}
