# Execution Platform V2 Golden Validation Suite

## 验证报告（gitignored 运行产物）

| 报告 | 路径 |
|---|---|
| Golden Suite 首次运行 | `reports/execution-validation/golden_suite_run_2026-07-17.json` |
| Execution Statistics 全量 | `reports/execution-validation/execution_statistics_cn_full_2026-07-17.json` |
| Evidence Trace 全量 | `reports/execution-validation/evidence_trace_cn_full_2026-07-17.md` |
| Evidence Trace JSON | `reports/execution-validation/evidence_trace_cn_full_2026-07-17.json` |
| Distribution Coverage Review | `reports/execution-validation/distribution_coverage_cn_full_2026-07-17.md` |
| Decision Margin Review | `reports/execution-validation/decision_margin_cn_full_2026-07-17.md` |
| Decision Gate Analysis | `reports/execution-validation/decision_gate_cn_full_2026-07-17.md` |
| Decision Gate JSON | `reports/execution-validation/decision_gate_cn_full_2026-07-17.json` |

> 注意：`reports/` 目录是 gitignored 的运行产物，上述报告文件仅在本地 workspace 中保留。

## 目的

这个目录保存 Execution Platform V2 的**永久回归测试数据集**。它不是临时测试，而是平台每次重大演进（Observation、Evidence、Assessment、Decision、Policy、Evaluation）都需要重新跑一遍的基准。

## 文件

| 文件 | 说明 |
|---|---|
| `execution_validation_suite.yaml` | 10 个黄金案例，每个案例覆盖一个特定 Decision Boundary |
| `README.md` | 本文档：说明 Schema、使用方式、Review 流程和 Acceptance Checklist |

## 案例设计原则

每个案例选择的不是"行情好/坏"，而是"决策边界"：

- 强趋势：应该 BuyNow
- 弱趋势：BuyNow / Wait 边界
- 假突破：应该 Wait
- 高开过远：NoChase 边界
- 派发：Reduce 边界
- 恐慌下跌：不应抄底
- 恢复：从 State 到执行
- 横盘：无方向
- 流动性崩溃：Risk 主导
- 体制切换：最难的保守边界

## Schema

```yaml
cases:
  - id: CASE-ID-001
    symbol: "000001"
    date: "2024-01-01"
    scope: "cn"
    market_regime: "Bullish"
    pattern_type: "StrongTrend"
    expected_decision: "BuyNow"
    reason: "为什么选这个案例"
    validated_by: "Manual"
    notes: "Review 时的备注"
```

字段说明：

| 字段 | 说明 |
|---|---|
| `id` | 永久编号，格式 `<PATTERN_TYPE>-NNN` |
| `symbol` | 标的代码 |
| `date` | 案例日期 `YYYY-MM-DD` |
| `scope` | `global` / `cn` / `hk` |
| `market_regime` | 当时的市场体制标签，来自 `ResearchContext.market_state.label` |
| `pattern_type` | 人工模式分类 |
| `expected_decision` | 期望的决策结果：`BuyNow` / `Wait` / `Reduce` / `Skip` |
| `reason` | 为什么选这个案例 |
| `validated_by` | 案例来源：`Manual` / `HistoricalStudy` / `ResearchAsset` / `StrategyReview` |
| `notes` | 自由备注 |

## 使用方式

单个案例验证：

```bash
cargo run -p quant-cli -- validate-execution-replay \
  --symbol 510300 \
  --date 2024-07-15 \
  --scope cn \
  --output explain
```

```bash
cargo run -p quant-cli -- validate-execution-replay \
  --symbol 510300 \
  --date 2024-07-15 \
  --scope cn \
  --output trace
```

```bash
cargo run -p quant-cli -- validate-execution-replay \
  --symbol 510300 \
  --date 2024-07-15 \
  --scope cn \
  --output markdown
```

批量 Suite 运行（未来实现）：

```bash
cargo run -p quant-cli -- validate-execution-suite --suite research/validation/execution/execution_validation_suite.yaml
```

执行统计（Phase 2A-2）：

