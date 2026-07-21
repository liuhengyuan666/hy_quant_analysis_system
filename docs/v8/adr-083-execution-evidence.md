# ADR-083: Execution Evidence Model

## 状态

Proposed

## 背景

在 ADR-082 中，我们提出使用 **Evidence** 作为 Research、Execution、Review 的统一语义单元。当前系统存在以下问题：

1. **术语碎片化**：Research Layer 使用 `Observation` / `Summary`，Execution Layer 使用 `ReasonTag`，Review Layer 使用 `Insight` / `Comment`。不同 Consumer 需要用不同语言理解同一件事。
2. **Payload 无类型**：许多系统把 `metadata` 存成 `serde_json::Value`，导致 Formatter 依赖字符串 key 访问字段，类型系统失效，重构时容易遗漏。
3. **文本硬编码**：`ReasonTag` 直接携带 human-readable 字符串，导致 CLI / Desktop / PDF / LLM 无法根据语言和场景自定义表达。
4. **Observation 与 Decision 直接耦合**：当前 V5 的 `IntradaySnapshot` 直接输入模式匹配规则，没有经过语义抽象层，难以扩展和回测。

## 目标

定义一个统一、类型安全、可扩展的 `Evidence` 模型，使所有模块在进入执行评估或消费者呈现之前，都转换为同一个语义单元。

## 最终方案

### Evidence 定义

```rust
pub struct Evidence {
    pub kind: EvidenceKind,
    pub confidence: f64,        // 0.0 ~ 1.0
    pub direction: f64,         // +1.0 偏多 / -1.0 偏空 / 0.0 中性
    pub source: EvidenceSource,
    pub payload: EvidencePayload,
}
```

- `kind`：语义类型，描述这条证据代表什么市场含义。
- `confidence`：证据的可信度，0.0 到 1.0。
- `direction`：证据对市场方向的影响，+1.0 偏多，-1.0 偏空，0.0 中性。
- `source`：证据来源，用于追溯和审计。
- `payload`：类型化的结构化数据，供 Formatter 使用。

### EvidenceKind

```rust
pub enum EvidenceKind {
    // 来自盘中观察
    TrendParticipation,
    MarketAcceptance,
    MomentumExpansion,
    MomentumFailure,
    Distribution,
    RiskCompression,
    RiskExpansion,
    LeadershipRotation,
    LiquidityConfirmation,

    // 来自 Research Layer
    Confirmation,
    Recovery,
    Breadth,
    Stretch,

    // 来自策略状态
    StrategyState,

    // 来自信号模型
    SignalStrength,
}
```

`EvidenceKind` 是语义化的，不是实现细节。例如：

- `MomentumExpansion`：动量正在扩张，资金参与度高。
- `Distribution`：出现派发结构，风险上升。
- `MarketAcceptance`：价格行为被成交量和市场广度确认。

### EvidenceSource

```rust
pub enum EvidenceSource {
    ResearchContext,
    IntradayObservation,
    StrategyState,
    SignalModel,
}
```

`source` 用于追溯证据来自哪一层，这对 Replay 和 Audit 非常重要。

### EvidencePayload

```rust
pub enum EvidencePayload {
    Gap { gap_pct: f64 },
    Volume { volume_ratio: f64 },
    Breadth { breadth_pct: f64, delta_5d: f64 },
    Close { close_position: f64 },
    Distribution { distribution_score: f64 },
    Confirmation {
        trend_score: f64,
        participation_score: f64,
        risk_score: f64,
    },
    Rotation {
        rotation_state: String,
        leadership_stability: f64,
    },
    StrategyState {
        state_label: String,
        recommended_position_pct: f64,
    },
    Signal {
        final_score: f64,
        signal_label: String,
    },
    Empty,
}
```

`EvidencePayload` 是 **Typed Enum**，不是 `serde_json::Value`。每种 `EvidenceKind` 对应一种或多种 Payload 变体。Formatter 通过模式匹配使用这些字段，而不是通过字符串 key 访问 JSON。

### Observation → Evidence 转换

盘中观察（`IntradayObservation`）是对盘中特征的语义化描述，例如：

```rust
pub enum IntradayObservation {
    BuyingPressure(f64),
    SellingPressure(f64),
    TrendPersistence(f64),
    BreakoutAttempt(f64),
    VolatilityExpansion(f64),
    LiquidityDryUp(f64),
}
```

在 Evidence Builder 中，Observation 被转换为 Evidence：

```rust
pub trait EvidenceBuilder {
    fn build(
        &self,
        observations: &[IntradayObservation],
        market_view: &ExecutionMarketView,
        signal: &SignalSnapshot,
        state: &StrategyStateSnapshot,
    ) -> Vec<Evidence>;
}
```

例如：

