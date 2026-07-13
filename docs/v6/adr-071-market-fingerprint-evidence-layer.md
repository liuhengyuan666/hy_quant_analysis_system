# ADR-071: Evidence Layer Freeze — Fingerprint / Matcher / SearchResult Contract

## 状态

Accepted

## 背景

V7.2 完成了两个关键子阶段：

- **V7.2A**：定义了 `MarketFingerprint` 作为规范化的历史特征表示，以及从 `ResearchContext` 构建指纹的 `MarketFingerprintBuilder`。
- **V7.2B**：实现了 Evidence Retrieval Engine，包括 Normalizer、`DistanceMetric` trait、`SimilarityMatcher`、`OutcomeProfiler` 和 CLI `research analogues`。

V7.2B 是 Research Layer 第一次真正引入历史检索引擎。它的接口稳定性与 V6 的 `ResearchContext` + Reporting Platform（ADR-068/069）处于同一等级。如果 Evidence Layer 的契约不先冻结，V7.3 Consensus 将建立在一个可能反复变化的地基上，导致后续大量回改。

因此，本 ADR 冻结 Fingerprint / Evidence Layer 的架构规则、契约边界和演进纪律。

## 目标

1. 将 `MarketFingerprint` 确立为 Research Layer 的 Canonical Historical Representation。
2. 将 Fingerprint Builder、Matcher、DistanceMetric、Normalizer、OutcomeProfiler、SearchResult 的边界和职责固定下来。
3. 明确 Evidence Layer 与上层 Consensus 的隔离关系：Consensus 只能消费 Evidence，不能修改 Evidence。
4. 为 V7.3 Consensus 的启动提供可检查的冻结条件。

## 最终方案

### 架构分层

```text
Engine / Store
    ↓
research-context       # 语义契约（已冻结，ADR-068）
    ↓
MarketFingerprintBuilder
    ↓
MarketFingerprint      # Canonical Historical Representation（本 ADR 冻结）
    ↓
Normalizer
    ↓
DistanceMetric         # Strategy Pattern（本 ADR 冻结）
    ↓
SimilarityMatcher
    ↓
HistoricalMatch
    ↓
OutcomeProfiler
    ↓
SearchResult           # Evidence 输出契约（本 ADR 冻结）
    ↓
Consensus              # V7.3，只能消费，不能修改
```

### Architecture Rules

> **Rule-01: Fingerprint Builder 只能消费 `ResearchContext`，不能消费 `ResearchDataset`，不能访问 Market Store。**
>
> 正确链路：`ResearchDataset → ResearchContext → MarketFingerprintBuilder → MarketFingerprint`。
> 错误链路：`MarketFingerprintBuilder → MarketStore` 或 `MarketFingerprintBuilder → ResearchDataset`。
> 原因：Fingerprint 是语义层的产物，不是数据层的产物。Builder 不应知道信号、环境、轮动的原始存储格式。

> **Rule-02: Similarity Engine 只接受 `Vec<MarketFingerprint>`，不能回调 AppContext、ResearchContext 或 Store。**
>
> `SimilarityMatcher` 的输入必须是已经预加载的指纹集合。匹配算法不能为了获取历史指纹而重新查询数据库或重建语义模型。
> 原因：历史检索是 Evidence Layer 内部的纯计算，如果 Matcher 反向依赖应用服务，性能会退化为 O(N) 次语义重建，且测试和替换距离函数都会变困难。

> **Rule-03: Fingerprint matching must operate on preloaded historical observations, never by repeatedly reconstructing semantic models through application services.**
>
> 调用方（如 `app-service::research_analogues`）必须在调用 Matcher 之前，通过批量查询将构建 fingerprint 所需的全部历史数据加载到内存中。
> Matcher 只比较已经存在的 fingerprint。这保证了 Evidence Layer 的可测试性、可替换性和可扩展性。

> **Rule-04: Evidence Layer 不能知道 Consensus。**
>
> `MarketFingerprint`、`ObservationVector`、`EvolutionVector`、`DistanceMetric`、`SimilarityMatcher`、`SearchResult`、`OutcomeProfile` 中禁止出现任何 Consensus 相关的字段或方法（如 `consensus_score`、`transition`、`bias`）。
> 原因：Consensus 是 V7.3 的语义聚合层，必须构建在稳定 Evidence 之上。如果 Evidence 提前感知 Consensus，会形成循环依赖。

> **Rule-05: Fingerprint 结构冻结为 `ObservationVector` + `EvolutionVector`。**
>
> ```rust
> struct MarketFingerprint {
>     pub scope: AnalysisScope,
>     pub date: NaiveDate,
>     pub observation: ObservationVector,
>     pub evolution: EvolutionVector,
> }
> ```
> `ObservationVector` 包含 `environment`、`signal`、`stretch`、`rotation`。
> `EvolutionVector` 包含 `confirmation`、`recovery`。
> 未来新增维度（如 V8 的 Fear、Liquidity）应通过新增 Vector 或版本升级实现，不应把 Evidence 或 Consensus 字段混入当前结构。

