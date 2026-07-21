# ADR-084: LLM Boundary in Execution Platform

## 状态

Proposed

## 背景

随着 LLM 能力被集成到桌面端（`LlmAnalysisTrigger` / `LlmAnalysisPanel`）和研究报告生成（`analyze-with-llm`），存在一个架构风险：LLM 可能逐渐被当作决策来源，而不是解释工具。在 Execution Platform 中，这个边界尤其重要，因为执行决策直接影响下游交易行为。

当前系统（V4.5）已经删除了 Agent Profile、Skill Registry、技能路由等 LLM 决策能力，只保留 5 个纯解释性 action。但如果没有明确的 ADR，未来仍可能重新引入 LLM 决策的诱惑。

## 目标

明确 LLM 在 Execution Platform 中的职责边界：LLM 只解释由确定性系统产生的可验证事实，不参与任何决策、信号生成或风险评估。

## 最终方案

### LLM 在 Execution Platform 中的职责

LLM 可以执行以下五类操作：

| 职责 | 说明 | 示例 |
|---|---|---|
| **Explain** | 解释一个 ExecutionDecision 的成因 | "今日出现 FailedBreakout，叠加 Breadth Weak，因此系统给出 Wait。" |
| **Summarize** | 汇总多个 Evidence 的关键信息 | "今日有三个证据支持谨慎：广度收缩、动量失败、流动性确认偏弱。" |
| **Compare** | 对比当前决策与历史相似案例 | "当前 Evidence 组合与 2025-08-15 的 SRD 样本类似。" |
| **Highlight** | 强调最关键的风险或机会点 | "最需要关注的风险：尾盘收于当日区间下沿，放量下跌。" |
| **Recommend Reading** | 推荐阅读相关研究文档 | "建议阅读 `research-srd --scope global` 和 `research-stretch --scope cn`。" |

### LLM 在 Execution Platform 中禁止的行为

LLM **不得**执行以下任何操作：

| 禁止行为 | 原因 |
|---|---|
| **Signal Generation** | 信号必须由 `signal-engine` 基于指标、轮动、策略偏好等确定性规则产生。 |
| **Strategy Decision** | 策略状态（`NoTrade` / `LeftProbe` / `ConfirmAdd` / `FullTrend` / `DeRisk`）必须由 `macro-engine` 基于 regime 和 environment 产生。 |
| **Risk Evaluation** | 风险等级（`RiskLevel`）必须由 `AssessmentEngine` 基于 Evidence 聚合产生。 |
| **Execution State** | 执行状态（`BuyNow` / `Wait` / `Reduce` / `NoChase` / `Skip`）必须由 `DecisionEngine` 基于 `ExecutionAssessment` + `ExecutionPolicy` 产生。 |
| **Policy Modification** | `ExecutionPolicy` 的阈值、权重、开关只能由人类或基于 Research Asset 的校准流程调整，不能由 LLM 实时修改。 |
| **Override Decision** | 任何情况下，LLM 不能覆盖或修改 `ExecutionDecision` 的 `state` 字段。 |

### 数据流

```
Execution Engine
        │
        ▼
ExecutionDecision
        │
        ▼
report-engine
        │
        ▼
ExecutionExplanation
        │
        ├──────────┬──────────┬──────────┬──────────┐
        ▼          ▼          ▼          ▼          ▼
       CLI        Desktop     PDF        LLM        API
```

LLM 消费的输入是 `ExecutionExplanation`，不是 `ExecutionDecision` 的原始字段，也不是 Engine 内部状态。

`ExecutionExplanation` 的结构：

```rust
pub struct ExecutionExplanation {
    pub summary: String,
    pub key_points: Vec<String>,
    pub risk_points: Vec<String>,
    pub supporting_evidence: Vec<Evidence>,
    pub conflicting_evidence: Vec<Evidence>,
    pub market_view: ExecutionMarketView,
    pub intraday_context: IntradayContext,
}
```

LLM 从这个结构生成自然语言，例如：

> 今日盘中出现 FailedBreakout，上午冲高后下午回落，收于当日区间下沿。研究层面显示 Confirmation 偏弱、Breadth 收缩，因此系统给出 Wait。建议关注明日开盘是否能在关键均线处获得支撑。

