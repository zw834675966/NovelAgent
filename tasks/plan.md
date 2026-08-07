# Implementation Plan: 人物卡片 Agent（Character Card Agent）

## Overview

在 NovelAgent（Rust + rig + Topcoat）中落地一个 **人物卡片元 Agent**：从概念/剧情/半成品卡生成 **SillyTavern V2 兼容** 人物卡，并蒸馏挂载 **系统/用户提示词、约束、生成 loop、知识图谱种子、分层记忆**。科学依据与映射见 [`papers/DISTILL_character_card_agent.md`](../papers/DISTILL_character_card_agent.md)。

本阶段 **只规划与文档骨架**；实现按垂直切片推进，每片可测。

## Skills 已用

| Skill | 用途 |
|-------|------|
| `using-matt-addy-workflow` | DEFINE→PLAN；未直接 BUILD |
| `spec-driven-development` | 目标/边界/成功标准先于代码 |
| `planning-and-task-breakdown` | 垂直切片 + 验收 + checkpoint |
| 论文/ST 调研 | papers 摘要 + ST V2 spec + 剧本基因 md |

## Goals

1. 用户可提交「一句话人物概念」→ 得到合法 `chara_card_v2` JSON + NovelAgent 扩展。  
2. 生成物包含：`system_prompt`、`post_history_instructions`、示例对话、初始 lorebook（可选）。  
3. 内部有 **Self-Refine 评分环**（概念/人物/声浪/ToM/约束）。  
4. 记忆/KG 有可序列化 schema；**v1 语义检索 = LanceDB + Cohere embed-api**（见架构决策 #5）。  
5. 与现有 `model::build_agent_builder` / chat 路径可接，但不破坏现有 readiness/chat。

## Non-Goals（本里程碑）

- 完整长篇写作多 Agent（StoryWriter 全量）
- GraphRAG 社区摘要生产级 pipeline
- Qdrant / Neo4j / 向量 SaaS 托管（v1 不做）
- SillyTavern 全功能 UI 复刻 / PNG tEXt 导出（JSON 足够）
- 知乎文逐句对齐（待用户提供正文）

## Architecture Decisions