- `IntradayObservation::BuyingPressure(0.8)` → `Evidence { kind: MomentumExpansion, confidence: 0.8, direction: 1.0, ... }`
- `IntradayObservation::LiquidityDryUp(0.7)` → `Evidence { kind: LiquidityConfirmation, confidence: 0.7, direction: -1.0, ... }`（当确认度低时）

### Formatter 从 Evidence 生成文本

Formatter 不读取预计算字符串，而是根据 `EvidenceKind` + `EvidencePayload` 生成文本：

```rust
pub trait EvidenceFormatter {
    fn format(&self, evidence: &Evidence) -> String;
}
```

例如：

```rust
match evidence.payload {
    EvidencePayload::Volume { volume_ratio } => {
        format!("量能放大，量比 {:.2}", volume_ratio)
    }
    EvidencePayload::Gap { gap_pct } => {
        format!("跳空 {:.2}%", gap_pct * 100.0)
    }
    _ => evidence.kind.to_string(),
}
```

这样：
- CLI 可以生成简短文本。
- Desktop 可以生成卡片标题。
- PDF 可以生成正式段落。
- LLM 可以直接读取 Evidence 结构，不需要解析字符串。

## 未采纳方案（Rejected Alternatives）

### 1. Evidence 携带 `reason: String`

**原因未采纳**：预计算文本会锁死 Consumer 的表达方式。CLI 需要简短，LLM 需要自然语言，PDF 需要正式。文本应该由 Formatter 从结构化数据生成，而不是硬编码在 Evidence 中。

### 2. Evidence Payload 使用 `serde_json::Value`

**原因未采纳**：JSON 会让 Rust 的类型系统失效。Formatter 必须记住字符串 key（如 `metadata["gap"]`），重构和审计时容易出错。Typed Enum 可以在编译期保证完整性。

### 3. Observation 直接进入 Assessment

**原因未采纳**：Observation 是盘中语义，Research Context 是研究语义，Strategy State 是策略语义。如果没有统一转换为 Evidence，AssessmentEngine 需要理解三种不同的输入模型，难以扩展。

### 4. 使用 trait object 的 `EvidencePayload`

**原因未采纳**：虽然 trait object 更灵活，但会带来序列化、反序列化和跨 crate 边界传递的复杂性。Typed Enum 足够表达当前需求，且更符合 Rust 的 zero-cost 抽象。

### 5. 为每个 Consumer 定义不同的 Evidence 类型

**原因未采纳**：这会导致术语再次碎片化（如 `ExecutionEvidence`、`ResearchEvidence`、`LLMEvidence`）。统一类型是 Evidence 模型的最大价值。

## V8 边界

**做**：

- 定义 `Evidence`、`EvidenceKind`、`EvidenceSource`、`EvidencePayload`。
- 实现 `EvidenceBuilder`，将 `IntradayObservation`、`ExecutionMarketView`、`SignalSnapshot`、`StrategyStateSnapshot` 转换为 `Evidence`。
- 提供默认 `EvidenceFormatter` 实现，供 CLI / Desktop / PDF 使用。
- 在 `ExecutionAssessment` 中保存 `supporting_evidence` 和 `conflicting_evidence`。

**不做**：

- 不为 Evidence 增加 `reason: String` 或 `description: String`。
- 不使用 `serde_json::Value` 作为 Payload。
- 不在 Evidence 中直接引用 `ResearchContext` 或 `IntradaySnapshot` 等原始类型。
- 不将 Observation 和 Evidence 合并为同一概念。

## 验证

- `cargo check`：全 workspace 通过。
- `cargo test -p execution-engine`：Evidence Builder 的单元测试通过。
- Formatter 可以正确从 Typed Payload 生成中英文文本（演示即可）。
- Evidence 可以序列化为 JSON 供 Research Asset 和 LLM 消费，且不丢失类型信息。

## 演进路径

- **Phase 1（ADR Freeze）**：ADR-082、ADR-083、ADR-084 冻结。
- **Phase 2（DTO Freeze）**：在 Rust 中定义 `Evidence` 相关 DTO。
- **Phase 3（Builder）**：实现第一批 `EvidenceBuilder`，覆盖主要盘中观察和研究上下文。
- **Phase 4（Formatter）**：实现 CLI 和 Markdown 的 `EvidenceFormatter`。
- **Phase 5（Research Asset）**：将 Evidence 作为 V8 Research Asset 的一种内容类型持久化。
- **Phase 6（Long-term Calibration）**：基于历史 Evidence 的有效率，调整 Evidence 的置信度计算和权重。

## 相关文档

- `docs/v8/adr-082-execution-platform.md`
- `docs/v8/adr-084-llm-boundary.md`
- `docs/v6/adr-068-research-context-reporting-layer.md`
- `docs/v6/adr-077-research-platform-freeze.md`
- `docs/architecture-invariants.md`
