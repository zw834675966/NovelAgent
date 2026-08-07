# PR-4 — Phase 5: 接入面（Topcoat 0.5 procedure + form）

> **Status (2026-08-07): DONE+**  
> create/chat 已落地；并超范围实现 list/delete/regenerate（CLI + `ui_*`）。  
> 实现形态是 Topcoat procedure + 单页 UI，非早期稿的独立 `/characters/new` Form 路由。

## Goal
把 `create_card` 暴露到 Topcoat 0.5 UI。**优先** `#[route(POST "/characters")]` + `Form<NewCharacter>` 表单；不引流式。

## Scope
- `src/lib.rs` — 重新导出 `create_card_from_form`（如需新 fn）
- `src/character/agent.rs` — 暴露 `CreateCardOutcome` 字段给 procedure
- `src/web/chat.rs`（或新建 `src/web/character.rs`）— 新增 `#[page("/characters/new")]` 表单页 + `#[route(POST "/characters")]` 处理器
- `CLAUDE.md` — cold-start 流程补 `topcoat asset bundle` 子命令
- `README.md` — 端到端小节：CLI → Topcoat 表单

## Files touched
- `src/web/chat.rs` 或新 `src/web/character.rs`
- `src/lib.rs`（仅 re-export）
- `src/character/agent.rs`（仅 doc + 必要 pub 字段）
- `CLAUDE.md`（cold-start 段）
- `README.md`（小节）

## TDD plan
- `web::character_form_deserializes_concept_only` — `Form<NewCharacter>` 单字段解析
- `web::character_form_rejects_empty_concept` — 空概念 400/422
- `web::character_route_returns_card_id_and_artifacts_paths` — 成功 → 包含 `id` + `mem_path` + `kg_path` 的 `String`
- `web::character_route_bundles_creation_error_into_string` — LLM 失败时返回 `Ok("(error: ...)")`，与 `send_chat` 模式一致
- 已有 `agent::create_card` 行为复用 — Phase 3 测试作回归基线

## Verify
```powershell
topcoat asset bundle           # 一次，cold start 不再依赖 topcoat dev
cargo run                       # 启动服务
# 浏览器 http://127.0.0.1:3000/characters/new 填表 → 提交
# 验证 data/characters/{id}.json 落盘
cargo test --lib --all-features
cargo clippy --lib --all-features -- -D warnings
```

## Acceptance
- 4 个 web 单测全绿（mock `LlmBackend`，不走真模型）
- 浏览器手测：1 概念 → 1 落盘卡 → UI 提示 OK
- 失败回 `"(error: …)"` 字符串，不泄漏 stack

## Risk
- **中**。Topcoat 0.5 缺 `VecSurrogate`，多卡列表需 `impl_surrogate!` newtype（本 PR 不做，列表后置）
- 表单提交走 POST/Redirect/Get 或 fetch 都要 `application/x-www-form-urlencoded`
- 流式 LLM 延后（0.5 + datastar feature 未验证，MVP 不引）
- `StringSurrogate` 0.5 仍无 `is_ok`，继续走"内嵌错误"模式
