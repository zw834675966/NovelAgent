# 任务：从概念创建人物卡

## 任务类型

`create`

## 输入概念

{{concept}}

## 你必须完成

1. 把概念扩写为 **一个** 可扮演角色（中文默认）。
2. 产出 **完整** SillyTavern V2 根对象 JSON（`chara_card_v2` / `2.0`）。
3. 填满 `data.extensions.novelagent` 工程字段（见 system 中 schema）。
4. 写好可直接挂聊天的：
   - `system_prompt`（含 `{{char}}` / `{{user}}` 占位）
   - `post_history_instructions`（短、硬、可每轮追加）
5. `mes_example` 至少 1 段可辨声浪的示例交换。
6. `relationships` ≥ 2；`constraints` 含 `C-TOM`、`C-NO-USER`、`C-VOICE`、`C-DESIRE-NEED`、`C-SCHEMA`。

## 禁止

- 输出非 JSON 文本
- 空 `name` / 空壳只有名字无设定
- 替 `{{user}}` 写台词或内心
- 堆砌与概念无关的世界观百科
- 英文占位 lorem（除非概念明确要求外语角色且说明理由）

## 输出

仅输出一个 JSON 对象，从 `{` 开始到 `}` 结束。