```bash
# Representative Sample: Golden Suite
cargo run -p quant-cli -- execution-statistics \
  --suite research/validation/execution/execution_validation_suite.yaml \
  --output markdown

# Full Population: historical date range
cargo run -p quant-cli -- execution-statistics \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

## Review 流程

对每一个案例，输出必须满足以下检查清单：

### 1. ExecutionEvent 完整性

- [ ] `symbol`、`date`、`execution_id`、`timestamp` 存在
- [ ] `versions` 字段完整（schema/engine/policy/research）
- [ ] `policy` 和 `policy_hash` 存在
- [ ] `request` 包含 signal、strategy_state、quote、market_view
- [ ] `market_view.market_regime_label` 存在
- [ ] `features`、`observations`、`evidences`、`assessment`、`decision` 存在

### 2. Evidence 合理性

- [ ] 支撑证据方向与决策方向一致
- [ ] 冲突证据被正确识别
- [ ] Evidence 来源可追溯到具体输入层（Signal / State / Market View / Observation）

### 3. Assessment 合理性

- [ ] `confidence` 与证据强度一致
- [ ] `consensus` 反映证据方向一致性
- [ ] `risk` 与风险证据一致
- [ ] `dominant_direction` 与多数证据方向一致

### 4. Decision 合理性

- [ ] `state` 与 `expected_decision` 一致（或偏差可被解释）
- [ ] `confidence` 高于 Policy 阈值时才能 BuyNow
- [ ] Risk 高时决策被抑制

### 5. Outcome 真实性

- [ ] T+20 / T+60 / T+120 收益从真实历史行情计算
- [ ] MFE / MAE / Max Drawdown 数值合理
- [ ] 无未来数据泄漏

### 6. Evaluation 合理性

- [ ] `Evaluation` 标签与 `Outcome` + `Decision` 一致
- [ ] `evaluation_version` 记录规则版本
- [ ] 标签能从 `(Event, Outcome)` 规则推导

## 通过标准

- 10 个案例全部满足 Acceptance Checklist。
- 如果某个案例失败，记录失败原因，判断是：
  - **Domain 缺陷**：需要回到 `execution-engine` 修改
  - **Evaluation 规则缺陷**：需要更新 `RuleBasedEvaluationEngine` 或 `ExecutionEvaluation` 分类
  - **案例本身问题**：需要更新 Suite 中的案例描述或日期

## 演进规则

- 新增案例需要 PR Review，并更新版本号。
- 修改 `expected_decision` 或 `pattern_type` 需要说明原因。
- 每次平台重大变更后，必须重新跑 Suite 并更新结果记录。
- 案例日期应优先选择有完整历史数据的真实交易日。

## 相关文档

- `docs/v8/adr-082-execution-platform.md`
- `docs/v8/adr-085-execution-evaluation.md`
- `docs/v8/execution-event-sufficiency-review.md`

## 第一次验证结果（2026-07-17）

使用命令：

```bash
cargo run -p quant-cli -- validate-execution-suite \
  --suite research/validation/execution/execution_validation_suite.yaml \
  --output detail
```

结果：

```text
Total:   10
Passed:  8
Failed:  2
Pass Rate: 80.0%
Decision Accuracy: 80.0%
```

| 状态 | 案例 | 预期 | 实际 |
|---|---|---|---|
| PASS | STRONG_TREND-001 | BuyNow | BuyNow |
| PASS | WEAK_TREND-001 | Wait | Wait |
| PASS | FALSE_BREAKOUT-001 | Wait | Wait |
| PASS | GAP_TOO_HIGH-001 | Wait | Wait |
| FAIL | DISTRIBUTION-001 | Reduce | Wait |
| PASS | PANIC_SELLOFF-001 | Wait | Wait |
| FAIL | RECOVERY-001 | BuyNow | Wait |
| PASS | SIDEWAYS-001 | Wait | Wait |
| PASS | LIQUIDITY_COLLAPSE-001 | Wait | Wait |
| PASS | REGIME_SWITCH-001 | Wait | Wait |

### 失败归因

| 失败案例 | 根因层级 | 说明 |
|---|---|---|
| DISTRIBUTION-001 | Decision / State | 信号为 Reduce、状态为 DeRisk，但当前 State 证据过度抑制，Decision 输出为 Wait |
| RECOVERY-001 | Decision / State | 信号为 Buy、状态为 NoTrade，State 门槛阻止了 BuyNow |

结论：当前 Execution Platform 在 State 为 NoTrade / DeRisk 时过度保守，导致本应执行的 BuyNow 和本应减仓的 Reduce 都被压为 Wait。这不是 Evidence 层问题，而是 Assessment/Decision 层对 State 证据的权重问题。

### 后续行动

1. 在 ADR 中记录此发现：State 证据权重需要校准。
2. 不要立刻修改 Policy 阈值，先积累更多 Research Asset（≥100 条）后再做系统校准。
3. 保留这两个 FAIL 案例作为 State 权重校准的基准。

## 第一次 Execution Statistics（2A-2）结果（2026-07-17）

使用命令：

```bash
cargo run -p quant-cli -- execution-statistics \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