> **Rule-06: `DistanceMetric` 是 Strategy Pattern，可替换。**
>
> ```rust
> pub trait DistanceMetric {
>     fn distance(&self, a: &FeatureVector, b: &FeatureVector) -> f64;
> }
> ```
> 当前默认实现为 `CosineDistance`。未来可新增 `EuclideanDistance`、`WeightedCosine`、`Mahalanobis` 等实现，均不修改 Fingerprint 或 Matcher 结构。

> **Rule-07: Normalizer 独立 from DistanceMetric。**
>
> 归一化发生在 `MarketFingerprint → FeatureVector` 阶段，由 Normalizer 负责。DistanceMetric 只接收已经归一化的 `FeatureVector`。
> 链路：`Fingerprint → Normalizer → FeatureVector → DistanceMetric → Ranking`。

> **Rule-08: Similarity 对外不暴露原始百分比。**
>
> CLI / Desktop / API 只展示等级（`Very High` / `High` / `Moderate` / `Weak`）或排名（`#1` / `#2`）。
> 内部可以保留 distance，但 distance 是距离函数的产物，会随算法变化，直接暴露百分比会误导用户。

> **Rule-09: `OutcomeProfile` 是独立对象。**
>
> `OutcomeProfiler` 消费 `HistoricalMatch` 列表和 `ForwardReturnProvider`，输出 `OutcomeProfile`。
> `OutcomeProfile` 包含 `median`、`mean`、`best`、`worst`、`win_rate`、`median_max_drawdown` 等统计量，与 `MarketFingerprint` 和 `SimilarityMatcher` 解耦。

> **Rule-10: `SearchResult` 是 Evidence Layer 的稳定输出契约。**
>
> ```rust
> struct SearchResult {
>     pub searched_days: usize,
>     pub filtered_days: usize,
>     pub average_distance: f64,
>     pub matches: Vec<HistoricalMatch>,
>     pub outcome: Option<OutcomeProfile>,
> }
> ```
> `HistoricalMatch` 包含 `date` 和 `level`（不暴露原始 distance）。
> 这个结构供 CLI、Desktop、API、LLM 统一消费。

> **Rule-11: Fingerprint 使用版本号。**
>
> `MarketFingerprint` 应携带 `version: u32` 字段，从 `1` 开始。如果未来字段发生不兼容变化（如新增 Vector、修改字段语义），必须提升版本号。
> Matcher 可以据此拒绝或兼容不同版本的 fingerprint。这避免了隐式字段漂移导致的匹配错误。
> 本 ADR 冻结此规则；`version` 字段的具体实现可在冻结审查后的代码调整中补充。

## Fingerprint 版本化

### V1（当前）

```rust
MarketFingerprint {
    scope: AnalysisScope,
    date: NaiveDate,
    observation: ObservationVector {
        environment: f64,
        signal: f64,
        stretch: f64,
        rotation: Vec<(String, f64)>,
    },
    evolution: EvolutionVector {
        confirmation: f64,
        recovery: f64,
    },
}
```

> 冻结审查后，V1 将补充 `version: u32 = 1` 字段。在此之前，所有序列化/反序列化默认按 V1 理解。

### 未来版本升级原则

- 向后兼容的新增字段：使用 `#[serde(default)]`。
- 不兼容的结构变化：提升 `version` 字段，并新增对应版本的 Matcher/Normalizer 支持。
- 旧版本 fingerprint 可用于只读展示，但不应与新版本 fingerprint 混合进行相似度匹配。

## V7.2C Research Calibration

Evidence Layer 冻结后，进入 **Research Calibration** 阶段。Calibration 分为三个阶段推进：

| 阶段 | 状态 | 目标 |
|------|------|------|
| Phase A：Evidence Layer Freeze Review | 已完成 | 审查并冻结 V7.2A/V7.2B 契约，形成 ADR-071 |
| Phase B：Calibration Framework | 已实现 | 构建可运行的校准报告框架，输出四章节 Markdown 报告 |
| Phase B+：Calibration Baseline Freeze | 进行中 | 为报告增加距离分布直方图与基线元数据，建立版本化基线，多次运行检测漂移 |

Phase A/B 完成后，必须继续完成 Phase B+：

1. 对最近 30~60 个交易日连续运行：
   ```bash
   quant-cli research calibration --scope global
   quant-cli research calibration --scope cn
   ```
2. 命令会自动运行 `confirmation`、`recovery`、`analogues` 并聚合统计。
3. 记录输出等级、关键驱动、匹配日期、前向收益统计。
4. 检查：
   - 指标是否具有足够区分度（避免长期固定在一个等级）。
   - 是否存在异常值（如 Recovery 连续多日 100，或 Confirmation 永远 Moderate）。
   - 相似日匹配结果是否符合市场直觉、是否稳定。
   - 权重、阈值、距离函数是否需要微调。
   - **距离分布直方图是否显示匹配过度集中在某个 bucket**。
5. 形成 `reports/calibration/research-calibration-{scope}-{start}-{end}.md`，报告必须包含：
   - 报告头：`Calibration Baseline Version`、`Generated At`、`Scope`、`Window`。
   - 第 3 章：`Distance Distribution (all target-vs-historical pairs)` 直方图。

