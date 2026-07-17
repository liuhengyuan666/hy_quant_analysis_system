# ADR-082: Execution Platform Architecture

## 状态

Proposed

## 背景

当前系统的 V5 Execution Layer（`crates/execution-engine`）是一个基于规则的模式库（Pattern Library）。它接收 `SignalSnapshot`、`IndicatorSnapshot` 和腾讯实时行情，通过硬编码的 if-else 规则输出 `BuyNow / Wait / NoChase / Reduce / Skip` 等执行状态。

在 V6/V7 演进之后，系统已经拥有统一的 Research Pipeline：

```
Raw Data → Observation → Semantic Context → Consumer
```

但 V5 Execution Layer 仍然游离在这个体系之外，存在以下问题：

1. **StrategyState 被当作硬门槛**：`NoTrade` 直接返回空列表，其他状态没有差异化影响。
2. **信号粒度太粗**：只看 `SignalLabel`（StrongBuy / Buy），忽略 `final_score` 的连续信息。
3. **指标利用率低**：`IndicatorSnapshot` 中 EMA、MACD、RSI、ATR 等字段完全没有参与执行决策。
4. **缺少盘中语义层**：实时行情只被用于计算价格和成交量，没有形成 `BuyingPressure`、`FailedBreakout`、`Distribution` 等语义观察。
5. **与 Research Layer 不打通**：V6/V7 已经产出 `Confirmation`、`Stretch`、`Breadth`、`Recovery` 等研究上下文，但 Execution Layer 完全不消费它们。
6. **模式匹配是短路规则**：`NoChase → Distribution → StrongClose → Wait` 的 if-else 优先级缺乏扩展性，难以引入新的证据维度。
7. **决策输出太薄**：`ExecutionDecision` 只有 `state` 和 `reasons`，没有 `confidence`、`risk` 等 Consumer 需要的信息。
8. **缺少可复现的执行资产**：执行决策没有被记录为可追溯、可回测的事实，无法进入 V8 Research Asset 体系。

## 目标

建立一个新的 **Execution Platform**，作为 V8 Research Asset 的下游 Consumer，与 V6/V7 已经验证成功的 Research Pipeline 使用同一种架构语言：

> **Raw Data → Feature → Observation → Evidence → Assessment → Decision → ExecutionEvent → Consumer**

该平台的目标不是替代 V5，而是为未来的执行层消费者提供一个可扩展、可回测、可解释的上下文驱动执行框架。平台产出的核心不是投资建议，而是**可验证事实（ExecutionEvent）**，所有下游消费者（Replay、Research Asset、Report、Desktop、LLM）都必须基于同一个 Event 推导自己的输出。

## 最终方案

### 架构原则

> **Principle-01: Execution Platform produces verifiable facts, not investment advice. Every downstream consumer—including Replay, Reports, Desktop, and LLM—must derive its outputs from the same deterministic execution event.**
>
> 平台输出的是 `Strong Close`、`Breadth Weak`、`Confidence 0.73`、`Risk High`、`BuyNow` 等事实。Replay、Research Asset、Report、Desktop、LLM 都基于同一个 `ExecutionEvent`，不允许任何消费者自行推导新的交易结论。最终是否执行由人类或外部风控系统决定。

> **Principle-02: ResearchContext 是 Canonical Model，Execution 是 Consumer。**
>
> Execution 不拥有 `ResearchContext` 的字段，也不复制它们。Execution 通过 `ExecutionMarketView` 投影消费 `ResearchContext` 中需要的部分（如 `Confirmation`、`Breadth`、`Recovery`、`Rotation`）。

> **Principle-03: StrategyState 是证据，不是门槛。**
>
> `NoTrade`、`LeftProbe`、`ConfirmAdd`、`FullTrend`、`DeRisk` 不再作为硬门槛。它们作为带方向性的 Evidence 贡献给 Assessment，影响最终决策的置信度和风险判断。