样本：CN 2024-01-01 至 2025-06-30，共 8,616 条 `ExecutionResearchRecord`。

### 关键统计事实

| 统计项 | 结果 |
|---|---|
| **Decision Distribution** | BuyNow: 1.42% (122) / Wait: 98.58% (8,494) / Reduce: 0.00% (0) |
| **Prior Distribution** | DE_RISK: 50.14% / NO_TRADE: 20.06% / LEFT_PROBE: 18.38% / CONFIRM_ADD: 9.75% / FULL_TREND: 1.67% |
| **Evidence Frequency** | StrategyState/Confirmation/Breadth/Recovery/LeadershipRotation/SignalStrength 各约 14.72%（每记录固定 6 条） |
| **动态 Evidence** | MarketAcceptance 4.22%、MomentumExpansion 3.69%、TrendParticipation 3.04%、RiskExpansion 0.74% |
| **Outcome Matrix** | 因 Reduce=0，仅 BuyNow 与 Wait 有结果；大部分 Wait 的 Outcome 为 Miss 或 Unknown |

### 关键发现

1. **Reduce 决策完全缺失**：8,616 条记录中没有一条产生 Reduce。这与 Golden Suite 中 `DISTRIBUTION-001` 的失败一致。
2. **Prior 以 DeRisk 为主**：50% 的 Prior Evidence 是 DeRisk，20% 是 NoTrade。这意味着系统默认处于风险规避状态。
3. **RiskExpansion Evidence 极稀有**：仅占 0.74%。如果系统缺少风险扩张证据，Reduce 阈值再低也不会触发。
4. **动态 Evidence 不足**：MarketAcceptance / MomentumExpansion / TrendParticipation / RiskExpansion 这些盘中观察证据占比很低，说明盘中语义层尚未充分激活。

### 对 Calibration 的启示

- 不要降低 `reduce_threshold`，因为即使降到极低，也缺乏 RiskExpansion / Distribution Evidence 来触发 Reduce。
- 应该优先研究：**为什么 RiskExpansion / Distribution Evidence 这么少？** 是盘中观察引擎没生成，还是生成条件太严格？
- Prior 权重过高是事实，但直接调整权重前，需要确认动态 Evidence 是否足以支撑决策。

### 后续行动

1. 检查盘中观察引擎（ObservationEngine）生成 RiskExpansion / Distribution 的条件。
2. 对比手动标注的 Reduce 案例，确认它们是否会产生这些 Evidence。
3. 在动态 Evidence 充足之前，不调整 Prior 权重或阈值。
4. 继续积累 Research Asset，目标 ≥300 条，再进入 Calibration。

## 第一次 Evidence Trace / Funnel（2A-3）结果（2026-07-17）

> **更新说明**：在最初运行 Evidence Trace 后，发现 `build_execution_event` 中 `quote.prev_close` 被写死为 `bar.close`（TODO 占位），导致 `today_return` 全部为 0，盘中观察条件失真。已在 2A-4 启动前修复为真实前收盘价。本表数字为修复后重跑结果。

使用命令：

