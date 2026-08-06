# 蒸馏：人物卡片 Agent 的科学与工程映射

> 输入源：`papers/*`、SillyTavern Character Card V2、剧本基因分析  
> 用途：给「人物卡片 Agent」的系统/用户提示词、约束、loop、KG、记忆体系提供可引用原则  
> 非目标：复述论文全文；不写实现代码

## 1. 外部产品参考：SillyTavern 人物卡

### 1.1 字段分层（V2 / `chara_card_v2`）

| 层 | 字段 | 注入方式 | 工程含义 |
|----|------|----------|----------|
| **永久设定** | `name`, `description`, `personality`, `scenario` | 几乎每轮 | 身份契约 / 世界锚点 |
| **控制面** | `system_prompt` | 替换全局 system | 行为规则与格式硬约束 |
| **尾部纠偏** | `post_history_instructions` | 历史之后（UJB 位） | 防出戏、防替 user 说话、防总结腔 |
| **少样本** | `mes_example` | 早期 / 可挤出 | 声浪与格式 few-shot |
| **开场** | `first_mes`, `alternate_greetings` | 仅首轮 / swipe | 场景钩子 |
| **动态知识** | `character_book`（lorebook） | 关键词触发 + constant | 选择性记忆 / 世界观按需注入 |
| **元数据** | `creator_notes`, `tags`, `creator`, `character_version` | **不进 prompt** | 作者备注、检索标签 |
| **扩展** | `extensions` | 可选 | NovelAgent 自有 schema 挂载点 |

### 1.2 Lorebook 工程要点

- `constant`：常驻（预算内）
- `keys` / `selective` + `secondary_keys`：触发条件
- `insertion_order` / `priority` / `token_budget`：上下文预算与裁剪
- `position`: `before_char` | `after_char`
- 人物书优先于全局 world book

### 1.3 与「提示词工程」的对应

```
system_prompt          → 系统提示词（持久契约）
description+personality+scenario → 角色/世界 context 块
mes_example            → few-shot user/assistant 风格
post_history_instructions → 每轮 user 尾部约束（或 assistant preamble）
character_book         → 检索式记忆（非全量塞入）
```

---

## 2. papers → 五大工程层

### 2.1 提示词工程（System / User）

| 来源 | 可蒸馏原则 | 落到人物卡 |
|------|------------|------------|
| **Generative Agents** | 身份用短段落 seed；行为由记忆+计划条件化，而非巨型静态 prompt | `description` 保持精炼；细节进 memory/lore |
| **ReAct** | 推理与行动交织：Thought / Action / Observation | Agent **创作流程**用 ReAct；卡内角色扮演默认 **不** 暴露 Thought |
| **Emotional StoryGen** | 情绪弧线可被监督与奖励 | 卡内增加 `emotion_arc` 扩展；system 要求展示而非宣告情绪 |
| **Hi-ToM** | 高阶心智：他者信念递归 | multi-char 时约束「所知边界」；防全知 |
| **剧本基因 (gemini_…)** | 外在欲望 vs 内在需求；人物网络；潜台词对白；道德论证 | 卡结构强制：`desire`, `need`, `weakness`, `relationships[]`, `voice` |
| **DOC / StoryWriter** | 大纲层级先于正文 | 创作 Agent 的 user 模板：先 outline 再 card 字段 |

**System 提示词模板原则（卡级）**

1. 身份与边界（只扮演 {{char}}，不写 {{user}}）
2. 声浪与格式（对白 / 动作 / 内心）
3. 所知边界与 ToM
4. 禁止项（出戏、总结句、复述历史）
5. 可选：情绪弧阶段、主题道德轴

**User 提示词模板原则（Agent 操作级）**

1. 任务类型：`create | upgrade | critique | export_st | inject_memory`
2. 输入：一句话概念 / 已有 JSON / 剧情片段
3. 输出 schema：严格 JSON（V2 + NovelAgent extensions）
4. 质量门禁维度（见约束工程）

### 2.2 约束工程（Constraints）

从剧本评估矩阵 + ST Critical Constraints 蒸馏：

| 约束 ID | 内容 | 来源 |
|---------|------|------|
| C-VOICE | 去掉名字仍可区分声浪 | 剧本基因 §五 |
| C-SUBTEXT | 对白是动作，禁止 on-the-nose | 剧本基因 |
| C-DESIRE-NEED | 必须可陈述外在欲望与内在需求 | Truby / 剧本基因 |
| C-NETWORK | 至少 2 个关系节点定义主角 | 人物网络 |
| C-TOM | 不写角色不可能知道的信息 | Hi-ToM + ST |
| C-NO-USER | 禁止代写 user 言行 | ST 社区实践 |
| C-NO-BUTTON | 禁止道德总结/收束金句结尾 | ST immersive prompts |
| C-EMOTION | 情绪用身体/行为外化，不贴标签堆砌 | Emotional StoryGen |
| C-BUDGET | lore/记忆插入受 token_budget | ST lorebook |
| C-SCHEMA | 导出必须合法 `chara_card_v2` | ST V2 |

