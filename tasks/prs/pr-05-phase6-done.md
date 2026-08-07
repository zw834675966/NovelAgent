# PR-5 — Phase 6 (收口): 覆盖率硬门 + DONE 报告

> **Status (2026-08-07): PARTIAL → coverage 硬门 DONE**  
> ✅ `docs/character-card-agent-done.md` 已写；README/CLAUDE 已链。  
> ✅ **覆盖率硬门已升**（2026-08-07）：实测行覆盖 **81.91%** ≥ 80 → `ai-gate.ps1` L2 改 `--fail-under-lines 80`，复验 exit 0。78.08% 基线 → 补 10 个离线测试（embed 缺 key/假 key、vector_store RecordBatch/空流/校验短路、web utc_hms）冲过线。  
> ✅ **分支覆盖已用 nightly 实测**（2026-08-07）：`cargo +nightly llvm-cov --branch` → **65.55%**（238 分支/82 未覆盖）。流程：手动 `RUSTFLAGS="-C instrument-coverage -Z coverage-options=branch"` + 干净正斜杠 `LLVM_PROFILE_FILE` 跑测试 → `llvm-cov report --branch --ignore-filename-regex '\.cargo'`。**原因**：llvm-cov 0.8.7 在 nightly 1.99 下用混排反斜杠路径导致 0 profraw（stable 运行库接受、nightly 拒绝）；0.8.7 亦无 `--fail-under-branches` flag（仅 lines/functions/file-lines/regions），故分支覆盖**只测量记录、不设硬门**。gate 保持 stable 行覆盖（81.91%）。  
> ❌ `proptest-regressions` 仍 deferred（依赖 PR-2，未开工）。  
> Phase 6 计划口径：**L0+L1 + DONE** 即收口；80% 硬门此前为 aspirational，**现已达成（行覆盖）**。  
> 跟踪：`tasks/todo.md` Deferred「PR-5 覆盖率硬门 [x] / proptest [ ]」。

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