```bash
cargo run -p quant-cli -- execution-evidence-trace \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

样本：CN 2024-01-01 至 2025-06-30，共 8,615 条 `ExecutionResearchRecord`。

### 核心发现（修复 prev_close 后）

| EvidenceKind | Observation | Obs→Evd | Evidence | Evd→Asm | Assessment | Wait | BuyNow | Reduce |
|---|---|---|---|---|---|---|---|---|
| RiskExpansion | 440 | 100% | 440 | 100% | 440 | 98.2% | 1.8% | **0.0%** |
| Distribution | 2,043 | 100% | 2,043 | 100% | 2,043 | 100.0% | 0.0% | **0.0%** |
| MomentumFailure | 16 | 100% | 16 | 100% | 16 | 100.0% | 0.0% | **0.0%** |
| MarketAcceptance | 2,470 | 100% | 2,470 | 100% | 2,470 | 95.0% | 5.0% | 0.0% |
| MomentumExpansion | 2,526 | 100% | 2,526 | 100% | 2,526 | 81.7% | 4.1% | 0.0% |
| TrendParticipation | 1,779 | 100% | 1,779 | 100% | 1,779 | 94.2% | 5.8% | 0.0% |

### 根因定位（更新）

1. **Distribution 并非死在 Observation 层。**
   - 修复 `prev_close` 后，2,043 条记录触发了 Distribution 观察（占全部记录 23.7%）。
   - 2,043 次观察 → 2,043 条 Evidence（100% 转换）。
   - 2,043 条 Evidence → 2,043 次进入 Assessment（100% 保留），全部被归类为 **Conflicting**。
   - 但决策结果：Wait=2,043 / BuyNow=0 / Reduce=0。

2. **RiskExpansion 同样死在 Assessment → Decision。**
   - 440 次观察 → 440 条 Evidence → 440 次 Assessment，全部 Conflicting。
   - 决策结果：Wait=432 / BuyNow=8 / Reduce=0。

3. **MomentumFailure 数量极少。**
   - 仅 16 条，同样全部 Conflicting → Wait，未产生 Reduce。

### 结论（更新）

**Reduce 为 0 的根因不在 Observation 层，而在 Assessment → Decision 层。**

- Observation 层：在真实前收盘价基础上，Distribution 能够正常触发，条件转换率 100%。
- Assessment 层：这些 bearish 证据被正确识别为 Conflicting（方向 -1.0）。
- Decision 层：尽管存在大量 bearish Assessment，系统从未输出 Reduce。

因此，下一步需要重点审查 **Decision 层** 的阈值、confidence/consensus 门槛、State 证据权重，而不是调整 Observation 条件。

### 修复 `prev_close` 的代码变更

- 文件：`crates/app-service/src/execution_replay.rs`
- 变更：在 `build_execution_event` 中，从 `market-store` 拉取当前日期前最近一个交易日的收盘价作为 `quote.prev_close`，不再使用 `bar.close` 占位。
- 影响：所有依赖 `today_return` 的盘中观察（Distribution、FailedBreakout、BreakoutAttempt 等）恢复为真实数据。
- 注意：`volume_ma20` 仍为 `1.0` 占位，因此 `volume_ratio` 当前为绝对成交量而非比值；这是 Distribution Coverage Review 中需要特别说明的限制。

## 2A-4A: Distribution Coverage Review（2026-07-17）

使用命令：

```bash
cargo run -p quant-cli -- execution-distribution-coverage \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

样本：CN 2024-01-01 至 2025-06-30，共 8,615 条 `ExecutionResearchRecord`。

### 特征百分位

| Feature | Count | Min | P10 | P25 | P50 | P75 | P90 | P95 | Max | Mean |
|---------|------:|----:|----:|----:|----:|----:|----:|----:|----:|-----:|
| close_position | 8,615 | 0.000 | 0.048 | 0.190 | 0.492 | 0.800 | 0.955 | 1.000 | 1.000 | 0.494 |
| volume_ratio | 8,615 | 61,863 | 763,402 | 2,223,551 | 7,807,645 | 83,677,504 | 214,638,218 | 309,008,444 | 1,313,460,195 | 65,301,695 |
| today_return | 8,615 | -0.133 | -0.019 | -0.009 | 0.000 | 0.008 | 0.020 | 0.030 | 0.179 | 0.000 |

### 条件覆盖统计

| 条件 | 满足记录数 | 占全部记录 | 占下跌日 |
|---|---:|---:|---:|
| `today_return < 0.0` | 4,252 | 49.4% | 100% |
| 下跌日 + `close_position < 0.2` | 2,042 | 23.7% | 48.0% |
| 下跌日 + `volume_ratio > 1.5` | 4,252 | 49.4% | 100% |
| **满足全部三个条件** | **2,042** | **23.7%** | **48.0%** |
| 实际产生 Distribution 观察 | 2,042 | 23.7% | 48.0% |
| 条件覆盖率 | 100% | — | — |

### 结论

1. **Distribution 条件转换率 = 100%**：所有满足三个条件的记录都产生了 Distribution 观察。
2. **Distribution 条件满足率极高**：23.7% 的记录满足条件，48% 的下跌日满足条件。这在直觉上偏宽松，但原因见下方限制。
3. **条件本身不是严格，而是 `volume_ratio` 被高估**：`volume_ma20` 在 `ExecutionRequest` 中被硬编码为 `1.0`，因此 `volume_ratio` 实际等于绝对成交量（动辄几十万、几百万），远大于 `1.5` 阈值。这导致 `volume_ratio > 1.5` 对所有记录恒成立。
4. **因此，Distribution 条件实际上退化为 `close_position < 0.2 && today_return < 0.0`**。

