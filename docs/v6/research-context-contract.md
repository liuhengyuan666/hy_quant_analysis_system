# Research Context Contract

> **Version:** 1.0
> **Scope:** V6 Semantic Foundation
> **Status:** Active
>
> This document is the design contract for `crates/research-context`. It complements ADR-068 and is the first place to read when adding a new Summary or consuming ResearchContext from a new surface.

---

## 第一页：设计原则

### 1.1 Canonical Semantic Model

`ResearchContext` is the **canonical semantic model** for all cross-consumer research outputs in the quant analysis system.

- Any engine that wants to communicate Market State, Breadth, Rotation, Signal, Divergence, or Trust semantics to a consumer must do so through the Summaries defined in `ResearchContext`.
- Consumers (CLI, Desktop, API, GPT, PDF, Email) are not allowed to build their own ad-hoc interpretations of raw engine outputs.
- If a semantic concept is missing from `ResearchContext`, add it there first; do not add it to a consumer-specific DTO.

### 1.2 Consumer-Neutral

> **Rule-01: ResearchContext is consumer-neutral.**

- Fields in `ResearchContext` may only be justified by **research semantics**, not by the needs of a particular consumer.
- Forbidden justifications for adding a field:
  - "GPT needs this prompt fragment."
  - "Desktop wants to display a card title."
  - "Markdown needs a section header."
  - "CLI needs a formatted string."
- Allowed justifications:
  - "This number is a meaningful research metric (e.g. breadth percentage)."
  - "This describes the state of a market regime."
  - "This captures the confidence/reliability of the data."

### 1.3 Stable Contract

- `ResearchContext` evolves through **additive changes** only.
- Existing fields must keep their meaning once published.
- Breaking changes require a new major version of the `ResearchContext` struct and an explicit migration plan.
- New optional fields must use `#[serde(default)]` or equivalent schema-evolution guards.

### 1.4 Summary Only

- `ResearchContext` aggregates **semantic summaries**, not raw data.
- It does **not** contain OHLCV bars, full symbol lists, DataFrames, or per-symbol raw scores.
- If a consumer needs raw data, it should fetch it from `market-store` or the relevant engine directly, outside the ResearchContext contract.

### 1.5 Engine Agnostic

- `ResearchContext` must not leak implementation details of any specific engine.
- It describes the **research result**, not the **algorithm that produced it**.
- Example: use `trend_score: i32` instead of `ma20_ma60_crossover_state`.

### 1.6 Conservative vs. Rapid Evolution

> **ResearchContext evolves conservatively; Presentation evolves rapidly.**

- `ResearchContext` is the slow-moving foundation.
- `ReportDocument`, `Section`, `SectionKind`, and `Formatter` are the fast-moving presentation layer.
- Presentation can experiment with new layouts, card types, and output formats without touching ResearchContext.

---

## 第二页：Summary 职责边界

| Summary | 负责 | 不负责 |
|---|---|---|
| `MarketStateSummary` | 当前市场状态（趋势、波动、流动性维度），以及综合风险/状态评分 | 原始 OHLC 数据、个股行情、具体策略信号 |
| `BreadthSummary` | 市场宽度百分比、5 日变化、触发条件、 proxy 说明 | 个股列表、板块明细、原始 bar 数据 |
| `RotationSummary` | 轮动排名 top/bottom、轮动状态、领先/落后板块或资产 | 实时价格、历史收益率曲线、具体持仓建议 |
| `SignalSummary` | 最终信号列表、看多/看空数量、平均强度、方向分布 | 策略内部得分、回测结果、执行计划 |
| `DivergenceSummary` | Signal-State 背离持续天数、样本数量、方向 | 解释性结论、修复动作、归因分析 |
| `TrustSummary` | 数据可信度等级、headline、关键警告 | 具体修复命令、自动化修复逻辑 |

### 字段来源规则

对于每个 Summary，新增字段前必须回答：

1. 这个字段是**研究语义**本身，还是为了方便某个 Consumer 显示？
2. 如果 Consumer 改变（例如从 Markdown 变为 Desktop Card），这个字段是否还有意义？
3. 这个字段是否可以由 `ResearchContext` 中已有的字段推导出来？

只有问题 1 和 2 的答案为「是」，且问题 3 的答案为「否」时，才允许新增字段。

---

## 跨文档引用

- `docs/v6/adr-068-research-context-reporting-layer.md`：架构决策与 crate 边界
- `docs/v6/platform-evolution-v6.md`：V6 平台演进与架构大图
- `docs/v6/reporting-consumer-inventory.md`：消费者矩阵
- `docs/v6/reporting-output-inventory.md`：字段映射
- `docs/v6/reporting-boundary-inventory.md`：owner/producer/consumer/lifecycle
