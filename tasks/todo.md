# Todo: 人物卡片 Agent

## Phase 0 — Spec / 计划（当前）

- [x] 加载 workflow / spec / planning skills
- [x] 盘点 `papers/` 并写蒸馏映射 → `papers/DISTILL_character_card_agent.md`
- [x] SillyTavern V2 字段与 lorebook 对齐
- [x] 写 `tasks/plan.md`
- [x] **架构锁定**：v1 = LanceDB + Cohere embed-api（`embed-multilingual-v3.0`）+ 默认中文（2026-08-06）
- [x] 其余推荐：CLI 优先、JSON V2 导出、无 PNG、fastembed 非默认
- [ ] 知乎文正文补入 DISTILL（可选）

## Phase 1 — Schema + 硬约束

- [x] Task 1.1: `src/character/card.rs` V2 + `extensions.novelagent` 类型
- [x] Task 1.2: `constraints.rs` 硬校验
- [x] Task 1.3: `embed.rs` Cohere 常量 + document/query builder（无 LanceDB 尚）
- [x] `.env.example` 增加 `COHERE_API_KEY`；真实 key 仅在 gitignored `.env`

## Checkpoint A

- [x] `cargo test` / clippy L0 绿（Phase 1）

## Phase 2 — 提示词资产与组装

- [x] Task 2.1: `prompts/character/*.md` 四件套（meta system / create / critique / refine）
  - Acceptance: 文件存在且含 schema 说明与 C-* 约束引用
  - Scope: S
- [x] Task 2.2: `prompt_pack.rs` 从卡渲染 system + PHI
  - Acceptance: fixture 卡组装含 name/边界/声浪
  - Verify: 单测
  - Scope: S

## Checkpoint B

- [x] 组装器单测绿

## Phase 3 — Meta-Agent loop

- [x] Task 3.1: `rubric.rs` 五维评分结构
- [x] Task 3.2: `agent.rs` draft→critique→refine（接 `model::build_agent_builder`）
- [x] Task 3.3: JSON repair-once + validate
- [x] Task 3.4: mock/fake 响应单测（不依赖外网）

## Checkpoint C

- [x] 有 key 时 live 跑通一条概念→JSON（`create_card_live` / `live_create_card_checkpoint_c`）
  - 证据：`data/characters/live_checkpoint_c_{card,report}.json`（gitignored）
  - 质量：五维 ≥4，refine_rounds=2；must_fix 仍有「建议级」软项（见对齐报告）

## Phase 4 — Lore / Memory / KG

- [x] Task 4.1: lorebook 从扩展生成 entries（`lorebook.rs`，3–10 条，挂 `character_book`）
- [x] Task 4.2: memory stream 元数据读写（`memory.rs`：ts/kind/importance/text + recency×importance 排序）
- [x] Task 4.3: kg 从 relationships 生成边并序列化（`kg.rs`）
- [x] `seed_card_artifacts` + `create_card` 结束后自动 seed

## Checkpoint D

- [x] 一张示例卡产物三联：card + mem meta + kg
  - 离线：`checkpoint_4a_writes_card_mem_kg_artifacts` → `data/characters/checkpoint_4a_{card,memory,kg}.json`
  - live 顺带写 `live_checkpoint_c_{memory,kg}.json`

## Phase 4b — LanceDB + Cohere embed-api（v1 锁定）

- [x] Task 4b.1: `rig` feature `lancedb` + deps `lancedb`/`arrow-array`；L1 deny/machete 过
- [x] Task 4b.2: `embed.rs` — 已有 document/query builders（Phase 1）
- [x] Task 4b.3: `vector_store.rs` — `data/lancedb/{slug}/` 表 `memory` insert（replace）
- [x] Task 4b.4: top-k ENN + hybrid re-rank（`HybridSearchOpts` / `hybrid_score`）
- [x] Task 4b.5: `#[ignore]` live：`live_lancedb_memory_zh_checkpoint_4b` 绿

## Checkpoint D2

- [x] Windows：`cargo check` 绿（本机 `D:\…\target` 下 bigdecimal build-script 曾 Access Denied → 用 `CARGO_TARGET_DIR` 指向用户目录）；`data/lancedb/checkpoint_4b_*` 有表；key 未进 git

## Phase 5 — 接入

- [x] Task 5.1: lib 导出 + 薄 API（CLI `character-create` + Topcoat `character_create`）
- [x] Task 5.2: chat 注入选中卡 system（CLI `character-chat` + procedure `character_chat`）

## Checkpoint E

- [x] 端到端路径文档化于 README 小节（Character card agent）

## Phase 6 — 门禁

- [x] L0 `scripts/ai-gate.ps1`（并补跑 L1：deny/audit/machete）
- [x] 更新 README / CLAUDE.md 架构段（CLI、LanceDB 首跑、character 层）
- [x] DONE 报告（命令证据）→ `docs/character-card-agent-done.md`

## Deferred

- [ ] GraphRAG 社区摘要批处理
- [ ] ToT 分支选型
- [ ] PNG tEXt 导出
- [ ] Qdrant / fastembed 离线 fallback（feature，非 v1 默认）
- [ ] StoryWriter 级多 Agent 长文