### 限制

- `volume_ratio` 不是真实比值，而是绝对成交量。在 `volume_ma20` 修复为真实 20 日均量之前，**不能根据当前覆盖率判断条件是否过严或过松**。
- 即便在当前失真条件下，Distribution 也能被 Observation 层正常触发，说明 Observation 层不是 Reduce=0 的瓶颈。

### 后续行动

1. 修复 `volume_ma20` 占位，从 `market-store` 拉取真实 20 日成交量均线。
2. 在真实 `volume_ratio` 下重新运行 Distribution Coverage Review。
3. 根据真实覆盖率，判断是否需要调整 `close_position` 或 `volume_ratio` 阈值。
4. 在真实 `volume_ratio` 出来之前，不调整 Observation 条件。

## 2A-4B: Decision Margin Review（2026-07-17）

使用命令：

```bash
cargo run -p quant-cli -- execution-decision-margin \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

样本：CN 2024-01-01 至 2025-06-30，共 8,615 条 `ExecutionResearchRecord`。

### 总体：Assessment.dominant_direction → Decision 映射

对每条记录都存在的固定证据（Confirmation / Recovery / Breadth / StrategyState / LeadershipRotation / SignalStrength）：

- 记录总数：8,615
- 方向为负（`dominant_direction < 0`）且低于 Reduce 阈值（`-0.3`）的记录：152
- 这些记录中最终为 Reduce 的：0
- 这些记录中最终为 Wait 的：152
- **Reduce Recall = 0.0%**

这意味着：**有 152 条记录的 Assessment 已经明确偏向 bearish 并跨过了 Reduce 阈值，但 Decision 层仍然输出 Wait。**

### 关键证据的 Decision Margin

| EvidenceKind | 记录数 | `dominant_direction < -0.3` 记录数 | 最终为 Reduce | 最终为 Wait | Reduce Recall |
|---|---|---:|---:|---:|---:|
| Distribution | 2,043 | 91 | 0 | 91 | 0.0% |
| RiskExpansion | 440 | 84 | 0 | 84 | 0.0% |
| MarketAcceptance | 2,470 | 0 | 0 | 0 | — |
| MomentumExpansion | 2,168 | 0 | 0 | 0 | — |
| TrendParticipation | 1,779 | 0 | 0 | 0 | — |
| 固定证据 | 8,615 | 152 | 0 | 152 | 0.0% |

### 方向分布直方图（固定证据，全部 8,615 条记录）

| Range | Total | BuyNow | Wait | Reduce |
|---|--:|--:|--:|--:|
| [-0.40, -0.30) | 144 | 0 | 144 | 0 |
| [-0.30, -0.20) | 587 | 0 | 587 | 0 |
| [-0.20, -0.10) | 1,298 | 0 | 1,298 | 0 |
| [-0.10, 0.00) | 2,009 | 0 | 2,009 | 0 |
| [0.00, 0.10) | 1,291 | 0 | 1,291 | 0 |
| [0.10, 0.20) | 1,144 | 0 | 1,144 | 0 |
| [0.20, 0.30) | 579 | 0 | 579 | 0 |
| [0.30, 0.40) | 534 | 0 | 534 | 0 |
| [0.40, 0.50) | 626 | 0 | 626 | 0 |
| [0.50, 0.60) | 271 | 61 | 210 | 0 |
| [0.60, 0.70) | 122 | 61 | 61 | 0 |
| [0.70, 0.80) | 2 | 2 | 0 | 0 |

### 关键发现

1. **负向方向高度集中在 [-0.10, 0.00) 区间**：2,009 条记录（约 23.3%）的 `dominant_direction` 在 -0.1 ~ 0.0 之间，非常接近中性但没有跨过 Reduce 阈值。这是最常见的 bearish 但不触发 Reduce 的区域。
2. **没有记录跨到 -0.4 以下**：最 bearish 的区间 `[-0.40, -0.30)` 只有 144 条记录，且全部在 -0.4 以上。系统几乎没有遇到极度 bearish 的共识。
3. **BuyNow 集中在 [0.50, 0.70)**：124 个 BuyNow 全部来自 `dominant_direction >= 0.5` 的记录，说明 BuyNow 阈值（0.3）和实际分布一致。
4. **Reduce 阈值附近是问题区域**：144 条在 `[-0.40, -0.30)` 和 587 条在 `[-0.30, -0.20)` 的记录都接近或低于 Reduce 阈值，但没有任何一条输出 Reduce。这意味着即使方向足够负，其他条件（confidence、consensus、risk）在抑制 Reduce。

### 为什么跨阈值仍然 Wait？

查看 `execution-engine` 的 `DecisionEngine` 逻辑（顺序判断）：

```rust
1. if risk == Critical -> Wait
2. if risk == High -> Wait
3. if confidence < confidence_threshold -> Wait
4. if consensus < consensus_threshold -> Wait
5. if dominant_direction > buy_threshold -> BuyNow
6. if dominant_direction < reduce_threshold -> Reduce
7. else -> Wait
```

因此，即使 `dominant_direction < -0.3`（满足第 6 条），也可能因为前面的 `risk`、`confidence` 或 `consensus` 不满足而提前退出到 Wait。

从 Decision Margin 数据看，这是 **最主要的可能性**：系统在 bearish 方向上积累了足够证据，但 confidence 或 consensus 没有过门。

### 结论

**Reduce = 0 的根因不是 threshold 设置过高，而是系统在 bearish 方向上无法同时满足 confidence/consensus 门槛。**

- 如果降低 `reduce_threshold`（比如从 -0.3 降到 -0.2），会纳入更多 bearish 记录，但这些记录仍然可能因为 confidence/consensus 低而 Wait，不会增加 Reduce。
- 如果直接强制输出 Reduce，可能会因为 confidence 不足而引入错误 Reduce。
- 真正的方向是：**检查 bearish 证据的 confidence / consensus 聚合方式，以及 Prior（State）证据对 bearish 方向的抑制权重。**

### 后续行动

1. 抽样 152 条 `missed Reduce` 记录，输出它们的 `confidence`、`consensus`、`risk`、`coverage` 以及各证据的 confidence 和 direction。
2. 判断是：
   - `consensus` 太低（证据方向不统一）？
   - `confidence` 太低（证据本身置信度不够）？
   - `risk` 被评定为 High/Critical（虽然是 bearish 但风险被判断为不可交易）？
   - Prior（State）证据太强（DeRisk/NoTrade 的 bearish 权重压倒盘中观察）？
3. 在以上判断明确之前，不修改任何 threshold、weight 或 policy。
4. 此发现写入 `docs/v8/adr-095-decision-path-review.md`。

## 2A-4 综合结论

### 已验证的事实

| 问题 | 原假设 | 验证结果 |
|---|---|---|
| Distribution 为什么不产生？ | Observation 条件过严 | **否**。修复 `prev_close` 后条件转换率 100%，问题在于 `volume_ma20` 占位导致 `volume_ratio` 失真 |
| RiskExpansion 为什么不产生 Reduce？ | Threshold 过高 | **否**。有 91~152 条记录跨过了 Reduce 阈值，但 confidence/consensus 或 risk 门槛抑制了 Reduce |
| Observation 层是瓶颈？ | 可能 | **否**。Observation → Evidence → Assessment 转换率接近 100% |
| Decision 层是瓶颈？ | 可能 | **是**。大量 bearish Assessment 没有输出 Reduce |

### 下一步不是修改代码，而是继续分析

目前我们已经把问题缩小到 **Decision 层内部**。但 Decision 层内部有三个可能：

1. **confidence / consensus 门槛**：bearish 证据不够统一或不够强。
2. **risk 评估**：系统把 bearish 市场判断为 High/Critical Risk，从而抑制交易。
3. **Prior 权重**：StrategyState（DeRisk / NoTrade）的 bearish 方向太强，但 confidence 不高，导致整体 consensus 被压低。

在 2A-5 Calibration Proposal 之前，需要再增加一个 **Confidence/Consensus/Risk 分解 Review**，输出每个 `missed Reduce` 记录的这些指标，才能判断调哪里。

### 当前代码修改记录

| 文件 | 修改 | 原因 |
|---|---|---|
| `crates/app-service/src/execution_replay.rs` | `quote.prev_close` 从 `bar.close` 占位改为真实前收盘价 | 否则 `today_return` 恒为 0，所有盘中观察失真，Review 无法进行 |
| 其他文件 | 无 | 未修改 Observation、Evidence、Assessment、Decision 逻辑或任何 Policy/Threshold |

**未修复的已知占位**：`volume_ma20` 仍为 `1.0`，导致 `volume_ratio` 失真。需要在继续校准前修复。

## 2A-4.5 / 2A-4C: Decision Gate Analysis（2026-07-17）

使用命令：

```bash
cargo run -p quant-cli -- execution-decision-gate \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

