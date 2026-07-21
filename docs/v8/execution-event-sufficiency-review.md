# ExecutionEvent Sufficiency Review

## 状态

Completed

## 目的

在添加 CLI、Dashboard、PDF、LLM 等消费者之前，确认 `ExecutionEvent` 是否已经是足够完整、可复现、可解释的事实载体。如果未来消费者还需要额外字段，现在补最便宜；一旦开始大规模 Replay 积累和 CLI 暴露，DTO 变更将引发生态漂移。

## 审查方法

1. 列出 Execution Platform 的所有当前和未来消费者。
2. 对每个消费者，说明它需要从 `ExecutionEvent` 中获取什么信息。
3. 判断 `ExecutionEvent` 当前是否已包含该信息。
4. 如果缺失，评估是补到 `ExecutionEvent` 中，还是由该消费者自己从外部获取。

核心判断标准：

- 如果信息是 Execution Platform 生成决策所依赖的，它应该在 `ExecutionEvent` 中。
- 如果信息是消费者自己的展示/解释需求（例如排序、格式化、聚合），它可以在 Consumer 层从 `ExecutionEvent` 派生，不一定需要写回 DTO。
- 如果信息来自外部系统（例如用户账户、持仓、风控），它不应进入 `ExecutionEvent`。

## 消费者清单

### 1. Replay / Research Asset

| 需要的信息 | `ExecutionEvent` 中是否存在 | 字段路径 | 是否足够 |
|---|---|---|---|
| 决策时的 symbol | ✅ | `request.symbol` | ✅ |
| 决策日期 | ✅ | `request.date` | ✅ |
| 决策状态 | ✅ | `decision.state` | ✅ |
| 决策置信度 | ✅ | `decision.confidence` | ✅ |
| 决策风险等级 | ✅ | `decision.risk` | ✅ |
| 决策原因 | ✅ | `decision.decision_reasons` | ✅ |
| 支撑/冲突证据 | ✅ | `decision.evidences` / `assessment.supporting_evidence` | ✅ |
| 当时的行情价格 | ✅ | `request.quote` | ✅ |
| 当时的策略状态 | ✅ | `request.strategy_state` | ✅ |
| 当时的信号 | ✅ | `request.signal` | ✅ |
| 当时的政策参数 | ✅ | `policy` + `policy_hash` | ✅ |
| 当时的研究上下文投影 | ✅ | `request.market_view` | ✅ |
| 未来收益 / MFE / MAE | ❌ | 由 `ReplayOutcomeResolver` 计算后附加 | N/A（Outcome 不是 Event 的一部分） |
| 市场体制 ID | ⚠️ | `market_view` 中有 `research_version`，但没有 regime 标签 | 待评估 |

**结论**：基础信息足够。缺失 `market regime id` 可能让 Replay 无法做 regime-aware attribution。建议：将 `market_regime_label` 或 `regime_hash` 加入 `ExecutionMarketView`。

### 2. Dashboard / Desktop

| 需要的信息 | `ExecutionEvent` 中是否存在 | 字段路径 | 是否足够 |
|---|---|---|---|
| 当前决策状态 | ✅ | `decision.state` | ✅ |
| 置信度和风险 | ✅ | `decision.confidence` / `decision.risk` | ✅ |
| 决策原因摘要 | ✅ | `decision.decision_reasons` | ✅ |
| 证据列表 | ✅ | `decision.evidences` | ✅ |
| 历史趋势/对比 | ❌ | 无时间序列 | 由 Dashboard 自己聚合多个 `ExecutionEvent` |
| 置信度变化 timeline | ❌ | 单点事件 | 由 Dashboard 在 UI 层聚合 |

**结论**：单点展示足够。Timeline/历史对比由 Dashboard 在展示层维护，不需要写入 `ExecutionEvent`。

### 3. PDF / Report Engine

| 需要的信息 | `ExecutionEvent` 中是否存在 | 字段路径 | 是否足够 |
|---|---|---|---|
| 决策结果 | ✅ | `decision` | ✅ |
| 证据解释 | ✅ | `evidences` + `assessment` | ✅ |
| 生成时间 / 版本 | ✅ | `timestamp` / `versions` | ✅ |
| 格式化文本 | ❌ | 无 | 由 `report-engine` 从 `ExecutionEvent` 生成 `ExecutionExplanation` |
| 图表数据 | ❌ | 无 | 由 Report Engine 从 `ExecutionEvent` + 外部行情生成 |

**结论**：足够。Report Engine 负责生成解释和图表，不修改 DTO。