> **Principle-04: Evidence 是 Research、Execution、Review 的统一语言。**
>
> 所有模块（包括盘中观察、研究上下文、策略状态）在进入执行评估前都转换为 `Evidence`。Consumer 看到的是统一的 Evidence 列表，而不是 Observation、Reason、Insight 等异构概念。

> **Principle-05: LLM 只解释，不决策。**
>
> LLM 在 Execution Platform 中的职责限定为 `Explain / Summarize / Compare / Highlight / Recommend Reading`。它不能生成信号、决定策略、评估风险或确定执行状态。完整边界见 ADR-084。

> **Principle-06: ExecutionEvent is the canonical output of the Execution Platform.**
>
> `ExecutionEvent` is the canonical, deterministic fact produced by the Execution Platform. It is the only contract that downstream consumers are allowed to depend on. `ExecutionDecision` is merely the final decision result inside the event; the event itself is the complete, reproducible, and verifiable fact record. Replay, Research Asset, Report Engine, Desktop, and LLM all consume this event, not the engine internals.

### 架构分层

```
Research Layer (V6/V7)
        │
        ▼
   ResearchContext
        │
        ▼
ExecutionMarketView  (Projection)
        │
        ▼
Input Layer
        │
        ├── SignalSnapshot
        ├── StrategyStateSnapshot
        ├── QuoteSnapshot
        └── ExecutionPolicy
        │
        ▼
   ExecutionRequest
        │
        ▼
Execution Engine Pipeline
        │
        ├── FeatureExtractor     → IntradayFeatures
        ├── ObservationEngine    → IntradayObservation
        ├── EvidenceBuilder      → Evidence[]
        ├── AssessmentEngine     → ExecutionAssessment
        └── DecisionEngine       → ExecutionDecision
        │
        ▼
   ExecutionEvent
        │
        ├────────────┬────────────┬────────────┐
        ▼            ▼              ▼            ▼
      Replay     Research Asset  report-engine   API
                    │                │
                    ▼                ▼
               Calibration    ExecutionExplanation
                                          │
                                          ▼
                                         LLM
```

### Crate 职责

| Crate | 职责 | 依赖限制 |
|---|---|---|
| `crates/execution-engine` | Execution Pipeline 的 Domain 实现：`FeatureExtractor`、`ObservationEngine`、`EvidenceBuilder`、`AssessmentEngine`、`DecisionEngine`、`ExecutionEvent` | 依赖 `core-domain`、`research-context`；不依赖 `report-engine`、`report-builder`、`report-renderer`、`market-store` |
| `crates/execution-replay` | Replay 的 Contract 与实现：消费 `ExecutionEvent`，解析未来行情，产出 `ExecutionReplayRecord`，写入 Research Asset | 依赖 `execution-engine`、`market-store`；不实现 Execution 内部算法 |
| `crates/research-context` | Canonical Semantic Model：`ResearchContext` 及 Summary | 保持冻结，不依赖 Execution |
| `crates/report-engine` | Explanation 层：从 `ExecutionEvent` 构建 `ExecutionExplanation`，供 Formatter 和 LLM 消费 | 依赖 `execution-engine` 的 Public DTO；不依赖 Engine 内部 Pipeline |
| `crates/reporting` / `report-renderer` | Formatter 输出 Markdown / JSON / Text | 只消费 `ExecutionExplanation`，不消费 Engine 内部 |
| `crates/app-service` | 编排：从 Engine/Store 构建 `ExecutionRequest` 和 `ExecutionMarketView`，调用 Execution Engine、Replay 和 Report Engine | 可依赖 `execution-engine`、`execution-replay`、`research-context`、`report-engine` |
| V8 Workspace (`crates/app-service/src/workspace.rs`) | 持久化 `ExecutionEvent` 与 `ExecutionReplayRecord` 为 Research Asset | 不依赖 Engine 内部 Pipeline |

### Architecture Rules