**约束落地方式（三层）**

1. **Prompt 内声明**（软）
2. **结构化字段校验**（硬，Rust schema）
3. **Critique loop 打分**（半硬，LLM-as-judge + 规则）

### 2.3 Loop 工程（Agent 运行时）

| Loop | 论文 | 人物卡 Agent 用法 |
|------|------|-------------------|
| **Self-Refine** | 生成→自评→改写 | 默认卡生成：`draft → critique_rubric → refine`（1–3 轮） |
| **Reflexion** | 语言反思写入 episodic memory | 用户点「不满意」时：写 reflection，下次同标签卡复用教训 |
| **ReAct** | 工具调用 | 读/写文件、检索 lore、查 KG、导出 JSON |
| **ToT** | 多路思想树 | 仅高难度：多个性格方向并行再选优（贵，默认关） |
| **DOC / StoryWriter 管线** | outline → plan → write | `concept → character_network → card_fields → prompts → lore_entries` |
| **Harness 门禁** | SWE-bench 精神 | 单元测试：schema、必填约束、禁词、token 预算模拟 |

**推荐默认 loop（便宜可验）**

```text
IN  → parse intent + seed
    → draft CharacterSpec (structured)
    → critique (rubric: Concept/Character/Voice/ToM/Constraints)
    → refine (max 2)
    → assemble ST V2 + NovelAgent extensions
    → optional: inject lorebook from KG
OUT → JSON + human-readable prompt pack
```

### 2.4 知识图谱工程（KG）

| 来源 | 原则 | 人物卡映射 |
|------|------|------------|
| **GraphRAG** | 实体图 + 社区摘要；全局问题走 community，局部走邻域 | 世界观/长篇设定：实体抽取 → 关系边 → 社区主题摘要 → 问答/注入 |
| **StoryWriter outline** | 事件节点 + 人物 + 事件关系 | 剧情卡：`Event`–`involves`–`Character`，`Event`–`causes`–`Event` |
| **Generative Agents world tree** | 世界区域树 + 智能体可见子图 | 场景/地点层级进 KG；角色只「记得见过的」 |

**最小 KG schema（v0）**

```text
Node kinds: Character | Trait | Desire | Need | Relationship | Location | Event | LoreFact | EmotionBeat
Edges: has_trait, wants, needs, knows, related_to(type), located_in, participates_in, causes, contradicts, about
```

**检索策略**

- 对话触发关键词 / 实体 → 邻域 1–2 hop + 相关 LoreFact
- 全局「这个人物是谁」→ GraphRAG 式 community summary（人物 ego-graph 摘要）
- 注入前按 `token_budget` 截断

### 2.5 记忆存储体系

对齐 **Generative Agents 三件套** + **ST lorebook** + **分层记忆工业实践**：

| 层 | 名称 | 内容 | 存储建议（NovelAgent） |
|----|------|------|---------------------------|
| L0 | Working / 会话 | 当前 chat transcript | 内存 / Topcoat signal |
| L1 | Card 静态 | description, personality, system, PHI | `data/characters/{id}.json` |
| L2 | Lorebook | 触发式世界条目 | 同卡 `character_book`；可同步 embed 进向量库 |
| L3 | Memory Stream | 观察/对话摘要、带时间戳 | JSON 元数据 + **v1 向量副本进 LanceDB** |
| L4 | Reflection | 高阶总结（价值观、关系状态） | kind=`reflection`；同样可 embed |
| L5 | Episodic lessons | Reflexion 教训（创作失败复盘） | Agent 级；可选进 LanceDB |
| L6 | Vector index | 语义检索 | **v1 锁定：LanceDB + fastembed**（见下） |
| L7 | KG | 结构化关系 | JSON 边表；GraphRAG 社区摘要后置 |

### L6 决策（v1 锁定，2026-08-06 修订）

| 角色 | 选型 | 理由 |
|------|------|------|
| 向量库 | **LanceDB**（`rig` feature `lancedb`） | Rust/embedded、Windows 单机、rig 一等集成、无 Docker |
| 嵌入 | **Cohere embed-api** · **`embed-multilingual-v3.0`** | 中文/多语 1024 维；trial 额度；rig 内置 `providers::cohere` |
| 密钥 | **`COHERE_API_KEY`**（仅 `.env`） | 与 `cohere::Client::from_env()` 一致 |
| 卡/提示语言 | **默认中文** | 产品面向中文创作 |
| 人物卡格式 | **SillyTavern V2** | 兼容生态；扩展进 `extensions.novelagent` |
| 非 v1 默认 | fastembed 离线 / Qdrant | 可选 feature |