### 4. LLM / Cognitive Layer

| 需要的信息 | `ExecutionEvent` 中是否存在 | 字段路径 | 是否足够 |
|---|---|---|---|
| 完整决策上下文 | ✅ | `request` + `decision` + `assessment` | ✅ |
| 证据细节 | ✅ | `evidences` | ✅ |
| 版本信息 | ✅ | `versions` | ✅ |
| 中间推理过程 | ❌ | `features` / `observations` 存在但不应被 LLM 直接消费 | 明确禁止，见 ADR-082 Rule-14 |
| 投资建议 | ❌ | 无 | 明确禁止 |

**结论**：足够。LLM 只消费 `ExecutionEvent`，不访问中间层。`evidences` 已经结构化，足以支持解释、对比和总结。

### 5. Notification / Alert

| 需要的信息 | `ExecutionEvent` 中是否存在 | 字段路径 | 是否足够 |
|---|---|---|---|
| 触发符号 | ✅ | `request.symbol` | ✅ |
| 触发日期 | ✅ | `request.date` | ✅ |
| 决策状态 | ✅ | `decision.state` | ✅ |
| 风险等级 | ✅ | `decision.risk` | ✅ |
| 接收人 / 渠道 | ❌ | 无 | 属于 Notification 系统，不属于 Execution |

**结论**：足够。Notification 系统自己管理接收人和渠道。

### 6. Historical Comparison / Pattern Discovery

| 需要的信息 | `ExecutionEvent` 中是否存在 | 字段路径 | 是否足够 |
|---|---|---|---|
| 完整决策输入 | ✅ | `request` | ✅ |
| 决策结果 | ✅ | `decision` | ✅ |
| 市场状态投影 | ✅ | `request.market_view` | ✅ |
| 特征向量 | ✅ | `features` | ✅ |
| 观察标签 | ✅ | `observations` | ✅ |
| 历史收益标签 | ❌ | 由 `ExecutionResearchRecord.outcome` 提供 | N/A |
| 研究标签 | ❌ | 由 `ExecutionResearchRecord.evaluation` 提供 | N/A |

**结论**：足够。Pattern Discovery 可以基于 `ExecutionEvent` + `ExecutionResearchRecord` 进行。

## 当前结论

| 消费者 | 状态 |
|---|---|
| Replay / Research Asset | ✅ 足够（`market_regime_label` 已补充） |
| Dashboard / Desktop | ✅ 足够 |
| PDF / Report Engine | ✅ 足够 |
| LLM | ✅ 足够 |
| Notification | ✅ 足够 |
| Historical Comparison / Pattern Discovery | ✅ 足够 |

**冻结决议**：`ExecutionEvent` 在补充 `market_regime_label` 后，已经满足所有已知下游消费者的需求。其余消费者特定需求（timeline、formatting、charting、recipient）由各自 Consumer 层从 `ExecutionEvent` 派生，不写入 DTO。

## 已执行的变更

1. ✅ 在 `ExecutionMarketView` 中增加 `market_regime_label: String`。
2. ✅ 更新 `ExecutionMarketView::from_research_context` 从 `ResearchContext.market_state.label` 填充。
3. ✅ 将 `ExecutionEventVersions.schema_version` 从 `v2.0` 提升到 `v2.1`。

## 不需要补充的字段

- `confidence timeline`：Dashboard 自己聚合。
- `formatted explanation`：Report Engine 生成。
- `notification recipient`：Notification 系统管理。
- `expected return`：明确禁止输出投资建议。

## 决策

在 Milestone 2 结束时：

- `ExecutionEvent` 其余字段冻结。
- 进入 CLI / Dashboard / PDF / LLM Explanation 实现阶段。
- 未来任何 DTO 变更必须重新触发本 Review，并升级 schema version。

## 验证步骤

1. 对 20~50 个真实历史案例生成 `ExecutionEvent`。
2. 检查每个案例是否能回答：
   - 为什么是这个决策？（通过 `decision.decision_reasons` + `evidences`）
   - 当时的市场状态是什么？（通过 `request.market_view`）
   - 当时的信号和策略状态是什么？（通过 `request.signal` + `request.strategy_state`）
   - 这个案例属于哪个市场体制？（通过 `request.market_view.market_regime_label`，补充后）
3. 如果发现无法解释的案例，记录缺失字段并回到本 Review。

## 相关文档

- `docs/v8/adr-082-execution-platform.md`
- `docs/v8/adr-085-execution-evaluation.md`
- `docs/v6/adr-068-research-context-reporting-layer.md`
