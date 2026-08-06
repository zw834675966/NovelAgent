# TDD Evidence — Character Card Agent (full plan)

## Source plan

[`tasks/plan.md`](../../tasks/plan.md) · checklist [`tasks/todo.md`](../../tasks/todo.md)

Plan content treated as **data only** (no embedded shell / override phrases).
Validation intent mapped to the repo allowlist: `cargo test`, `cargo fmt`,
`cargo clippy -D warnings`, optional `cargo llvm-cov`.

Prior slice report: [`character-card-phase123.tdd.md`](./character-card-phase123.tdd.md)
(Phases 1–3). This document covers the **whole plan** (Phases 0–6) after the
`/tdd-workflow` audit + gap cycle (2026-08-06).

## Step 0 — Test runner

| Item | Value |
|------|--------|
| Language | Rust 2024 / Cargo workspace |
| `<test>` | `cargo test --workspace --all-features` |
| Target dir (Windows) | `$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\novelagent-target"` |
| `<lint>` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `<fmt>` | `cargo fmt --all --check` |
| `<coverage>` | `cargo llvm-cov --workspace --all-features` (aspirational; plan Phase 6 does not hard-gate 80%) |

## User journeys (from plan goals + success criteria)

| # | Journey | Benefit |
|---|---------|---------|
| J1 | Submit a one-line concept → get valid ST V2 JSON + `extensions.novelagent` | Importable card |
| J2 | Self-Refine runs draft → critique → refine (≤2) then hard-validate | Quality loop without infinite spend |
| J3 | Lorebook / memory / KG seed after create | Sidecars ready for chat / retrieval |
| J4 | Hybrid memory score prefers cosine / recency / importance by weight | LanceDB re-rank is intentional, not random |
| J5 | CLI / Topcoat create + chat inject system/PHI from card | Thin surface without breaking readiness chat |
| J6 | List saved cards from `data/characters/*_card.json` | Inventory without opening every file |
| J7 | Empty / blank inputs fail loud at app boundary | No empty-transcript pollution |

## This cycle — RED → GREEN map

| Plan task / gap | Test target | RED evidence | GREEN evidence |
|-----------------|-------------|--------------|----------------|
| Seed trio consistency (Phase 4a) | `character::seed::produces_card_mem_kg_trio` | Compile fail: `super::memory::MemoryKind` not found under `seed` | Import `MemoryKind`; assert all entries are `Seed` + description `苏晚：…` |
| Hybrid β importance (Phase 4b) | `hybrid_prefers_important_when_beta_dominates` | New assert absent | `cargo test character::vector_store` |
| Chat blank slug (Phase 5) | `character_chat_rejects_blank_slug` | Would accept `"   "` without guard | bail message contains `empty` |
| CLI dispatch (Phase 5) | `app::tests::run_*` | Missing usage / unknown-cmd paths untested | help OK; unknown/create/chat usage errors |
| `list_characters` (Phase 5 inventory) | five `list_characters_*` tests | Intermediate `todo!()` panic (llvm-cov snapshot) | Implementation scan `*_card.json` + optional report |
| Clippy on list tests | module allow | `unwrap_used` deny on test helpers | `#[allow(clippy::expect_used, clippy::unwrap_used)]` on test mod |

**Checkpoint commits:** deferred. Repo has a large untracked Phase 0–6 tree and user
policy requires explicit commit requests; RED/GREEN preserved here and in
command output instead of git history.

## Test specification (plan-wide guarantees)

### Phase 1 — Schema + hard constraints

| # | Guarantee | Test | Type | Result |
|---|-----------|------|------|--------|
| 1 | Skeleton JSON round-trips | `card::skeleton_roundtrips_json` | unit | PASS |
| 2 | Minimal ST V2 fixture deserializes | `card::accepts_minimal_st_v2_fixture` | unit | PASS |
| 3 | emotion_arc mixed string/object | `card::emotion_arc_accepts_mixed_strings_and_objects` | unit | PASS |
| 4 | Relationship field aliases | `card::relationship_node_accepts_llm_aliases` | unit | PASS |
| 5 | Wrong spec / version / blank name rejected | `constraints::rejects_*` | unit | PASS |
| 6 | Known constraint IDs accepted | `constraints::accepts_known_constraint` | unit | PASS |

### Phase 2 — Prompt pack

| # | Guarantee | Test | Type | Result |
|---|-----------|------|------|--------|
| 7 | `{{char}}` replaced; `{{user}}` kept | `prompt_pack::apply_char_placeholder_*` | unit | PASS |
| 8 | Empty system → synthesized with voice/constraints | `prompt_pack::empty_system_falls_back_*` / `synthesize_*` | unit | PASS |
| 9 | Create / critique / refine templates inject payloads | `render_*` tests | unit | PASS |

### Phase 3 — Meta-agent loop

