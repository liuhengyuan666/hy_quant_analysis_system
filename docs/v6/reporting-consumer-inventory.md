# Reporting Consumer Inventory

> 目标：明确所有报告消费者及其对 Research Context / Report 的需求，验证 V6 Reporting Layer 的 Contract 设计是否覆盖主要消费者。
> 范围：当前已实现 + 未来可能接入的消费者。

---

## 1. 消费者总览

| 消费者 | 当前入口 | 优先级 | 实时性 | 主要需求 |
|---|---|---|---|---|
| CLI 用户 | `quant-cli research*` / `audit*` | P0 | 按需 | 结构化研究/审计输出，支持 markdown/json/text |
| GPT / LLM | `analyze-with-llm` / `research-skills` | P0 | 每日 | 完整研究语义上下文，偏好 JSON |
| Desktop 用户 | Tauri Dashboard + Research Layer | P1 | 刷新时 | Snapshot + 可视化 + 简洁文本 |
| 未来 API | REST / WebSocket（规划中） | P2 | 实时/按需 | Machine-friendly JSON，稳定字段 |
| 未来 PDF | 报告导出（规划中） | P2 | 按需 | Executive + Detail + 图表 |
| 未来 Email/Telegram | 推送（规划中） | P3 | 每日 | 极简 Executive，纯文本/HTML |

---

## 2. CLI 用户

### 2.1 已用命令

| 命令 | 当前输出 | 所需 Context | 所需格式 |
|---|---|---|---|
| `research srd` | Duration, StrongBuy 数, Average Signal, Breadth trend, Rotation pattern, Historical percentile | State, Signal, Breadth, Rotation | markdown（默认） |
| `research stretch` | Crowding/Breadth/Momentum/Leverage 维度等级 + Evidence | Rotation, Breadth, Signal | markdown |
| `research analytics` | 条件前向收益统计 | Signal, State, Breadth, Rotation | markdown |
| `research review` | 季度 SRD/Stretch/Analytics 聚合报告 | State, Signal, Breadth, Rotation, Divergence | markdown |
| `audit rotation-ranking` | Top/Bottom 轮动排名 | Rotation | markdown / json |
| `audit symbol-scoreboard` | 标的综合得分板 | Signal, Rotation | markdown / json |
| `audit state-audit` | 状态分布统计 | State | markdown / json |
| `audit signal-divergence` | Signal 与 State 背离样本 | Signal, State, Divergence | markdown / json |

### 2.2 当前痛点

- 各命令独立拼 markdown，格式不一致。
- `ResearchSnapshot` 是 `apps/cli/src/commands/research.rs` 内部结构，无法复用。
- `audit` 命令直接 `println!` 输出表格和文本，Business 与 Presentation 耦合。

### 2.3 V6 期望

- 所有 Research/Audit 命令通过统一 `ReportDocument → Formatter` 输出。
- CLI 只解析参数、调用 Service、输出 Renderer 结果。

---

## 3. GPT / LLM

### 3.1 当前入口

| 入口 | 输入 | 当前处理方式 |
|---|---|---|
| `analyze-with-llm` | `DashboardSnapshot` | `research-context` 从 DashboardSnapshot 构建 `ResearchContext` |
| `research-skills` | skill JSON | `research-renderer` 解析 skill JSON 生成 `ResearchSummary` |

### 3.2 当前痛点

- LLM 上下文从 `DashboardSnapshot`（Production Surface）构建，违反 Production Surface 冻结原则。
- `research-context` 当前依赖 `report-engine`，方向倒置。
- GPT 真正消费的是**语义**，不是 Markdown；当前先渲染成 Markdown 再让 GPT 重新理解，造成信息损失。

### 3.3 V6 期望

- GPT 直接消费 `ResearchContext` JSON。
- `ResearchContext` 不依赖 `report-engine`。
- `reporting` / `report-builder` 从 `ResearchContext` 生成报告，而不是反向。

---

## 4. Desktop 用户

### 4.1 当前入口

- Dashboard 面板：Market Regime、Environment、Trust、Rotation、Signals、Backtest
- Research Layer 按钮：5 个按钮生成只读 Markdown 分析

### 4.2 当前痛点

- Desktop 直接消费 `DashboardSnapshot` JSON，与 CLI/GPT 的 Research Context 不统一。
- 未来若 Desktop 增加 Timeline/Leader 视图，需要重新解析不同来源的数据。

### 4.3 V6 期望

- Desktop 未来可消费 `ResearchContext` 和 `ReportDocument`。
- V6 不迁移 Desktop，仅预留接口。

---

## 5. 未来 API

### 5.1 假设需求

| 需求 | 对应能力 |
|---|---|
| 获取当前市场状态 | `ResearchContext.market_state` |
| 获取轮动排名 | `ResearchContext.rotation` |
| 获取信号列表 | `ResearchContext.signal` |
| 获取历史序列 | Timeline（V6 后续阶段） |

### 5.2 V6 期望

- `ResearchContext` 设计为 Stable Contract，支持 JSON 序列化。
- `ReportDocument` 可作为 API 返回的 Presentation Layer。

---

## 6. 未来 PDF

### 6.1 假设需求

- Executive Summary（V6 后续阶段，Explainability Layer）
- Detail Report
- Timeline 图表（V6 后续阶段）

### 6.2 V6 期望

- `ReportDocument` 不持有 Snapshot，是纯 Presentation Model，便于 PDF/HTML 渲染。
- V6 不实现 PDF Renderer，仅预留 `Formatter` trait 扩展点。

---

## 7. 消费者-字段需求矩阵

| 字段/能力 | CLI | GPT | Desktop | API | PDF | Email |
|---|---|---|---|---|---|---|
| Market State | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| Breadth | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| Rotation | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| Signal | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| Divergence | ✔ | ✔ | ✗ | ✔ | ✔ | ✗ |
| Trust | ✗ | ✔ | ✔ | ✔ | ✔ | ✗ |
| Timeline | ✔ | ✔ | ✗ | ✔ | ✔ | ✗ |
| Executive | ✗ | ✗ | ✔ | ✗ | ✔ | ✔ |
| Markdown | ✔ | ✗ | ✔ | ✗ | ✗ | ✔ |
| JSON | ✔ | ✔ | ✔ | ✔ | ✗ | ✗ |
| Text | ✔ | ✗ | ✗ | ✗ | ✗ | ✔ |

> 注：✗ 表示该消费者当前不需要或 V6 不覆盖。

---

## 8. 结论与 Gate A 输入

1. **主要消费者**：CLI 和 GPT 是 V6 必须覆盖的消费者。
2. **核心需求**：统一 `ResearchContext` 语义层，让 CLI/GPT/Desktop/API 共享同一套语义。
3. **Production Surface 隔离**：`DashboardSnapshot` / `export-report` 保持冻结，不属于 V6 Reporting Layer 范围。
4. **风险点**：当前 `research-context` crate 从 `DashboardSnapshot` 构建，方向与目标架构冲突，需在 Boundary Inventory 中解决 Owner 问题。