Calibration 期间可以微调权重、阈值、距离函数，但**不能修改 Fingerprint 结构**。Phase B+ 期间，**暂时不修改距离函数，只增加统计维度**，为后续阈值调整提供数据依据。任何调整必须回写 `设计规划-v7.md` 和本 ADR。

### Phase B+ 新增规则

> **Rule-12: Calibration Report 必须携带版本化 Baseline。**
>
> 每次 Calibration 报告必须携带 `Baseline Version`（从 1 开始），当校准方法或报告结构发生不兼容变化时递增。`Baseline Version` 只应在 Evidence 语义变化时递增，例如：修改距离函数、Normalizer、特征权重、匹配阈值、或报告统计维度的语义。实现层面的优化（如 `app-service` 的 bulk fetch 重构）不应导致 Baseline Version 变化。

> **Rule-13: Calibration Report 必须记录生成时间与窗口。**
>
> 报告头必须记录 `Generated At` 与 `Window`，使多次运行可比较，并支持漂移检测。

> **Rule-14: Calibration Report 必须输出 Distance Distribution 直方图。**
>
> 直方图覆盖所有 `target-vs-historical` 距离对，按 `0.0-0.2 / 0.2-0.4 / 0.4-0.6 / 0.6-0.8 / 0.8-1.0` 分桶。用于判断匹配等级是否过度集中在某个区间，以及阈值是否需要调整。

> **Rule-15: Phase B+ 完成后，Calibration Baseline 进入冻结。**
>
> V7.3 Consensus 只能基于冻结后的 Calibration Baseline 运行。Consensus 不能修改 Calibration 报告的统计维度、元数据或距离函数。

## V7.3 启动条件

Consensus 实现必须满足以下全部条件：

1. **Architecture Stable**：Fingerprint Builder、DistanceMetric、Normalizer、SimilarityMatcher、SearchResult、OutcomeProfile 的接口和职责已冻结，并形成 ADR-071。
2. **Compile Stable**：`cargo check` 与相关 crate 测试通过。
3. **Behavior Stable**：V7.2C Research Calibration 已完成，对最近 30~60 个交易日的观察显示输出具有区分度、无异常跳变、匹配结果稳定。
4. **Semantic Stable**：LLM / 人类可以稳定解释每个输出的含义，输出与底层 Observation / Evolution 数据一致。
5. **Calibration Baseline Frozen**：V7.2C Phase B+ 已完成，Calibration Report 已携带版本化基线（`Baseline Version`）、生成时间（`Generated At`）和距离分布直方图（`Distance Distribution`），并且至少对 Global 和 CN 两个 scope 各运行过一次，报告已写入 `reports/calibration/`。

> **冻结纪律**：Consensus 只能消费 Observation、Evolution、Evidence 三层输出，不能修改它们的结构或计算逻辑。Consensus 同样不能修改 Calibration Baseline 的统计维度、元数据或距离函数。

## 未采纳方案（Rejected Alternatives）

### 1. 让 Fingerprint 直接包含 Consensus 或 Evidence 字段

**原因未采纳**：Fingerprint 是 Canonical Historical Representation，只应包含 Observation 和 Evolution。如果包含 Consensus 或 Evidence 结果，会导致语义层循环引用，并阻碍距离函数和匹配算法的独立演进。

### 2. 让 Matcher 在匹配时动态查询历史 MarketFingerprint

**原因未采纳**：动态查询会重新引入 O(N) 次存储访问和语义重建，破坏 Evidence Layer 的独立性和性能。Matcher 必须只操作预加载的指纹集合。

### 3. 将 Normalizer 嵌入 DistanceMetric

**原因未采纳**：归一化是特征工程步骤，距离是度量步骤。耦合后无法独立实验不同的归一化策略（如 z-score、rank、min-max）和距离函数。

### 4. 直接暴露 similarity 百分比给用户

**原因未采纳**：percentage 会随距离函数和特征权重变化而变化，用户会误将其当作固定概率。等级和排名更稳定、更不易误导。

### 5. 在 V7.2B 后立即进入 V7.3 Consensus

**原因未采纳**：Evidence Layer 刚实现，未经真实市场验证。如果 Consensus 提前建立在不稳定的 Evidence 上，后续底层微调会导致 Consensus 反复回改。

## 验证

- `cargo check`：全 workspace 通过。
- `cargo test -p market-fingerprint-engine -p app-service`：通过。
- CLI 输出：`quant-cli research analogues --scope global --horizon 20 --top-n 5` 在合理时间内返回稳定的 `SearchResult`。

## 演进路径

- **V7.2C**：Research Calibration，微调阈值/权重/距离函数，不修改 Fingerprint 结构。
- **V7.3**：Consensus，在冻结的 Evidence Layer 之上构建语义聚合层。
- **V8 及以后**：如需新增 Fingerprint 维度，通过 `version` 升级和新增 Vector 实现，保持旧版本兼容或明确隔离。

## 相关文档

- `设计规划-v7.md`
- `docs/v6/adr-068-research-context-reporting-layer.md`
- `docs/architecture-invariants.md`
