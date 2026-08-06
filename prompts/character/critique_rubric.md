# 任务：人物卡五维 Critique

## 任务类型

`critique`

## 待评卡片 JSON

{{card_json}}

## 评分维度（每维 1–5 整数；1=不可用，3=能用但平，5=可导入且有辨识度）

| 维 | 名称 | 判据 |
|----|------|------|
| Premise | 概念/前提 | 欲望—需求冲突清晰；scenario 有戏剧钩子；非万能人设 |
| Character | 人物工程 | desire/need/weakness/moral_axis 齐全且互锁；relationships≥2 且定义主角 |
| Voice | 声浪 | 去掉名字仍可辨；mes_example 非说明书腔；voice_markers 具体 |
| ToM | 心智/边界 | knowledge_bounds 明确；无全知；不写角色不可能知道的事 |
| Constraints | 约束落地 | system/PHI 覆盖 C-NO-USER/C-TOM/C-NO-BUTTON 等；constraints 列表与正文一致 |

## 额外硬检查（布尔）

- `schema_ok`：是否像合法 `chara_card_v2`（有 name、spec 字段合理）
- `placeholders_ok`：system/PHI/示例是否正确使用 `{{char}}`/`{{user}}`
- `locale_ok`：默认中文卡是否中文主导

## 输出 JSON schema（仅此对象，无围栏）

```text
{
  "scores": {
    "premise": 1-5,
    "character": 1-5,
    "voice": 1-5,
    "tom": 1-5,
    "constraints": 1-5
  },
  "flags": {
    "schema_ok": true/false,
    "placeholders_ok": true/false,
    "locale_ok": true/false
  },
  "issues": ["具体问题，可操作", "..."],
  "must_fix": ["低于阈值或硬失败必须改的点"],
  "summary": "一两句总评"
}
```

## 评分纪律

- 只评 **已给出的 JSON**，不发明卡中不存在的优点。
- `issues` / `must_fix` 写可执行修改指令，不写空话（禁止「更生动一些」而无字段指向）。
- 任一 score ≤ 2 或 `schema_ok=false` → 必须出现在 `must_fix`。