> **Rule-01: ExecutionMarketView is a Projection, not a Copy.**
>
> `ExecutionMarketView` 从 `ResearchContext` 投影出执行层需要的子集（`Confirmation`、`Breadth`、`Recovery`、`RotationState`）。它不复制字段，也不是 `ResearchContext` 的别名。`ResearchContext` 新增字段时，`ExecutionMarketView` 无需重新编译。

> **Rule-02: ExecutionRequest is an Input Contract, not a Pipeline State.**
>
> `ExecutionRequest` 只包含外部输入：`SignalSnapshot`、`StrategyStateSnapshot`、`QuoteSnapshot`、`ExecutionMarketView`、`ExecutionPolicy`。`IntradayFeatures`、`IntradayObservation`、`Evidence`、`ExecutionAssessment` 都是 Engine 内部产物，不应出现在 `ExecutionRequest` 中。

> **Rule-03: Evidence is the Only Cross-Layer Semantic Unit.**
>
> 所有进入执行评估的语义信息都必须转换为 `Evidence`。`Observation` 和 `Reason` 不能直接进入 `AssessmentEngine`。

> **Rule-04: Evidence Payload is Typed.**
>
> `Evidence` 携带结构化 Typed Payload，不是 `serde_json::Value`。Formatter 通过模式匹配 `EvidenceKind` + `EvidencePayload` 生成文本，而不是解析 JSON 字段。

> **Rule-05: AssessmentEngine produces ExecutionAssessment, not a Decision.**
>
> `AssessmentEngine` 输出 `ExecutionAssessment`（包含 `confidence`、`risk`、`dominant_direction`、`supporting/conflicting_evidence`）。`DecisionEngine` 再根据 `ExecutionAssessment` + `ExecutionPolicy` 输出 `ExecutionDecision`。

> **Rule-06: ExecutionDecision does not expose internal score.**
>
> `ExecutionDecision` 包含 `state`、`confidence`、`risk`、`evidences`、`assessment`，不包含 `score`。`score` 是 Aggregator 的内部实现细节，不应泄漏到 Consumer 契约中。

> **Rule-07: ExecutionExplanation belongs to Presentation, not Domain.**
>
> `ExecutionExplanation` 由 `report-engine` 从 `ExecutionEvent` 构建，属于 Presentation Layer。`execution-engine` 不感知 LLM、Desktop、PDF 等 Consumer 的存在。

> **Rule-08: Policy is the only place for thresholds.**
>
> `ExecutionPolicy` 是系统中唯一可以配置阈值、开关、风险预算的地方。`ExecutionEngine` 内部不能出现硬编码的 `0.7`、`1.3`、`2.0` 等 Magic Number。

> **Rule-09: ExecutionEvent is the canonical fact for Replay and Research Asset.**
>
> `Replay` 和 `Research Asset` 不直接读取 Engine 内部状态。它们消费 `ExecutionEvent`，后者包含完整的 Pipeline 输入、中间产物和最终决策。客观收益（Outcome）和评价（Evaluation）附加在 Event 上，作为可回测事实。

> **Rule-10: ExecutionEvent is versioned.**
>
> `ExecutionEvent` 必须携带 schema version、engine version、policy version 和 research version，以便 Historical Replay 知道是哪套规则产生了该事件，从而正确复现或解释。

> **Rule-11: ExecutionPolicy is serializable and reproducible.**
>
> `ExecutionPolicy` 必须可序列化（JSON），并且携带稳定的 hash。`ExecutionEvent` 同时保存 policy 和 policy hash，确保 Replay 时可以完全复现当时的决策条件，即使后续默认 policy 发生变化。

> **Rule-12: Replay lives outside `execution-engine`.**
>
> `execution-engine` 只负责生成 `ExecutionEvent`。未来行情解析、MFE/MAE 计算、收益归因等 Replay 逻辑属于 `crates/execution-replay`（或等价下游模块），依赖 `market-store` 而非 Engine 内部。

> **Rule-13: LLM is outside the Execution Pipeline.**
>
> LLM 不能调用 `ExecutionEngine` 或修改 `ExecutionPolicy`。它只能消费 `ExecutionExplanation` 并生成自然语言。详见 ADR-084。

