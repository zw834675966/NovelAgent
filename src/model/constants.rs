//! Static configuration for the upstream model.

/// `OpenAI`-compatible base URL for the [`OpenCode`](https://opencode.ai) Go plan.
pub const OPENCODE_GO_BASE_URL: &str = "https://opencode.ai/zen/go/v1";

/// Model identifier for `DeepSeek` V4 `Flash` on [`OpenCode`](https://opencode.ai) Go.
pub const DEEPSEEK_V4_FLASH_MODEL_ID: &str = "deepseek-v4-flash";

/// Environment variable that holds the [`OpenCode`](https://opencode.ai) Go API key.
pub const OPENCODE_GO_API_KEY_ENV: &str = "OPENCODE_GO_API_KEY";
