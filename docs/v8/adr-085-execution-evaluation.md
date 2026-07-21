# ADR-085: Execution Evaluation

## 状态

Proposed

## 背景

V8 Execution Platform 已经建立了从 `Quote` → `Feature` → `Observation` → `Evidence` → `Assessment` → `Decision` → `ExecutionEvent` 的完整确定性管道。该管道的产物是 `ExecutionEvent`，它是 Execution Platform 的 canonical output。

但 `ExecutionEvent` 本身只是“事实声明”。它声明：在某一时刻，基于特定输入、策略和策略，系统做出了某个决策。这个事实是否“有价值”，取决于它在真实市场中的后续表现，以及我们能否从中提炼出可学习的模式。

因此，Execution Platform 需要进入第二阶段：从“能运行”演进为“能学习”。Replay 不是传统意义上的回测（backtest），而是 **Execution Platform 的监督学习数据生成器**。它的目标不是计算收益，而是把 `ExecutionEvent` 转化为带标签的研究资产（Research Asset），最终支持 `ExecutionPolicy` 的校准与演进。

## 目标

明确以下职责边界：

1. **Replay 只负责事实**：给定 `ExecutionEvent` 和未来行情，计算客观的 `ExecutionOutcome`（收益、风险、持有时间等）。
2. **Evaluation 只负责判断**：基于 `ExecutionEvent` 和 `ExecutionOutcome`，判断这次决策属于哪种类型（成功 / 失败 / 过早 / 过晚 / 信号失效 / 市场体制变化等）。
3. **Research Asset 只负责沉淀**：把 `(ExecutionEvent, ExecutionOutcome, ExecutionEvaluation)` 组合为可追溯、可审计、可版本化的资产。
4. **Policy Calibration 只负责学习**：基于 Research Asset 的统计分布，调整 `ExecutionPolicy` 的权重、阈值和开关。
5. **LLM 不参与以上任何一层**：LLM 只消费这些事实，生成解释、对比、总结或假设，绝不能生成信号、决策或评估标签。

最终形成三层架构：

```text
Deterministic Layer:
  Quote → Feature → Observation → Evidence → Assessment → Decision → ExecutionEvent

Research Layer:
  ExecutionEvent → Replay → ExecutionOutcome → Evaluation → Research Asset

Cognitive Layer:
  ExecutionEvent + Outcome + Evaluation + Research Asset → LLM → Explanation / Pattern Discovery
```

## 最终方案

### 数据模型

#### ExecutionEvent（由 execution-engine 产出）

见 ADR-082。它是确定性管道的完整输出，包含请求、特征、观察、证据、评估、决策和版本信息。

#### ExecutionOutcome（由 ReplayOutcomeResolver 产出）

客观事实，只包含可计算字段，不包含主观判断。

```rust
pub struct ExecutionOutcome {
    pub t20_return: Option<f64>,
    pub t60_return: Option<f64>,
    pub t120_return: Option<f64>,
    pub mfe: Option<f64>,
    pub mae: Option<f64>,
    pub holding_days: Option<u32>,
    pub benchmark_return: Option<f64>,
    pub alpha: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub stop_loss_hit: Option<bool>,
    pub take_profit_hit: Option<bool>,
}
```

#### ExecutionEvaluation（由 EvaluationEngine 产出）

主观判断，基于 `ExecutionEvent` 和 `ExecutionOutcome`，回答“这次决策为什么成功 / 失败 / 属于哪一类”。它是 Research Asset 的学习标签。

```rust
pub enum ExecutionEvaluation {
    AwaitingOutcome,

    // Successful outcomes
    Hit,
    TimingAcceptable,
    RiskWellManaged,

    // Timing failures
    TooEarly,
    TooLate,

    // Direction failures
    TrendLost,
    FalseBreakout,
    ReversalMissed,

    // Policy failures
    PolicyTooAggressive,
    PolicyTooConservative,
    PolicyIgnoredRisk,

    // Signal failures
    SignalFalsePositive,
    SignalFalseNegative,
    SignalDecay,

    // Market regime failures
    MarketRegimeChanged,
    LiquidityCollapse,
    BreadthDeterioration,

    // Execution / market microstructure failures
    GapSlippage,
    VolumeInsufficient,

    // Catch-all
    EvaluationFailure,
}
```