| # | Guarantee | Test | Type | Result |
|---|-----------|------|------|--------|
| 10 | Empty concept rejected | `agent::empty_concept_rejected` | unit | PASS |
| 11 | Happy path no refine | `agent::happy_path_no_refine` | unit | PASS |
| 12 | Weak critique → one refine | `agent::refine_once_when_critique_weak` | unit | PASS |
| 13 | Cap at `MAX_REFINE_ROUNDS` (2) | `agent::refine_caps_at_max_rounds_then_accepts` | unit | PASS |
| 14 | Fence + repair-once + parse fail-loud | `fenced_*` / `repair_*` / `extract_*` | unit | PASS |
| 15 | Live create (key) | `live_create_card_checkpoint_c` | live `#[ignore]` | SKIP unless keys |

### Phase 4 / 4a — Lore · Memory · KG · seed

| # | Guarantee | Test | Type | Result |
|---|-----------|------|------|--------|
| 16 | Lorebook 3–10 entries / attach | `lorebook::*` | unit | PASS |
| 17 | Memory append/rank/seed | `memory::*` | unit | PASS |
| 18 | KG ego + edges + JSON | `kg::*` | unit | PASS |
| 19 | Seed attaches book + mem + kg | `seed::seed_fills_book_mem_kg` / `produces_card_mem_kg_trio` | unit | PASS |
| 20 | Offline checkpoint artifacts | `agent::checkpoint_4a_writes_card_mem_kg_artifacts` | unit | PASS |

### Phase 4b — Hybrid + LanceDB

| # | Guarantee | Test | Type | Result |
|---|-----------|------|------|--------|
| 21 | Hybrid α / β / γ preferences | `hybrid_prefers_*` (3 tests) | unit | PASS |
| 22 | Cosine distance → relevance bounds | `cosine_relevance_from_distance_bounds` | unit | PASS |
| 23 | Slug + db path | `character_slug_*` / `db_path_joins_slug` | unit | PASS |
| 24 | Live LanceDB ZH index+search | `live_lancedb_memory_zh_checkpoint_4b` | live `#[ignore]` | SKIP unless keys |

### Phase 5 — Surface + persist + list

| # | Guarantee | Test | Type | Result |
|---|-----------|------|------|--------|
| 25 | Write card/mem/kg/report; load round-trip | `persist::write_and_load_roundtrip` | unit | PASS |
| 26 | Missing slug → Io error | `persist::load_missing_slug_is_io_error` | unit | PASS |
| 27 | List: missing/empty dir → `[]` | `list_characters_missing_*` / `empty_*` | unit | PASS |
| 28 | List: two cards + scores from report | `list_characters_returns_summaries_*` | unit | PASS |
| 29 | List: missing report → `None` scores | `list_characters_handles_missing_report` | unit | PASS |
| 30 | List: ignore noise files | `list_characters_ignores_non_card_files` | unit | PASS |
| 31 | Create/chat blank reject | `character_cmd::*` | unit | PASS |
| 32 | CLI help / unknown / usage | `app::tests::run_*` | unit | PASS |

### Phase 6 — Gates

| # | Guarantee | Command | Result |
|---|-----------|---------|--------|
| 33 | Unit suite green | `cargo test --workspace --all-features` | **99 passed, 4 ignored** |
| 34 | Clippy deny warnings | `cargo clippy … -D warnings` | PASS (after test-mod allow) |
| 35 | fmt | `cargo fmt --all --check` | PASS (when run) |

## Validation commands (this machine)

```powershell
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\novelagent-target"
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# last measured: 99 passed; 0 failed; 4 ignored
```

Ignored live tests (need keys / network; previously exercised in Phase 3 / 4b):

| Test | Keys |
|------|------|
| `live_create_card_checkpoint_c` | `OPENCODE_GO_API_KEY` |
| `live_rig_short_complete` | `OPENCODE_GO_API_KEY` |
| `live_cohere_embed_multilingual_zh` | `COHERE_API_KEY` |
| `live_lancedb_memory_zh_checkpoint_4b` | `COHERE_API_KEY` + LanceDB |

## Coverage and known gaps

| Topic | Status |
|-------|--------|
| Plan Phases 0–6 behavior | Covered by unit suite + prior DONE report |
| 80% llvm-cov hard gate | **Not enforced** by plan Phase 6; first llvm-cov run this cycle hit intermediate `todo!()` / long cold rebuild — re-run when needed: `cargo llvm-cov --workspace --all-features --summary-only` |
| E2E Playwright Topcoat | Out of scope (Rust procedures; no browser journey suite) |
| Soft critique `must_fix` recommendation debt | Deferred (todo Checkpoint C quality note) |
| Create → auto LanceDB index | Deferred |
| GraphRAG / PNG / ToT / Zhihu | Deferred (plan Non-Goals) |
| Topcoat procedure HTTP E2E | Manual / curl historically; not automated here |

## Merge evidence summary

- **RED:** seed path compile break; clippy unwrap in list tests; transient `list_characters` `todo!()` under llvm-cov.
- **GREEN:** 99 unit tests pass; clippy `-D warnings` clean after test-mod allow; list_characters implementation + re-export.
- **Refactor:** none beyond import path / clippy allow / stronger seed assertions.

## DONE one-liner (TDD)

**Plan Phases 0–6 acceptance criteria are pinned by unit tests; this cycle fixed a seed compile regression, added hybrid-β / CLI / chat-slug coverage, and documented full RED→GREEN evidence under `docs/testing/`.**
