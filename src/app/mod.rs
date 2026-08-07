//! Application-level orchestration: environment bootstrap + agent run.
//!
//! `main.rs` is the only caller. Keep this module's public surface flat — no
//! re-export wrappers, no builder-of-builder patterns.

pub mod agent;
pub mod character_cmd;
pub mod constants;
pub mod env;

pub use agent::{prompt_message, run_readiness_check, validate_user_message};
pub use character_cmd::{
    character_chat, character_create, character_delete, character_list, character_regenerate,
    format_character_list_summary, format_delete_summary,
};
pub use env::load_environment;

use anyhow::{Context, Result, bail};

/// One-call entry: load environment, build agent, send readiness prompt.
///
/// # Errors
///
/// Propagates any error from [`load_environment`] (missing API key) or
/// [`run_readiness_check`] (builder / upstream model failure).
pub async fn bootstrap() -> Result<String> {
    load_environment()?;
    run_readiness_check().await
}

/// Binary entry after env load: CLI subcommands or Topcoat web server.
///
/// | args | action |
/// |------|--------|
/// | _(empty)_ | start Topcoat chat UI |
/// | `character-create <concept…>` | live create + write `data/characters/` |
/// | `character-chat <slug> <msg…>` | one turn as saved card |
/// | `character-list` | enumerate saved cards under `data/characters/` |
/// | `character-delete <slug>` | remove all sidecar files for a saved card |
/// | `character-regenerate <slug>` | re-run Self-Refine with the stored concept |
///
/// # Errors
///
/// Missing args, create/chat/list/delete/regenerate failure, or Topcoat start
/// failure.
pub async fn run(args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        None => {
            let router = crate::web::router()?;
            topcoat::start(router)
                .await
                .map_err(|err| anyhow::anyhow!("{err}"))
        }
        Some("character-create") => {
            let concept = args.get(1..).map_or(String::new(), |parts| parts.join(" "));
            if concept.trim().is_empty() {
                bail!("usage: novelagent character-create <concept…>");
            }
            let summary = character_create(&concept).await?;
            println!("{summary}");
            Ok(())
        }
        Some("character-chat") => {
            let slug = args
                .get(1)
                .map(String::as_str)
                .filter(|s| !s.is_empty())
                .context("usage: novelagent character-chat <slug> <message…>")?;
            let message = args.get(2..).map_or(String::new(), |parts| parts.join(" "));
            if message.trim().is_empty() {
                bail!("usage: novelagent character-chat <slug> <message…>");
            }
            let reply = character_chat(slug, &message).await?;
            println!("{reply}");
            Ok(())
        }
        Some("character-list") => {
            println!("{}", format_character_list_summary(&character_list()?));
            Ok(())
        }
        Some("character-delete") => {
            let slug = args
                .get(1)
                .map(String::as_str)
                .filter(|s| !s.is_empty())
                .context("usage: novelagent character-delete <slug>")?;
            let summary = character_delete(slug)?;
            println!("{summary}");
            Ok(())
        }
        Some("character-regenerate") => {
            let slug = args
                .get(1)
                .map(String::as_str)
                .filter(|s| !s.is_empty())
                .context("usage: novelagent character-regenerate <slug>")?;
            let summary = character_regenerate(slug).await?;
            println!("{summary}");
            Ok(())
        }
        Some("help" | "--help" | "-h") => {
            print_usage();
            Ok(())
        }
        Some(other) => {
            bail!("unknown command `{other}`\n{}", usage_text())
        }
    }
}

fn usage_text() -> &'static str {
    "usage:\n  \
     novelagent                         # Topcoat chat UI\n  \
     novelagent character-create <concept…>\n  \
     novelagent character-chat <slug> <message…>\n  \
     novelagent character-list          # enumerate saved cards under data/characters/\n  \
     novelagent character-delete <slug> # remove all sidecar files for a saved card\n  \
     novelagent character-regenerate <slug> # re-run Self-Refine with the stored concept\n  \
     novelagent help"
}

fn print_usage() {
    println!("{}", usage_text());
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_help_exits_ok() {
        run(vec!["help".into()]).await.expect("help");
        run(vec!["--help".into()]).await.expect("--help");
        run(vec!["-h".into()]).await.expect("-h");
    }

    #[tokio::test]
    async fn run_unknown_command_errors() {
        let err = run(vec!["nope".into()]).await.expect_err("unknown");
        let msg = err.to_string();
        assert!(msg.contains("unknown command"));
        assert!(msg.contains("character-create"));
    }

    #[tokio::test]
    async fn run_character_create_requires_concept() {
        let err = run(vec!["character-create".into()])
            .await
            .expect_err("missing concept");
        assert!(err.to_string().contains("usage"));
    }

    #[tokio::test]
    async fn run_character_chat_requires_message() {
        let err = run(vec!["character-chat".into(), "苏晚".into()])
            .await
            .expect_err("missing message");
        assert!(err.to_string().contains("usage"));
    }

    #[tokio::test]
    async fn run_character_list_is_not_unknown() {
        // Must not bail as unknown command; empty dir → ok (prints 0 cards).
        run(vec!["character-list".into()])
            .await
            .expect("character-list should be a known command");
    }

    #[test]
    fn usage_mentions_character_list() {
        let u = usage_text();
        assert!(u.contains("character-list"));
        assert!(u.contains("character-create"));
        assert!(u.contains("character-chat"));
        assert!(u.contains("character-delete"));
        assert!(u.contains("character-regenerate"));
    }
}