1. **领域文件夹** `src/character/`（AGENTS.md §12.2）：`card` schema、`prompt` 组装、`memory`、`kg`、`agent` 编排。  
2. **ST 兼容**：根导出为 V2（借鉴 [SillyTavern](https://github.com/SillyTavern/SillyTavern) / [character-card-spec-v2](https://github.com/malfoyslastname/character-card-spec-v2)）；NovelAgent 专有字段进 `data.extensions.novelagent`。  
3. **错误分层**：character 库内 `thiserror`；app/web 边界 `anyhow`。  
4. **Loop 默认**：Self-Refine ≤2；ReAct 仅当有 tool；ToT feature-flag 默认 off。  
5. **向量 + 嵌入（v1 锁定，2026-08-06 修订）**  
   - **向量库：LanceDB**（embedded，`data/lancedb/`；`rig` feature `lancedb` → `rig::lancedb`）。  
   - **嵌入：Cohere API（embed-api）**，默认模型 **`embed-multilingual-v3.0`**（中文 + 多语，1024 维）。  
   - 环境变量 **`COHERE_API_KEY`**（仅 `.env`，永不进 git / plan 正文）。  
   - 经 rig：`cohere::Client::from_env()` → `EmbeddingsBuilder` → LanceDB。  
   - **fastembed**：可选离线 fallback（feature），**非 v1 默认**。  
   - 加依赖 / 开 `lancedb` feature 后跑 **L1**。  
6. **默认语言：中文** — 卡字段、system/PHI、meta-agent 提示词、示例对话默认中文。  
7. **首版交互：CLI / lib API 优先**；Topcoat 表单后置。  
8. **导出：JSON V2 足够**；不做 PNG tEXt。  
9. **卡片存储**：`data/characters/{id}.json`；向量不进 JSON。  
10. **提示词资产**：`prompts/character/*.md`。  
11. **评测**：schema 单测；Cohere+LanceDB 用 `#[ignore]` 或需 key 的集成测。

## Target Module Layout

```text
src/character/
  mod.rs              # 公共面
  card.rs             # V2 + NovelAgent extensions 类型
  constraints.rs      # 硬校验（schema 级）
  prompt_pack.rs      # system/user/PHI 组装
  lorebook.rs         # character_book 构建
  memory.rs           # memory stream 元数据 + 与向量索引桥接
  vector_store.rs     # LanceDB 打开/写入/top_k（v1）
  embed.rs            # Cohere embedding 句柄（v1 默认）
  kg.rs               # 最小图类型 + 序列化
  agent.rs            # draft → critique → refine 编排
  rubric.rs           # 评分维度与 critique prompt 载荷
prompts/character/    # 中文模板
  system_meta_agent.md
  user_create.md
  critique_rubric.md
  refine.md
data/characters/      # 卡 JSON（gitignore 内容）
data/lancedb/         # LanceDB 表目录（gitignore）
papers/DISTILL_...
tasks/plan.md
tasks/todo.md
```

## Vertical Slices（实现顺序）

### Phase 0 — Spec 冻结（文档）

- 冻结字段表、扩展 schema、rubric 五维、loop 伪代码  
- 补知乎文（若用户粘贴）  
- **Checkpoint 0**：人审 `DISTILL` + 本 plan

### Phase 1 — Schema + 硬约束（无 LLM）

- Rust 类型：`TavernCardV2`、`NovelAgentCharExt`  
- `validate_card`：必填、禁空 name、extensions 命名空间  
- serde JSON 往返测试  
- **Checkpoint 1**：`cargo test -p novelagent character::` 绿

### Phase 2 — 提示词包 + 组装器

- 加载 `prompts/character/*`  
- `assemble_prompt_pack(card) -> System/User/PHI strings`  
- 单元测试：占位符 `{{char}}`、空 system 回落  
- **Checkpoint 2**：给定 fixture 卡，组装结果快照或 asserts

### Phase 3 — Meta-Agent loop（接 rig）

- `create_card(concept) -> Card`：draft → critique → refine  
- 始终 `Ok` 路径对外（或 thiserror 可映射）；失败写进卡/日志  
- mock 测试：固定 fake LLM 响应解析  
- **Checkpoint 3**：本地有 key 时手动 `create` 一条卡 JSON

### Phase 4 — Lorebook + Memory 元数据 + KG seed ✅ (2026-08-06)

- 从扩展字段生成 3–10 条 lore entries → `lorebook.rs`  
- memory stream 元数据 append（ts / kind / importance / text）→ `memory.rs`  
- KG：从 relationships 抽边，序列化 → `kg.rs`  
- `seed_card_artifacts` 挂到 `create_card` 尾部  
- **Checkpoint 4a**：`data/characters/checkpoint_4a_{card,memory,kg}.json`（离线单测）

### Phase 4b — LanceDB + Cohere embed-api（v1 向量）✅ (2026-08-06)

- `rig` feature：`lancedb` + 直接依赖 `lancedb` 0.30 / `arrow-array` 58  
- `embed.rs`：`COHERE_API_KEY` + `embed-multilingual-v3.0`（document/query input_type）  
- `vector_store.rs`：`data/lancedb/{slug}/` 表 `memory`；`index_memory_stream` + `search_memory_hybrid`  
- 混合分：`score = α·recency + β·importance + γ·cosine_relevance`（默认 0.2/0.3/0.5）  
- **Checkpoint 4b**：`live_lancedb_memory_zh_checkpoint_4b` 中文 ≥3 条写入 → query 命中；L1 deny/machete 过  
- **Windows 备注**：本机仓库 `target/` 下偶发 build-script Access Denied（`bigdecimal`）；可用  
  `$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo\novelagent-target"`；`protoc` 已装
### Phase 5 — 接入面（薄）✅ (2026-08-06；UI 管理能力 2026-08-07 超范围)

- CLI（`app::run`）：`character-create` / `character-chat` / `character-list` / `character-delete` / `character-regenerate` / `help`
- Topcoat：`web/character.rs` procedures + `web/chat.rs` 的 `ui_*` 薄包装（创建 / 列表 / 对话 / 删除 / 重生）
- 落盘：`data/characters/{slug}_{card,memory,kg,report,concept}.json`（`persist.rs`；concept 供 regenerate）
- chat 注入：`assemble_prompt_pack` → preamble（未强制 LanceDB 检索片段）
- **Checkpoint 5**：README 小节 + formatter / UI 薄包装单测；live create 需 key（同 Phase 3）

### Phase 6 — Harness 与文档 ✅ (2026-08-06；计数 2026-08-07 校正)

- README：Character 小节 + LanceDB/Windows `CARGO_TARGET_DIR` / protoc 首跑说明
- CLAUDE.md：`app::run` 分发 + `character/` 领域 + live 测命令
- L0 + L1 `scripts/ai-gate.ps1` 绿（**114 unit pass / 4 ignored**，2026-08-07 复跑）
- **Checkpoint 6**：[`docs/character-card-agent-done.md`](../docs/character-card-agent-done.md)
- **非 Phase 6 必做**（见 `tasks/todo.md` Deferred / PR 表）：PR-2 dev-deps；PR-5 的 llvm-cov 80% 硬门
- PR 切片状态 SSOT：`tasks/todo.md`「PR 切片状态」表（勿仅看 `tasks/prs/*.md` 无状态头）

## Data Model (extensions.novelagent)

```json
{
  "desire": "外在可见目标",
  "need": "内在必须克服的缺陷/认知",
  "weakness": "开局致命弱点",
  "moral_axis": "故事要论证的价值张力",
  "emotion_arc": ["hope", "fear", "resolve"],
  "relationships": [
    { "name": "…", "type": "ally|rival|…", "defines_protagonist_how": "…" }
  ],
  "voice_markers": ["用词", "句式", "禁忌表达"],
  "constraints": ["C-TOM", "C-NO-USER"],
  "knowledge_bounds": "该角色不知道什么"
}
```

## Prompt Engineering Deliverables

| 资产 | 角色 |
|------|------|
| `system_meta_agent.md` | 元 Agent 身份：剧作顾问 + ST 卡工程师 |
| `user_create.md` | 输入概念 → 要求 JSON schema 输出 |
| `critique_rubric.md` | 五维打分：Premise / Character / Voice / ToM / Constraints |
| `refine.md` | 据 critique 改卡，禁止扩大 scope |
| 卡内 `system_prompt` | 扮演契约（从字段渲染） |
| 卡内 `post_history_instructions` | 尾部纠偏 |

## Loop Engineering Deliverables

```text
create_card:
  draft    = LLM(user_create, concept)
  parse    = strict JSON or repair-once
  critique = LLM(rubric, draft) → scores + issues
  if any score < threshold and rounds < 2:
    draft = LLM(refine, draft, critique)
  validate_hard(draft)
  assemble packs + optional lore/kg/memory seeds
```

## Memory & KG Deliverables

- Memory entry: `{ id, ts, kind: observation|reflection|seed, text, importance }`  
- **v0 路径**：元数据 JSON + recency×importance  
- **v1 路径（锁定）**：同一文本经 **Cohere embed-multilingual-v3.0** → **LanceDB**；检索 = 向量相似度 × 时间/重要性加权  
- KG: nodes/edges JSON；从 `relationships` + lore facts 生成  
- **不做（本里程碑）**：GraphRAG 社区摘要、Qdrant；fastembed 仅可选 fallback

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Topcoat 表达式/Surrogate 限制拖垮 UI | Med | 先 CLI/procedure 纯 Rust；UI 后置 |
| LLM JSON 不稳定 | High | repair-once + schema validate；失败返回结构化错误 |
| Prompt 膨胀 | Med | 薄卡厚记忆；lore 预算 |
| 知乎主张未对齐 | Low | 开放问题；用户补文后修订 DISTILL |
| 过度设计 GraphRAG | High | v0/v1 仅 ego 关系图 + 文件/LanceDB |
| LanceDB / Arrow / protoc Windows 链 | Med | 文档写 protoc；先 `cargo check --features lancedb`；失败再评估 |
| Cohere 额度 / 密钥泄露 | High | 仅 `.env`；聊天中出现过 key 则建议轮换；测试不打印 key |
| 中文 embedding | Low | 已选 multilingual-v3.0；固定中文 query 回归 |

## Open Questions

1. ~~首版交互~~：**已决 — CLI / lib API 优先**  
2. ~~向量/嵌入~~：**已决 — LanceDB + Cohere `embed-multilingual-v3.0`（`COHERE_API_KEY`）**  
3. ~~SillyTavern 导出~~：**已决 — JSON V2，不做 PNG**  
4. **知乎文**：仍待粘贴（可选）  
5. ~~语言~~：**已决 — 默认中文**  
6. ~~embedding 模型~~：**已决 — embed-multilingual-v3.0**

## Success Criteria

- [ ] DISTILL + plan 经你认可  
- [ ] `CharacterCard` JSON 可被 serde 验证且含 V2 必填字段  
- [ ] `create_card` 对固定概念产出可导入结构（手动或测试）  
- [ ] Self-Refine 至少跑通 1 次 critique→refine  
- [ ] 记忆或 lore 至少一种动态注入路径有单测  
- [ ] **v1**：LanceDB 写入 + **Cohere** 查询 top-k 有可复现证据  
- [ ] L0 门禁绿；引入 lancedb feature 后 L1 过或记 SKIP  

## Commands（仓库惯例）

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# 或
pwsh -File scripts/ai-gate.ps1
```