### 全项目引用

本 ADR 不仅适用于 Execution Platform，也适用于 Research Layer 和 Reporting Layer：

- **Research Layer**：LLM 可以解释 `ResearchContext` 中的 `Confirmation`、`Stretch`、`Recovery` 等结论，但不能生成这些结论。
- **Reporting Layer**：LLM 可以润色 `ReportDocument`，但不能决定报告中出现哪些 Section 或结论。
- **Execution Layer**：LLM 可以解释 `ExecutionDecision`，但不能改变或替代它。

## 未采纳方案（Rejected Alternatives）

### 1. 让 LLM 根据实时行情给出 ExecutionDecision

**原因未采纳**：实时行情是原始数据，LLM 对其推理过程不可控、不可回测。如果 LLM 可以直接决策，系统将失去可验证性。

### 2. 让 LLM 根据多个证据投票决定最终状态

**原因未采纳**：这本质上是让 LLM 作为 `AssessmentEngine` 或 `DecisionEngine`。LLM 的权重和偏见无法审计，且不同模型可能给出不同结果。

### 3. 让 LLM 生成 ExecutionPolicy

**原因未采纳**：Policy 是系统级配置，涉及风险预算、阈值、开关等。如果由 LLM 生成，将难以解释、审计和复现。Policy 只能由人类或基于 Research Asset 的校准流程调整。

### 4. 让 LLM 读取 Engine 内部状态（如 IntradaySnapshot）

**原因未采纳**：这会让 LLM 绕过 `ExecutionExplanation` 层，直接消费原始数据。原始数据应通过 Feature → Observation → Evidence 的 Pipeline 处理后才能进入 LLM Prompt。

### 5. 让 LLM 作为消费者解释工具，但不限制它不能做什么

**原因未采纳**：仅有正面职责描述不够。必须明确禁止行为，否则未来功能扩展时容易越界。

## V8 边界

**做**：

- 在 `report-engine` 中实现 `ExecutionExplanation` 构建器。
- 在 LLM Prompt 构建器中只使用 `ExecutionExplanation` 作为输入。
- 在 LLM action 列表中只保留解释性 action（如 `preclose_review`）。
- 在日志中记录 LLM 输入和输出，确保可审计。

**不做**：

- 不新增任何让 LLM 改变 `ExecutionDecision` 的 action。
- 不新增任何让 LLM 生成 Policy 或阈值的 action。
- 不将 LLM 输出写回 `ExecutionDecision` 或 `ExecutionAssessment`。
- 不在 LLM Prompt 中直接包含原始 `IntradaySnapshot` 或 `ResearchContext` 的全部字段。

## 验证

- 检查 `research-skills/src/action.rs` 和所有 LLM Prompt：没有生成信号、状态、Policy 的 action。
- 检查 `execution-engine` 代码：不引用 LLM 或 Prompt 构建逻辑。
- 检查 `report-engine`：`ExecutionExplanation` 从 `ExecutionDecision` 构建，不反向修改 `ExecutionDecision`。
- 桌面端 LLM 面板只展示解释，不展示可修改的决策控件。

## 演进路径

- **Phase 1（ADR Freeze）**：ADR-084 冻结，与 ADR-082、ADR-083 一起形成 V8 Execution Platform 的边界约定。
- **Phase 2（DTO Freeze）**：定义 `ExecutionExplanation` DTO。
- **Phase 3（Explanation Builder）**：在 `report-engine` 中实现 `ExecutionExplanation` 构建器。
- **Phase 4（LLM Prompt 更新）**：将所有 Execution 相关的 LLM Prompt 改为消费 `ExecutionExplanation`。
- **Phase 5（Audit）**：建立 LLM 输出审计日志，确保无越界行为。

## 相关文档

- `docs/v8/adr-082-execution-platform.md`
- `docs/v8/adr-083-execution-evidence.md`
- `docs/v6/adr-048-llm-desktop-integration.md`
- `docs/v6/adr-049-llm-desktop-integration-phase2.md`
- `docs/v6/adr-068-research-context-reporting-layer.md`
- `docs/shadow-production-playbook.md`