> **Rule-14: LLM only consumes ExecutionEvent, never intermediate layers.**
>
> LLM 不得访问 `QuoteSnapshot`、`IntradayFeatures`、`IntradayObservation`、`Evidence` 或 `ExecutionAssessment` 来重新推导执行决策。`ExecutionEvent` 是唯一允许进入 LLM 执行解释的事实载体。如果 LLM 需要证据细节，必须从 `ExecutionEvent.evidences` 中读取，而不是从 Engine 内部重新生成。

> **Rule-15: ExecutionEvent preserves market regime from ResearchContext without interpretation.**
>
> `ExecutionMarketView.market_regime_label` is sourced from `ResearchContext.market_state.label` and never generated or altered by the Execution Platform. If the source label is missing, the Execution Platform falls back to `"Unknown"` rather than substituting a default business interpretation.

### 稳定性约束

- `ExecutionRequest`、`ExecutionMarketView`、`ExecutionPolicy`、`Evidence`、`ExecutionAssessment`、`ExecutionDecision`、`ExecutionEvent`、`ExecutionEventVersions` 属于 Stable Contract，演进时优先采用 additive changes。
- `ExecutionEventVersions` 的字段语义本身也受稳定性约束：schema version 变更表示 Event 结构演进；engine version 变更表示 Pipeline 算法演进；policy version 变更表示决策条件演进；research version 变更表示输入投影演进。
- `ExecutionEngine` 内部 Pipeline 可以自由替换算法（如从线性聚合改为贝叶斯或 LLM-assisted），只要 DTO 契约不变。
- `ResearchContext` 保持冻结，不因 Execution 需求而新增字段。
- V5 Execution Layer 保持冻结，新平台不修改其代码路径。
- `execution-replay` 作为独立 crate，允许与 `market-store` 耦合，但不得反向依赖 `execution-engine` 的私有实现。

## 未采纳方案（Rejected Alternatives）

### 1. 直接修改 V5 Execution Layer 的 if-else 规则

**原因未采纳**：V5 已经进入 Shadow Production 观察期（ADR-065），修改其规则会破坏既有观察基线。正确做法是在 V8 之上新增平台，V5 保持冻结作为对照。

### 2. 让 Execution 直接消费整个 ResearchContext

**原因未采纳**：`ResearchContext` 会持续演进，直接依赖会导致 Execution 因 ResearchContext 新增字段而频繁重新编译。通过 `ExecutionMarketView` 投影可以隔离变化。

### 3. 在 ExecutionContext 中复制 ResearchContext 字段

**原因未采纳**：这是 God Object 反模式，违反 Single Source of Truth。ResearchContext 是 Canonical Model，Execution 不应拥有其字段的副本。

### 4. 让 Pattern 直接返回 ExecutionDecision

**原因未采纳**：直接返回 Decision 会导致 Pattern 与 Decision 耦合，无法引入新的证据维度。通过 Pattern → Evidence → Assessment → Decision 的分层，可以独立扩展每个环节。

### 5. 在 ExecutionDecision 中暴露 `score` 字段

**原因未采纳**：`score` 是 Aggregator 的内部实现细节。如果未来 Aggregator 从线性评分改为贝叶斯或投票制，`score` 将失去意义。Consumer 应关心 `confidence` 和 `risk`。

### 6. 把 ExecutionExplanation 放在 execution-engine 中

**原因未采纳**：Explanation 属于 Presentation，不是 Domain。放在 `execution-engine` 会导致 CLI、Desktop、PDF、LLM 全部依赖 `execution-engine`，破坏 crate 边界。

### 7. 让 LLM 参与 Execution Decision

**原因未采纳**：这会引入不可解释、不可回测的决策来源，违反 V6/V7 的 Evidence-First 哲学。LLM 只负责解释，不负责判断。

## V8 边界

**做**：