数据目录：`data/lancedb/`、`data/characters/`（gitignore 内容）。**禁止**把 API key 写入卡 JSON 或仓库文档。

**检索评分（Generative Agents 风格）**

```text
score = α·recency + β·importance + γ·cosine_relevance
```

- v0 可先做 α/β（无向量）把 stream API 跑通。  
- **v1**：`γ` 来自 LanceDB；入库用 `input_type=search_document`，查询用 `search_query`（Cohere v3 约定）。

**写入策略**

- 每 N 轮对话或场景结束后：摘要 → memory stream
- importance ≥ 阈值 → 触发 reflection 批处理
- reflection 结果写回 stream + 可选更新 KG 边（关系变化）

---

## 3. 「人物卡片 Agent」定义（产品语义）

**是什么**

一个 **元 Agent**：输入概念/剧情/残缺卡，输出：

1. 合法 **SillyTavern 兼容** 人物卡（V2 JSON）
2. NovelAgent **扩展字段**（欲望/需求/网络/情绪弧/约束清单）
3. 可直接挂到对话系统的 **system / user / PHI 提示词包**
4. 可选 **lorebook + 初始 memory seeds + KG 种子**

**不是什么**

- 不是完整小说写作器（那是 StoryWriter 级多 Agent，后续）
- 不是通用聊天 UI（可复用现有 Topcoat chat）
- 不是通用向量 SaaS：v1 仅 **嵌入式 LanceDB + 本地 fastembed**，不为托管向量产品本身

---

## 4. 知乎文说明

目标 URL：`https://zhuanlan.zhihu.com/p/2036048794261901878`  
当前环境 **登录墙 / 抓取失败**，正文未完整纳入本蒸馏。  
**待办**：用户粘贴正文或提供可访问镜像后，补一节「知乎特有主张 → 工程改动」。

临时假设（不作为已验证事实）：中文社区常见「人物卡 = 结构化提示词 + 世界观条目 + 示例对话」与 ST V2 同构；若文中强调「约束卡 / 思维链分离 / 记忆分层」，与本文 §2 已对齐，冲突处以 **可测验收** 为准。

---

## 5. 论文文件清单（本目录）

| 文件 | 主题 | 主映射层 |
|------|------|----------|
| Generative_Agents.pdf | 记忆流·检索·反思·计划 | 记忆 + loop |
| GraphRAG_KnowledgeGraph.pdf | 实体图·社区摘要 | KG |
| ReAct_Prompting.pdf | 推理+行动 | Agent loop / tools |
| Reflexion_Loop.pdf | 语言强化学习 | 失败复盘记忆 |
| Self_Refine_Loop.pdf | 自反馈迭代 | 默认生成 loop |
| Tree_of_Thoughts.pdf | 搜索式推理 | 可选昂贵 loop |
| DOC_Long_Story.pdf | 细大纲+控制器 | 长设定一致性 |
| RecurrentGPT_LongStory.pdf | 循环状态长文 | 长会话状态 |
| StoryWriter_MultiAgent.pdf | 多 Agent 大纲/规划/写作 | 管线拆分 |
| Emotional_StoryGen.pdf | 情绪弧 | 约束 + 扩展字段 |
| Hi_ToM_Psychology.pdf | 高阶 ToM | 约束 + multi-char |
| SWE_bench_Harness.pdf | 真实任务评测 harness | 验收与门禁哲学 |
| gemini_优秀剧本共通特性分析.md | 剧本工业基因 | 人物结构与评分维度 |

---

## 6. 设计原则（给实现与评审用）

1. **卡要薄，记忆要厚**：静态 prompt 只保留契约与声浪；细节走 lore/memory/KG。  
2. **软约束进 prompt，硬约束进 schema/测试**。  
3. **默认 Self-Refine 2 轮；ToT 默认关**。  
4. **ST 兼容优先**：导出能被 SillyTavern 导入；扩展只放 `extensions.novelagent`。  
5. **证据门禁**：声称一致性/ToM 改善必须有 rubric 样例或测试，禁止 vibe-done。  
6. **v1 向量栈固定**：LanceDB + Cohere `embed-multilingual-v3.0`；换模型/后端须改 plan，不在实现中静默替换。  
7. **默认中文**；导出 JSON V2（不做 PNG）；交互 CLI/lib 优先。
