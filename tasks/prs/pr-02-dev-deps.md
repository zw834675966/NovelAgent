# PR-2 — Phase 6 (dev-deps): insta / rstest / proptest / mockall

## Goal
把项目级 TDD 工具链补齐。**不改 L0/L1/L2 行为**，仅扩 dev-deps + 1 个 L2 子步骤。

## Scope
- `Cargo.toml` `[dev-dependencies]`
- `scripts/install-ai-tools.ps1` — 加 `cargo-insta`
- `scripts/ai-gate.ps1` — L2 块加 `proptest-regressions` 子步骤

## Files touched
- `Cargo.toml`（dev-deps 4 行）
- `scripts/install-ai-tools.ps1`（1 行）
- `scripts/ai-gate.ps1`（5 行 L2 块内）

## TDD plan
本 PR **不为新功能写测试**；为现有 24 个 TDD 报告里的测试**添加** 1 个示例，证明新工具可用：
- `card::emotion_arc_with_rstest` — 用 `#[rstest]` 参数化 4 个 fixture（已有 `emotion_arc_accepts_mixed_strings_and_objects` 行为，重新表达）
- `agent::mockall_lm_backend` — `#[automock]` 替手写 `ScriptedLlm`，跑 `happy_path_no_refine`
- `kg::proptest_round_trip` — `proptest! { fn round_trip(graph in arb_kg()) }`（arb 待写）
- `prompt_pack::insta_snapshot` — `assemble_prompt_pack` 快照测试

## Verify
```powershell
cargo test --lib --all-features
cargo clippy --lib --all-features -- -D warnings
cargo fmt --all -- --check
# L2 子步骤：
$env:PROPTEST_CASES = "128"
cargo test --workspace --all-features -- proptest_regressions
cargo install cargo-insta --locked
cargo insta review  # 接受新快照
```

## Acceptance
- 4 个示范测试全绿
- 现有 68 测试无回归
- `cargo insta review` 通过

## Risk
低。dev-deps 改动 0 影响生产二进制。`mockall` 改 `LlmBackend` 不删 `ScriptedLlm`（保留为备）。`proptest` arb 失败要立刻缩范围（`PROPTEST_CASES=128`）。