- 在 `crates/execution-engine` 中新增 Execution Platform 的 Domain 实现（Pipeline 各层 DTO 和 Engine）。
- 新增 `ExecutionRequest`、`ExecutionMarketView`、`ExecutionPolicy`、`Evidence`、`ExecutionAssessment`、`ExecutionDecision`、`ExecutionEvent` 等 DTO。
- 实现 `ExecutionMarketView::from_research_context()` 投影。
- 实现第一批 `IntradayFeature` 和 `IntradayObservation`（如 `CloseStrength`、`FailedBreakout`、`Distribution`）。
- 实现第一批 `EvidenceKind`（如 `TrendParticipation`、`MomentumExpansion`、`DistributionRisk`）。
- 在 `crates/report-engine` 中实现 `ExecutionExplanation` 的构建。
- 在 V8 Workspace 中增加 `ExecutionEvent` 的写入路径，作为 Research Asset 的事实来源。
- 提供新的 CLI 命令（如 `preclose-analysis-v2`）作为 V8 平台的演示入口，不替换 `preclose-analysis`。

**不做**：

- 不修改 V5 Execution Layer 的代码、接口或行为。
- 不修改 V6/V7 Research Platform 的语义或接口。
- 不修改 `ResearchContext` 以迎合 Execution 需求。
- 不在 `execution-engine` 中直接调用 LLM 或构建 LLM Prompt。
- 不在 `execution-engine` 中实现 Historical Replay 或收益计算。
- 不在 `ExecutionDecision` 中输出投资建议或预期收益。
- 不在实现初期引入复杂权重学习；MVP 使用等权重或 Policy 配置权重，长期权重学习依赖 V8 Research Asset 积累。

## 验证

- `cargo check`：全 workspace 通过。
- `cargo test -p execution-engine`：新增 Pipeline 单元测试通过。
- V5 CLI 命令 `preclose-analysis` 输出保持不变。
- 新命令 `preclose-analysis-v2`（或类似）可以产出 `ExecutionEvent` 的演示输出。
- Historical Replay 可以运行新 Execution Pipeline，并将 `ExecutionEvent` 写入 Research Asset。

## 演进路径

- **Phase 1（Architecture Freeze）**：完成 ADR-082、ADR-083、ADR-084，冻结架构原则和 DTO 契约。
- **Phase 2（DTO Freeze）**：在 Rust 中定义所有 DTO，包括 `ExecutionEvent` 和 `ExecutionEventVersions`。
- **Phase 3（Pipeline 实现）**：依次实现 Feature → Observation → Evidence → Assessment → Decision，并在末端生成 `ExecutionEvent`。
- **Phase 4（Architecture Gate）**：确认 `ExecutionEvent` 为 Canonical Output，确认 Replay 独立模块，确认 Policy 版本化，确认版本字段语义。
- **Phase 5（Replay Contract）**：在 `crates/execution-replay` 中定义 `ExecutionReplayRecord`、`ExecutionOutcome`、`ExecutionEvaluation`，以及 `ReplayOutcomeResolver` / `ReplayEvaluator` / `ReplayStore` trait。
- **Phase 6（Research Asset）**：在 V8 Workspace 中增加 `ExecutionEvent` 与 `ExecutionReplayRecord` 的写入路径，作为 Research Asset 的事实来源。
- **Phase 7（Explanation）**：在 `report-engine` 中实现 `ExecutionExplanation` 和 Formatter，供 Desktop / CLI / PDF / LLM 消费。
- **Phase 8（Policy Calibration）**：基于积累的 Research Asset，校准 `ExecutionPolicy` 的权重和阈值。

## 相关文档

- `docs/v6/adr-068-research-context-reporting-layer.md`
- `docs/v6/adr-077-research-platform-freeze.md`
- `docs/v8/adr-083-execution-evidence.md`
- `docs/v8/adr-084-llm-boundary.md`
- `docs/v5/adr-065-shadow-production-v1.md`
- `docs/architecture-invariants.md`
- `docs/shadow-production-playbook.md`
