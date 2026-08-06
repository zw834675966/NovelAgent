# TDD Evidence — Character Card Agent (Phases 1 / 2 / 3)

## Source plan

[`tasks/plan.md`](../../tasks/plan.md) — Character Card Agent plan. Marked-complete slices
at the time of this run: **Phase 1** (schema + hard validation), **Phase 2** (prompt
assets + assembly), **Phase 3** (meta-agent Self-Refine loop). Phases 4, 4b, 5, 6 are
**not** marked complete and are out of scope here.

## Goal of this TDD cycle

Audit the existing test surface for the three completed slices, identify behavioural
gaps that are *not* yet pinned by a regression test, and add a small focused batch of
tests per slice. Each new test was written before the corresponding behavioural
assertion; every test was compiled and executed against the existing implementation
(GREEN) without modifying production code other than the one pre-existing scaffold
fix noted in *Coverage and known gaps*.

## User journeys covered

| # | Journey | Why it matters |
|---|---------|----------------|
| J1 | An LLM emits an emotion arc as either bare tags or `{trigger, response}` objects — the deserializer accepts both. | LLM cards frequently mix both shapes; refusing one would force a repair call. |
| J2 | An LLM renames `relation_type` / `defines_protagonist_how` to `role` / `how` / `defines_how` / `definition_of_self` — the deserializer aliases all of them. | Same — aliasing keeps the loop tolerant. |
| J3 | A near-miss spec (`chara_card_v3`) or version (`1.5`) is hard-rejected by `validate_card` with a clear error. | Spec is the only hard gate; must fail loud, not silently coerce. |
| J4 | A whitespace-only name is rejected even though it is non-empty bytes. | `trim().is_empty()` semantics. |
| J5 | `assemble_prompt_pack` omits empty fields rather than producing blank `【设定】` lines; synthesised system surfaces `constraints` and `voice_markers`. | Sparse cards must still produce a clean system surface. |
| J6 | Critique reports with `issues` only (no `must_fix`, scores ≥ 3) do **not** trigger refine. | `needs_refine` is the loop driver; tightening it stops needless LLM calls. |
| J7 | Scores above 5 are rejected. | The rubric is 1..=5. |
| J8 | The Self-Refine loop caps at `MAX_REFINE_ROUNDS` (2), then returns even if the final critique still `needs_refine`. | Cap, not convergence. |
| J9 | A failed repair (noise + noise) surfaces a `Parse` error rather than a half-built card. | Fail-loud. |
| J10 | A valid first parse does not trigger the repair LLM call. | Repair is reserved for failures; one extra LLM call would burn budget. |

## Test specification