分类体系设计意图：

- **Timing failures**：决策方向对，但入场时机太早或太晚。
- **Direction failures**：市场走势与决策方向不一致，包含假突破、趋势反转等。
- **Policy failures**：决策本身与 Policy 配置有关，例如过于激进或过于保守。
- **Signal failures**：Signal 模型给出了错误的方向或强度。
- **Market regime failures**：市场体制发生变化，例如流动性、广度、宏观状态突变。
- **Execution failures**：微观结构或执行条件导致无法按预期成交。

#### ExecutionResearchRecord（写入 Research Asset）

Replay 和 Evaluation 的最终产物，进入 V8 Workspace。

```rust
pub struct ExecutionResearchRecord {
    pub event: ExecutionEvent,
    pub outcome: ExecutionOutcome,
    pub evaluation: ExecutionEvaluation,
    pub evaluation_version: String,
    pub evaluated_at: DateTime<Utc>,
}
```

`outcome` 一旦从历史行情计算完成就不可变；`evaluation` 是可重新运行的：同一个 `(event, outcome)` 可以由新版本的 `EvaluationEngine` 重新打标签。`evaluation_version` 记录了使用哪一版规则，保证统计口径可比。

### 职责边界

| 组件 | 职责 | 禁止事项 |
|---|---|---|
| `ReplayOutcomeResolver` | 读取历史行情，计算 `ExecutionOutcome` | 不能判断成功/失败；不能修改 `ExecutionEvent` |
| `EvaluationEngine` | 基于 Event + Outcome，输出 `ExecutionEvaluation` | 不能读取未来行情；不能访问 `market-store` |
| `ReplayStore` | 保存/加载 `ExecutionResearchRecord` | 不能计算 Outcome 或 Evaluation |
| V8 Workspace | 持久化 `ExecutionResearchRecord` 为 Research Asset | 不能解释事件或评估标签 |
| LLM | 消费 `ExecutionResearchRecord` 生成解释 | 不能生成 Outcome、Evaluation 或修改 Policy |

### 架构规则

> **Rule-01: Outcome is a fact, not a verdict.**
>
> `ExecutionOutcome` 只包含客观可计算的字段（return、MFE、MAE、drawdown）。任何“成功 / 失败”的判断必须交给 `EvaluationEngine`。

> **Rule-02: Evaluation is a Research Label, not a trading signal.**
>
> `ExecutionEvaluation` 的目的是支持后续 Policy Calibration 和 Pattern Discovery，不是用于实时交易。它必须基于已经发生的 `ExecutionEvent` 和 `ExecutionOutcome`。

> **Rule-03: Evaluation Taxonomy is closed at runtime.**
>
> `ExecutionEvaluation` 是一个固定枚举。新增标签必须经过 ADR 审议，不能由运行时动态产生。这是为了保证 Research Asset 的统计可比性。

> **Rule-04: Outcome is immutable; Evaluation is re-runnable.**
>
> `ExecutionOutcome` 从历史行情计算一次后必须长期保持不变。`ExecutionEvaluation` 可以随着规则演进被重新计算。`ExecutionResearchRecord` 必须保存 `evaluation_version`，使消费者知道标签来自哪一版规则，并支持按版本重新聚合。

> **Rule-05: Evaluation may be multi-label in the future, but single-label in MVP.**
>
> MVP 阶段每个 `ExecutionResearchRecord` 只携带一个 `ExecutionEvaluation`。未来可以通过扩展为 bitset 或多标签向量来支持更细粒度的归因。

> **Rule-06: LLM does not evaluate.**
>
> LLM 可以解释为什么某个 `ExecutionEvaluation` 被标记为 `FalseBreakout`，但它不能决定这个标签。标签由确定性的 `EvaluationEngine` 根据规则产生。

> **Rule-07: Evaluation rules are part of the platform contract.**
>
> 判定规则（如“t20 return < -3% 且 mfe > 2% 判定为 FalseBreakout”）必须显式定义、可审计、可版本化。它们可能存在于 `ExecutionPolicy` 的 evaluation 部分，或作为独立的 `EvaluationConfig`。