样本：CN 2024-01-01 至 2025-06-30，共 8,616 条 `ExecutionResearchRecord`。

### Decision Gate Funnel

```
Bearish Assessment Candidates
dominant_direction < -0.300
152

  |
  +-- Risk Critical: 0
  |
  +-- Risk High: 54
  |
  +-- Confidence too low: 98
  |
  +-- Consensus too low: 0
  |
  +-- Passed all gates: 0
  |
  +-- Final Reduce: 0
```

### 汇总表

| Gate | Count | % of Candidates |
|------|------:|----------------:|
| Risk Critical | 0 | 0.0% |
| Risk High | 54 | 35.5% |
| Confidence too low | 98 | 64.5% |
| Consensus too low | 0 | 0.0% |
| Passed all gates | 0 | 0.0% |
| **Final Reduce** | 0 | 0.0% |

### 关键发现

1. **Confidence 是主要阻塞门**：64.5% 的 bearish 候选因 `confidence < 0.6` 被阻塞。
2. **Risk High 是次要阻塞门**：35.5% 的 bearish 候选因 `risk == High` 被阻塞。
3. **Consensus 从未阻塞**：所有候选的 `consensus >= 0.5`，说明 bearish 证据方向相对一致。
4. **没有候选通过所有门**：即使通过了 risk/confidence/consensus 检查，也会进入 `if dominant_direction < reduce_threshold -> Reduce` 分支，但当前数据表明所有候选都被前面的门拦截了。

