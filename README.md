# NovelAgent

Rust project wired against the [`rig`](https://github.com/0xPlaygrounds/rig)
LLM framework (0.41.0) and a [Topcoat](https://github.com/tokio-rs/topcoat) web
UI. Talk to **DeepSeek V4 Flash** (OpenCode Go) from the browser.

## Setup

1. Install the Rust toolchain (`rustup`).
2. Copy the env template and fill in your OpenCode Go API key
   (issued from the [OpenCode Zen console](https://opencode.ai/auth)):

   ```bash
   cp .env.example .env
   # then edit .env
   ```

3. (Recommended) Install the Topcoat CLI so assets + hot reload work:

   ```bash
   cargo install topcoat-cli
   ```

4. Start the chat UI:

   ```bash
   # preferred: builds assets, watches, reloads
   topcoat dev

   # or plain cargo (UI works after assets are present)
   cargo run
   ```

   Open <http://127.0.0.1:3000>, type a message, and send. Override bind with
   `HOST` / `PORT` if needed.

## Character card agent

Generate a SillyTavern V2 card from a Chinese concept, write JSON sidecars under
`data/characters/` (gitignored), then optionally chat as that card.

| Env | When |
|-----|------|
| `OPENCODE_GO_API_KEY` | create + roleplay chat + readiness |
| `COHERE_API_KEY` | optional: LanceDB semantic memory (Phase 4b) |

Milestone DONE + gate evidence: [`docs/character-card-agent-done.md`](./docs/character-card-agent-done.md).  
Design map: [`papers/DISTILL_character_card_agent.md`](./papers/DISTILL_character_card_agent.md) · plan [`tasks/plan.md`](./tasks/plan.md).

### CLI

```powershell
# create + write data/characters/{slug}_{card,memory,kg,report}.json
cargo run -- character-create "雨夜便利店的夜班店员，克制，怕被看穿"

# one turn with the saved card's system / PHI / role context as preamble
cargo run -- character-chat 苏晚 "外面还在下雨。"

# list cards already under data/characters/
cargo run -- character-list

cargo run -- help
```

Chat injects the card via `assemble_prompt_pack` as agent preamble. It does **not**
auto-run LanceDB retrieval; index/search is a separate API / `#[ignore]` live test.

### Topcoat procedures

With the UI server running (`cargo run` or `topcoat dev`), POST the discovered
procedure endpoints (same pattern as chat):

```text
POST /_topcoat/procedures/character_create
POST /_topcoat/procedures/character_chat
POST /_topcoat/procedures/character_list
```

Bodies follow Topcoat’s procedure codec (string args). Empty input returns a
timestamped `(empty …)` string rather than a hard error so the browser Surrogate
path stays simple.

### In-browser UI

The Topcoat chat page (`/`) now hosts four panels backed by the same backend
procedures, so you can drive the character agent without touching a terminal:

| Panel | Backend path | Notes |
|-------|--------------|-------|
| LLM 直接对话 | `send_chat` (LLM) | bare DeepSeek V4 Flash turn |
| 已存角色 | `ui_character_list` | scan `data/characters/`; survives corrupt files |
| 创建新角色 | `ui_character_create` | Self-Refine, ~30 s per round |
| 角色对话 | `ui_character_chat` | prompt pack + LLM; slug from list above |

Each panel has a `清空` button to wipe its log. Buttons show a pulsing dot via
`:disabled::before` while the LLM round is in flight. Local `ui_*` procedures
in `web/chat.rs` are thin `Result<String>` wrappers around the `app::*`
functions — they exist so the `view!` macro can call them through its
`.call(...)` RPC path (plain `pub fn` returning non-`Surrogated` types cannot
be invoked from `view!`).

### LanceDB / first-run notes (optional)

1. Install `protoc` (protobuf compiler) if `lancedb` / Arrow build fails on missing
   protoc.
2. On Windows, if repo-local `target/` build-scripts fail with Access Denied, set:

   ```powershell
   $env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\novelagent-target"
   ```

3. After create, memory JSON lives under `data/characters/`. To vector-index:

   ```powershell
   cargo test live_lancedb_memory_zh -- --ignored --nocapture
   ```

   Writes under `data/lancedb/{slug}/` (gitignored). Model:
   Cohere `embed-multilingual-v3.0` (1024-d). **fastembed is not the v1 default.**

### Layout notes

| Path | Role |
|------|------|
| `src/character/` | schema, Self-Refine loop, lore/memory/KG, LanceDB, persist |
| `src/app/character_cmd.rs` | create + roleplay orchestration (`anyhow`) |
| `src/web/character.rs` | Topcoat procedures |
| `prompts/character/*.md` | meta-agent templates |
| `data/characters/` | exported cards (local only) |
| `data/lancedb/` | per-character vector DB (local only) |

## Project layout

```text
src/
├── lib.rs           # crate root re-exports
├── main.rs          # thin entry: load env + app::run
├── app/             # env, readiness, character CLI commands
├── character/       # ST V2 card agent domain
├── model/           # OpenCode Go / DeepSeek client
└── web/             # Topcoat chat + character procedures
```

## Quality gate

```powershell
pwsh -File scripts/ai-gate.ps1           # L0: fmt + clippy + test
pwsh -File scripts/ai-gate.ps1 -Level L1 # + deny / audit / machete (if installed)
```

## Code conventions

See [`AGENTS.md`](./AGENTS.md) for the project-wide AI / contributor rules
(immutability, error handling, lint gates, etc.). Contributor map for agents:
[`CLAUDE.md`](./CLAUDE.md).
