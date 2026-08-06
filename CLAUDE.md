# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Rust binary wrapping the [`rig`](https://github.com/0xPlaygrounds/rig) LLM framework (0.41.0) with a [Topcoat](https://github.com/tokio-rs/topcoat) chat UI. Single model runtime: DeepSeek V4 Flash served by the OpenCode Go plan (`https://opencode.ai/zen/go/v1`). The `README.md` is the user-facing intro; this file is the contributor/AI operating manual.

## Build / run / test

Setup (Windows / PowerShell):

```powershell
cp .env.example .env        # OPENCODE_GO_API_KEY (+ COHERE_API_KEY for LanceDB memory)
cargo install topcoat-cli   # once: asset bundle + dev server
topcoat dev                 # preferred; or `cargo run`
# open http://127.0.0.1:3000
# override bind: $env:HOST = "0.0.0.0"; $env:PORT = "8080"
```

Cold `cargo run` without a prior `topcoat dev` / `topcoat asset bundle` will fail at `web::router()` with `topcoat asset bundle missing next to the binary` — the asset bundle is a runtime file, not a build artifact. First-time setup is `topcoat dev` once, then `cargo run` works.

**Windows note:** if build scripts under the repo-local `target/` hit Access Denied (`bigdecimal`, etc.), point the target dir outside the tree:

```powershell
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\novelagent-target"
```

CLI character commands (no Topcoat assets required):

```powershell
cargo run -- character-create "一句话人物概念…"
cargo run -- character-chat <slug> "用户台词…"
cargo run -- help
```

Quality gates — `scripts/ai-gate.ps1` is the single source of truth. L0 is the must-pass set (fmt + clippy + test); L1 adds supply-chain checks (`cargo-deny`, `cargo-audit`, `cargo-machete`); L2 adds coverage / mutation. Manual equivalents:

```powershell
pwsh -File scripts/ai-gate.ps1              # L0
pwsh -File scripts/ai-gate.ps1 -Level L1
cargo test --lib character                  # domain unit tests
cargo test live_create_card_checkpoint_c -- --ignored --nocapture
cargo test live_lancedb_memory_zh -- --ignored --nocapture
```

Install optional L1/L2 tools once: `pwsh -File scripts/install-ai-tools.ps1`.

Character milestone DONE report: [`docs/character-card-agent-done.md`](./docs/character-card-agent-done.md). Plan/todo: `tasks/plan.md`, `tasks/todo.md`.

## Architecture

The binary either runs CLI character subcommands or starts a Topcoat HTTP server. Layers, top-down:

```
main.rs (≤15 lines)
  ├─> app::load_environment()          // dotenv + OPENCODE_GO_API_KEY
  └─> app::run(args)
        ├─> (empty)  → topcoat::start(web::router())
        │     ├─> web/chat.rs  #[page("/")] + #[procedure] send_chat
        │     └─> web/character.rs  character_create / character_chat
        ├─> character-create → app::character_create → character::create_card_live + persist
        └─> character-chat   → load card by slug → assemble_prompt_pack preamble → prompt
```

Key boundaries:

- **`main.rs`** — tokio runtime + env load + `app::run`. No business logic. AGENTS.md §12.1 (≤15 lines).
- **`app/`** — environment + orchestration. `env.rs` owns dotenv + key validation; `agent.rs` owns `prompt_message` / `validate_user_message` / `run_readiness_check`; `character_cmd.rs` owns create/chat CLI flows (`anyhow`); `mod.rs::run` dispatches CLI vs Topcoat; `bootstrap` is the one-call lib entry for tests.
- **`character/`** — ST V2 card domain (`thiserror`). Schema + hard validate, prompt pack, Self-Refine meta-agent (`create_card` / `create_card_live`), lorebook / memory stream / KG seed, Cohere embed + LanceDB hybrid search, disk persist under `data/characters/`.
- **`model/`** — upstream wiring. `client.rs` is a single flat builder (`build_agent_builder` reads env → builds OpenAI-compatible client → attaches DeepSeek V4 Flash). Typed `ModelError`.
- **`web/`** — Topcoat page + procedures. `send_chat` / `character_*` always return `Ok(String)` and embed errors in the string (Topcoat 0.5 `StringSurrogate` has no `is_ok` / `unwrap`). Client transcript is a `String` signal (no `Vec` in shared vocab).
- **`lib.rs`** — flat re-exports of public `app` / `character` / `model` surface. Call sites use `novelagent::…`, not deep paths.

Env keys: `OPENCODE_GO_API_KEY` (create + chat + readiness); `COHERE_API_KEY` (LanceDB memory index/search only).

## Conventions

Project-wide AI/contributor rules live in [`AGENTS.md`](./AGENTS.md). The non-obvious ones to keep in mind:

- **Hard constraints** in §12: thin `main`, folder-per-domain layout, ≤400 lines per function, no builder-of-builder / wrapper-of-wrapper nesting, no multi-level re-export chains. Read §12 before restructuring anything.
- **Error layers**: library code uses `thiserror` (`model::error::ModelError`); the binary layer uses `anyhow` (`app::bootstrap` returns `anyhow::Result`). Don't mix them.
- **Lint config** is in `Cargo.toml` `[lints.*]`. `clippy::all = "deny"`, plus `unwrap_used` / `expect_used` / `panic` as warnings (which `-D warnings` promotes to errors). Tests may `#[allow(clippy::expect_used, ...)]` at the module level.
- **Env mutation in tests** uses the `ENV_LOCK` mutex pattern in `model/client.rs` — `env::set_var` is `unsafe` in edition 2024. Use the existing `with_api_key` helper when you need a key set; grab `lock()` directly when you need to assert the *missing* case. Don't bypass the lock or call `env::set_var` outside it.
- **Topcoat error channel**: procedures return `Result<String>` but `send_chat` always serialises the error into the reply string instead of propagating. Reason: Topcoat 0.5 `StringSurrogate` has no failure variant the browser can branch on. Don't change this without checking the Topcoat version's `StringSurrogate` surface.

## Docs drift

Keep `README.md` project layout and the Character card section aligned with `src/` when you add domains or CLI verbs. Architecture detail for agents lives here; user-facing run commands live in README.
