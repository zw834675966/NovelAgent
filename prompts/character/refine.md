# 任务：据 Critique 精修人物卡

## 任务类型

`refine`

## 当前卡片 JSON

{{card_json}}

## Critique JSON

{{critique_json}}

## 规则

1. **只改必须改的**：优先处理 `must_fix`，其次高严重 `issues`。
2. **禁止扩大 scope**：不新增与概念无关的势力/世界线/多主角。
3. **保持 schema**：仍输出完整 `chara_card_v2` 根对象；`spec`/`spec_version` 正确；`name` 非空。
4. **保留优点**：未点名的强字段尽量原样保留（尤其已成立的声浪示例）。
5. **占位符**：继续使用 `{{char}}` / `{{user}}`。
6. **中文默认**：locale 与中文卡面保持一致。
7. **约束对齐**：若 critique 指出 C-* 未落地，同步改 `system_prompt` / `post_history_instructions` / `constraints[]`。

## 阈值提示

- 默认：任一 score < 3 或 `schema_ok=false` 视为必须修。
- 若 critique 已全部 ≥ 4 且 flags 全 true：仅做最小润色或原样返回合法卡（仍输出完整 JSON）。

## 输出

仅输出精修后的 **完整** 人物卡 JSON 对象，从 `{` 到 `}`。不要输出 critique，不要解释。
