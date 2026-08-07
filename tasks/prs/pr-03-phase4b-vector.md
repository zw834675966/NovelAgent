# PR-3 — Phase 4b: LanceDB + Cohere embed-api

> **Status (2026-08-07): DONE** — `embed.rs` + `vector_store.rs` + live `#[ignore]` 测。  
> 文档债：`docs/COSTS.md` 未写（可选，见 `tasks/todo.md` Deferred）。

## Goal
v1 向量路径落地。**锁定栈不动**：`rig 0.41.0` + `lancedb 0.30` + Cohere `embed-multilingual-v3.0`。

## Pre-flight
**首次构建前必须**：
```powershell
choco install protoc  # 或 winget install protobuf
cargo check --features lancedb  # 5-10 分钟，依赖编译长
```

## Scope
- `src/character/embed.rs` — `CohereEmbed::document()` / `query()` 双实例工厂
- `src/character/vector_store.rs` — `data/lancedb/` insert + `recall_top_k` + 混合重排
- `.env.example` — 确认 `COHERE_API_KEY`（已存在则跳过）
- `docs/COSTS.md` — 新文件，写入 Trial 1K/月、2K/分限、不硬编码单价

## Files touched
- `src/character/embed.rs`（改）
- `src/character/vector_store.rs`（改或新）
- `docs/COSTS.md`（新）
- `README.md`（小节：Windows 装 `protoc`、首次 `cargo build --features lancedb` 预期时间）

## TDD plan
- `embed::document_model_uses_search_document` — 构造时断言 `input_type` 字段
- `embed::query_model_uses_search_query` — 同上
- `vector_store::insert_batches_at_most_96` — 100 条输入分 2 批
- `vector_store::recall_returns_renormalized_scores` — `recall_top_k` 重排后分数 ∈ [0,1]
- `vector_store::hybrid_weights_sum_to_one` — α + β + γ = 1.0（默认）
- `vector_store::recency_decays_monotonically` — 同卡片，时间越远分越低
- `vector_store::empty_table_returns_empty` — 空表不 panic
- `vector_store::integration_zh_3_inserts_zh_query_hits` — `#[ignore = "network + COHERE_API_KEY"]`，3 条中文 lore + 中文 query 命中（`live_create_card_checkpoint_c` 风格）

## Verify
```powershell
cargo test --lib --all-features character::embed character::vector_store
cargo test --lib --all-features -- --include-ignored  # 需 key
cargo clippy --lib --all-features -- -D warnings
# 数据落盘（gitignored）：
ls data/lancedb/
```

## Acceptance
- 7 个新单测 + 1 个 `#[ignore]` 集成测全绿
- live 集成测 3 条中文插入 → 1 条中文 query 命中正确条目
- key 不进 git / 文档 / 测试输出
- `data/lancedb/` 目录结构在 `data/.gitignore` 内

## Risk
- **中**。Windows `protoc` 缺会阻塞首次构建
- LanceDB 文件锁在 Windows 上偶发（避免并发写）
- Cohere 单卡 ~17 texts = 2 调用，trial key 免费；超限需切付费 key
- v1 不调 `optimize()`（数据集 < 100K），feature flag 留接口
