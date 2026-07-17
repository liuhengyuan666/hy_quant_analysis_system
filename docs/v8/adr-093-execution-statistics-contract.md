# ADR-093: Execution Statistics Contract Freeze

## 状态

Accepted

## 背景

V8 Execution Platform 的 Phase 2A 已经满足入口条件：Pipeline、Replay、Validation CLI、Golden Suite 均已闭环，且 `market_regime_label` 的事实来源已经恢复（ADR-092 / 2A-1 完成）。

Phase 2A 的目标不是继续增加架构或 DTO，而是进入 **Research Calibration 的事实层（Fact Layer）**：用统计方法把 `ExecutionResearchRecord` 集合转化为可重复的**经验事实（Empirical Facts）**，为后续 Calibration 提供客观依据。

为了避免 Statistics 模块演变成无所不能的 Analytics God Module，在进入实现之前，必须先把职责范围冻结。

## 目标

定义 `ExecutionStatistics` 的精确职责边界、输出契约和验收标准，确保：

1. Statistics 只输出**事实**，不输出解释、建议或权重。
2. Formatter 负责 Markdown/JSON 等展示形式，Statistics 不感知展示层。
3. 后续 Calibration 只能消费这些统计事实，不能要求 Statistics 新增新口径。

## 最终方案

### 职责边界

```text
ExecutionResearchRecord
        │
        ▼
ExecutionStatistics  (Domain: empirical facts)
        │
        ▼
ExecutionStatisticsFormatter  (Presentation)
        │
        ├──────┬──────┐
        ▼      ▼      ▼
      JSON  Markdown  HTML/CLI
```

- `ExecutionStatistics` 属于 Domain，是稳定的计算契约。
- Formatter 属于 Presentation，可随 Consumer 需求扩展。
- LLM、Calibration、Report 只能消费 `ExecutionStatistics`，不能反向要求 Statistics 改变口径。

### 第一阶段统计范围（冻结）

以下六类统计构成 Phase 2A-2 的完整输出。任何新增统计类型必须经过 ADR 修订。

#### 1. Evidence Frequency

每个 `EvidenceKind` 出现的次数和占比。

| EvidenceKind | Count | Ratio |
|-------------|------:|------:|
| TrendParticipation | 213 | 41.3% |
| Confirmation | 192 | 37.2% |
| Distribution | 8 | 1.6% |

#### 2. Evidence Pair Matrix

两个 EvidenceKind 在同一条 `ExecutionResearchRecord` 中共同出现的次数。

```text
TrendParticipation + Confirmation = 312
Distribution + RiskExpansion = 17
```

#### 3. Decision Distribution

最终 `ExecutionDecision` 的分布。

```text
BuyNow = 125
Wait = 8958
Reduce = 0
```

#### 4. Prior Distribution

Prior Evidence 的分布。注意：统计的是 Prior Evidence，而不是原始的 `StrategyState`。

```text
NoTrade = 523
DeRisk = 412
LeftProbe = 98
```

#### 5. Assessment Histograms

至少四张直方图：

- `confidence`
- `consensus`
- `coverage`
- `risk`

#### 6. Outcome Matrix

将 `ExecutionDecision` 与 `ExecutionOutcome` 交叉，形成分类矩阵。

```text
BuyNow → Hit / Miss / TooEarly / TooLate
Wait   → (observed outcome)
Reduce → Hit / Miss / TooEarly / TooLate
```

### 输出数据结构

```rust
pub struct ExecutionStatistics {
    pub meta: ExecutionStatisticsMeta,
    pub evidence_frequency: EvidenceFrequency,
    pub evidence_pairs: EvidencePairMatrix,
    pub decision_distribution: DecisionDistribution,
    pub prior_distribution: PriorDistribution,
    pub assessment_histograms: AssessmentHistograms,
    pub outcome_matrix: OutcomeMatrix,
}

pub struct ExecutionStatisticsMeta {
    pub record_count: usize,
    pub scope: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub generated_at: DateTime<Utc>,
    pub execution_engine_version: String,
    pub policy_hash: Option<String>,
}
```

具体子结构（如 `EvidenceFrequency`、`EvidencePairMatrix` 等）在实现 ADR 中定义，但顶层 `ExecutionStatistics` 字段在 Phase 2A-2 内冻结。

### 样本策略（非硬编码）

验证顺序采用三层渐进式：

```text
Representative Sample
        ↓
Expanded Sample
        ↓
Full Population
```

- 第一层：Golden Suite 或代表性子集，用于调试 Statistics 代码。
- 第二层：扩大样本，确认统计输出在更多数据上仍然合理。
- 第三层：全量 Discovery 数据，建立 Evidence Frequency Baseline。

具体数字不作为契约；契约是这三层方法本身。

## 不做

Phase 2A-2 不做以下事情：

- 不做相关性分析（Correlation）
- 不做特征重要性（Feature Importance）
- 不做 SHAP 或模型可解释性
- 不做机器学习或权重学习
- 不做投资建议或校准结论

以上属于 Phase 2C / 2D / 2E。

## 验收标准

### Contract

- ✅ `ExecutionStatistics` 数据结构冻结。

### Functionality

- ✅ 六类统计全部输出。

### Presentation

- ✅ 支持 JSON 输出。
- ✅ 支持 Markdown 输出。

### Validation

- ✅ 能跑 Golden Suite（Representative Sample）。
- ✅ 能跑 Representative Sample。
- ✅ 能跑 Full Dataset（当前 Discovery 约 9,000+ 条）。

### Regression

- ✅ 同输入同输出可重复。

## 验证

- `cargo test -p execution-replay`：Statistics 相关测试通过。
- `cargo check --workspace`：全 workspace 通过。
- CLI 命令可运行：`execution-statistics --suite <path>` 和 `execution-statistics --scope <scope> --from <date> --to <date>`。

## 相关文档

- `docs/v8/adr-082-execution-platform.md`
- `docs/v8/adr-085-execution-evaluation.md`
- `docs/v8/execution-event-sufficiency-review.md`
- `docs/v8/adr-092-phase-2a-plan.md`
