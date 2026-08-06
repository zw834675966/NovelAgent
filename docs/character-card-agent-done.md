# Character Card Agent — DONE

**Date:** 2026-08-06  
**Milestone:** Phases 0–6 (`tasks/plan.md` · `tasks/todo.md`)  
**Scope:** SillyTavern V2 card meta-agent (Self-Refine) + lore/memory/KG seed + LanceDB/Cohere hybrid memory + CLI/Topcoat thin surface + harness docs.

## Deliverables

| Area | Evidence |
|------|----------|
| Schema + hard constraints | `src/character/card.rs`, `constraints.rs` |
| Prompt assets | `prompts/character/*.md` + `prompt_pack.rs` |
| Meta-agent loop | `agent.rs` (draft → critique → refine ≤2) + `rubric.rs` |
| Lore / memory / KG | `lorebook.rs`, `memory.rs`, `kg.rs`, `seed.rs` |
| Vector memory (v1) | `embed.rs` (Cohere `embed-multilingual-v3.0`) + `vector_store.rs` (LanceDB) |
| Persist | `persist.rs` → `data/characters/{slug}_{card,memory,kg,report}.json` |
| CLI | `cargo run -- character-create …` / `character-chat <slug> …` (`app::run`) |
| Topcoat | `web/character.rs` procedures `character_create` / `character_chat` |
| Distill map | `papers/DISTILL_character_card_agent.md` |
| User docs | `README.md` Character section; agent map `CLAUDE.md` |

## Gate evidence (this machine)

Env used for Windows target dir:

```powershell
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\novelagent-target"
```

| Gate | Command | Result |
|------|---------|--------|
| **L0** | `pwsh -File scripts/ai-gate.ps1 -Level L0` | **PASS** — fmt OK, clippy OK (`-D warnings`), nextest **82 passed / 4 skipped** |
| **L1** | `pwsh -File scripts/ai-gate.ps1 -Level L1` | **PASS** — cargo-deny (warnings only: unmatched license allow + transitive duplicates), cargo-audit (unmaintained advisories only), cargo-machete clean |
| Unit (lib) | `cargo test --workspace --all-features` | **82 passed, 0 failed, 4 ignored** |

Ignored live tests (need keys / network; previously green in Phase 3 / 4b):

| Test | Keys |
|------|------|
| `live_create_card_checkpoint_c` | `OPENCODE_GO_API_KEY` |
| `live_rig_short_complete` | `OPENCODE_GO_API_KEY` |
| `live_cohere_embed_multilingual_zh` | `COHERE_API_KEY` |
| `live_lancedb_memory_zh_checkpoint_4b` | `COHERE_API_KEY` + LanceDB |

Local live artifacts (gitignored, when re-run):

- `data/characters/live_checkpoint_c_{card,memory,kg,report}.json`
- `data/characters/checkpoint_4a_{card,memory,kg}.json` (offline unit)
- `data/lancedb/checkpoint_4b_*` (Phase 4b)

## Architecture snapshot

```text
main → load_environment → app::run
  empty            → Topcoat (chat + character procedures)
  character-create → create_card_live + write_create_outcome
  character-chat   → load_card_by_slug + assemble_prompt_pack preamble → LLM
```

Chat injects card system/PHI only; does **not** auto-query LanceDB after create.

## Deferred (not this milestone)

- GraphRAG community summaries  
- ToT branch selection  
- PNG tEXt export  
- Qdrant / fastembed offline fallback (feature, non-v1 default)  
- StoryWriter multi-agent longform  
- Create → LanceDB auto-index  
- Soft critique “建议级” must_fix debt  
- Optional Zhihu paste into DISTILL  
- PR-5 style 80% llvm-cov hard gate (aspirational; not required by plan Phase 6)

## DONE one-liner

**人物卡片 Agent 垂直切片 Phases 0–6 已落地：库域可测、CLI/Topcoat 可调用、L0+L1 门禁绿；语义记忆为可选 LanceDB+Cohere，不阻塞 create。**