> **Rule-08: Evaluation first, calibration second.**
>
> 在积累足够多 `ExecutionResearchRecord` 之前，不能修改 `ExecutionPolicy` 的权重或阈值。推荐门限：至少 1000 条记录、覆盖至少 30 个交易日、至少 2 个不同市场体制。

## 未采纳方案

### 1. 把 Replay 和 Evaluation 合并在 execution-engine 中

**原因未采纳**：执行与验证属于不同职责。`execution-engine` 是确定性管道，不应依赖 `market-store` 或历史行情。Evaluation 也需要稳定的输入契约，不应与 Engine 内部耦合。

### 2. 让 LLM 根据收益判断成功 / 失败

**原因未采纳**：这会让 LLM 进入 Research Layer，破坏确定性边界。LLM 只能解释已经由 EvaluationEngine 打好的标签。

### 3. 动态 / 学习式 Evaluation 标签

**原因未采纳**：Runtime 动态标签会导致 Research Asset 的统计口径不可比。标签体系必须在编译期 / 配置期固定，变更通过 ADR 和新版本号管理。

### 4. 把 Evaluation 作为 ExecutionEvent 的字段

**原因未采纳**：`ExecutionEvent` 在决策时就已经完成，而 Evaluation 需要未来的 Outcome。两者时间维度不同，不能混在同一个事件对象里。它们通过 `ExecutionResearchRecord` 组合。

## 边界

**做**：

- 在 `crates/execution-replay` 中定义 `ExecutionOutcome`、`ExecutionEvaluation`、`ExecutionResearchRecord` 和对应 trait。
- 实现一个基于 `market-store` 的 `ReplayOutcomeResolver`，计算 T+20 / T+60 / T+120 收益、MFE、MAE、alpha 等。
- 实现一个基于规则的 `EvaluationEngine`，把 Outcome 映射为 Evaluation 标签。
- 在 V8 Workspace 中定义 `ExecutionResearchRecord` 的写入路径和索引。
- 提供 CLI 命令（如 `research replay-v2`）批量运行 Replay + Evaluation，并把结果写入 Research Asset。

**不做**：

- 不修改 `ExecutionEvent` 的结构以容纳 Outcome 或 Evaluation。
- 不在 `execution-engine` 中实现 Replay 或 Evaluation。
- 不让 LLM 生成 Evaluation 标签。
- 不在数据不足时进行 Policy Calibration。
- 不把 Evaluation 作为实时交易输入。

## 验证

- `cargo check -p execution-replay` 通过。
- Evaluation 分类体系可以覆盖常见的成功与失败场景。
- 用真实历史数据跑一轮 Replay，确认 `ExecutionEvent` 包含足够信息支持 Evaluation。
- 至少积累 100 条 `ExecutionResearchRecord` 后，检查标签分布是否合理。

## 演进路径

- **Phase 1（Contract）**：定义 `ExecutionOutcome`、`ExecutionEvaluation`、`ExecutionResearchRecord` 和 trait。
- **Phase 2（Taxonomy）**：确定 MVP 所需的 Evaluation 标签，写入 ADR-085。
- **Phase 3（Outcome Resolver）**：实现基于 `market-store` 的 `ReplayOutcomeResolver`。
- **Phase 4（Evaluation Engine）**：实现基于规则的 `EvaluationEngine`。
- **Phase 5（Asset Writer）**：在 V8 Workspace 写入 `ExecutionResearchRecord`。
- **Phase 6（First Replay Run）**：用真实历史数据跑一轮，验证标签分布。
- **Phase 7（Pattern Discovery）**：消费 Research Asset 生成统计摘要，但不修改 Policy。
- **Phase 8（Policy Calibration）**：在满足门限后，基于 Research Asset 调整 `ExecutionPolicy`。

## 相关文档

- `docs/v8/adr-082-execution-platform.md`
- `docs/v8/adr-083-execution-evidence.md`
- `docs/v8/adr-084-llm-boundary.md`
- `docs/v6/adr-079-research-snapshot.md`
- `docs/v6/adr-080-research-asset-lifecycle.md`
- `docs/v6/adr-081-research-asset-identity.md`
