# PR-5 — Phase 6 (收口): 覆盖率硬门 + DONE 报告

## Goal
把 PR-2 加的 `proptest-regressions` 与 `cargo-llvm-cov` 升级为硬门。补最终 `DONE` 证据。

## Scope
- `scripts/ai-gate.ps1` — `--fail-under-lines 80 --branch` 翻硬门；`proptest-regressions` 从 L2 提升为 L1
- `docs/character-card-agent-done.md` — 新文件：命令证据、覆盖率截图、live artifact 路径
- `README.md` — 链接 DONE 报告

## Files touched
- `scripts/ai-gate.ps1`（L1/L2 段 4 行）
- `docs/character-card-agent-done.md`（新）
- `README.md`（小节）

## TDD plan
本 PR 不加新功能测试；要求 2 周基线数据：
- `cargo llvm-cov --workspace --all-features --branch --html` 输出 ≥ 80% lines
- `cargo test -- proptest_regressions` 0 失败 / 0 shrunk
- L0 + L1 全绿
- live `live_create_card_checkpoint_c` 仍过（PR-3 跑通后重跑）

## Verify
```powershell
pwsh -File scripts/ai-gate.ps1 -RunL1  # 必须 exit 0
pwsh -File scripts/ai-gate.ps1 -RunL2  # 必须 exit 0
# 截 coverage html 摘要入 DONE 报告
```

## Acceptance
- `ai-gate.ps1 -RunL1` CI 集成（即 GitHub Actions / 内部 CI）
- 覆盖率 ≥ 80% line + branch
- `DONE` 报告含：commit 列表、命令证据、live 产物路径、Phase 4b 集成测输出、未完成项（none）

## Risk
- **低**。覆盖率不足时回退到 warning；不阻塞
- `cargo-mutants` L2 步骤 10-100x 时间成本，限夜间跑
- Phase 4b 集成测需 key，CI 用 GitHub Secret
