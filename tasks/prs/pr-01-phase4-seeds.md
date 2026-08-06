# PR-1 — Phase 4: lorebook / memory / kg seeds

## Goal
把 Phase 4 三件（lorebook / memory meta / kg）从脚手架升到可测。**无新外部依赖**。

## Scope
- `src/character/lorebook.rs` — `build_lorebook` / `attach_lorebook` / `MAX_LORE_ENTRIES` 行为覆盖
- `src/character/memory.rs` — `MemoryStream` 元数据追加 / 重要性裁剪
- `src/character/kg.rs` — `KnowledgeGraph` 边去重 / serde 往返
- `src/character/seed.rs` — 三联产物（card + mem meta + kg）单文件

## Files touched
- `src/character/{lorebook,memory,kg,seed}.rs`（仅 `#[cfg(test)]` 模块）

## TDD plan（先红后绿）
- `lorebook::attaches_lorebook_within_budget` — `attach_lorebook` 在 ≤ `MAX_LORE_ENTRIES` 时全收
- `lorebook::truncates_when_over_max` — 超过 `MAX_LORE_ENTRIES` 时按 `priority` 截
- `lorebook::constant_entries_always_active` — `constant: true` 不受 key 命中影响
- `memory::append_preserves_order` — 按 `ts` 升序追加
- `memory::importance_clamped_to_0_1` — 越界值截断
- `memory::filters_by_kind` — `kind = reflection` 独立可查
- `kg::dedupes_reciprocal_edges` — A→B 与 B→A 合并为单向带 `bidirectional: true`
- `kg::serializes_round_trip` — JSON 往返后节点 / 边相等
- `seed::produces_card_mem_kg_trio` — fixture card → `seed_card_artifacts` 三件齐

## Verify
```powershell
cargo test --lib --all-features character::lorebook character::memory character::kg character::seed
cargo clippy --lib --all-features -- -D warnings
```

## Acceptance
- 9 个新测试全绿
- `validate_card` 仍过（lorebook 附加后 schema 兼容）
- 现有 68 测试无回归

## Risk
低。`lorebook.rs` 脚手架已知 1 个 bug（已修），其余未读源码；写测试时按需小幅 fix。**禁止**改外部接口。
