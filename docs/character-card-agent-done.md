# Character Card Agent — DONE

**Date:** 2026-08-06（计数与 CLI 面 2026-08-07 校正）  
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
| Persist | `persist.rs` → `data/characters/{slug}_{card,memory,kg,report,concept}.json` |
| CLI | `character-create` / `character-chat` / `character-list` / `character-delete` / `character-regenerate` / `help`（`app::run`） |
| Topcoat | `web/character.rs` + `web/chat.rs` `ui_*`（create/chat/list/delete/regenerate） |
| Distill map | `papers/DISTILL_character_card_agent.md` |
| User docs | `README.md` Character section; agent map `CLAUDE.md` |
| Alignment audit | `docs/tasks-alignment-report.md`（PR 切片 vs Phase 真相对齐） |

## Gate evidence (this machine)

Env used for Windows target dir:

```powershell
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\novelagent-target"
```

| Gate | Command | Result |
|------|---------|--------|
| **L0** | `pwsh -File scripts/ai-gate.ps1 -Level L0` | **PASS**（历史：2026-08-06 nextest 曾记 82；以最新 unit 行为准） |
| **L1** | `pwsh -File scripts/ai-gate.ps1 -Level L1` | **PASS** — cargo-deny (warnings only: unmatched license allow + transitive duplicates), cargo-audit (unmaintained advisories only), cargo-machete clean |
| Unit (workspace) | `cargo test --workspace --all-features` | **124 passed, 0 failed, 4 ignored**（2026-08-07 复跑；+10 覆盖率测试，见下） |
| **L2 (coverage)** | `cargo llvm-cov --workspace --all-features --fail-under-lines 80` | **PASS** — 行覆盖 **81.91%**；硬门已从 0 升 80（2026-08-07） |

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
  empty                 → Topcoat (chat + character UI)
  character-create      → create_card_live + write_create_outcome
  character-chat        → load_card_by_slug + assemble_prompt_pack preamble → LLM
  character-list        → list_characters
  character-delete      → delete_character
  character-regenerate  → load_concept + create_card_live + write
```

Chat injects card system/PHI only; does **not** auto-query LanceDB after create.

## PR slice honesty (vs `tasks/prs/`)

| PR | Status | Note |
|----|--------|------|
| PR-1 seeds | done | |
| PR-2 dev-deps | **deferred** | no insta/rstest/proptest/mockall in tree |
| PR-3 vector | done | optional `docs/COSTS.md` still missing |
| PR-4 web | done+ | list/delete/regenerate beyond original PR text |
| PR-5 hard gates | partial→**coverage DONE** | DONE exists; **coverage hard gate raised to `--fail-under-lines 80`** (81.91%); proptest-regressions still deferred (needs PR-2) |

## Deferred (not this milestone)

- GraphRAG community summaries  
- ToT branch selection  
- PNG tEXt export  
- Qdrant / fastembed offline fallback (feature, non-v1 default)  
- StoryWriter multi-agent longform  
- Create → LanceDB auto-index  
- Soft critique “建议级” must_fix debt  
- Optional Zhihu paste into DISTILL  
- **PR-2** TDD toolchain (insta / rstest / proptest / mockall) — deferred deliberately  
- **PR-5** 80% llvm-cov hard gate + proptest-regressions promote — aspirational until coverage measured  

## DONE one-liner

**人物卡片 Agent 垂直切片 Phases 0–6 已落地：库域可测、CLI/Topcoat 可调用、L0+L1 门禁绿；语义记忆为可选 LanceDB+Cohere，不阻塞 create。**
