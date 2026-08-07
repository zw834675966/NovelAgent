# `tasks/` 与项目实况对齐分析报告

**生成时间：** 2026-08-07  
**审计复验：** 2026-08-07（debugging-and-error-recovery）  
**方法：** 静态读 `tasks/{plan,todo}.md` + `tasks/prs/*.md` + 源码/脚本 + 复跑 `cargo test`。  
**结论一句话：** Phases 0–6 **产品切片**已落地且 L0 全绿（**114 pass / 4 ignored**）；`tasks/prs` 细切片中 PR-2 **未开工（已显式 deferred）**、PR-5 **硬门 partial**；初版报告大体真实，但「todo 假装勾完 PR-2」表述过重——todo 从未列出 PR-2 项，根因是 **Phase SSOT 与 PR 切片双层未交叉标注**。

---

## 0. 复验判决（相对初版报告）

| 初版断言 | 复验 | 判决 |
|----------|------|------|
| PR-2 无 dev-deps / 无 cargo-insta / 无 proptest 子步骤 | `Cargo.toml` 无 insta/rstest/proptest/mockall；`install-ai-tools.ps1` 无 cargo-insta；`ai-gate.ps1` 无 proptest-regressions | **真** |
| PR-5 `fail-under-lines` 仍为 0 | `scripts/ai-gate.ps1` L90：`--fail-under-lines 0` | **真** |
| DONE 写 82，实机 114 | 本机 `cargo test --workspace --all-features` → **114 passed · 4 ignored** | **真** |
| `docs/COSTS.md` 缺失 | 不存在 | **真** |
| PR-1/3/4 代码兑现 | lore/memory/kg/seed 测齐；embed+vector_store；web+CLI 管理面 | **真**（个别测名微差，见 §3） |
| Phase 5 超 plan（list/delete/regen） | 工作区 + 源码已有 CLI/UI | **真** |
| 「todo 勾上 = 假装完成 PR-2」 | todo Phase 6 只勾 L0/L1/README/DONE，**从未**列 insta 等 | **过重** → 改为「双层 SSOT 无交叉状态」 |
| 「PR-5 半完成却 Phase 6 完成」矛盾 | DONE **已写** 80% 门为 aspirational；plan Phase 6 范围是 L0+L1+DONE | **部分真**：PR 文档期望 > plan 范围，非 Phase 假绿 |
| fmt 跑前 fail | 复验时 `cargo fmt --all --check` 已 PASS | **当时真 / 现已过** |

**根因（不是运行时 bug）：**  
`tasks/plan.md` + `todo.md` 跟踪 **产品 Phase**；`tasks/prs/*.md` 是更细的 **TDD PR 计划**。PR-2/PR-5 余项未回写 Deferred，读 PR 目录会以为未交付，读 todo 会以为全完——**文档状态机分叉**。

---

## 1. `tasks/` 三件套读后

| 文件 | 角色 | 当前状态（复验后） |
|------|------|--------------------|
| `tasks/plan.md` | Phases 0–6 + 架构决策 | Phase 5/6 已校正计数与 CLI 面 |
| `tasks/todo.md` | 复选框 + **PR 切片状态表** | Phases 1–6 [x]；PR-2/PR-5 余项进 Deferred |
| `tasks/prs/*.md` | TDD 子步骤 | 各文件顶栏 **Status** 头（done / deferred / partial） |

---

## 2. 客观门禁证据（复验）

| 命令 | 结果 |
|------|------|
| `cargo fmt --all --check` | **PASS** |
| `cargo test --workspace --all-features` | **114 passed · 0 failed · 4 ignored** |
| `cargo --version` | cargo 1.97.1 / rustc 1.97.1（环境） |

ignored 4 = live（需 `OPENCODE_GO_API_KEY` / `COHERE_API_KEY`），与 plan Checkpoint 一致。

---

## 3. PR-by-PR 对齐矩阵（复验 + 处置）

| PR | 对齐 | 处置（本审计） |
|----|------|----------------|
| **PR-1** seeds | ✅ 测齐（注：`importance_clamps_zero_to_one`、`serializes_round_trip_preserves_fields` 名与 PR 稿略异） | Status → **DONE** |
| **PR-2** dev-deps | ❌ 未开工 | **选 B 显式 deferred**（YAGNI；不引入 4 个 dev-deps） |
| **PR-3** vector | ✅ 代码到位；`docs/COSTS.md` 仍缺 | Status → **DONE**；COSTS 进 Deferred 可选 |
| **PR-4** web | ✅**超 PR** list/delete/regenerate | Status → **DONE+** |
| **PR-5** 收口 | ⚠️ DONE 有 / 80% 硬门与 proptest 无 | Status → **PARTIAL**；硬门进 Deferred |

---

## 4. Phases 0–6（plan vs src）

与初版一致：**产品路径全绿**。Phase 6 的「完成」= L0+L1 + DONE 文档，**不等于** PR-2 工具链或 llvm-cov 80% 硬门。

---

## 5. 已执行修复（解决分叉）

1. **`tasks/todo.md`**：增加「PR 切片状态」表；Deferred 写入 PR-2、PR-5 余项、`docs/COSTS.md`；Phase 5 标明超 plan CLI。
2. **`tasks/plan.md`**：Phase 5 CLI/UI 面与 Phase 6 测试数 114 校正；标明 PR 表 SSOT。
3. **`docs/character-card-agent-done.md`**：82→114；CLI 六子命令；PR honesty 表；Deferred 与 todo 对齐。
4. **`tasks/prs/pr-0{1..5}-*.md`**：顶栏 Status（done / deferred / partial / done+）。
5. **本报告 §0/§5**：复验判决 + 修复记录，避免再被当「未验证断言」。

**刻意未做（方案 B / YAGNI）：**

- 不引入 insta/rstest/proptest/mockall（PR-2 全文实现）
- 不把 `fail-under-lines` 提到 80（无覆盖率基线前会假绿或误红）
- 不写 `docs/COSTS.md`（可选债，无阻塞）

若改选 **方案 A**（做完 PR-2）：按 `tasks/prs/pr-02-dev-deps.md` 开工，再考虑 PR-5 硬门。

---

## 6. 残留风险（非本审计阻塞）

| 项 | 级别 | 说明 |
|----|------|------|
| 工作区未提交 list/delete/regen 代码 | 中 | 与文档「超 plan」一致；需另次 commit |
| llvm-cov 从未实测 | 中 | 升 80% 前先跑 `cargo llvm-cov …` |
| LanceDB 并发写（Windows） | 中 | plan Risks 已记 |
| 知乎文 Open | 低 | 可选 |

---

## 7. 一句话总评（更新）

**初版对齐报告核心事实成立；「假完成」的精确表述应是：Phase 完成 ≠ PR 细切片完成，且缺少交叉状态表。已用 todo PR 表 + PR Status 头 + DONE/plan 数字校正消除分叉。L0 复验 114/0/4 绿。PR-2 主动 deferred；PR-5 硬门仍 partial/deferred。**