### 抽查记录特征

前 50 条 `missed Reduce` 记录特征：

| 特征 | 观察 |
|---|---|
| StrategyState | 全部为 `NoTrade`（因为 State 按 scope 统一） |
| dominant_direction | -0.303 ~ -0.388 |
| confidence | 0.412 ~ 0.558，集中在 0.44~0.53 |
| consensus | 0.587 ~ 0.674，普遍高于 0.5 门槛 |
| risk | Medium 或 High |

### 结论

**Reduce = 0 的直接原因：Confidence 阈值 0.6 对 bearish 场景来说过高。**

- 大多数 bearish 候选的 confidence 在 0.45~0.55 之间，距离 0.6 只差 0.05~0.15。
- Consensus 不是问题，因为所有候选都通过了 0.5 的 consensus 门槛。
- Risk High 阻塞了 35.5% 的候选，这本身也提出了一个语义问题：为什么 High Risk 不直接对应 Reduce，而是 Wait？

### 对 Calibration 的启示

| 方向 | 可能性 | 依据 |
|---|---|---|
| 降低 `confidence_threshold` | 高 | 98 条记录因 confidence 低于 0.6 被阻塞；降低 0.05~0.1 可能释放大量 Reduce |
| 调整 Risk 语义 | 中 | 54 条记录因 `risk == High` 被阻塞；"High Risk" 当前意味着 "不交易" 而非 "减仓" |
| 调整 `reduce_threshold` | 低 | 已经有 152 条记录跨过 -0.3；再降低只会增加候选，但不会解决 confidence 阻塞 |
| 调整 `consensus_threshold` | 极低 | 没有任何候选被 consensus 阻塞 |

### 未修复占位说明

- `volume_ma20 = 1.0` 仍未修复，因此 `volume_ratio` 仍然失真。但本 Review 发现的主要阻塞门是 confidence，与 `volume_ratio` 无直接关系。继续按用户要求延后修复 `volume_ma20`。

### 后续行动

1. 在 `docs/v8/adr-096-decision-gate-analysis.md` 中记录本发现。
2. 进入 2A-4C **Risk Semantics Review**（用户建议）：专门分析那 54 条 `Risk High` 阻塞的记录，判断 "High Risk" 应该对应 "Wait" 还是 "Reduce"。
3. 在 Risk Semantics Review 完成后，才进入 2A-5 Calibration Proposal。
4. 在 Calibration Proposal 中，需要评估是否区分 buy/reduce 的 `confidence_threshold`（例如，buy 保持 0.6，reduce 降至 0.45），而不是单一阈值。