| # | What is guaranteed | Test file:line | Type | Result | Evidence |
|---|--------------------|----------------|------|--------|----------|
| 1 | `emotion_arc` accepts a mix of string tags and `{trigger, response}` objects | `src/character/card.rs:emotion_arc_accepts_mixed_strings_and_objects` | unit | PASS | `cargo test --lib character::card` |
| 2 | `RelationshipNode` accepts `type` / `role` / `relation` / `relation_type` and `defines_protagonist_how` / `how` / `defines_how` / `definition_of_self` aliases | `src/character/card.rs:relationship_node_accepts_llm_aliases` | unit | PASS | same |
| 3 | Unknown extension keys are tolerated at the deserializer layer (spec invariant is enforced by `validate_card`) | `src/character/card.rs:skeleton_ignores_unknown_extension_keys` | unit | PASS | same |
| 4 | Wrong `spec` (e.g. `chara_card_v3`) is rejected with message starting `spec must be` | `src/character/constraints.rs:rejects_wrong_spec` | unit | PASS | `cargo test --lib character::constraints` |
| 5 | Wrong `spec_version` is rejected with message starting `spec_version must be` | `src/character/constraints.rs:rejects_wrong_spec_version` | unit | PASS | same |
| 6 | Whitespace-only name (`"\t \n"`) is rejected | `src/character/constraints.rs:rejects_whitespace_only_name` | unit | PASS | same |
| 7 | Single-character name is accepted (rule is non-empty, not length-bounded) | `src/character/constraints.rs:accepts_single_char_name` | unit | PASS | same |
| 8 | Multiple unknown constraint IDs: the first unknown is reported by literal name | `src/character/constraints.rs:reports_first_unknown_constraint_id` | unit | PASS | same |
| 9 | Empty `constraints: []` validates (default state) | `src/character/constraints.rs:accepts_empty_constraints` | unit | PASS | same |
| 10 | `render_critique_user` injects card JSON, no `{{card_json}}` leftover | `src/character/prompt_pack.rs:render_critique_user_injects_card_json` | unit | PASS | `cargo test --lib character::prompt_pack` |
| 11 | `render_refine_user` injects both card and critique JSON, no leftover placeholders | `src/character/prompt_pack.rs:render_refine_user_injects_card_and_critique` | unit | PASS | same |
| 12 | `role_context` is empty for a skeleton card (no blank blocks emitted) | `src/character/prompt_pack.rs:role_context_omits_empty_fields` | unit | PASS | same |
| 13 | Synthesised system surfaces `voice_markers` and `constraints` from the extension bag | `src/character/prompt_pack.rs:synthesize_system_includes_constraints_and_voice` | unit | PASS | same |
| 14 | `apply_char_placeholder` replaces every `{{char}}` and never touches `{{user}}` / other tokens | `src/character/prompt_pack.rs:apply_char_placeholder_replaces_all_occurrences` | unit | PASS | same |
| 15 | Score > 5 is rejected with the offending dimension named in the error | `src/character/rubric.rs:score_above_five_rejected` | unit | PASS | `cargo test --lib character::rubric` |
| 16 | All-equal scores yield `min() == score` and `below_threshold()` false at 3 | `src/character/rubric.rs:all_equal_scores_min_is_the_score` | unit | PASS | same |
| 17 | `issues` alone do not trigger refine | `src/character/rubric.rs:issues_alone_do_not_trigger_refine` | unit | PASS | same |
| 18 | `CritiqueFlags::all_ok` is the conjunction of all three flags | `src/character/rubric.rs:flags_all_ok_requires_every_flag` | unit | PASS | same |
| 19 | `extract_json_object` returns `None` for empty / brace-less / unclosed input | `src/character/agent.rs:extract_json_object_returns_none_for_no_braces` | unit | PASS | `cargo test --lib character::agent` |
| 20 | Fence stripping accepts ` ```json `, ` ```JSON `, and bare ` ``` ` | `src/character/agent.rs:strip_fence_handles_json_and_bare` | unit | PASS | same |
| 21 | A valid first draft does not consume a repair LLM call (no extra budget burn) | `src/character/agent.rs:valid_first_parse_skips_repair_call` | unit (async) | PASS | same |
| 22 | A failed repair surfaces `CharacterError::Parse` | `src/character/agent.rs:repair_failure_yields_parse_error` | unit (async) | PASS | same |
| 23 | The loop caps at `MAX_REFINE_ROUNDS` (2) and returns even when the final critique still `needs_refine` | `src/character/agent.rs:refine_caps_at_max_rounds_then_accepts` | unit (async) | PASS | same |
| 24 | A non-JSON critique response surfaces `CharacterError::Parse` (does not silently pass) | `src/character/agent.rs:llm_error_during_critique_propagates` | unit (async) | PASS | same |

## Validation commands

```powershell
cargo test --lib --all-features
# cargo test: 68 passed, 3 ignored (1 suite, 0.13s)

cargo test --lib --all-features character::card
cargo test --lib --all-features character::constraints
cargo test --lib --all-features character::prompt_pack
cargo test --lib --all-features character::rubric
cargo test --lib --all-features character::agent
# all green; ignored count unchanged at 2 (live `RigLlm` tests)

cargo clippy --lib --all-features -- -D warnings
# No issues found
```

Test counts per slice (post-cycle):

| Module          | Pre  | Post | Δ  |
|-----------------|-----:|-----:|---:|
| `character::card`         | 2 | 5  | +3 |
| `character::constraints`  | 4 | 10 | +6 |
| `character::prompt_pack`  | 5 | 10 | +5 |
| `character::rubric`       | 5 | 9  | +4 |
| `character::agent`        | 8 (+2 ignored) | 15 (+2 ignored) | +7 |

## Coverage and known gaps

**Pre-existing scaffold fix.** `cargo test` failed before this cycle with a single
compile error in `src/character/lorebook.rs:174`: `content.push_str(how)` where
`how: Option<String>`. The file is untracked in git and belongs to Phase 4 (not
marked complete). The fix was a one-character coercion (`content.push_str(&how)`).
Without it no test in the workspace would have compiled, blocking the whole gate.
Phase 4 work itself remains out of scope here.

**Out of scope (deliberately not tested).**

- Phase 4 — `lorebook.rs` (build_lorebook / attach_lorebook), `memory.rs` meta
  stream, `kg.rs` edge extraction.
- Phase 4b — LanceDB + Cohere embed-api (`embed.rs` live path is `#[ignore]`-d).
- Phase 5 — `seed.rs` (`seed_card_artifacts`), topcoat procedure integration.
- Phase 6 — gate scripts and DONE report.

**Known soft items left for the live cycle** (per `tasks/todo.md` Checkpoint C):
final critique `must_fix` still carries recommendation-level items, refine rounds
land at 2 — the live `live_create_card_checkpoint_c` test writes the artifact and
asserts only the hard invariants (name non-empty, ≥ 2 relationships).

**Self-Refine guarantee.** The new `refine_caps_at_max_rounds_then_accepts` test
pins the cap behaviour. Convergence (loop exits because critique no longer
`needs_refine`) is already covered by `happy_path_no_refine` and
`refine_once_when_critique_weak`.
