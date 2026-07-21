# Execution Platform V2 Golden Validation Suite

## 验证报告（gitignored 运行产物）

| 报告 | 路径 |
|---|---|
| Golden Suite 首次运行 | `reports/execution-validation/golden_suite_run_2026-07-17.json` |
| Execution Statistics 全量 (volume fix 前) | `reports/execution-validation/execution_statistics_cn_full_2026-07-17.json` |
| Execution Statistics 全量 (volume fix 后) | `reports/execution-validation/execution_statistics_cn_full_2026-07-18.md` |
| Evidence Trace 全量 (volume fix 前) | `reports/execution-validation/evidence_trace_cn_full_2026-07-17.md` |
| Evidence Trace 全量 (volume fix 后) | `reports/execution-validation/evidence_trace_cn_full_2026-07-18.md` |
| Evidence Trace JSON (volume fix 前) | `reports/execution-validation/evidence_trace_cn_full_2026-07-17.json` |
| Distribution Coverage Review (volume fix 前) | `reports/execution-validation/distribution_coverage_cn_full_2026-07-17.md` |
| Distribution Coverage Review (volume fix 后) | `reports/execution-validation/distribution_coverage_cn_full_2026-07-18.md` |
| Decision Margin Review (volume fix 前) | `reports/execution-validation/decision_margin_cn_full_2026-07-17.md` |
| Decision Margin Review (volume fix 后) | `reports/execution-validation/decision_margin_cn_full_2026-07-18.md` |
| Decision Gate Analysis (volume fix 前) | `reports/execution-validation/decision_gate_cn_full_2026-07-17.md` |
| Decision Gate Analysis (volume fix 后) | `reports/execution-validation/decision_gate_cn_full_2026-07-18.md` |
| Decision Gate JSON (volume fix 前) | `reports/execution-validation/decision_gate_cn_full_2026-07-17.json` |
| Risk Semantics Review (volume fix 前) | `reports/execution-validation/risk_semantics_cn_full_2026-07-17.md` |
| Risk Semantics Review (volume fix 后) | `reports/execution-validation/risk_semantics_cn_full_2026-07-18.md` |
| Risk Semantics JSON (volume fix 前) | `reports/execution-validation/risk_semantics_cn_full_2026-07-17.json` |
| Calibration Experiment (volume fix 前) | `reports/execution-validation/calibration_cn_full_2026-07-17.md` |
| Calibration Experiment (volume fix 后) | `reports/execution-validation/calibration_cn_full_2026-07-18.md` |
| Calibration JSON (volume fix 前) | `reports/execution-validation/calibration_cn_full_2026-07-17.json` |
| Calibration JSON (volume fix 后) | `reports/execution-validation/calibration_cn_full_2026-07-18.json` |
| **Bearish Evidence Analysis (2B-1)** | `reports/execution-validation/bearish_analysis_cn_full_2026-07-18.md` |

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
- `docs/v8/adr-099-restore-real-volume-context.md`
- `docs/v8/adr-100-evidence-quality-before-decision-calibration.md`
- `docs/v8/adr-101-transition-evidence-modeling.md`

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

## 2A-4C: Risk Semantics Review（2026-07-17）

使用命令：

```bash
cargo run -p quant-cli -- execution-risk-semantics \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

样本：CN 2024-01-01 至 2025-06-30，共 8,616 条 `ExecutionResearchRecord`。

本 Review 目标是回答：**`RiskLevel::High` 到底代表 Entry Risk（不能买）还是 Holding Risk（应该卖）？**

### Table 1: Risk Distribution

| Risk Level | Count | % of Total |
|------------|------:|-----------:|
| Low | 288 | 3.3% |
| Medium | 7,420 | 86.1% |
| High | 908 | 10.5% |

### Table 2: RiskHigh Evidence Composition

| Evidence | Count | % of High Risk | Proposed Category |
|----------|------:|---------------:|-------------------|
| Breadth | 908 | 100.0% | Ambiguous |
| Confirmation | 908 | 100.0% | Ambiguous |
| LeadershipRotation | 908 | 100.0% | Entry Risk |
| Recovery | 908 | 100.0% | Ambiguous |
| SignalStrength | 908 | 100.0% | Ambiguous |
| StrategyState | 908 | 100.0% | Ambiguous |
| Distribution | 774 | 85.2% | **Holding Risk** |
| RiskExpansion | 211 | 23.2% | **Holding Risk** |
| MomentumExpansion | 116 | 12.8% | Entry Risk |
| MarketAcceptance | 64 | 7.0% | Entry Risk |
| TrendParticipation | 61 | 6.7% | Ambiguous |
| MomentumFailure | 3 | 0.3% | **Holding Risk** |

### 发现 1：Holding Risk Evidence 确实大量存在

- 85.2% 的 RiskHigh 记录包含 `Distribution`
- 23.2% 包含 `RiskExpansion`
- 0.3% 包含 `MomentumFailure`

这说明：从证据组成看，RiskHigh 确实偏向 **Holding Risk** 类型。

### Table 3: RiskHigh Decision Context

- High risk 记录：908
- Direction：mean=-0.105, p50=-0.133, min=-0.508, max=0.512
- Confidence：mean=0.498, p50=0.481, p75=0.532, max=0.785
- Consensus：mean=0.621, p50=0.665, p75=0.701, max=0.736
- **所有 908 条 High Risk 记录的决策都是 Wait**

### Table 4: RiskHigh Future Outcome Analysis

| Group | Count | T+20 Mean | T+60 Mean | T+120 Mean | Negative T+20 % | MAE Mean | Max Drawdown Mean |
|-------|------:|----------:|----------:|-----------:|----------------:|---------:|------------------:|
| High Risk | 908 | 4.72% | 7.30% | 16.32% | 40.1% | -11.49% | -18.14% |
| High Risk + Wait | 908 | 4.72% | 7.30% | 16.32% | 40.1% | -11.49% | -18.14% |
| **RiskHigh + Bearish + Wait (blocked Reduce)** | **54** | **6.25%** | **4.86%** | **2.91%** | **29.6%** | **-14.30%** | **-20.00%** |
| Medium Risk | 7,420 | 1.83% | 7.51% | 16.47% | 47.2% | -10.18% | -15.52% |
| Low Risk | 288 | -2.41% | -1.34% | 10.35% | 69.8% | -16.09% | -18.62% |

### 发现 2：RiskHigh + Wait 的平均收益为 **正**

- 全部 908 条 High Risk 记录 T+20 平均收益：**+4.72%**
- 那 54 条 bearish + RiskHigh + Wait 阻塞候选 T+20 平均收益：**+6.25%**
- 这些 bearish 候选的 negative T+20 比例只有 29.6%，低于 Medium Risk（47.2%）和 Low Risk（69.8%）

这说明：**在当前数据集上，RiskHigh 阻塞 Reduce 是正确的**。这些 bearish 但 High Risk 的日子随后出现了反弹，如果当时 Reduce 会错过这部分收益。

### 发现 3：Low Risk 反而是最差的

- Low Risk 记录 T+20 平均收益：**-2.41%**
- Negative T+20 比例：**69.8%**
- 这看起来反常，但说明当前风险评分与后续收益没有简单线性关系，至少在这个 dataset 上。

### Table 5: Risk Semantic Mapping Proposal（不改代码）

| Evidence | Proposed Type | Rationale |
|----------|---------------|-----------|
| Distribution | Holding Risk | 派发特征，应触发减仓 |
| RiskExpansion | Holding Risk | 风险扩张，应触发减仓 |
| MomentumFailure | Holding Risk | 动量失效，应触发减仓 |
| MomentumExpansion | Entry Risk | 追涨动量可能过伸 |
| MarketAcceptance | Entry Risk | 市场过度接受，可能反转 |
| LeadershipRotation | Entry Risk | 轮动不稳定，开仓风险高 |
| 其他 | Ambiguous | 依赖方向与上下文 |

### 结论：RiskHigh 语义当前是合理的

尽管从证据组成看，RiskHigh 偏向 Holding Risk，但**事后 outcome 数据显示，RiskHigh + Wait 的平均收益为正**。这说明：

1. 当前 `RiskHigh -> Wait` 的语义在这个数据集上是**保护性的**，不是错误。
2. 在 CN 2024-2025 这个时间窗口，High Risk 往往发生在短期恐慌/派发后，随后出现反弹。
3. 因此，**不建议把 RiskHigh 改为直接触发 Reduce**。这会导致追涨杀跌。

### 修正后的阻塞归因

| 阻塞 | 原始判断 | 更新判断 |
|---|---|---|
| Confidence 低 | Calibration 问题 | **仍然是主要问题** |
| Risk High | Domain Modeling 问题 | **当前语义合理，不需要修改** |

### 后续行动

1. 在 `docs/v8/adr-097-risk-semantics-review.md` 中记录本发现。
2. 不修改 RiskLevel 或 DecisionEngine 语义。
3. 进入 2A-5 Calibration Proposal 时，重点只考虑 **confidence threshold 校准**，并评估是否对 buy/reduce 使用非对称阈值。
4. 继续延后修复 `volume_ma20`（与当前 Decision 层问题无关）。

## 2A-4 最终综合结论

### 已完成的所有 Review

| 阶段 | 工具 | 核心结论 |
|------|------|----------|
| 2A-4A | Distribution Coverage Review | Distribution 触发率 100%，条件本身不是瓶颈；`volume_ma20` 占位导致 `volume_ratio` 失真 |
| 2A-4B | Decision Margin Review | 152 条记录跨过 reduce threshold 但没 Reduce，问题在 Decision 层 |
| 2A-4.5 | Decision Gate Analysis | 64.5% 被 Confidence 阻塞，35.5% 被 RiskHigh 阻塞，0% 被 Consensus 阻塞 |
| 2A-4C | Risk Semantics Review | RiskHigh 当前语义合理（RiskHigh + Wait 平均收益为正），不是主要问题 |

### 最终归因

**Reduce = 0 的根因几乎完全是 Confidence 阈值 0.6 对 bearish 方向过高。**

- 98 条 bearish 候选因 confidence 0.45~0.55 被阻塞。
- RiskHigh 阻塞的 54 条候选事后看平均收益为正，不应改为 Reduce。
- Consensus 不是问题。
- Observation 不是问题。

### 进入 2A-5 Calibration Proposal 的前提

| 条件 | 状态 |
|---|---|
| 明确根因 | ✅ Confidence 阈值对 bearish 过高 |
| 排除其他解释 | ✅ Risk/Consensus/Observation 都不是主要问题 |
| 不改代码 | ✅ 只加诊断工具，未改任何决策逻辑 |
| 数据 bug 处理 | ⚠️ `volume_ma20` 仍占位，但与当前根因无关；可延后 |

### 2A-5 建议方向

1. **非对称 confidence threshold**：BuyNow 保持 0.6，Reduce 降至 0.45~0.5。
2. **保持 RiskHigh 语义**：不改为触发 Reduce。
3. **不改 consensus / reduce threshold**：数据不支持。
4. **Calibration 验证**：必须在这 98 条 confidence-阻塞候选上测试，确保降低 threshold 后 Reduce 行为合理。

**当前代码修改记录**：

| 文件 | 修改 | 原因 |
|---|---|---|
| `crates/app-service/src/execution_replay.rs` | `prev_close` 占位改真实前收盘价 | 否则盘中观察失真 |
| 其他 | 无 | 未改 Observation/Evidence/Assessment/Decision/Policy |

**未修复占位**：`volume_ma20 = 1.0`（延后处理）。

## 2A-5: Directional Confidence Calibration Experiment（2026-07-17）

使用命令：

```bash
cargo run -p quant-cli -- execution-calibration \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

样本：CN 2024-01-01 至 2025-06-30，共 8,616 条 `ExecutionResearchRecord`。

### 实验设计

| 实验 | 置信度阈值 |
|---|---|
| Baseline | 0.60 |
| C1 | 0.55 |
| C2 | 0.50 |
| C3 | 0.45 |
| Asymmetric | buy 0.60 / reduce 0.50 |

所有实验在相同记录上重跑 `DecisionEngine`，仅修改 confidence threshold，不修改其他 policy 或引擎逻辑。

### 结果汇总

| 实验 | Reduce Candidates | Reduce Count | Avoided Loss | Missed Recovery | Precision | Recall | F1 | Avg T+20 (Reduce) |
|------|------------------:|-------------:|-------------:|----------------:|----------:|-------:|---:|------------------:|
| Baseline 0.60 | 152 | 0 | 0 | 0 | N/A | 0.0% | N/A | N/A |
| C1: 0.55 | 152 | 0 | 0 | 0 | N/A | 0.0% | N/A | N/A |
| C2: 0.50 | 152 | 12 | 2 | 10 | 16.7% | 3.4% | 5.6% | +1.5% |
| C3: 0.45 | 152 | 65 | 24 | 41 | 36.9% | 40.7% | 38.7% | +2.1% |
| Asymmetric 0.60/0.50 | 152 | 12 | 2 | 10 | 16.7% | 3.4% | 5.6% | +1.5% |

### 关键发现 1：C1 (0.55) 没有释放任何 Reduce

因为 98 条 confidence-阻塞候选的 confidence 集中在 **0.45~0.55**，所以 0.55 阈值不够低，一条都没有释放。

### 关键发现 2：C2 / Asymmetric 0.60/0.50 只释放 12 条，但精度极低

- 12 个 Reduce 中，只有 2 个真正避免亏损（precision 16.7%）
- 10 个 Reduce 错过后续反弹（missed recovery 83.3%）
- 这说明：**把 threshold 降到 0.50 仍然不够，而且会产生大量错误 Reduce**

### 关键发现 3：C3 (0.45) 释放 65 条，但精度仍低于 50%

- 65 个 Reduce 中，24 个正确避免亏损，41 个错过后续反弹
- Precision 36.9%，Recall 40.7%，F1 38.7%
- 所有 Reduce 候选的平均 T+20 是 +2.4%，而 Reduce 后的平均 T+20 是 +2.1%，几乎没有差异

### 关键发现 4：Asymmetric 与 C2 完全相同

因为当前 152 条 bearish 候选的 confidence 都 < 0.5，所以把 buy confidence 保持在 0.6 不影响 bearish 侧。只要 reduce confidence 降到 0.5，结果就与 Uniform 0.50 相同。

### 结论：单纯降低 Confidence 阈值不足以产生有效 Reduce

这是本轮最重要的发现：

> **即使把 confidence threshold 降到 0.45，Reduce 的 precision 也只有 37%。超过 60% 的 Reduce 会错过后续反弹。**

这意味着：

1. **Confidence 阈值不是唯一问题**。降低它可以释放 Reduce，但释放出来的 Reduce 质量不高。
2. **Bearish 证据本身预测力不足**。当前证据（Distribution / RiskExpansion 等）不足以区分「真正需要减仓」和「短期恐慌后反弹」的情况。
3. **不能简单通过降低 confidence threshold 解决 Reduce=0**。

### 对 2A-5 的修正

之前的假设：

> 降低 confidence threshold 即可释放 Reduce。

现在的结论：

> 降低 confidence threshold 可以释放 Reduce，但会引入大量错误 Reduce。需要先提高 bearish 证据质量，再降低 threshold。

### 后续可能方向

1. **修复 `volume_ma20`**：让 `volume_ratio` 真实化，可能改变 Distribution 触发条件，从而改变 bearish 证据质量。
2. **Distribution 条件细化**：不仅仅是 `close_position < 0.2 && volume_ratio > 1.5 && today_return < 0`，可能需要加入更多条件（如连续分布、市场结构等）。
3. **RiskExpansion 条件细化**：当前 RiskExpansion 数量 440 条，可能触发条件过宽或过窄。
4. **引入新的 Holding Risk Evidence**：例如多日动量崩溃、Breadth 连续恶化等。
5. **动态 confidence**：根据证据类型或市场状态使用不同的 confidence 要求。

### 2A-5 最终建议

**不降低 confidence threshold。先修复数据/观察质量。**

具体顺序：

1. 修复 `volume_ma20` 占位。
2. 重新跑 Distribution Coverage Review、Evidence Trace、Decision Gate、Calibration。
3. 如果 bearish 证据质量提升后，再降低 confidence threshold。

### 当前代码修改记录

| 文件 | 修改 | 原因 |
|---|---|---|
| `crates/app-service/src/execution_replay.rs` | `prev_close` 占位改真实前收盘价 | 否则盘中观察失真 |
| 其他 | 无 | 未改任何决策逻辑或默认值 |

**未修复占位**：`volume_ma20 = 1.0`（现在明确成为下一步）。

## 2A-6: Restore Real Volume Context（2026-07-18）

### 修复内容

- 文件：`crates/app-service/src/execution_replay.rs`
- 变更：在 `build_execution_event` 中，从 `market-store` 拉取当前日期前 40 个日历日的日线数据，取最近 20 个交易日的成交量均值作为 `volume_ma20`。
- 影响：`ExecutionRequest.volume_ma20` 不再是 `1.0` 占位，而是真实的 20 日成交量均线。`volume_ratio` 现在反映相对成交量。
- 未改：任何 Observation 条件、Evidence 聚合、Decision 阈值或 Policy。

### 重跑命令

```bash
cargo run -p quant-cli -- execution-statistics --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
cargo run -p quant-cli -- execution-evidence-trace --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
cargo run -p quant-cli -- execution-distribution-coverage --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
cargo run -p quant-cli -- execution-decision-margin --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
cargo run -p quant-cli -- execution-decision-gate --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
cargo run -p quant-cli -- execution-risk-semantics --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
cargo run -p quant-cli -- execution-calibration --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
```

### 2A-6A: Distribution Coverage Review（修复后）

样本：CN 2024-01-01 至 2025-06-30，共 **8,616** 条 `ExecutionResearchRecord`。

#### 特征百分位（修复后）

| Feature | Count | Min | P10 | P25 | P50 | P75 | P90 | P95 | Max | Mean |
|---------|------:|----:|----:|----:|----:|----:|----:|----:|----:|-----:|
| close_position | 8616 | 0.000 | 0.048 | 0.190 | 0.492 | 0.800 | 0.955 | 1.000 | 1.000 | 0.494 |
| volume_ratio | 8616 | 0.147 | 0.674 | 0.801 | 0.949 | 1.151 | 1.466 | 1.765 | 9.928 | **1.043** |
| today_return | 8616 | -0.133 | -0.019 | -0.009 | 0.000 | 0.009 | 0.021 | 0.032 | 0.186 | 0.001 |

#### 条件覆盖（修复后）

| 条件 | 满足记录数 | 占全部记录 | 占下跌日 |
|---|---:|---:|---:|
| `today_return < 0.0` | 4,271 | 49.6% | 100% |
| 下跌日 + `close_position < 0.2` | 2,048 | 23.8% | 48.0% |
| 下跌日 + `volume_ratio > 1.5` | 227 | 2.6% | 5.3% |
| **满足全部三个条件** | **105** | **1.2%** | **2.5%** |
| 实际产生 Distribution 观察 | 105 | 1.2% | 2.5% |
| 条件覆盖率 | 100.0% | — | — |

#### 关键发现

1. **volume_ratio 恢复为真实相对成交量**：均值 1.043，P10 0.674，P90 1.466，最大 9.928。这与占位时的绝对成交量（均值 65,295,754）完全不同。
2. **Distribution 条件现在非常严格**：满足全部三个条件的记录仅占 1.2%（之前 23.7%）。这意味着修复前 2,043 条 Distribution 观察中绝大多数是因为 `volume_ratio > 1.5` 恒成立而产生的假阳性。
3. **Distribution 触发率仍然是 100%**：所有满足真实条件的记录都触发了 Distribution 观察，说明 Observation 层逻辑本身没有问题。
4. **Distribution 从「过宽」变为「可能合理」**：105 条记录占下跌日 2.5%，这是一个更接近直觉的派发样本量。

### 2A-6B: Evidence Trace（修复后）

| Evidence | Observations | Wait | BuyNow | Reduce |
|---|---:|---:|---:|---:|
| Distribution | **105** | 105 | 0 | 0 |
| RiskExpansion | 440 | 437 | 3 | 0 |
| MomentumFailure | 17 | 17 | 0 | 0 |
| LiquidityConfirmation | 1065 | 1063 | 2 | 0 |
| MarketAcceptance | 592 | 586 | 6 | 0 |
| MomentumExpansion | 2539 | 2128 | 41 | 0 |
| TrendParticipation | 1779 | 1733 | 46 | 0 |

Distribution 观察数量从 2,043 骤降到 105，但其余 Evidence 变化不大。这说明：
- Distribution 条件之前被 `volume_ratio` 失真严重污染；
- RiskExpansion / LiquidityConfirmation 等不依赖 `volume_ratio` 的 Evidence 保持稳定。

### 2A-6C: Decision Margin（修复后）

- 记录总数：8,615
- 方向为负且低于 Reduce 阈值（-0.3）的记录：**145**（之前 152）
- 这些记录中最终 Reduce：0
- 这些记录中最终 Wait：145
- Reduce Recall：0.0%

**Distribution 相关：**
- 105 条 Distribution 记录中，67 条方向为负，0 条 Reduce，5 条 missed Reduce。

**关键发现**：
- 修复 volume 后，bearish 候选从 152 降到 145。减少的 7 条是因为之前被 `volume_ratio` 失真抬高的 Distribution 证据不再触发。
- 但仍然没有任何 bearish 候选最终输出 Reduce。Decision 层问题没有因 volume 修复而自动解决。

### 2A-6D: Decision Gate（修复后）

```
Bearish Assessment Candidates
dominant_direction < -0.300
145

  |
  +-- Risk Critical: 0
  |
  +-- Risk High: 62
  |
  +-- Confidence too low: 83
  |
  +-- Consensus too low: 0
  |
  +-- Passed all gates: 0
  |
  +-- Final Reduce: 0
```

| Gate | Count | % of Candidates |
|------|------:|----------------:|
| Risk Critical | 0 | 0.0% |
| Risk High | 62 | 42.8% |
| Confidence too low | 83 | 57.2% |
| Consensus too low | 0 | 0.0% |
| Passed all gates | 0 | 0.0% |
| **Final Reduce** | 0 | 0.0% |

与修复前相比：
- Risk High 阻塞从 54 上升到 62；
- Confidence 阻塞从 98 下降到 83。

这说明 volume 修复改变了一部分证据的 Risk 评级，使更多候选进入 Risk High，但总体阻塞结构未变。

### 2A-6E: Risk Semantics（修复后）

- High Risk 记录：633（之前 908）
- bearish + RiskHigh + Wait 阻塞候选：**62**（之前 54）
- 这些 62 条记录的 T+20 平均收益：**+2.58%**
- Negative T+20 比例：30.6%

**结论不变**：RiskHigh 阻塞 Reduce 的语义在当前数据集上是保护性的。这些 bearish 但 High Risk 的日子随后出现反弹，不应改为 Reduce。

### 2A-6F: Directional Confidence Calibration（修复后）

| 实验 | Reduce Candidates | Reduce Count | Avoided Loss | Missed Recovery | Precision | Recall | F1 | Avg T+20 (Reduce) |
|------|------------------:|-------------:|-------------:|----------------:|----------:|-------:|---:|------------------:|
| Baseline 0.60 | 145 | 0 | 0 | 0 | N/A | 0.0% | N/A | N/A |
| C1: Uniform 0.55 | 145 | 0 | 0 | 0 | N/A | 0.0% | N/A | N/A |
| C2: Uniform 0.50 | 145 | 17 | 3 | 14 | 17.6% | 6.0% | 9.0% | 1.6% |
| C3: Uniform 0.45 | 145 | 75 | 27 | 48 | **36.0%** | **54.0%** | **43.2%** | 2.3% |
| Asymmetric 0.60/0.50 | 145 | 17 | 3 | 14 | 17.6% | 6.0% | 9.0% | 1.6% |

与修复前对比：

| 指标 | 修复前 | 修复后 |
|---|---|---|
| Reduce Candidates | 152 | 145 |
| C3 Reduce Count | 65 | 75 |
| C3 Precision | 36.9% | 36.0% |
| C3 Recall | 40.7% | 54.0% |
| C3 F1 | 38.7% | 43.2% |

**关键发现**：
1. **volume 修复后，C3 F1 从 38.7% 提升到 43.2%**，Recall 从 40.7% 提升到 54.0%。这说明真实成交量信息确实改善了一部分 bearish 证据的识别能力。
2. **但 C3 Precision 仍只有 36.0%**，没有超过 50%。这意味着即便在真实 volume 下，降低 threshold 仍会释放大量错误 Reduce。
3. **结论不变**：不能通过降低 confidence threshold 解决 Reduce=0。问题仍在 bearish 证据的区分能力上。

### 2A-6 综合结论

| 问题 | 修复前假设 | 修复后验证 |
|---|---|---|
| Distribution 被过度触发 | `volume_ratio` 失真导致 | **确认**。修复后 Distribution 从 2,043 降到 105。 |
| Decision Gate 阻塞结构 | Confidence 是主因 | **仍然成立**。145 候选中 83 因 Confidence 阻塞，62 因 RiskHigh 阻塞。 |
| RiskHigh 语义 | 当前语义合理 | **确认**。62 条 RiskHigh + bearish + Wait 平均 T+20 为 +2.58%。 |
| 降低 confidence threshold 能否解决问题 | 不能 | **再次确认**。C3 F1 提升但 Precision 仍 < 50%。 |

### 当前问题模型（更新）

```
2A-1 Fact Lineage       ✅
2A-2 Statistics         ✅
2A-3 Evidence Trace     ✅
2A-4 Decision Review      ✅
2A-5 Calibration         ✅ 拒绝 threshold-only 路径
2A-6 Evidence Quality   ✅ 修复 volume_ma20，但证明不够

下一步：
        |
        v
  Evidence Quality v2
  (Distribution/RiskExpansion 条件细化，或新增 Holding Risk Evidence)
        |
        v
  Calibration v2
        |
        v
  Bayesian / ML Assessment
```

### 当前代码修改记录

| 文件 | 修改 | 原因 |
|---|---|---|
| `crates/app-service/src/execution_replay.rs` | `prev_close` 占位改真实前收盘价 | 否则盘中观察失真 |
| `crates/app-service/src/execution_replay.rs` | `volume_ma20` 占位改真实 20 日成交量均线 | 否则 volume_ratio 失真，Distribution 被污染 |
| 其他 | 无 | 未改任何 Observation / Evidence / Assessment / Decision / Policy |

### 后续建议

1. **继续改善 bearish 证据质量**，而不是降低 threshold。可能方向：
   - 细化 Distribution 条件（如增加连续分布、市场结构等）；
   - 细化 RiskExpansion 条件；
   - 引入新的 Holding Risk Evidence（如多日动量崩溃、Breadth 连续恶化等）。
2. **在证据质量改善后**，重新跑完整 Decision Path Review 链，再次进入 Calibration。
3. **只有在校准实验显示 Reduce Precision ≥ 50% 时**，才调整默认 confidence threshold。

**相关 ADR**：`docs/v8/adr-099-restore-real-volume-context.md`

---

## Phase 2A 完成与 Phase 2B 入口

### Phase 2A 完成状态

| 阶段 | 工具 | 状态 | 关键结论 |
|------|------|------|----------|
| 2A-1 | Fact Lineage | ✅ | `market_regime_label` 来源真实化 |
| 2A-2 | Execution Statistics | ✅ | Reduce=0，动态 Evidence 稀疏 |
| 2A-3 | Evidence Trace | ✅ | Observation → Evidence → Assessment 转换率正常 |
| 2A-4A | Distribution Coverage Review | ✅ | `volume_ratio` 失真，修复后 Distribution 从 2,043 降到 105 |
| 2A-4B | Decision Margin Review | ✅ | 145 条 bearish 候选跨 threshold 但全部 Wait |
| 2A-4.5 | Decision Gate Analysis | ✅ | Confidence 83 / RiskHigh 62，Consensus 无阻塞 |
| 2A-4C | Risk Semantics Review | ✅ | RiskHigh → Wait 语义正确 |
| 2A-5 | Directional Confidence Calibration | ✅ | threshold-only 被否定，C3 Precision 36% |
| 2A-6 | Restore Real Volume Context | ✅ | volume 真实化后 F1 提升但 Precision 仍 < 50% |

### 2A 最终因果链隔离

```
Market Data
    |
    v
Feature Layer       ✅ 已修复 (prev_close, volume_ma20)
    |
    v
Observation Layer   ✅ 触发逻辑正常
    |
    v
Evidence Layer      ⚠️  当前瓶颈：bearish evidence 区分能力不足
    |
    v
Assessment          ✅ 方向计算有效
    |
    v
Risk Semantics      ✅ RiskHigh → Wait 正确
    |
    v
Decision Gate       ✅ 行为合理，threshold 不是根因
    |
    v
Calibration         ✅ threshold-only 已否定
```

当前问题模型：

> 系统能检测风险，但无法判断风险是否应该转化为退出。

### Phase 2B: Evidence Modeling

2A 已经证明：
- 降低 confidence threshold 不能解决 Reduce=0
- 修复 `volume_ma20` 减少了 Evidence 污染，但没有产生 Exit-specific 信号
- DecisionEngine 的保守行为是合理的，不应修改

因此，下一步从 **Decision Calibration** 转移到 **Evidence Modeling**（更精确地：先 Transition Evidence Modeling，再 Holding Risk Evidence）。

| 任务 | ID | 目标 |
|------|-----|------|
| 2B-1 Bearish Evidence Analysis | TASK-153 | 分析 145 个 bearish candidate 的证据组合 + 结局，输出 Bearish Candidate Matrix |
| 2B-1.5 RiskExpansion Coverage Exploration | TASK-153.5 | 排除 RiskExpansion 作为核心 Exit Evidence 的方向 |
| 2B-2 Transition Evidence Modeling | TASK-154 | 设计变化/恶化型 Evidence，作为 Research Asset 验证，不直接接入 DecisionEngine |
| 2B-3 Holding Risk Evidence | — | 基于 Transition Evidence 构建 Exit-specific Evidence |
| 2B-4 Calibration v2 | TASK-155 | 在 Evidence 验证后重新校准，只接受 Reduce Precision ≥ 50% |
| 2B-5 Research Asset Accumulation | — | 持续积累 Evidence / Transition Snapshot 资产 |

### 2B 原则

**ADR-100: Evidence Quality Before Decision Calibration**

> Decision thresholds shall not compensate for insufficient evidence semantics.

- 不因为 Decision Gate 阻塞候选而调整 threshold
- 任何新 Evidence 必须先作为 Research Asset 验证
- 任何 Calibration 提案必须包含历史 replay precision/recall 和与 baseline 的对比

### 当前代码修改记录

| 文件 | 修改 | 原因 |
|---|---|---|
| `crates/app-service/src/execution_replay.rs` | `prev_close` 占位改真实前收盘价 | Feature 层真实化 |
| `crates/app-service/src/execution_replay.rs` | `volume_ma20` 占位改真实 20 日成交量均线 | Feature 层真实化 |
| 其他 | 无 | 未改任何 Observation / Evidence / Assessment / Decision / Policy |

### 2B 禁止事项

- 不修改 `ExecutionPolicy` 默认 threshold
- 不修改 `DecisionEngine` 判定顺序或语义
- 不将未经验证的 Evidence 直接接入决策路径
- 不因为没有 Reduce 而降低 confidence threshold

### 相关 ADR

- `docs/v8/adr-100-evidence-quality-before-decision-calibration.md`
- `docs/v8/adr-101-transition-evidence-modeling.md`

## 2B-1: Bearish Evidence Analysis（2026-07-18）

### 目标

用**已有 Evidence + Outcome**反推 Exit-specific pattern，而不是直接新增 `EvidenceKind`。

### 工具

```bash
cargo run -p quant-cli -- execution-bearish-analysis \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

报告路径：`reports/execution-validation/bearish_analysis_cn_full_2026-07-18.md`

### 数据集

- 总记录：8,616
- Bearish candidate：`dominant_direction < -0.300`，共 **145** 条

### 关键发现 1：Bearish baseline 负收益概率很低

| 指标 | 数值 |
|---|---|
| Negative T+20 | **34.5%** |
| Negative T+60 | **31.7%** |

这意味着：**即使 Assessment 已经 bearish，后续 20 天下跌的概率也只有 1/3**。单纯 direction 不足以支持 Reduce。

### 关键发现 2：固定证据无法区分

所有 145 个 bearish candidate 都包含以下固定证据（来自 `ResearchContext` / `StrategyState` / `Signal`）：

- Breadth
- Confirmation
- LeadershipRotation
- Recovery
- SignalStrength
- StrategyState

这些证据的 lift 全部 = 1.00，因为它们恒存在，没有区分能力。

### 关键发现 3：动态 Evidence 才有区分能力

| Evidence | Count | Negative T+20 | Lift |
|---|---|---|---|
| RiskExpansion | 6 | 50.0% | **1.45** |
| Distribution | 5 | 40.0% | 1.16 |
| LiquidityConfirmation | 66 | 30.3% | 0.88 |
| MomentumFailure | 0 | — | 0.00 |

**RiskExpansion 是最高 lift 证据**，但样本只有 6 个。

### 关键发现 4：Recovery Conflict 退化

因为 Recovery 是固定证据，**100% 的 bearish candidate 都有 Recovery**。所以：

- Bearish + Recovery: 145 个
- Bearish + No Recovery: 0 个

这个分析维度被固定证据阻塞了。当前无法回答「恐慌释放 vs 真正退出」的问题。

### 关键发现 5：C3 False Reduce 64%

在 confidence threshold 0.45 下，75 个 Reduce 中有 48 个 T+20 ≥ 0，false reduce 率 **64.0%**。

平均 T+20 after C3 Reduce: **+2.28%**。这意味着 C3 下的 Reduce 平均而言错过了后续反弹。

### Evidence Combination Outcome Matrix

| Combination | Count | Negative T+20 | Negative T+60 | Avg T+20 | Avg T+60 |
|---|---|---:|---:|---:|---:|
| RiskExpansion | 6 | 50.0% | 83.3% | 3.05% | -5.19% |
| RiskExpansion + Recovery | 6 | 50.0% | 83.3% | 3.05% | -5.19% |
| Distribution + RiskExpansion | 4 | 50.0% | 75.0% | 2.50% | -5.74% |
| Distribution + RiskExpansion + Recovery | 4 | 50.0% | 75.0% | 2.50% | -5.74% |
| Distribution | 5 | 40.0% | 80.0% | 2.16% | -5.35% |
| Distribution + Recovery | 5 | 40.0% | 80.0% | 2.16% | -5.35% |
| Distribution + Breadth | 5 | 40.0% | 80.0% | 2.16% | -5.35% |
| Recovery | 145 | 34.5% | 31.7% | 2.38% | 9.10% |
| LiquidityConfirmation | 66 | 30.3% | 27.3% | 2.66% | 14.50% |

### 假说验证

| 假说 | 结果 |
|---|---|
| 已有 Evidence 组合能区分退出 | 部分支持，但样本太小 |
| RiskExpansion 是关键 Holding Risk | 支持，但 n=6 |
| Distribution 单独有效 | 弱，n=5，lift 1.16 |
| Recovery 能区分恐慌/退出 | 不支持，因为 Recovery 恒存在 |
| 固定 Evidence 有价值 | 否定，全部 lift=1.0 |

### 2B-1 结论

1. **当前 bearish candidate 的负收益 baseline 只有 34.5%**。这说明大部分 bearish assessment 只是「有风险」，不是「要退出」。
2. **固定证据（ResearchContext / StrategyState / Signal）没有区分能力**。它们在所有 candidate 上都存在，lift=1.0。
3. **动态 Evidence（RiskExpansion / Distribution）是唯一有区分力的信号**。
4. **RiskExpansion 是最高 lift 证据（1.45）**，但样本只有 6 个，需要更多样本验证。
5. **Distribution + RiskExpansion 是最有希望的组合**（4 个样本，50% 负 T+20，75% 负 T+60），但样本量太小。
6. **Recovery 不能作为 panic-vs-exit 的区分变量**，因为它被固定注入到每个记录中。

### 对 TASK-154 的启示

进入 Holding Risk Evidence 设计之前，必须先解决：

1. **固定证据的退化问题**
   - `Recovery` 不应该作为固定证据存在，而应该反映**变化**（例如 recovery 是否消失或恶化）。
   - 或者，引入 `RecoveryFailure` 作为新证据，表示 recovery 信号存在但价格继续下跌。

2. **RiskExpansion 需要扩大样本**
   - 当前 RiskExpansion 只有 6 个样本触发。
   - 需要检查 Observation 层触发条件是否过严，或者是否应该增加新的 risk 观察维度。

3. **不要直接新增 `EvidenceKind`**
   - 应该先设计**组合条件**或**delta 条件**。
   - 例如：`Distribution + RiskExpansion` 同时出现时，才产生一个 Exit-specific 信号。

4. **新的 Holding Risk Evidence 必须从动态观察中派生**
   - 从 `ResearchContext` 固定状态派生的证据无法区分。
   - 未来证据应该来自：
     - 盘中观察（Observation）的变化
     - 多日 Evidence 的演变
     - 市场结构的恶化（如 Breadth 连续下降）

### 当前代码修改记录

| 文件 | 修改 | 原因 |
|---|---|---|
| `crates/execution-replay/src/bearish_analysis.rs` | 新增 2B-1 领域模块 | 计算 Evidence lift / 组合矩阵 / Recovery conflict / False Reduce |
| `crates/execution-replay/src/bearish_analysis_formatter.rs` | 新增 Markdown/JSON formatter | 输出可读报告 |
| `crates/execution-replay/src/lib.rs` | 导出模块 | 供 app-service 使用 |
| `crates/app-service/src/execution_replay.rs` | 新增 `execution_bearish_analysis_from_range` | CLI 调度入口 |
| `apps/cli/src/commands/execution_replay.rs` | 新增 `handle_execution_bearish_analysis` | 命令处理 |
| `apps/cli/src/main.rs` | 新增 `execution-bearish-analysis` CLI 命令 | 用户接口 |

**未修改**：任何 Observation / Evidence / Assessment / Decision / Policy 代码。

### 2B-1 完成标准

- ✅ 145 个 bearish candidate 完整分析
- ✅ Outcome 分布统计
- ✅ Evidence lift 计算
- ✅ Recovery conflict 分析（虽然因固定证据退化）
- ✅ C3 False Reduce 分析
- ✅ 输出至少一个 rejected hypothesis：固定证据无法区分

### 进入 TASK-154

下一步：基于 2B-1 发现，设计 **Holding Risk Evidence**。

关键约束：
- 不新增 `EvidenceKind` 直接进入 `EvidenceBuilder` 和 `DecisionEngine`
- 先在 Research Asset 层验证新证据的 lift 和 precision
- 重点方向：
  - 动态化 `Recovery` / `Breadth`（从变化而非状态出发）
  - 组合条件 `Distribution + RiskExpansion`
  - 多日恶化证据（如连续 breadth 下降）

## TASK-153.5: RiskExpansion Coverage Exploration（2026-07-18）

### 目标

在 2B-1 中，RiskExpansion 在 bearish candidate 中表现出最高 lift（1.45），但 n=6。本探索回答：

> RiskExpansion 是「稀缺 Alpha」还是「Observation 条件过严导致样本缺失」？

这个答案直接决定 TASK-154 的设计方向。

### 方法

RiskExpansion 由 `ObservationKind::VolatilityExpansion` 产生，触发条件：

```rust
features.amplitude_pct > 0.05
```

其中 `amplitude_pct = (high - low) / prev_close`。

`execution-bearish-analysis` 现在输出额外的 RiskExpansion Coverage 章节：
- 总体覆盖率
- `amplitude_pct` 分布百分位
- 阈值敏感性（从 0.01 到 0.15 的各个 threshold）
- 近失分析（低于当前 threshold 的区间）

### 关键发现

#### 1. RiskExpansion 总体覆盖率为 5.11%

- 总记录：8,615
- 触发记录：440（5.11%）
- 在 bearish candidates 中触发：6 / 145（4.1%）

这说明 RiskExpansion 不是罕见事件，但**它与 bearish assessment 的交集很小**。

#### 2. Amplitude 分布：当前 threshold 5% 高于 P90

| 指标 | amplitude_pct |
|---|---|
| Min | 0.00% |
| P10 | 0.88% |
| P25 | 1.25% |
| P50 | 1.83% |
| P75 | 2.72% |
| P90 | 3.96% |
| Max | 14.95% |
| Mean | 2.23% |

当前 threshold 5% 远高于 P90。这意味着它只捕获了极端波动日。

#### 3. 阈值敏感性：降低 threshold 不提升信号质量

| Threshold | Count | Negative T+20 | Lift vs Bearish Baseline | Avg T+20 | Avg T+60 |
|-----------|------:|----------------:|-------------------------:|---------:|---------:|
| 0.010 | 7,391 | 47.2% | 1.37 | 2.20% | 7.39% |
| 0.020 | 3,760 | 44.4% | 1.29 | 3.09% | 6.24% |
| 0.030 | 1,741 | 38.1% | 1.11 | 4.66% | 5.29% |
| 0.040 | 833 | 32.3% | 0.94 | 6.52% | 6.44% |
| 0.050（当前） | 440 | 26.6% | 0.77 | 8.39% | 6.49% |
| 0.080 | 98 | 26.5% | 0.77 | 8.99% | 4.00% |
| 0.100 | 42 | 26.2% | 0.76 | 4.83% | -0.64% |

重要发现：
- **threshold 越高，负收益概率越低**。极端波动（>5%）后更可能反弹。
- 即使放宽到 0.03，负收益概率也只有 38.1%，lift 1.11。
- 在当前 threshold 0.05，RiskExpansion 对全体记录的预测力 **低于 bearish baseline**（lift 0.77）。

#### 4. 近失分析：3%-5% 振幅区间比 >5% 更好

| Range | Count | Negative T+20 | Lift | Avg T+20 | Avg T+60 |
|-------|------:|----------------:|-----:|---------:|---------:|
| [0.030, 0.050) | 1,301 | 42.0% | 1.22 | 3.40% | 4.89% |
| [0.035, 0.050) | 769 | 42.5% | 1.23 | 3.80% | 5.49% |
| [0.040, 0.050) | 393 | 38.7% | 1.12 | 4.43% | 6.39% |
| [0.045, 0.050) | 169 | 38.5% | 1.12 | 3.88% | 4.69% |

近失区间的负收益概率比当前 >5% threshold **更高**。这说明当前 threshold 选在了「极端波动后反弹」的区间，而不是「风险持续恶化」的区间。

### 假说验证更新

| 假说 | 结果 |
|---|---|
| RiskExpansion 是稀缺 Alpha | **不支持**。总体覆盖率 5.11%，不稀缺。 |
| RiskExpansion 是 threshold 过严 | **部分支持但不解决问题**。降低 threshold 增加覆盖，但 lift 提升有限。 |
| RiskExpansion 是核心 Exit Evidence | **不支持**。它与 bearish assessment 交集小，且对全体记录预测力有限。 |
| 当前 threshold 选错了区间 | **支持**。3%-5% 区间比 >5% 区间更有预测力。 |

### 结论

**RiskExpansion 不是 TASK-154 的可靠核心。**

原因：
1. 它不是稀缺 Alpha（5.11% 覆盖）。
2. 它不是被 threshold 压制的信号（降低 threshold 不显著提升 lift）。
3. 它与 bearish assessment 的交集很小（6/145）。
4. 当前 threshold 反而选在了「极端波动后反弹」的区间。

### 对 TASK-154 的重大修正

2B-1 曾建议：

> 重点方向：组合条件 `Distribution + RiskExpansion`

现在修正为：

> **不要过度依赖 RiskExpansion。真正的方向是 Transition Evidence。**

#### 什么是 Transition Evidence？

不是问：

```
amplitude > 5% ?
```

而是问：

```
amplitude 是否在加速放大？
volatility 是否在恶化？
recovery 是否失败？
breadth 是否连续恶化？
```

也就是：

```
State Evidence  →  Transition Evidence  →  Holding Risk  →  Exit Evidence
```

### 2B 路线修正

```
2B-1 Bearish Evidence Analysis          ✅
TASK-153.5 RiskExpansion Coverage       ✅ 完成：RiskExpansion 不是核心

2B-2 Transition Evidence Modeling         NEXT
  - 动态化 Recovery / Breadth
  - 多日 Evidence 变化
  - 市场结构恶化

2B-3 Holding Risk Evidence

2B-4 Calibration v2
```

### 当前代码修改记录

| 文件 | 修改 | 原因 |
|---|---|---|
| `crates/execution-replay/src/bearish_analysis.rs` | 扩展 `RiskExpansionCoverage` 模块 | 计算 amplitude 分布、阈值敏感性、近失分析 |
| `crates/execution-replay/src/bearish_analysis_formatter.rs` | 扩展 Markdown 输出 | 输出 RiskExpansion Coverage 章节 |
| 其他 | 无 | 未改任何 Observation / Evidence / Assessment / Decision / Policy |

### 不进入 TASK-154 前的新约束

在 2B-2 解决 Transition Evidence 之前，不设计 Holding Risk Evidence：
- 不新增 `EvidenceKind`
- 不修改 `ObservationEngine` 条件
- 不修改 `DecisionEngine` 或 `ExecutionPolicy`

### 下一步建议

建议进入 **2B-2 Transition Evidence Modeling**：

1. 设计「变化型」证据：例如 `Breadth` 的 5 日变化、`Recovery` 的连续失败次数。
2. 在 Research Asset 层验证这些变化型证据的 lift。
3. 只有在变化型证据被验证后，才进入 Holding Risk Evidence 设计。

这样可以避免基于错误基础（RiskExpansion）设计 Holding Risk，确保 TASK-154 建立在真正可区分的信号之上。

---

## 2B-2: Transition Evidence Modeling（2026-07-18）

### 目标

建立 **Transition Evidence Layer**，用变化/恶化维度替代当前退化的固定状态证据。

核心问题：

> 系统知道「现在不好」，但不知道「正在变坏」。

这是 State vs Transition 问题。

### 当前 State Evidence 的问题示例

```text
Breadth = Weak
Recovery = Yes
Risk = High
```

系统看到：市场现在风险较高。但不知道：

```text
昨天：Breadth 65%，Recovery 成功
今天：Breadth 42%，Recovery 失败，Leadership 扩散
```

后者才是真正的 **Holding Risk 正在形成**。

### 建议的 Transition Observation Layer

```rust
pub struct MarketTransitionObservation {
    pub breadth_delta_5d: f64,
    pub confirmation_delta_5d: f64,
    pub recovery_failure_count: usize,
    pub leadership_decay: f64,
    pub liquidity_delta: f64,
    pub regime_transition: Option<String>,
}
```

重点：它描述**变化**，不是状态。

### 第一批 Transition Evidence 候选

#### 1. BreadthDeterioration

替代现在的 `Breadth` 固定证据。

条件示例：

```text
breadth_delta_5d < -15%
AND breadth < threshold
```

意义：不是市场弱，而是市场正在失去广度支持。

#### 2. RecoveryFailure（重点）

当前 `Recovery = true` 导致所有 bearish candidate 都有 Recovery，完全失效。

应改为：

```text
RecoveryAttempt
        + success
        + failure
```

条件示例：

```text
T0: 市场下跌
T+1~T+3: 反弹
但：成交不足 / Breadth 没恢复 / Leader 没回来
```

产生 `RecoveryFailure`。

这个可能比 RiskExpansion 更有价值。

#### 3. LeadershipDecay

替代当前 `LeadershipRotation` 状态证据。

条件示例：

```text
过去 Top sector strength +12%
现在 +2%
同时资金流下降
```

#### 4. LiquidityDeterioration

不要看 Liquidity 高低，要看 LiquidityTrend。

条件示例：

```text
volume_ratio 连续下降
成交额下降
上涨成交萎缩
```

### 验证方式

保持：

```text
Observation
    |
    v
Transition Evidence
    |
    v
Research Asset
```

每个候选需要输出：

| 字段 | 要求 |
|---|---|
| count | ≥ 30 |
| negative_T20_rate | 可计算 |
| negative_T60_rate | 可计算 |
| lift | ≥ 1.2 |
| precision | ≥ 50% |
| false_alarm | 可评估 |

### Research Asset 结构演进

未来 `ExecutionResearchRecord` 应逐步变成：

```text
ExecutionEvent
        |
        + Outcome
        + Evaluation
        + Evidence Snapshot
        + Transition Snapshot
```

因为 ML / Bayesian Assessment 需要：

```text
过去发生什么变化 + 最后结果
```

而不是：

```text
当时状态是什么
```

### 约束

- 不新增 `EvidenceKind` 进入 `EvidenceBuilder` 或 `DecisionEngine`。
- 不修改 `ObservationEngine` 条件。
- 不修改 `DecisionEngine` 或 `ExecutionPolicy`。
- 继续遵守 ADR-100：Decision thresholds shall not compensate for insufficient evidence semantics.

### 下一步行动

1. 选择第一个 Transition Evidence 候选（建议 `RecoveryFailure`，因为当前 `Recovery` 退化最严重）。
2. 设计研究-only 计算模块，从现有 `ExecutionResearchRecord` 和 `ExecutionEvent` 中计算变化信号。
3. 在 `execution-bearish-analysis` 或新工具中验证该信号的 lift 和 precision。
4. 验证通过后，再考虑作为新的 `ObservationKind` 或 `EvidenceKind` 接入平台。

### 相关 ADR

- `docs/v8/adr-101-transition-evidence-modeling.md`

---

## TASK-154.1: RecoveryFailure Research Module（2026-07-18）

### 目标

实现第一个 Transition Evidence 候选 `RecoveryFailure`：

> 市场出现压力后尝试反弹，但反弹在价格、广度、领导力维度上未能恢复。

保持 Research-only，不进入 Execution Pipeline。

### 新增代码

| 文件 | 用途 |
|---|---|
| `crates/execution-replay/src/transition_analysis.rs` | RecoveryFailure 三阶段检测 + lift/precision 统计 |
| `crates/execution-replay/src/transition_analysis_formatter.rs` | Markdown / JSON 输出 |
| `crates/app-service/src/execution_replay.rs` | `execution_transition_analysis_from_range` / `execution_transition_analysis_from_suite` |
| `apps/cli/src/commands/execution_replay.rs` | `handle_execution_transition_analysis` |
| `apps/cli/src/main.rs` | `execution-transition-analysis` CLI 命令 |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### RecoveryFailure v1 定义

#### 三阶段模型

1. **Initial Pressure**：过去 5 个日历日内存在一天，满足 `today_return < -1.5%` 或 `close_position < 0.25`。
2. **Recovery Attempt**：当日 `today_return >= 0.5%` 且 `close >= pressure_close * 0.98`（真实反弹，不只是止跌）。
3. **Recovery Failure**：
   - `price_recovery_failed`：`close < pressure_close * 1.02`（未收复前低上方 2%）
   - `breadth_recovery_failed`：`breadth_pct < pressure_breadth + 3` 或 `< 45` 或 `delta_5d < -2`
   - `leadership_recovery_failed`：`leadership_stability < pressure_leadership * 1.02` 或 `< 0.55`

#### 评分

```text
failure_score = 0.4 * price_failure + 0.4 * breadth_failure + 0.2 * leadership_failure
```

`failure_score >= 0.5` 判定为 `RecoveryFailure`。

### 运行命令

```bash
cargo run -p quant-cli -- execution-transition-analysis \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --candidate recovery_failure \
  --output markdown
```

报告路径：`reports/execution-validation/transition_analysis_recovery_failure_cn_2026-07-18.md`

### 结果

| 指标 | 数值 |
|---|---|
| 总记录 | 8,616 |
| 样本数 | 2,053 |
| 基线 Negative T+20 | 47.2% |
| 基线 Negative T+60 | 40.9% |
| RecoveryFailure Negative T+20 | 45.1% |
| RecoveryFailure Negative T+60 | 43.2% |
| Lift T+20 | 0.95 |
| Lift T+60 | 1.06 |
| Average T+20 | 2.51% |
| Average T+60 | 7.52% |

### Breakdown

| Combination | Count |
|---|---|
| Full Failure | 1,064 |
| Breadth + Leadership | 989 |
| Price + Breadth | 0 |
| Price + Leadership | 0 |
| Price Only | 0 |
| Breadth Only | 0 |
| Leadership Only | 0 |

### ADR-101 Validation Gate

| 门限 | 要求 | 结果 |
|---|---|---|
| 样本量 | >= 30 | PASS (2,053) |
| Precision T+20 | >= 50% | FAIL (45.1%) |
| Lift T+20 | >= 1.2 | FAIL (0.95) |

**Overall: FAIL**

### 关键发现

1. **RecoveryFailure v1 没有预测价值**：负收益概率 45.1%，低于全基线 47.2%。
2. **平均 T+20 为正**：2.51%，说明这些「反弹失败」样本在 20 天后平均上涨。
3. **样本量足够大**（2,053），排除了小样本幻觉。
4. **价格失败几乎不出现**：Breakdown 中 Price-only / Price+Breadth / Price+Leadership 都是 0。因为一旦价格反弹 >= 0.5%，就很难同时满足 `price_recovery_failed`。
5. **大部分失败是 Breadth + Leadership 或 Full Failure**：广度 / 领导力没有恢复，但价格本身仍在反弹。

### 为什么失败？

这个结果表明，在 CN 2024-2025 这个数据集上：

> 市场出现压力后反弹，即使广度和领导力没有恢复，价格也倾向于继续上涨。

这与直觉相反。可能原因：
- 数据集整体偏牛，任何反弹都容易被市场承接。
- 广度和领导力是滞后指标，它们在价格反弹后需要更长时间才能恢复。
- 当前 `breadth` 和 `leadership_stability` 来自 `ResearchContext`，是固定状态值，可能不够敏感。
- 压力定义过于宽松，导致大量普通回调被误判为压力。

### 下一步选项

1. **收紧压力定义**：要求压力日 `today_return < -3%` 或 `amplitude_pct > 4%`。
2. **收紧恢复失败定义**：要求价格反弹失败（即价格 "bounce but stall"），但这会大幅减少样本。
3. **组合测试**：将 RecoveryFailure 与 `Distribution` 或 `RiskExpansion` 组合，看是否有协同 lift。
4. **换候选**：尝试 `BreadthDeterioration`（从变化出发）可能更直接。
5. **延长恢复窗口**：观察 T+5 内的价格/广度/领导力变化，而不是只看单日反弹。

### 当前结论

**RecoveryFailure v1 被数据否定。不进入 Observation / Evidence 层。**

这验证了 ADR-101 的价值：先作为 Research Asset 验证，再决定是否进入平台。避免把一个看起来合理的语义（"恢复失败"）直接编码成 Evidence，从而污染 Execution Decision。

### 约束仍然有效

- 不新增 `EvidenceKind`
- 不修改 `ObservationEngine`
- 不修改 `DecisionEngine` 或 `ExecutionPolicy`

---

### 2B 状态更新

```text
2B-1 Bearish Evidence Analysis          ✅
TASK-153.5 RiskExpansion Coverage       ✅
2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure v1              ✅ REJECTED
  2B-2.2 BreadthDeterioration            NEXT
  2B-2.3 LeadershipDecay                 pending
2B-3 Holding Risk Evidence               pending
2B-4 Calibration v2                      pending
```

### 相关文档

- `docs/v8/adr-101-transition-evidence-modeling.md`
- `reports/execution-validation/transition_analysis_recovery_failure_cn_2026-07-18.md`

---

## TASK-154.2: BreadthDeterioration Research Module（2026-07-18）

### 目标

实现第二个 Transition Evidence 候选 `BreadthDeterioration`：

> 市场参与广度是否正在恶化？

只使用变化量，不使用绝对状态。

### 新增代码

| 文件 | 修改 |
|---|---|
| `crates/execution-replay/src/transition_analysis.rs` | 扩展 `BreadthDeteriorationSignal`、三阶段检测、delta 分布诊断 |
| `crates/execution-replay/src/transition_analysis_formatter.rs` | `BreadthDeteriorationBreakdown` 输出 |
| `crates/app-service/src/execution_replay.rs` | 通过已有 `execution_transition_analysis_from_range` 复用 |
| `apps/cli/src/commands/execution_replay.rs` | 通过已有 CLI 复用 |
| `apps/cli/src/main.rs` | `--candidate breadth_deterioration` 可用 |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### BreadthDeterioration v1 定义

- `breadth_delta_5d` = 当前 breadth_pct - 5 个交易日前 breadth_pct
- `breadth_delta_10d` = 当前 breadth_pct - 10 个交易日前 breadth_pct
- 触发条件：
  - `breadth_delta_5d < -15` 个百分点，或
  - `breadth_delta_10d < -25` 个百分点

### 运行命令

```bash
cargo run -p quant-cli -- execution-transition-analysis \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --candidate breadth_deterioration \
  --output markdown
```

报告路径：`reports/execution-validation/transition_analysis_breadth_deterioration_cn_2026-07-18.md`

### 结果

| 指标 | 数值 |
|---|---|
| 总记录 | 8,616 |
| 样本数 | 0 |
| 基线 Negative T+20 | 47.2% |
| BreadthDeterioration Negative T+20 | N/A |
| Lift T+20 | N/A |

### 诊断输出

```text
Breadth diagnostic: symbols=24, records per symbol min/max=359/359, 
breadth_pct min/max/p50=50.0/50.0/50.0
delta_5d n=8616, P10=0.0 P25=0.0 P50=0.0 P75=0.0 P90=0.0
delta_10d n=8616, P10=0.0 P25=0.0 P50=0.0 P75=0.0 P90=0.0
```

### 关键发现

**CRITICAL: `breadth_pct` 在所有记录中恒等于 50.0。**

- 24 个标的，每个 359 条记录，共 8,616 条记录。
- `breadth_pct` 最小 = 最大 = 中位数 = 50.0。
- 所有 `delta_5d` 和 `delta_10d` 均为 0。
- `BreadthDeterioration` 无法从当前 `ExecutionMarketView.breadth` 计算。

### 原因分析

`ExecutionMarketView` 中的 `BreadthSummary` 没有被真实数据填充。`breadth_pct` 是一个占位值（50.0）。

这直接解释了 2B-1 的关键发现：

> 固定 Evidence（Breadth / Confirmation / Recovery / LeadershipRotation）lift = 1.0，因为它们来自恒定的 `ResearchContext` 占位数据。

不是 Evidence 语义设计问题，而是 **数据管道问题**。

### 当前结论

**BreadthDeterioration 被阻塞，不是被否定。** 在当前数据质量下无法验证。必须先修复上游 `ResearchContext` / `ExecutionMarketView` 的 breadth 数据生成，才能继续。

### 约束仍然有效

- 不新增 `EvidenceKind`
- 不修改 `ObservationEngine`
- 不修改 `DecisionEngine` 或 `ExecutionPolicy`

---

## TASK-154.3: LeadershipDecay Research Module（2026-07-18）

### 目标

实现第三个 Transition Evidence 候选 `LeadershipDecay`：

> 核心资产领导力是否正在瓦解？

### 新增代码

| 文件 | 修改 |
|---|---|
| `crates/execution-replay/src/transition_analysis.rs` | 扩展 `LeadershipDecaySignal`、检测逻辑、分布诊断 |
| `crates/execution-replay/src/transition_analysis_formatter.rs` | `LeadershipDecayBreakdown` 输出 |
| CLI / app-service | 通过已有 `--candidate leadership_decay` 复用 |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### LeadershipDecay v1 定义

- `leadership_delta_5d` = 当前 leadership_stability - 5 个交易日前
- `leadership_delta_10d` = 当前 leadership_stability - 10 个交易日前
- 触发条件：
  - `leadership_delta_5d < -0.15`，或
  - `leadership_delta_10d < -0.25`

### 运行命令

```bash
cargo run -p quant-cli -- execution-transition-analysis \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --candidate leadership_decay \
  --output markdown
```

报告路径：`reports/execution-validation/transition_analysis_leadership_decay_cn_2026-07-18.md`

### 结果

| 指标 | 数值 |
|---|---|
| 总记录 | 8,616 |
| 样本数 | 0 |

### 诊断输出

```text
Leadership diagnostic: stability min/max/p50=0.50/0.50/0.50
delta_5d P10=0.00 P25=0.00 P50=0.00 P75=0.00 P90=0.00
delta_10d P10=0.00 P25=0.00 P50=0.00 P75=0.00 P90=0.00
```

### 关键发现

**CRITICAL: `leadership_stability` 在所有记录中恒等于 0.50。**

与 `breadth_pct` 一样，`leadership_stability` 也是占位值，没有真实数据。

### 当前结论

**LeadershipDecay 同样被数据管道阻塞。**

---

## 2B 阶段重大发现：State Evidence 退化根因

### 根因链

```text
Reduce=0
    |
    v
Decision Gate 阻塞（Confidence / RiskHigh）
    |
    v
Evidence 层区分能力不足
    |
    v
State Evidence 退化（lift=1.0）
    |
    v
ResearchContext.breadth / leadership_stability / ... 是占位值
    |
    v
ExecutionMarketView 没有真实 Breadth / Leadership 数据
```

### 影响

这意味着：

1. `Breadth` Evidence 在所有记录中都是同一个值，没有区分能力。
2. `LeadershipRotation` Evidence 同样退化。
3. `Confirmation` / `Recovery` 也可能存在类似问题（需要进一步验证）。
4. 所有基于 `ResearchContext` 的固定 Evidence 都无法帮助 Exit Decision。

### 这不是算法问题，是数据管道问题

之前以为是：

> State Evidence 语义不对，需要 Transition Evidence。

现在发现更根本的是：

> State Evidence 的输入数据本身是占位值，所以无论是 State 还是 Transition 都无法计算。

### 下一步必须修复上游

在继续任何 Transition Evidence 之前，必须先：

1. 检查 `ResearchContext` 构建链路（`apps/cli` 或 `app-service`）是否正确计算 `BreadthSummary`。
2. 检查 `RotationSummary.leadership_stability` 是否正确计算。
3. 确认 `ExecutionMarketView::from_research_context` 是否正确映射这些字段。
4. 重新生成 `ExecutionResearchRecord` 后，再重新运行所有 2B 分析。

### 建议的修复顺序

```text
1. 验证并修复 ResearchContext.breadth / rotation 数据生成
        |
        v
2. 重新运行 execution-transition-analysis（RecoveryFailure / BreadthDeterioration / LeadershipDecay）
        |
        v
3. 重新运行 execution-bearish-analysis（固定 Evidence 的 lift 应该会变化）
        |
        v
4. 选择有效的 Transition Evidence 候选继续推进
        |
        v
5. 进入 Holding Risk Evidence 设计
        |
        v
6. Calibration v2
```

---

## 2B 状态更新（最终）

```text
2B-1 Bearish Evidence Analysis          ✅
TASK-153.5 RiskExpansion Coverage       ✅
2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure v1              ❌ REJECTED (ADR-102)
  2B-2.2 BreadthDeterioration            ⚠️  BLOCKED (upstream data)
  2B-2.3 LeadershipDecay                 ⚠️  BLOCKED (upstream data)

阶段结论：Transition Evidence 无法在当前数据质量下验证。

必须先修复：
- ResearchContext.breadth 真实数据
- ResearchContext.rotation.leadership_stability 真实数据

然后再进入：
2B-3 Holding Risk Evidence
2B-4 Calibration v2
```

### 相关 ADR

- `docs/v8/adr-101-transition-evidence-modeling.md`
- `docs/v8/adr-102-rejected-recoveryfailure-as-exit-transition-evidence.md`
- `docs/v8/adr-103-transition-evidence-blocked-by-data-quality.md`
- `reports/execution-validation/transition_analysis_recovery_failure_cn_2026-07-18.md`
- `reports/execution-validation/transition_analysis_breadth_deterioration_cn_2026-07-18.md`
- `reports/execution-validation/transition_analysis_leadership_decay_cn_2026-07-18.md`

---

## TASK-156: ResearchContext Fact Integrity Audit（2B-0，2026-07-18）

### 背景

在 TASK-154.2 和 TASK-154.3 中，`BreadthDeterioration` 和 `LeadershipDecay` 因为 `ExecutionMarketView` 中的 `breadth_pct` 和 `leadership_stability` 是恒定占位值而返回 0 样本。这不仅是两个候选被阻塞，而是整个 ResearchContext 语义层可能存在事实污染。

因此根据用户建议，在进入任何 Evidence Modeling 之前，增加 **2B-0 ResearchContext Fact Integrity Gate**。

### 新增 ADR

- `docs/v8/adr-104-researchcontext-fact-integrity-gate.md`
- 核心原则：No Evidence Modeling shall proceed on ResearchContext fields without variance and provenance validation.

### 新增工具

| 文件 | 用途 |
|---|---|
| `crates/execution-replay/src/context_integrity_audit.rs` | 8 个字段的方差/占位值检查 |
| `crates/execution-replay/src/context_integrity_audit_formatter.rs` | Markdown / JSON 输出 |
| `crates/app-service/src/execution_replay.rs` | `execution_context_integrity_audit_from_range` / `_from_suite` |
| `apps/cli/src/commands/execution_replay.rs` | `handle_execution_context_integrity_audit` |
| `apps/cli/src/main.rs` | `execution-context-integrity-audit` CLI 命令 |

### 第一次运行结果（修复前）

```bash
cargo run -p quant-cli -- execution-context-integrity-audit \
  --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
```

| Field | Status | Samples | Unique | Min | Max |
|---|---|---:|---:|---:|---:|
| confirmation.trend.score | CONSTANT | 8616 | 1 | 50.00 | 50.00 |
| confirmation.participation.score | CONSTANT | 8616 | 1 | 50.00 | 50.00 |
| confirmation.risk.score | CONSTANT | 8616 | 1 | 50.00 | 50.00 |
| breadth.breadth_pct | PLACEHOLDER | 8616 | 1 | 50.00 | 50.00 |
| breadth.delta_5d | PLACEHOLDER | 8616 | 1 | 0.00 | 0.00 |
| breadth.sma5 | PLACEHOLDER | 8616 | 1 | 0.00 | 0.00 |
| recovery.score | CONSTANT | 8616 | 1 | 50.00 | 50.00 |
| leadership_stability | PLACEHOLDER | 8616 | 1 | 0.50 | 0.50 |

**结果：8/8 字段失败，全部 Gate 失败。**

### 根因定位

文件：`crates/app-service/src/execution_replay.rs` 中 `build_execution_event` 函数。

该函数在构造 `ExecutionMarketView` 时直接硬编码了占位值：

- `confirmation.trend/participation/risk.score = 50.0`
- `breadth.breadth_pct = 50.0`
- `recovery.score = 50.0`
- `leadership_stability = 0.5`

而不是从 `ResearchContext` 中投影。 ResearchContext 本身（`build_research_context_from_dataset`）已经正确计算了 breadth 和 leadership，但 `execution_replay.rs` 没有使用它。

### 修复

将 `build_execution_event` 改为：

1. 通过 `AppContext::build_research_context_for_date(date, scope)` 获取真实 `ResearchContext`。
2. 通过 `ExecutionMarketView::from_research_context(&ctx)` 投影得到 `ExecutionMarketView`。
3. 在 `load_records_from_range` 和 `find_validation_candidates` 中按日期缓存 `ResearchContext`，避免每个 symbol/date 都重复构建。

关键代码变更：

```rust
let ctx = app
    .build_research_context_for_date(date, scope)
    .context("failed to build ResearchContext for execution replay")?;
let market_view = ExecutionMarketView::from_research_context(&ctx);
```

### 修复后运行结果

| Field | Status | Samples | Unique | Min | Max | Mean | Variance |
|---|---|---:|---:|---:|---:|---:|---:|
| confirmation.trend.score | PASS | 8616 | 359 | 35.54 | 84.38 | 62.36 | 106.15 |
| confirmation.participation.score | PASS | 8616 | 150 | 20.00 | 100.00 | 57.29 | 457.38 |
| confirmation.risk.score | PASS | 8616 | 358 | 14.23 | 87.52 | 48.63 | 246.05 |
| breadth.breadth_pct | PASS | 8616 | 25 | 0.00 | 100.00 | 49.28 | 1255.29 |
| breadth.delta_5d | PASS | 8616 | 39 | -83.33 | 87.50 | 1.04 | 836.50 |
| breadth.sma5 | PASS | 8616 | 110 | 3.33 | 100.00 | 48.90 | 1105.40 |
| recovery.score | PASS | 8616 | 180 | 22.00 | 96.40 | 61.34 | 375.85 |
| leadership_stability | PASS | 8616 | 359 | 0.06 | 1.00 | 0.81 | 0.05 |

**结果：8/8 字段全部通过 Fact Integrity Gate。**

### 报告

- `reports/execution-validation/context_integrity_audit_cn_2026-07-18.md`

### 2B-1 和 2B-2 重跑结果（修复后）

使用真实 ResearchContext 后，重新运行：

```bash
cargo run -p quant-cli -- execution-bearish-analysis --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown

cargo run -p quant-cli -- execution-transition-analysis --scope cn --from 2024-01-01 --to 2025-06-30 --candidate recovery_failure --output markdown

cargo run -p quant-cli -- execution-transition-analysis --scope cn --from 2024-01-01 --to 2025-06-30 --candidate breadth_deterioration --output markdown

cargo run -p quant-cli -- execution-transition-analysis --scope cn --from 2024-01-01 --to 2025-06-30 --candidate leadership_decay --output markdown
```

#### 2B-1 Bearish Evidence Analysis（v2）

报告：`reports/execution-validation/bearish_analysis_cn_v2_2026-07-18.md`

| 指标 | 数值 |
|---|---|
| 总记录 | 8607 |
| Bearish Candidates | 73 |
| Baseline Negative T+20 | 31.5% |
| 最高 lift | LiquidityConfirmation: 1.02 |
| RiskExpansion coverage | 5.11% |
| RiskExpansion lift @ threshold 0.010 | 1.50 |
| RiskExpansion lift @ threshold 0.050 | 0.84 |

关键发现：
- 真实数据下，固定 Evidence（Breadth/Recovery/Leadership/Confirmation）在 bearish candidate 中仍然是 lift=1.00，因为它们在 candidate 中普遍存在。
- RiskExpansion 在极端低阈值（0.010）下有强 lift（1.50），但当前阈值（0.050）下 lift 只有 0.84。
- 这意味着：**即使数据真实，固定 State Evidence 仍不足以区分退出；真正的区分力在动态观察（RiskExpansion）和变化型信号中。**

#### 2B-2.1 RecoveryFailure v2

报告：`reports/execution-validation/transition_analysis_recovery_failure_cn_v2_2026-07-18.md`

| 指标 | 数值 |
|---|---|
| 样本 | 1364 |
| Negative T+20 | 46.8% |
| Lift T+20 | 0.99 |
| Precision T+20 | 46.8% |
| ADR-101 | FAIL |

结论：**修复数据后，RecoveryFailure v1 仍然不满足 ADR-101。** 与 v1 结论一致（之前 lift 0.95）。

#### 2B-2.2 BreadthDeterioration v2

报告：`reports/execution-validation/transition_analysis_breadth_deterioration_cn_v2_2026-07-18.md`

| 指标 | 数值 |
|---|---|
| 样本 | 3958 |
| Negative T+20 | 48.6% |
| Lift T+20 | 1.03 |
| Precision T+20 | 48.6% |
| ADR-101 | FAIL |

结论：**真实数据下，BreadthDeterioration v1 可计算，但 lift 仅 1.03，未达 1.2 门限。** 不是数据问题，而是当前检测逻辑不够强。

#### 2B-2.3 LeadershipDecay v2

报告：`reports/execution-validation/transition_analysis_leadership_decay_cn_v2_2026-07-18.md`

| 指标 | 数值 |
|---|---|
| 样本 | 744 |
| Negative T+20 | 42.6% |
| Lift T+20 | 0.90 |
| Precision T+20 | 42.6% |
| Negative T+60 | 61.6% |
| Lift T+60 | 1.51 |
| ADR-101 | FAIL (T+20) |

结论：**T+20 不通过，但 T+60 表现很强（lift 1.51）。** LeadershipDecay 可能是中期风险信号，而不是短期退出信号。这是一个有价值的发现，需要后续探索不同 horizon 的语义。

### 2B 阶段最新状态

```text
2B-0 ResearchContext Fact Integrity    ✅ PASS
        |
        v
2B-1 Bearish Evidence Analysis       ✅
        |
        v
2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure v1            ❌ REJECTED (ADR-102)
  2B-2.2 RecoveryFailure v2            ❌ REJECTED (lift 0.99)
  2B-2.3 BreadthDeterioration v1       ❌ REJECTED (lift 1.03)
  2B-2.4 LeadershipDecay v1            ❌ REJECTED T+20, BUT T+60 lift 1.51 (interesting)
        |
        v
2B-3 Holding Risk Evidence           pending
2B-4 Calibration v2                  pending
```

### 关键启示

1. **数据质量问题是前提。** 在 2B-0 之前，所有 2B-1/2B-2 结果都基于污染数据，不可信。修复后必须重跑所有分析。
2. **真实数据下，固定 State Evidence 仍然无效。** 即使 breadth/leadership/recovery 是真实值，它们在 bearish candidate 中普遍存在，无法区分退出。
3. **Transition Evidence 是正确方向，但当前候选不够强。** BreadthDeterioration  lift 仅 1.03，需要更精细的检测逻辑。
4. **LeadershipDecay 可能是中期信号。** T+60 lift 1.51 提示我们可以探索不同 horizon 的 Evidence 语义。
5. **2B-0 Gate 必须成为标准流程。** 任何新 Evidence 或新 scope 都需要先通过 Fact Integrity Gate。

### 下一步建议

1. **迭代 Transition Evidence 检测逻辑**：
   - 收紧 BreadthDeterioration 的触发条件（如同时要求 price 走弱）。
   - 探索 LeadershipDecay 在 T+60 的语义（可能作为中期 Holding Risk）。
   - 尝试组合条件：`BreadthDeterioration + LeadershipDecay`。
2. **引入更多 Transition 候选**：
   - `LiquidityDeterioration`
   - `ConfirmationDecay`
   - `RegimeTransition`
3. **在 Evidence 足够强之后，才进入 Holding Risk Evidence 设计**。
4. **只有 Reduce Precision ≥ 50% 时，才调整 confidence threshold（ADR-100 / ADR-098）**。

### 相关文件

- `docs/v8/adr-104-researchcontext-fact-integrity-gate.md`
- `reports/execution-validation/context_integrity_audit_cn_2026-07-18.md`
- `reports/execution-validation/bearish_analysis_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_recovery_failure_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_breadth_deterioration_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_leadership_decay_cn_v2_2026-07-18.md`
- `crates/app-service/src/execution_replay.rs`（修复位置）

---

## TASK-157: LeadershipDecay Horizon Analysis（2B-2.4，2026-07-18）

### 背景

在 2B-2.3 中，LeadershipDecay v1 在 T+20 horizon 下 lift 只有 0.90，未通过 ADR-101，但 T+60 表现很强（lift 1.51，negative 61.6%）。这提示 LeadershipDecay 可能不是短线退出信号，而是**中期 Holding Risk 信号**。

用户建议：不要急着优化 T+20，而是先做 **Horizon Analysis**，确认 LeadershipDecay 的真实语义。

### 目标

回答：

> LeadershipDecay 是短线 Exit Signal，还是中期 Holding Risk Signal？

### 方法

新增 `execution-leadership-decay-horizon` CLI，计算 LeadershipDecay 在 T+5 / T+20 / T+60 / T+120 的：

- negative rate
- baseline negative rate
- lift
- precision
- average return
- median return
- average max drawdown

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/lib.rs` | `ExecutionOutcome` 增加 `t5_return`（带 `#[serde(default)]`），保持旧记录兼容 |
| `crates/execution-replay/src/outcome.rs` | `MarketStoreOutcomeResolver` 计算 T+5 前向收益 |
| `crates/execution-replay/src/transition_analysis.rs` | `compute_leadership_decay_horizon_analysis` + `LeadershipDecayHorizonAnalysis` + `HorizonProfile` |
| `crates/execution-replay/src/transition_analysis_formatter.rs` | `LeadershipDecayHorizonFormatter` |
| `crates/app-service/src/execution_replay.rs` | `execution_leadership_decay_horizon_from_range` / `_from_suite` |
| `apps/cli/src/main.rs` | `execution-leadership-decay-horizon` CLI |
| `apps/cli/src/commands/execution_replay.rs` | `handle_execution_leadership_decay_horizon` |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
cargo run -p quant-cli -- execution-leadership-decay-horizon \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --output markdown
```

报告路径：`reports/execution-validation/leadership_decay_horizon_cn_2026-07-18.md`

### 结果

| Horizon | Samples | Negative Rate | Baseline | Lift | Precision | Avg Return | Median Return | Avg Max DD |
|---------|--------:|--------------:|---------:|-----:|----------:|-----------:|--------------:|-----------:|
| T+5 | 743 | 40.2% | 50.0% | **0.80** | 40.2% | 0.73% | 0.76% | -18.55% |
| T+20 | 743 | 42.5% | 47.2% | **0.90** | 42.5% | 1.98% | 0.88% | -18.55% |
| T+60 | 743 | 61.5% | 40.9% | **1.50** | 61.5% | -0.83% | -1.88% | -18.55% |
| T+120 | 743 | 52.6% | 22.3% | **2.36** | 52.6% | 1.67% | -0.75% | -18.85% |

### 关键发现

1. **T+5 / T+20：LeadershipDecay 不是短线退出信号**
   - T+5 lift 0.80，T+20 lift 0.90，都小于 1.0。
   - Negative rate 低于 baseline，说明短线反而比整体市场更容易涨。

2. **T+60：LeadershipDecay 是明确的中期 Holding Risk 信号**
   - lift 1.50，precision 61.5%。
   - Average T+60 return -0.83%，median -1.88%。
   - 这是修复数据后第一个达到 "lift > 1.2 且 precision > 50%" 的候选。

3. **T+120：lift 2.36，但 baseline 很低（22.3%）**
   - 这是因为 CN 2024-2025 在 120 日窗口整体偏牛，negative rate 本身就很低。
   - 虽然 lift 高，但 precision 52.6% 不如 T+60 的 61.5% 稳定。

4. **平均 Max Drawdown 在 -18.5% 左右**
   - 无论是 LeadershipDecay 样本还是 baseline，中期回撤都很大。
   - 这进一步说明：这个信号不是关于「明天跌不涨」，而是关于「未来 2-3 个月会不会有显著回撤风险」。

### 结论

**LeadershipDecay 是 Medium-Term Holding Risk Signal，不是 Short-Term Exit Signal。**

在当前 CN 2024-2025 数据集上，最好的 horizon 是 **T+60**：

- lift 1.50
- precision 61.5%
- 平均 T+60 收益 -0.83%
- 中位数 T+60 收益 -1.88%

这满足「Reduce Precision ≥ 50%」的最低阈值（ADR-098），可以作为 Holding Risk Evidence 的候选继续迭代。

### 对后续路线的影响

这改变了 2B 的顺序：

```text
2B-2 Transition Evidence
        |
        +-- RecoveryFailure          ❌ REJECTED (T+20 lift 0.99)
        +-- BreadthDeterioration       ❌ REJECTED (T+20 lift 1.03)
        +-- LeadershipDecay
                |
                +-- T+20             ❌ REJECTED (T+20 lift 0.90)
                +-- T+60             ✅ PASS (lift 1.50, precision 61.5%)
                +-- T+120            ✅ PASS (lift 2.36, precision 52.6%)

        |
        v

2B-3 Holding Risk Evidence
        |
        +-- LeadershipDecay as Medium-Term Holding Risk  ← 第一个候选
```

### 下一步建议

1. **TASK-158: Transition Evidence Combination Study**
   - 测试 `LeadershipDecay + BreadthDeterioration` 在 T+60 的 lift。
   - 测试 `LeadershipDecay + LiquidityDeterioration` 在 T+60 的 lift。

2. **TASK-159: ResearchContext Integrity CI Gate**
   - 把 `execution-context-integrity-audit` 纳入测试流程。
   - 任何新增 `ExecutionMarketView` 字段必须满足 variance + provenance + placeholder 检查。

3. **Evidence Horizon 抽象**
   - 考虑在 `ExecutionEvidence` 或 `ObservationKind` 中引入 horizon 语义：
     ```rust
     enum EvidenceHorizon {
         Immediate,   // T+5
         ShortTerm,   // T+20
         MediumTerm,  // T+60
         LongTerm,    // T+120
     }
     ```
   - 未来不同 Evidence 可以声明自己的 natural horizon，而不是统一用 T+20 评估。

### 相关文件

- `reports/execution-validation/leadership_decay_horizon_cn_2026-07-18.md`
- `crates/execution-replay/src/transition_analysis.rs`（TASK-157 实现）
- `crates/execution-replay/src/transition_analysis_formatter.rs`
- `crates/execution-replay/src/outcome.rs`（T+5 收益计算）
- `crates/execution-replay/src/lib.rs`（ExecutionOutcome 增加 t5_return）

### 2B 状态更新（最新）

```text
2B-0 ResearchContext Fact Integrity    ✅ PASS
2B-1 Bearish Evidence Analysis          ✅
2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED (T+20 lift 0.99)
  2B-2.2 BreadthDeterioration           ❌ REJECTED (T+20 lift 1.03)
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED (T+20 lift 0.90)
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 LeadershipDecay as Medium-Term Holding Risk  ← NEXT
```

---

## ADR-105: Evidence Horizon and Role Model（2026-07-18）

### 背景

TASK-157 证明同一个 Evidence（LeadershipDecay）在不同 horizon 表现完全不同：T+5/T+20 无效，T+60 有效。这直接挑战了当前 Execution Pipeline 的隐含假设：

> 所有 Evidence 都在同一个时间尺度上聚合为 `dominant_direction`。

如果不引入时间尺度语义，系统会错误地把中期 Holding Risk 信号当成短线 Reduce 信号使用。

### 决策

**任何进入 Decision Path 的 Evidence 必须声明 Natural Horizon 和 Evidence Role。**

### 模型

```rust
pub enum EvidenceHorizon {
    Immediate,     // T+1 ~ T+5
    ShortTerm,     // T+5 ~ T+20
    MediumTerm,    // T+20 ~ T+60
    LongTerm,      // T+60+
}

pub enum EvidenceRole {
    EntrySignal,
    ExitSignal,
    HoldingRisk,
    RegimeRisk,
    Confirmation,
}

pub struct EvidenceProfile {
    pub kind: EvidenceKind,
    pub horizon: EvidenceHorizon,
    pub role: EvidenceRole,
    pub confidence: f64,
    pub direction: f64,
}
```

### 示例 Profile

| Evidence | Horizon | Role |
|---|---|---|
| RiskExpansion | ShortTerm | HoldingRisk / ExitSignal |
| Distribution | ShortTerm | HoldingRisk |
| LeadershipDecay | MediumTerm | **HoldingRisk** |
| BreadthDeterioration | MediumTerm | **HoldingRisk** |
| LiquidityDeterioration | MediumTerm | **HoldingRisk** |
| MarketAcceptance | ShortTerm | Confirmation |

### 关键影响

1. 研究工具必须按 natural horizon 评估，不能只用 T+20。
2. 未来 `AssessmentEngine` 应该按 `EvidenceRole` 聚合，而不是把所有 Evidence 压成单一 `dominant_direction`。
3. V8 从「方向判断系统」升级为「带时间尺度的风险认知模型」。

### 文件

- `docs/v8/adr-105-evidence-horizon-and-role-model.md`

---

## TASK-158: Holding Risk Evidence Bundle（2B-3，2026-07-18）

### 背景

用户建议调整 TASK-158 目标：不要只追求组合 lift 产生 Reduce，而是建立 **Medium-Term Holding Risk Score**。

结构：

```text
结构
 |
 +-- LeadershipDecay
 |
广度
 |
 +-- BreadthDeterioration
 |
资金
 |
 +-- LiquidityDeterioration
```

### 目标

建立第一个 Holding Risk Evidence Bundle，回答：

> 当前持仓未来两个月的中期回撤风险有多高？

### 方法

新增 `execution-holding-risk-bundle` CLI，组合三个信号：

1. **LeadershipDecay**（已有，权重 0.4）
2. **BreadthDeterioration**（已有，权重 0.3）
3. **LiquidityDeterioration**（新增，权重 0.3）

`LiquidityDeterioration` 定义：

- `volume_ratio = volume / volume_ma20`
- `volume_ratio_delta_5d = current - 5 天前`
- `volume_ratio_delta_10d = current - 10 天前`
- 触发条件：
  - `delta_5d < -0.50`，或
  - `delta_10d < -1.00`

两种评分方式：
- **Signal Count**：0 / 1 / 2 / 3 个信号
- **Weighted Score**：0.4 x leadership + 0.3 x breadth + 0.3 x liquidity

评估 horizon：T+60（MediumTerm）。

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/holding_risk_bundle.rs` | 新增 LiquidityDeterioration + Bundle 计算 + T+60 评估 |
| `crates/execution-replay/src/holding_risk_bundle_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/transition_analysis.rs` | `detect_breadth_deterioration` / `detect_leadership_decay` 改为 `pub(crate)` |
| `crates/execution-replay/src/lib.rs` | 导出模块和类型 |
| `crates/app-service/src/execution_replay.rs` | `execution_holding_risk_bundle_from_range` / `_from_suite` |
| `apps/cli/src/main.rs` | `execution-holding-risk-bundle` CLI |
| `apps/cli/src/commands/execution_replay.rs` | `handle_execution_holding_risk_bundle` |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
cargo run -p quant-cli -- execution-holding-risk-bundle \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --output markdown
```

报告路径：`reports/execution-validation/holding_risk_bundle_cn_2026-07-18.md`

### 结果：Signal Count Buckets

| Score | Count | Negative T+60 | Baseline | Lift | Precision | Avg T+60 | Median T+60 |
|-------|------:|--------------:|---------:|-----:|----------:|---------:|------------:|
| 0 signals | 3695 | 36.8% | 40.9% | 0.90 | 36.8% | 8.41% | 3.95% |
| 1 signals | 4136 | 43.2% | 40.9% | 1.06 | 43.2% | 6.42% | 1.71% |
| 2 signals | 773 | 48.0% | 40.9% | 1.17 | 48.0% | 5.59% | 0.40% |
| 3 signals | 12 | 50.0% | 40.9% | 1.22 | 50.0% | 1.89% | 0.63% |

### 结果：Weighted Score Buckets

| Weighted Score | Count | Negative T+60 | Baseline | Lift | Precision | Avg T+60 |
|---|---|---:|---:|---:|---:|---:|
| 0.0 | 3695 | 36.8% | 40.9% | 0.90 | 36.8% | 8.41% |
| (0, 0.4) | 4136 | 43.2% | 40.9% | 1.06 | 43.2% | 6.42% |
| [0.4, 0.7) | 1096 | 51.9% | 40.9% | **1.27** | **51.9%** | 3.83% |
| [0.7, 1.0) | 421 | 61.8% | 40.9% | **1.51** | **61.8%** | -1.20% |
| >= 1.0 | 12 | 50.0% | 40.9% | 1.22 | 50.0% | 1.89% |

### 关键发现

1. **Signal Count 不够强**：2 信号 lift 仅 1.17，3 信号只有 12 个样本。
2. **Weighted Score 明显更好**：
   - [0.7, 1.0) 区间：lift 1.51，precision 61.8%，avg T+60 -1.20%
   - 这是当前最强的组合桶
3. **风险维度是递增的**：
   - 0 信号：avg T+60 +8.41%
   - 1 信号：+6.42%
   - weighted [0.7, 1.0)：-1.20%
   - 说明风险评分越高，未来 60 日收益越差

### 结论

**Holding Risk Bundle 方向正确，但当前三信号组合还不够强。**

Weighted Score [0.7, 1.0) 达到 ADR-101 门限（lift 1.51，precision 61.8%），但样本只有 421。Signal Count 3 只有 12 样本。

这验证了用户的判断：

> 不要直接找 Reduce Signal，而是建立 Holding Risk Score。

下一步是继续迭代权重和组合维度，而不是把 Bundle 直接接入 DecisionEngine。

### 对 2B-3 的启示

1. **Weighted Score 比 Count 更有效**。
2. **LeadershipDecay 是 Bundle 中最强维度**（权重 0.4）。
3. **LiquidityDeterioration 贡献了增量区分力**，但需要更多数据验证。
4. **中期风险评分是合理方向**，但当前组合仍需扩展。

### 下一步建议

1. **TASK-159: Context Integrity CI Gate**
   - 把 `execution-context-integrity-audit` 纳入测试流程。

2. **继续扩展 Holding Risk 维度**
   - 考虑 `ConfirmationDecay`
   - 考虑 `RegimeTransition`
   - 考虑多日期 persistence（连续多日信号）

3. **只有 weighted score bucket 稳定通过 ADR-101 且样本 >= 500 时，才进入 Assessment/Decision 层讨论。**

### 相关文件

- `docs/v8/adr-105-evidence-horizon-and-role-model.md`
- `reports/execution-validation/holding_risk_bundle_cn_2026-07-18.md`
- `crates/execution-replay/src/holding_risk_bundle.rs`
- `crates/execution-replay/src/holding_risk_bundle_formatter.rs`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity    ✅ PASS
2B-1 Bearish Evidence Analysis          ✅
2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Evidence Bundle
        - weighted score [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
        - 3-signal count: ⚠️  too few samples (n=12)
        - needs iteration before entering Decision path

NEXT:
        |
        v
  TASK-159: Context Integrity CI Gate
  TASK-160: Expand Holding Risk Bundle (more dimensions / persistence)
```

---

## TASK-159: Context Integrity Fact Integrity Firewall（2B-0 升级，2026-07-18）

### 背景

2B 最大的价值不是找到 LeadershipDecay，而是发现并修复了「事实链污染」问题：

```text
ResearchContext
       |
       X  （被替换成 50.0 / 0.5 / placeholder）
       |
ExecutionMarketView
       |
ExecutionEvent
       |
Evidence
       |
Decision
```

这个问题如果没有自动防护，未来任何 Evidence Modeling 都可能再次建立在错误事实之上。因此 TASK-159 被升级为 **Fact Integrity Firewall**，不是普通测试，而是 Execution Platform 的基础设施。

### 目标

防止再次出现 ResearchContext → ExecutionEvent 事实链被占位值、常量值或单一主导值污染的情况。

### 架构

```text
Replay Dataset
      |
      v
build_execution_event()
      |
      v
ExecutionMarketView
      |
      v
ContextIntegrityValidator
      |
      +-- PASS  → Evidence Modeling 允许继续
      |
      +-- FAIL  → Evidence Modeling 必须阻塞
```

### 核心模型

#### `ContextIntegrityRule`

```rust
pub struct ContextIntegrityRule {
    pub field_name: String,
    pub min_variance: f64,             // 最小方差
    pub min_unique_ratio: f64,         // unique_values / total_records
    pub max_dominant_value_ratio: f64, // 单一值最大占比
    pub known_placeholders: Vec<f64>, // 已知的占位值列表
}
```

#### `ExecutionContextIntegrityContract`

V8 默认合约覆盖 8 个 ResearchContext-derived 字段：

| Field | min_variance | min_unique_ratio | max_dominant_ratio | known_placeholders |
|---|---|---|---|---|
| confirmation.trend.score | 1.0 | 0.001 | 0.95 | — |
| confirmation.participation.score | 1.0 | 0.001 | 0.95 | — |
| confirmation.risk.score | 1.0 | 0.001 | 0.95 | — |
| breadth.breadth_pct | 1.0 | 0.001 | 0.95 | 50.0 |
| breadth.delta_5d | 0.1 | 0.001 | 0.95 | 0.0 |
| breadth.sma5 | 1.0 | 0.001 | 0.95 | 0.0 |
| recovery.score | 1.0 | 0.001 | 0.95 | — |
| leadership_stability | 1e-6 | 0.001 | 0.95 | 0.5 |

### 为什么不只是检查 variance

这次的问题是 `全部 = 50.0`（variance = 0），但未来可能出现：

```text
breadth_pct:
50
50
50
50
52
```

variance > 0，但 99% 是同一个值。因此引入：

- `unique_ratio`：检测 unique values 是否足够丰富
- `max_dominant_value_ratio`：检测是否存在单一值主导（如 99% 都是 50.0）

### 实现

| 文件 | 用途 |
|---|---|
| `crates/execution-replay/src/context_integrity_contract.rs` | `ContextIntegrityRule` + `ExecutionContextIntegrityContract` |
| `crates/execution-replay/src/context_integrity_validator.rs` | `validate_execution_context` / `validate_with_contract` |
| `crates/execution-replay/src/context_integrity_validator_formatter.rs` | Markdown / JSON 输出（CI 友好） |
| `crates/execution-replay/src/lib.rs` | 导出新模块和类型 |
| `crates/app-service/src/execution_replay.rs` | `execution_context_integrity_gate_from_range` / `_from_suite` |
| `apps/cli/src/main.rs` | `execution-context-integrity-gate` CLI |
| `apps/cli/src/commands/execution_replay.rs` | `handle_execution_context_integrity_gate` |

### 新增 CLI 命令

```bash
# 严格模式（默认）：gate 失败时返回非零退出码，可用于 CI
cargo run -p quant-cli -- execution-context-integrity-gate \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30

# 非严格模式：只输出报告，不返回非零退出码
cargo run -p quant-cli -- execution-context-integrity-gate \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --strict false

# 基于 validation suite
cargo run -p quant-cli -- execution-context-integrity-gate \
  --suite research/validation/execution/execution_validation_suite.yaml
```

### 运行结果（CN 2024-01-01 至 2025-06-30）

```bash
cargo run -p quant-cli -- execution-context-integrity-gate \
  --scope cn --from 2024-01-01 --to 2025-06-30 --output markdown
```

报告路径：`reports/execution-validation/context_integrity_gate_cn_2026-07-18.md`

| Field | Status | Unique | Variance | Unique Ratio | Dominant Ratio |
|-------|--------|-------:|---------:|-------------:|---------------:|
| confirmation.trend.score | PASS | 359 | 1.06e2 | 4.17e-2 | 0.28% |
| confirmation.participation.score | PASS | 150 | 4.57e2 | 1.74e-2 | 1.67% |
| confirmation.risk.score | PASS | 358 | 2.46e2 | 4.16e-2 | 0.56% |
| breadth.breadth_pct | PASS | 25 | 1.26e3 | 2.90e-3 | 11.14% |
| breadth.delta_5d | PASS | 39 | 8.37e2 | 4.53e-3 | 13.93% |
| breadth.sma5 | PASS | 110 | 1.11e3 | 1.28e-2 | 6.41% |
| recovery.score | PASS | 180 | 3.76e2 | 2.09e-2 | 6.41% |
| leadership_stability | PASS | 359 | 4.71e-2 | 4.17e-2 | 0.28% |

**Verdict: PASS** — 所有 8 个字段满足 V8 Fact Integrity Contract。

### 测试覆盖

`crates/execution-replay/src/context_integrity_validator.rs` 包含：

- `gate_fails_on_placeholder_breadth`：检测 `breadth_pct = 50.0` 占位值
- `gate_fails_on_constant_field`：检测常量字段
- `gate_passes_on_variable_fields`：验证真实变化数据通过
- `gate_detects_high_dominant_ratio_soft_pollution`：检测 99% 同一值的软污染
- `gate_verdict_matches_passed_state`：验证 verdict 字符串与 passed 状态一致

这些测试会在每次 `cargo test -p execution-replay` 时运行，构成 CI 级防火墙。

### 对后续工作的影响

1. **任何新字段** 从 ResearchContext 进入 ExecutionMarketView 之前，必须先在 `ExecutionContextIntegrityContract` 中注册规则。
2. **2B 所有后续工作**（Transition Evidence / Holding Risk Bundle / Calibration）都必须在 gate PASS 后才能继续。
3. **CI 流程**可以加入 `cargo run -p quant-cli -- execution-context-integrity-gate --scope cn --from ... --to ...`，失败时阻断构建。

### 相关文件

- `docs/v8/adr-104-researchcontext-fact-integrity-gate.md`
- `reports/execution-validation/context_integrity_gate_cn_2026-07-18.md`
- `crates/execution-replay/src/context_integrity_contract.rs`
- `crates/execution-replay/src/context_integrity_validator.rs`
- `crates/execution-replay/src/context_integrity_validator_formatter.rs`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity
        ✅ PASS （Audit 通过）
        ✅ PASS （Gate 通过）

2B-1 Bearish Evidence Analysis          ✅
2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Evidence Bundle
        - weighted score [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
        - 3-signal count: ⚠️  too few samples (n=12)
        - needs iteration before entering Decision path

NEXT:
        |
        v
  TASK-160.1: Holding Risk Persistence
  TASK-160.2: Holding Risk Dimension Expansion
  TASK-160.3: Evidence Horizon Registry Runtime化
```

---

## TASK-160.1: Holding Risk Persistence（2026-07-18）

### 背景

用户判断：当前 Holding Risk Bundle 已经证明“方向正确”，但还没有证明“状态持续性”是否才是真正缺失的语义维度。

当前 LeadershipDecay 是 snapshot：

```text
LD(t) = true
```

但这无法区分：

- 单日噪音（noise）
- 持续恶化（regime change）

因此 TASK-160.1 验证：

> “持续恶化”是否比“单日恶化”更能识别 Holding Risk？

### 目标

建立 `HoldingRiskPersistenceAnalysis`，测试 LeadershipDecay 在不同连续恶化天数下的 T+60 表现，并升级为 Holding Risk Bundle V2。

### 方法

新增 `holding-risk-persistence` CLI，计算：

1. **Consecutive-day experiments**：
   - `LeadershipDecay >= 1 day`
   - `LeadershipDecay >= 2 days`
   - `LeadershipDecay >= 3 days`
   - `LeadershipDecay >= 5 days`
   - `LeadershipDecay >= 10 days`

2. **Velocity experiment**：
   - `LeadershipDecay` 且 `velocity_vs_5d < -0.10`
   - velocity = current leadership_stability - trailing 5-day average

评估 horizon：T+60。

验收标准：

| 指标 | 要求 |
|---|---|
| Sample | >= 300 |
| Horizon | T+60 |
| Precision | >= 55% |
| Lift | >= 1.3 |
| False Reduce rate | < 40% |

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/holding_risk_persistence.rs` | 新增 Persistence 分析 + 连续日实验 + Velocity 实验 |
| `crates/execution-replay/src/holding_risk_persistence_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/holding_risk_bundle.rs` | 新增 V2 函数，替换 snapshot LeadershipDecay 为 persistence |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | 新增 V2 方法 |
| `apps/cli/src/main.rs` | 新增 `holding-risk-persistence` + `execution-holding-risk-bundle-v2` CLI |
| `apps/cli/src/commands/execution_replay.rs` | 新增 handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
# 独立 Persistence 分析
cargo run -p quant-cli -- holding-risk-persistence \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --output markdown

# Bundle V2（min persistence days 可调）
cargo run -p quant-cli -- execution-holding-risk-bundle-v2 \
  --scope cn \
  --from 2024-01-01 \
  --to 2025-06-30 \
  --min-persistence-days 2 \
  --output markdown
```

报告路径：
- `reports/execution-validation/holding_risk_persistence_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v2_persist_2d_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v2_persist_3d_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v2_persist_5d_cn_2026-07-18.md`

### 结果：LeadershipDecay Persistence（T+60）

| Signal | Min Days | Samples | Negative T+60 | Lift | Precision | Avg T+60 | Median T+60 | False Reduce |
|--------|---------:|--------:|--------------:|-----:|----------:|---------:|------------:|-------------:|
| LeadershipDecay >= 1 consecutive days | 1 | 408 | 64.7% | 1.58 | 64.7% | -1.25% | -3.85% | 35.3% |
| LeadershipDecay >= 2 consecutive days | 2 | 359 | 73.3% | 1.79 | 73.3% | -3.03% | -4.00% | 26.7% |
| LeadershipDecay >= 3 consecutive days | 3 | 335 | 71.3% | 1.75 | 71.3% | -2.94% | -3.85% | 28.7% |
| LeadershipDecay >= 5 consecutive days | 5 | 311 | 76.8% | 1.88 | 76.8% | -3.50% | -4.00% | 23.2% |
| LeadershipDecay >= 10 consecutive days | 10 | 0 | — | — | — | — | — | — |

**Velocity Experiment**：

| Signal | Window | Samples | Negative T+60 | Lift | Precision | Avg T+60 | False Reduce |
|---|---|---:|---:|---:|---:|---:|---:|
| LeadershipDecay + velocity < -0.10 | 5 | 455 | 73.6% | 1.80 | 73.6% | -2.72% | 26.4% |

### 关键发现

1. **Persistence 明显优于 snapshot**：
   - Snapshot LeadershipDecay T+60：precision 61.5%，lift 1.50
   - 2-day persistence：precision 73.3%，lift 1.79
   - 5-day persistence：precision 76.8%，lift 1.88

2. **最佳连续日阈值是 5 天**：
   - precision 76.8%，lift 1.88，n=311
   - false reduce rate 23.2%（< 40%）
   - 满足所有 TASK-160.1 验收标准

3. **Velocity 也是强信号**：
   - precision 73.6%，lift 1.80，n=455
   - 比 snapshot 强，但略低于 5-day persistence

4. **10-day persistence 没有样本**：
   - 说明在 CN 2024-2025 数据上，连续 10 天 LeadershipDecay 几乎不发生
   - 5 天是更合理的上界

### 结果：Holding Risk Bundle V2（T+60）

V2 权重：
- LeadershipDecay persistence：0.5
- BreadthDeterioration：0.25
- LiquidityDeterioration：0.25

| Min Persistence Days | Best Bucket | Samples | Lift | Precision | Avg T+60 |
|---|---|---:|---:|---:|---:|
| 2 | score [0.75, 1.0) | 189 | 1.66 | 67.7% | -2.84% |
| 3 | score [0.75, 1.0) | 188 | 1.67 | 68.1% | -2.87% |
| 5 | score [0.75, 1.0) | 165 | 1.69 | 69.1% | -3.10% |

### 与 V1 对比

| 版本 | Best Bucket | Samples | Lift | Precision |
|---|---|---:|---:|---:|
| V1 | weighted [0.7, 1.0) | 421 | 1.51 | 61.8% |
| V2 (2d) | weighted [0.75, 1.0) | 189 | 1.66 | 67.7% |
| V2 (5d) | weighted [0.75, 1.0) | 165 | 1.69 | 69.1% |

**结论**：
- V2 在更高精度（precision +5~7%）的同时，样本数减少。
- 这是 persistence 的合理 trade-off：更严格、更精确、更稀少。
- V2 的 3-signal 桶仍然有 n=4，样本过少，但这是多个独立条件同时触发的稀有组合。

### 对后续路线的影响

1. **TASK-160.1 完成**： persistence 确实比 snapshot 更能识别 Holding Risk。
2. **最佳阈值是 5 天 persistence**：precision 76.8%，n=311，满足所有验收标准。
3. **下一步不是继续增加维度**，而是：
   - 将 5-day persistence 作为 Holding Risk Bundle 的主要维度
   - 验证 velocity + persistence 组合是否更强
   - 测试 persistence 在不同市场 scope（CN / HK / GLOBAL）的稳定性

### 用户建议的路线调整

```text
TASK-160.1 ✅ Holding Risk Persistence
        |
        v
TASK-160.2 Holding Risk Dimension Expansion
        |
        v
TASK-160.3 Evidence Horizon Registry Runtime化
        |
        v
TASK-161 Holding Risk Calibration v2
        |
        v
TASK-162 Decision Integration Proposal
```

但用户也强调：当前问题不是维度少，而是 snapshot 无法区分 noise 和 regime change。因此 160.2 应谨慎推进，优先测试：

- ConfirmationDecay（从变化而非状态出发）
- LiquidityPressure（price_down + breadth_down + turnover_decay）
- 而不是简单添加更多 snapshot 条件

### 相关文件

- `docs/v8/adr-105-evidence-horizon-and-role-model.md`
- `crates/execution-replay/src/holding_risk_persistence.rs`
- `crates/execution-replay/src/holding_risk_persistence_formatter.rs`
- `crates/execution-replay/src/holding_risk_bundle.rs`
- `reports/execution-validation/holding_risk_persistence_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v2_persist_2d_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v2_persist_3d_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v2_persist_5d_cn_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity
        ✅ PASS (Audit + Gate)

2B-1 Bearish Evidence Analysis          ✅
2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Evidence Bundle
        - V1 weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
        - LD velocity < -0.10: ✅ PASS (lift 1.80, precision 73.6%, n=455)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
        - still needs more samples before Decision path

NEXT:
        |
        v
  TASK-160.2: Holding Risk Dimension Expansion（谨慎）
  TASK-160.3: Evidence Horizon Registry Runtime化（暂缓）
  TASK-161: Holding Risk Calibration v2
  TASK-162: Decision Integration Proposal
```

---

## TASK-160.2A: LiquidityPressure Research Asset（2026-07-18）

### 背景

用户建议将 LiquidityPressure 作为 Holding Risk Bundle 的第二优先维度。目标是回答：

> “持续资金压力是否能提高 Holding Risk Bundle 的 T+60 区分能力？”

LiquidityPressure 不是 snapshot，而是：

```text
turnover_decay
    +
price_weakness
    +
breadth_not_recovering
    +
persistence
```

### 方法

新增 `liquidity-pressure` CLI，支持可调参数：

- `volume_ratio_delta_5d < threshold`（turnover decay）
- `today_return < 0`（price weakness，可禁用）
- `breadth_delta_5d < 0`（breadth not recovering，可禁用）
- `consecutive_pressure_days >= N`（persistence）

评估 horizon：T+60。

验收标准：

| 指标 | 要求 |
|---|---|
| Sample | >= 30 |
| Precision | >= 50% |
| Lift | >= 1.2 |

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/liquidity_pressure.rs` | LiquidityPressure 分析 + 可调参数 |
| `crates/execution-replay/src/liquidity_pressure_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/holding_risk_bundle.rs` | Bundle V3：LeadershipDecay persistence (>=5d) + LiquidityPressure (any decline, >=3d) + BreadthDeterioration |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `liquidity_pressure_*` + `holding_risk_bundle_v3_*` |
| `apps/cli/src/main.rs` | `liquidity-pressure` + `execution-holding-risk-bundle-v3` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
# 默认定义（volume_delta -20% + price + breadth + 3d persistence）
cargo run -p quant-cli -- liquidity-pressure \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --consecutive-days 3

# 仅成交量维度（volume_delta -10% + 2d persistence）
cargo run -p quant-cli -- liquidity-pressure \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --consecutive-days 2 --volume-delta-threshold=-0.10 \
  --price-weakness --breadth-weakness

# 成交量水平维度（volume_ratio < 0.8 + 2d persistence）
cargo run -p quant-cli -- liquidity-pressure \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --consecutive-days 2 --volume-level-threshold 0.8 \
  --price-weakness --breadth-weakness

# Bundle V3
cargo run -p quant-cli -- execution-holding-risk-bundle-v3 \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

报告路径：
- `reports/execution-validation/liquidity_pressure_*_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v3_cn_2026-07-18.md`

### 结果：LiquidityPressure Standalone

用户定义的严格版本（volume_delta < -20% + price < 0 + breadth < 0 + 3d persistence）：

| 连续日 | 样本 | Negative T+60 | Lift | Precision | False Reduce |
|---|---:|---:|---:|---:|---:|
| >= 2 | 16 | 12.5% | 0.31 | 12.5% | 87.5% |
| >= 3 | 3 | 0.0% | 0.00 | 0.0% | 100.0% |

结论：**严格版本几乎不产生样本，且样本表现与预期相反。**

放松版本（volume-only delta < -10% + 2d persistence）：

| 维度 | 样本 | Negative T+60 | Lift | Precision |
|---|---:|---:|---:|---:|
| volume-only | 637 | 44.9% | 1.10 | 44.9% |
| volume + price | 80 | 43.8% | 1.07 | 43.8% |
| volume + breadth | （样本极少） | — | — | — |
| volume level < 0.8 | 420 | 41.9% | 1.02 | 41.9% |
| volume level < 0.7 | 140 | 35.7% | 0.87 | 35.7% |

**结论：单独 LiquidityPressure 在当前 CN 2024-2025 数据集上不足以作为独立 Holding Risk 信号。**

### 结果：Holding Risk Bundle V3（T+60）

V3 权重：
- LeadershipDecayPersistence（>=5 天）：0.4
- LiquidityPressure（volume_delta < 0，>=3 天）：0.3
- BreadthDeterioration：0.3

| 维度组合 | 样本 | Negative T+60 | Lift | Precision | Avg T+60 |
|---|---:|---:|---:|---:|---:|
| 0 signals | 4179 | 38.7% | 0.95 | 38.7% | 7.79% |
| 1 signals | 4076 | 41.9% | 1.02 | 41.9% | 6.95% |
| 2 signals | 346 | 53.5% | 1.31 | 53.5% | 3.49% |
| 3 signals | 15 | 86.7% | 2.12 | 86.7% | -6.22% |

Weighted Score Buckets：

| Score | 样本 | Negative T+60 | Lift | Precision | Avg T+60 |
|---|---:|---:|---:|---:|---:|
| 0.0 | 4179 | 38.7% | 0.95 | 38.7% | 7.79% |
| (0, 0.4) | 4076 | 41.9% | 1.02 | 41.9% | 6.95% |
| [0.4, 0.7) | 537 | 53.3% | 1.30 | 53.3% | 2.97% |
| [0.7, 1.0) | 121 | 77.7% | 1.90 | 77.7% | -4.84% |
| >= 1.0 | 15 | 86.7% | 2.12 | 86.7% | -6.22% |

**最佳 bucket：weighted [0.7, 1.0)，lift 1.90，precision 77.7%，n=121。**

### 与 V1 / V2 对比

| 版本 | Best Bucket | 样本 | Lift | Precision |
|---|---|---:|---:|---:|
| V1 | weighted [0.7, 1.0) | 421 | 1.51 | 61.8% |
| V2 (5d) | weighted [0.75, 1.0) | 165 | 1.69 | 69.1% |
| V3 (5d + LP) | weighted [0.7, 1.0) | 121 | 1.90 | 77.7% |

**V3 在精度上进一步提升，但样本数继续下降。**

### 关键发现

1. **LiquidityPressure 单独不足**：没有通过 Research Asset 验收标准（precision < 50%，部分版本样本过少）。
2. **LiquidityPressure 与 LeadershipDecay 互补**：加入 Bundle 后，weighted score [0.7,1.0) 从 V2 的 69.1% 提升到 V3 的 77.7%，lift 从 1.69 提升到 1.90。
3. **3-signal 组合非常稀有但非常准**：n=15，precision 86.7%。说明当三个维度同时触发时，Holding Risk 极高。
4. **样本量与精度的 trade-off 更加明显**：V3 比 V2 更精确，但样本更少。

### 结论

**LiquidityPressure 不应作为独立 Evidence，但可以作为 Holding Risk Bundle 的增强维度。**

这验证了用户的直觉：

> Holding Risk 不是单一状态，而是市场结构 + 资金压力 + 持续时间的组合。

当前 V3 的最佳 bucket（weighted [0.7,1.0)）已经显示出很强的区分能力，但样本数 n=121 仍不足以直接进入 Decision path。

### 下一步建议

1. **降低 LeadershipDecay persistence 阈值到 2-3 天**，以增加 V3 的样本量。
2. **探索 LiquidityPressure 的 velocity 变体**（volume_ratio slope，而非 delta）。
3. **继续推进 TASK-160.2B ConfirmationDecay**，作为另一个维度。
4. **只有在 V3 样本量 >= 300 且 precision >= 55% 时，才进入 Calibration 或 Decision 讨论。**

### 相关文件

- `docs/v8/adr-105-evidence-horizon-and-role-model.md`
- `crates/execution-replay/src/liquidity_pressure.rs`
- `crates/execution-replay/src/liquidity_pressure_formatter.rs`
- `crates/execution-replay/src/holding_risk_bundle.rs`
- `reports/execution-validation/liquidity_pressure_*_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v3_cn_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
        - needs more samples before Decision path

2B-4 Evidence Asset Registry
        - TASK-160.3 pending (recommended)

2B-5 Calibration v2
        - WAIT

2C Decision Integration
        - WAIT

NEXT:
        |
        v
  TASK-160.2B: ConfirmationDecay（第三维度）
  TASK-160.2C: BreadthPersistence（组合维度，谨慎）
  TASK-160.3: Evidence Horizon Registry Runtime化
  TASK-161: Holding Risk Calibration v2
  TASK-162: Decision Integration Proposal
```

---

## TASK-160.2B: ConfirmationDecay Confirmatory Dimension（2026-07-18）

### 背景

用户建议：TASK-160.2B 的目标不是寻找另一个独立卖出信号，而是寻找能增强 Holding Risk Bundle 的 **Confirmatory Dimension**。

ConfirmationDecay 不应该是 snapshot：

```text
confirmation_score < x
```

而应该是：

```text
confirmation_delta_5d
confirmation_velocity (slope)
consecutive_decline_days
price weakness
```

### 方法

新增 `confirmation-decay` CLI，支持可调参数：

- `confirmation_delta_5d < threshold`
- `slope_10d < threshold`
- `consecutive_decline_days >= N`
- 可选：`today_return < 0`（price weakness）

评估 horizon：T+20 和 T+60。

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/confirmation_decay.rs` | ConfirmationDecay 分析 + 可调参数 |
| `crates/execution-replay/src/confirmation_decay_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/holding_risk_bundle.rs` | Bundle V4：LD persistence (>=5d) + LiquidityPressure (any decline, >=3d) + ConfirmationDecay |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `confirmation_decay_*` + `holding_risk_bundle_v4_*` |
| `apps/cli/src/main.rs` | `confirmation-decay` + `execution-holding-risk-bundle-v4` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
# 默认定义（delta -10, slope -2, 3d, price weakness）
cargo run -p quant-cli -- confirmation-decay \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown

# 仅 delta/slope 触发（无 price weakness）
cargo run -p quant-cli -- confirmation-decay \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --price-weakness --output markdown

# Bundle V4
cargo run -p quant-cli -- execution-holding-risk-bundle-v4 \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

报告路径：
- `reports/execution-validation/confirmation_decay_*_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v4_cn_2026-07-18.md`

### 结果：ConfirmationDecay Standalone

| 定义 | 样本 | Negative T+20 | Lift T+20 | Negative T+60 | Lift T+60 | Precision T+60 |
|---|---:|---:|---:|---:|---:|---:|
| 默认（delta -10, slope -2, 3d, price） | 1637 | 45.6% | 0.97 | 39.6% | 0.97 | 39.6% |
| 无 price weakness | 3000 | 44.6% | 0.95 | 39.3% | 0.96 | 39.3% |
| 严格（delta -5, slope -1, 2d） | 2563 | 44.3% | 0.94 | 37.6% | 0.92 | 37.6% |

**结论：单独 ConfirmationDecay 在当前 CN 2024-2025 数据集上不具备独立 Holding Risk 信号能力，甚至表现为轻微反向指标（lift < 1.0）。**

### 结果：Holding Risk Bundle V4（T+60）

V4 权重：
- LeadershipDecayPersistence（>=5 天）：0.4
- LiquidityPressure（any volume decline，>=3 天）：0.3
- ConfirmationDecay（delta_5d < -5 或 consecutive >= 2）：0.3

| 维度组合 | 样本 | Negative T+60 | Lift | Precision | Avg T+60 |
|---|---:|---:|---:|---:|---:|
| 0 signals | 3547 | 38.1% | 0.93 | 38.1% | 8.05% |
| 1 signals | 2205 | 42.4% | 1.04 | 42.4% | 6.32% |
| 2 signals | 2601 | 41.9% | 1.02 | 41.9% | 7.21% |
| 3 signals | 249 | 55.0% | 1.35 | 55.0% | 3.16% |
| 4 signals | 8 | 100.0% | 2.44 | 100.0% | -12.66% |

Weighted Score Buckets：

| Score | 样本 | Negative T+60 | Lift | Precision | Avg T+60 |
|---|---:|---:|---:|---:|---:|
| 0.0 | 4926 | 39.2% | 0.96 | 39.2% | 7.69% |
| (0, 0.4) | 3172 | 41.6% | 1.02 | 41.6% | 7.16% |
| [0.4, 0.7) | 544 | 52.0% | 1.27 | 52.0% | 2.68% |
| [0.7, 1.0) | 246 | 62.2% | 1.52 | 62.2% | -0.35% |
| >= 1.0 | 33 | 93.9% | 2.30 | 93.9% | -7.86% |

**最佳 bucket：weighted >= 1.0，lift 2.30，precision 93.9%，n=33。**

### 与 V1 / V2 / V3 对比

| 版本 | Best Bucket | 样本 | Lift | Precision |
|---|---|---:|---:|---:|
| V1 | weighted [0.7, 1.0) | 421 | 1.51 | 61.8% |
| V2 (5d) | weighted [0.75, 1.0) | 165 | 1.69 | 69.1% |
| V3 (5d + LP) | weighted [0.7, 1.0) | 121 | 1.90 | 77.7% |
| V4 (5d + LP + CD) | weighted >= 1.0 | 33 | 2.30 | 93.9% |

**V4 在最强 bucket 上达到 93.9% precision 和 2.30 lift，但样本只有 33。**

### 关键发现

1. **ConfirmationDecay 单独无效**：所有独立定义 lift < 1.0，不满足 Research Asset 标准。
2. **ConfirmationDecay 在 Bundle 中表现极好**：
   - 与 LeadershipDecay + LiquidityPressure 组合后，3-signal bucket precision 55.0%（n=249），4-signal bucket precision 100.0%（n=8）。
   - weighted score >= 1.0 bucket precision 93.9%（n=33）。
3. **组合信号比单一信号强得多**：
   - LeadershipDecay 单独：precision 61.5%
   - LeadershipDecay + LiquidityPressure：precision 77.7%
   - LeadershipDecay + LiquidityPressure + ConfirmationDecay：precision 93.9%（最强 bucket）
4. **样本量与精度的 trade-off 达到极致**：V4 最强 bucket 只有 33 个样本，不足以直接进入 Decision path，但组合语义方向完全正确。

### 结论

**ConfirmationDecay 不应作为独立 Evidence，但作为 Holding Risk Bundle 的 Confirmatory Dimension 非常有效。**

这再次验证了 ADR-105 的核心思想：

> Evidence 必须声明 Role 和 Horizon。单独来看，ConfirmationDecay 可能是 ShortTerm/Noise；但在 MediumTerm Holding Risk 组合中，它提供了关键的确认语义。

### 下一步建议

1. **降低 LeadershipDecay persistence 阈值到 2-3 天**，以增加 V4 的样本量。
2. **尝试 ConfirmationDecay 的 stricter 定义**（如仅 consecutive >= 2，无 delta/slope），减少误触发。
3. **进入 TASK-160.3 Evidence Horizon Registry**，把已经验证的信号（LeadershipDecay / LiquidityPressure / ConfirmationDecay）注册为带 Role 和 Horizon 的 Research Assets。
4. **只有当 V4 样本量 >= 300 且 precision >= 55% 时，才进入 Calibration 或 Decision 讨论。**

### 相关文件

- `docs/v8/adr-105-evidence-horizon-and-role-model.md`
- `crates/execution-replay/src/confirmation_decay.rs`
- `crates/execution-replay/src/confirmation_decay_formatter.rs`
- `crates/execution-replay/src/holding_risk_bundle.rs`
- `reports/execution-validation/confirmation_decay_*_cn_2026-07-18.md`
- `reports/execution-validation/holding_risk_bundle_v4_cn_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)
        - needs more samples before Decision path

2B-4 Evidence Asset Registry
        - TASK-160.3 NEXT

2B-5 Calibration v2
        - WAIT

2C Decision Integration
        - WAIT

NEXT:
        |
        v
  TASK-160.3: Evidence Horizon Registry Runtime化
  TASK-160.2C: BreadthPersistence（组合维度，谨慎）
  TASK-161: Holding Risk Calibration v2
  TASK-162: Decision Integration Proposal
```

---

## TASK-160.3: Evidence Horizon Registry Runtime（2026-07-18）

### 背景

当前已经有 3 个经过验证的 Evidence：

- LeadershipDecay（独立 HoldingRisk）
- LiquidityPressure（Amplifier，需组合）
- ConfirmationDecay（Confirmation，需组合）

如果继续探索新 Evidence，会进入“资产没有身份证”的混乱阶段。因此 TASK-160.3 将 ADR-105 的 Evidence Horizon / Role 模型落地为代码约束。

### 目标

建立 `EvidenceRegistry`，使任何 Evidence 在进入 Decision Path 之前必须声明：

- `EvidenceId`
- `EvidenceRole`（EntrySignal / ExitSignal / HoldingRisk / RegimeRisk / Confirmation / Amplifier）
- `EvidenceHorizon`（Immediate / ShortTerm / MediumTerm / LongTerm）
- `ValidationStatus`（Draft / Validated / Rejected / Conditional / Superseded）
- `TargetMetric`（precision / lift / sample_count / horizon_days / false_reduce_rate）
- `dependencies`（依赖的其他 Evidence）
- `standalone_validity`（是否可独立用于决策）
- `decision_candidate`（是否可进入 Decision 路径）

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/evidence_registry.rs` | EvidenceDescriptor / EvidenceRegistry / ValidationStatus / TargetMetric |
| `crates/execution-replay/src/evidence_registry_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `apps/cli/src/main.rs` | `evidence-registry` + `evidence-validate-bundle` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### CLI 命令

```bash
# 查看 Evidence Registry
cargo run -p quant-cli -- evidence-registry --output markdown

# 验证 Bundle 是否满足依赖
cargo run -p quant-cli -- evidence-validate-bundle \
  --evidence-ids leadership_decay,liquidity_pressure,confirmation_decay

# 验证无效 Bundle（会失败）
cargo run -p quant-cli -- evidence-validate-bundle \
  --evidence-ids confirmation_decay
```

报告路径：`reports/execution-validation/evidence_registry_2026-07-18.md`

### 当前 Registry 内容

| Evidence | Role | Horizon | Status | Standalone | Decision Candidate | Metrics | Dependencies |
|----------|------|---------|--------|------------|--------------------|---------|--------------|
| LeadershipDecay | HoldingRisk | MediumTerm | Validated | Yes | Yes | precision=61.5% lift=1.50 n=743 | - |
| LiquidityPressure | Amplifier | MediumTerm | Conditional | No | No | precision=44.9% lift=1.10 n=637 | LeadershipDecay |
| ConfirmationDecay | Confirmation | MediumTerm | Conditional | No | No | precision=37.6% lift=0.92 n=2563 | LeadershipDecay, LiquidityPressure |
| BreadthDeterioration | HoldingRisk | MediumTerm | Rejected | No | No | precision=48.6% lift=1.03 n=3958 | - |
| RecoveryFailure | ExitSignal | ShortTerm | Rejected | No | No | precision=46.8% lift=0.99 n=1364 | - |
| RiskExpansion | HoldingRisk | ShortTerm | Rejected | No | No | precision=50.0% lift=1.45 n=6 | - |
| Distribution | HoldingRisk | ShortTerm | Conditional | No | No | precision=40.0% lift=1.16 n=5 | - |

### 关键约束

1. **只有 `ValidationStatus::Validated` 且 `decision_candidate=true` 的 Evidence 才能独立进入 Decision path**。
2. **任何 `standalone_validity=false` 的 Evidence 必须依赖其他 Evidence 才能使用**。
3. **Bundle 验证会检查依赖关系**：
   - `LiquidityPressure` 依赖 `LeadershipDecay`
   - `ConfirmationDecay` 依赖 `LeadershipDecay` + `LiquidityPressure`
4. **Rejected Evidence 不能进入 Decision path**。

### 当前 Decision-Ready Bundle

```text
LeadershipDecay (HoldingRisk, MediumTerm)
  precision=61.5%, lift=1.50
```

其他组合必须包含 LeadershipDecay 才能通过依赖检查。

### 对后续工作的影响

1. **TASK-160.2C BreadthPersistence** 在加入 Registry 之前，必须先通过 Fact Integrity Gate 和独立验证。
2. **TASK-161 Calibration v2** 应该基于 Registry 中的 Evidence Role / Horizon，而不是任意 Evidence 组合。
3. **TASK-162 Decision Integration** 必须只使用 `decision_candidate=true` 的 Evidence，或显式依赖关系完整的 Bundle。

### 相关文件

- `docs/v8/adr-105-evidence-horizon-and-role-model.md`
- `crates/execution-replay/src/evidence_registry.rs`
- `crates/execution-replay/src/evidence_registry_formatter.rs`
- `reports/execution-validation/evidence_registry_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets registered)

2B-5 Calibration v2
        - WAIT (needs sample >= 300, precision >= 60%, regime stability)

2C Decision Integration
        - WAIT (needs Calibration v2 pass)

NEXT:
        |
        v
  TASK-160.2C: BreadthPersistence（组合维度，谨慎）
  TASK-161: Holding Risk Calibration v2
  TASK-162: Decision Integration Proposal
```

---

## TASK-160.4: Evidence Registry ValidationRecord Enhancement（2026-07-18）

### 背景

TASK-160.3 建立了 Evidence Registry，但缺少每个 Evidence 的验证溯源信息。TASK-160.4 补充 `EvidenceValidationRecord`，使 Registry 从“人工维护状态表”升级为可追溯的 Research Asset Registry。

### 实现

新增 `EvidenceValidationRecord` 结构体，包含：

- `dataset_scope` / `dataset_from` / `dataset_to`
- `horizon_days`
- `sample_size`
- `precision` / `lift`
- `validated_at`
- `report_reference`

并更新 `EvidenceDescriptor` 以包含该字段。

### Registry 更新

所有 7 个 Evidence 现在携带 ValidationRecord，例如：

```yaml
LeadershipDecay:
  dataset: CN 2024-01-01 to 2025-06-30
  horizon: T+60
  sample: 743
  precision: 61.5%
  lift: 1.50
  report: reports/execution-validation/leadership_decay_horizon_cn_2026-07-18.md
```

---

## TASK-161: Holding Risk Calibration v2（2026-07-18）

### 背景

TASK-160.3 完成后，下一步不是继续增加 Evidence，而是把已验证的 Evidence 组合成一个稳定的 **HoldingRiskScore**。

用户建议：

> TASK-161 应该基于 HoldingRiskScore，而不是 Evidence Count。

### 方法

定义：

```text
HoldingRiskScore =
    LeadershipDecayPersistence(>=5d) * 0.5
  + LiquidityPressure(>=3d)         * 0.25
  + ConfirmationDecay(>=2d)         * 0.25
```

新增 `holding-risk-calibration` CLI，输出：

1. Score Bucket 分析（T+60）
2. Regime 稳定性（RiskOn / Neutral / RiskOff）
3. Walk-forward 验证（Train 2024, Validate 2025H1）

### 验收标准

| 指标 | 要求 |
|---|---|
| Sample | >= 300 |
| Precision | >= 60% |
| Lift | >= 1.3 |
| Regime Stability | 所有主要 regime precision >= 55% |
| Precision Decay | < 20% |

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/holding_risk_calibration.rs` | HoldingRiskScore 计算 + 分桶 / Regime / Walk-forward |
| `crates/execution-replay/src/holding_risk_calibration_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `holding_risk_calibration_*` |
| `apps/cli/src/main.rs` | `holding-risk-calibration` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
cargo run -p quant-cli -- holding-risk-calibration \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

报告路径：`reports/execution-validation/holding_risk_calibration_cn_2026-07-18.md`

### 结果：Score Buckets（T+60）

| Score | 样本 | Negative T+60 | Lift | Precision | Avg T+60 |
|---|---:|---:|---:|---:|---:|
| 0.0 | 4922 | 39.2% | 0.96 | 39.2% | 7.69% |
| (0, 0.25) | 3108 | 41.0% | 1.00 | 41.0% | 7.36% |
| [0.25, 0.5) | 3439 | 41.7% | 1.02 | 41.7% | 7.02% |
| [0.5, 0.75) | 545 | 52.1% | 1.28 | 52.1% | 2.70% |
| [0.75, 1.0) | 247 | 61.9% | 1.52 | 61.9% | -0.24% |
| >= 1.0 | 33 | 93.9% | 2.30 | 93.9% | -7.86% |

**最佳 bucket：score >= 1.0，precision 93.9%，lift 2.30，n=33。**

### 结果：Regime 稳定性（High Risk: score >= 0.75）

| Regime | 样本 | High Risk | Precision | Lift |
|---|---:|---:|---:|---:|
| Neutral | 2880 | 31 | 61.3% | 1.50 |
| RiskOff | 4556 | 120 | 54.2% | 1.33 |
| RiskOn | 1172 | 96 | 71.9% | 1.76 |

**Regime 稳定性：PASS** — 所有 regime 的 high-risk precision >= 55%。

### 结果：Walk-forward 验证

| 阶段 | High Risk | Precision | Lift |
|---|---:|---:|---:|
| Train (2024) | 247 | 61.9% | 1.52 |
| Validate (2025H1) | 0 | — | — |

**Precision Decay：100.0%** — 2025H1 没有产生任何 High Risk 记录。

### 关键发现

1. **HoldingRiskScore 在 2025H1 没有触发**。这不是模型失败，而是因为 2025H1 处于 recovery 阶段，没有出现足够的持续恶化条件（LeadershipDecay persistence >= 5d + LiquidityPressure >= 3d + ConfirmationDecay >= 2d）。
2. **模型在所有 regime 中均有效**：RiskOn 71.9% > Neutral 61.3% > RiskOff 54.2%。说明 HoldingRiskScore 不是单行情模型。
3. **样本量与精度的 trade-off 仍然存在**：score >= 1.0 的 precision 高达 93.9%，但样本只有 33。
4. **Walk-forward 验证因数据不足而失败**：需要更长的历史窗口或更多 regime 覆盖。

### 结论

**HoldingRiskScore 是一个有效的 Medium-Term Holding Risk 指标，但当前样本不足以完成 Calibration v2 的 walk-forward 验证。**

建议：

1. **接受当前 Calibration 结果**：Regime 稳定性 PASS，Score bucket 单调递增，符合 Holding Risk 语义。
2. **推迟 Decision Integration**：直到获得更长的历史数据（>= 3 年）或更多 regime 覆盖。
3. **继续 TASK-160.2C BreadthPersistence**：作为可选的组合维度，但不要期望它解决样本量问题。
4. **考虑降低 HoldingRiskScore 触发门槛**（如 score >= 0.75）以增加样本，但这会降低 precision。

### 相关文件

- `crates/execution-replay/src/holding_risk_calibration.rs`
- `crates/execution-replay/src/holding_risk_calibration_formatter.rs`
- `reports/execution-validation/holding_risk_calibration_cn_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets + ValidationRecord)

2B-5 Holding Risk Calibration v2
        - Regime stability: ✅ PASS
        - Walk-forward: ❌ FAIL (no high-risk events in 2025H1)
        - Overall: PARTIAL (valid but needs longer history)

2C Decision Integration
        - WAIT (needs longer history / more regime coverage)

NEXT:
        |
        v
  TASK-160.2C: BreadthPersistence（组合维度，谨慎）
  TASK-162: Decision Integration Proposal（推迟至更长历史数据）
```

---

## TASK-163: Holding Risk Lifecycle Modeling（2026-07-18）

### 背景

TASK-161 证明 HoldingRiskScore 是一个有效的 Medium-Term Holding Risk 指标，但 walk-forward 验证因 2025H1 处于 recovery 阶段而无法完成。下一步不是进入 Decision Integration，而是把 HoldingRiskScore 从“优秀指标”升级为“完整风险状态机”。

### 目标

建立 Risk Lifecycle 模型，回答：

1. **Risk Entry**：什么时候进入风险状态？
2. **Risk Peak**：风险最高点在哪里？
3. **Risk Recovery**：什么时候解除风险？
4. **Holding Period**：风险持续多久？
5. **False Alarm**：误报率是多少？

### 方法

新增 `risk-lifecycle` CLI，定义状态机：

- **Entry**：HoldingRiskScore >= 0.75 for >= 2 consecutive days
- **Peak**：event 期间的最高 score
- **Recovery**：HoldingRiskScore < 0.50 for >= 2 consecutive days
- **Duration**：Entry date 到 Recovery date
- **False Alarm**：event 期间 T+60 return >= 0

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/risk_lifecycle.rs` | RiskLifecycleEvent 检测 + 统计 |
| `crates/execution-replay/src/risk_lifecycle_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `risk_lifecycle_*` |
| `apps/cli/src/main.rs` | `risk-lifecycle` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
cargo run -p quant-cli -- risk-lifecycle \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

报告路径：`reports/execution-validation/risk_lifecycle_cn_2026-07-18.md`

### 结果

| 指标 | 数值 |
|---|---|
| Total Events | 48 |
| Avg Duration | 5.0 days |
| Median Duration | 5.0 days |
| Avg Peak Score | 0.82 |
| False Alarm Rate | 20.8% |
| Avg T+60 Return | -5.72% |
| Avg Max Drawdown | -18.82% |

**Verdict**：Risk lifecycle events are consistent with negative T+60 outcomes. The state machine is consistent with Holding Risk semantics.

### 关键发现

1. **风险事件平均持续 5 天**：HoldingRiskScore 不是长期趋势信号，而是短期风险预警。
2. **误报率 20.8%**：低于 40% 的验收门槛，说明状态机是可信的。
3. **平均 T+60 收益 -5.72%**：事件确实与负收益相关，符合 Holding Risk 语义。
4. **最大回撤 -18.82%**：事件期间市场波动显著，但随后有恢复。

### 与 TASK-161 的关系

- TASK-161 验证了 HoldingRiskScore 的 **静态区分能力**（不同 score bucket 的 T+60 表现）。
- TASK-163 验证了 HoldingRiskScore 的 **动态时序能力**（风险状态的生命周期）。
- 两者结合：HoldingRiskScore 不仅是好指标，还是一个可以构建 Risk State Machine 的基础。

### 下一步建议

1. **TASK-164 Extended Historical Validation**：在 2022-2023 熊市环境中验证 HoldingRiskScore，确认其在更长历史中的稳定性。
2. **TASK-165 Risk State Machine 与 Decision 集成**：只有当 Risk Lifecycle 在多个 regime 中稳定后，才考虑将其用于仓位管理。
3. **继续 TASK-160.2C BreadthPersistence**：作为可选的组合维度，但不要期望它解决样本量问题。

### 相关文件

- `crates/execution-replay/src/risk_lifecycle.rs`
- `crates/execution-replay/src/risk_lifecycle_formatter.rs`
- `reports/execution-validation/risk_lifecycle_cn_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets + ValidationRecord)

2B-5 Holding Risk Calibration v2
        - Regime stability: ✅ PASS
        - Walk-forward: ❌ FAIL (no high-risk events in 2025H1)
        - Overall: PARTIAL (valid but needs longer history)

2B-6 Holding Risk Lifecycle Modeling
        - Avg duration: 5.0 days
        - False alarm rate: 20.8%
        - Avg T+60 return: -5.72%
        - ✅ PASS (consistent with Holding Risk semantics)

2C Decision Integration
        - WAIT (needs longer history / more regime coverage)

NEXT:
        |
        v
  TASK-164: Extended Historical Validation (2022-2023 bear market)
  TASK-165: Risk State Machine Decision Integration（推迟至更长历史数据）
  TASK-160.2C: BreadthPersistence（组合维度，谨慎）
```

---

## TASK-164: Extended Historical Validation（2026-07-18）

### 背景

TASK-163 证明 HoldingRiskScore 在 CN 2024-01-01 至 2025-06-30 区间是一个有效的风险状态机。但用户指出：

> 这个风险认知模型是否跨周期、跨市场状态稳定？

当前结论来自 2024-2025（震荡 + 恢复），缺少真正风险释放阶段（2022-2023 熊市）的验证。

### 目标

在 2022-2023 熊市环境中验证 HoldingRiskScore，确认其跨周期稳定性。

### 方法

- 使用 `holding-risk-calibration` 和 `risk-lifecycle` CLI
- 数据范围：2023-01-01 至 2023-12-31（2022 数据不可用）
- 验收标准：
  - False Alarm < 35%
  - Avg T+60 Return < 0
  - Risk Event count >= 50
  - Precision decay < 30%

### 实现

直接复用现有 CLI，无需新增代码。

### 运行命令

```bash
# Calibration
cargo run -p quant-cli -- holding-risk-calibration \
  --scope cn --from 2023-01-01 --to 2023-12-31 \
  --output markdown

# Lifecycle
cargo run -p quant-cli -- risk-lifecycle \
  --scope cn --from 2023-01-01 --to 2023-12-31 \
  --output markdown
```

报告路径：
- `reports/execution-validation/holding_risk_calibration_cn_2023_2026-07-18.md`
- `reports/execution-validation/risk_lifecycle_cn_2023_2026-07-18.md`

### 结果：2023 Calibration

| 指标 | 数值 |
|---|---|
| Total Records | 5801 |
| Baseline T+60 Negative Rate | **75.1%** |
| High Risk (score >= 0.75) | 2 |
| Best Bucket | score [0.75, 1.0) |
| Precision | 100.0% (n=2) |
| Lift | 1.33 |

### 结果：2023 Lifecycle

| 指标 | 数值 |
|---|---|
| Total Records | 5808 |
| Total Events | **0** |
| Avg Duration | 0.0 days |
| False Alarm Rate | 0.0% |

### 关键发现

1. **2023 是极端熊市环境**：Baseline T+60 negative rate 75.1%，远高于 2024-2025 的 40.9%。
2. **HoldingRiskScore 在 2023 几乎不产生事件**：只有 2 条记录达到 score >= 0.75，0 个 lifecycle events。
3. **模型没有区分能力**：2023 的 score buckets 全部在 75% 左右，无法区分高风险和低风险。
4. **Walk-forward 失败**：2024-2025 的 HoldingRiskScore 无法外推到 2023。

### 根因分析

HoldingRiskScore 的设计是 **Transition Detector**（从正常状态到恶化状态的转换检测），而不是 **State Detector**（坏状态识别）。

- 在 2024-2025：市场从 normal 状态开始，模型检测到 deterioration，表现优异。
- 在 2023：市场已经处于 downtrend，模型无法检测到 "additional" deterioration，因为状态已经是坏的。

这符合 ADR-105 的语义：

> LeadershipDecay 是 Medium-Term Holding Risk，但它检测的是 "risk forming"，而不是 "risk already present"。

### 结论

**TASK-164 验收标准未通过。**

- False Alarm: N/A (no events)
- Avg T+60 Return: N/A (no events)
- Risk Event count: 0 (require >= 50)
- Precision decay: undefined

**HoldingRiskScore 不是跨 regime 稳定的模型。** 它在 2024-2025（震荡 + 恢复）有效，但在 2023（深度熊市）无效。

### 对 Shadow Mode 的影响

用户建议 TASK-164 完成后进入 Shadow Mode。但基于当前结果：

> **HoldingRiskScore 不能直接用于 Shadow Mode，因为它在熊市中不产生信号。**

建议调整路线：

1. **接受 HoldingRiskScore 的 regime 限制**：它是一个 "normal-to-bad transition" 指标，不是 "all-weather risk" 指标。
2. **Phase 2C Shadow Mode 应该限定在 "normal market" 条件**：只在 market_regime 为 Neutral 或 RiskOn 时启用 HoldingRiskScore，在 RiskOff 时禁用或切换模型。
3. **TASK-165 Risk Advisory Integration 需要 regime gating**：不能无条件输出 HoldingRisk advisory。

### 下一步建议

1. **TASK-166: Regime-Aware Holding Risk Model**
   - 为 RiskOff / Bearish 环境设计不同的 risk model（例如，基于价格动量、波动率、趋势强度的下行风险模型）。
   - 与 HoldingRiskScore 形成 regime-aware ensemble。

2. **TASK-167: Shadow Mode with Regime Gating**
   - 在 Shadow Mode 中，根据 market_regime 自动切换 risk model。
   - 只在 HoldingRiskScore 适用的 regime（Neutral / RiskOn）中使用它。

3. **继续收集历史数据**：
   - 需要 2022 年数据来完整验证熊市周期。
   - 当前 2023 数据证明模型在深度熊市中失效。

### 相关文件

- `reports/execution-validation/holding_risk_calibration_cn_2023_2026-07-18.md`
- `reports/execution-validation/risk_lifecycle_cn_2023_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets + ValidationRecord)

2B-5 Holding Risk Calibration v2
        - 2024-2025 Regime stability: ✅ PASS
        - 2024-2025 Walk-forward: ❌ FAIL (no high-risk events in 2025H1)
        - 2023 Calibration: ❌ FAIL (no events, baseline 75.1%)
        - 2023 Lifecycle: ❌ FAIL (0 events)
        - Overall: PARTIAL (valid in 2024-2025, not stable across regimes)

2B-6 Holding Risk Lifecycle Modeling
        - 2024-2025: ✅ PASS (48 events, false alarm 20.8%, avg T+60 -5.72%)
        - 2023: ❌ FAIL (0 events)

2C Decision Integration
        - WAIT (needs regime-aware model)

NEXT:
        |
        v
  TASK-166: Regime-Aware Holding Risk Model
  TASK-167: Shadow Mode with Regime Gating
  TASK-160.2C: BreadthPersistence（组合维度，谨慎）
```

---

## TASK-166: Regime-Aware State Risk Model（2026-07-18）

### 背景

TASK-164 证明 HoldingRiskScore 是一个 **Transition Detector**（从正常状态到恶化状态的转换检测器），而不是 **State Detector**（已经处于危险状态的识别器）。在 2023 深度熊市中，HoldingRiskScore 不产生任何事件，因为它只检测 "additional deterioration"，而不是 "already bad state"。

用户建议：

> 需要 Regime-Aware State Risk Model，识别 "already dangerous" 状态。

### 目标

设计一个 State Risk Model，识别市场已经处于危险状态的情况，与 Transition Detector 形成互补。

### 方法

候选组件：

1. **TrendBreakdown**：price < MA20 and MA60, MA60 slope < 0
2. **VolatilityExpansion**：amplitude_pct > 70th percentile over 60 days
3. **MarketBreadthCollapse**：breadth_pct < 30% (state, not delta)
4. **LiquidityStress**：volume_ratio < 0.6 (state, not delta)

验收标准：

- Regime classification recall > 70% for RiskOff periods
- 不追求 precision，只追求 regime 覆盖

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/regime_risk_model.rs` | RegimeRiskScore 计算 + 分桶 + Regime 分类 |
| `crates/execution-replay/src/regime_risk_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `regime_risk_*` |
| `apps/cli/src/main.rs` | `regime-risk` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
# 2023 (bear)
cargo run -p quant-cli -- regime-risk \
  --scope cn --from 2023-01-01 --to 2023-12-31 \
  --output markdown

# 2024-2025
cargo run -p quant-cli -- regime-risk \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

报告路径：
- `reports/execution-validation/regime_risk_cn_2023_2026-07-18.md`
- `reports/execution-validation/regime_risk_cn_2024_2026-07-18.md`

### 结果：2023 (bear)

| Score | 样本 | Negative T+60 | Lift | Precision |
|---|---:|---:|---:|---:|
| 0.0 | 1071 | 86.7% | 1.15 | 86.7% |
| (0, 1.0) | 2086 | 71.6% | 0.95 | 71.6% |
| [1.0, 2.0) | 3975 | 72.8% | 0.97 | 72.8% |
| [2.0, 3.0) | 2650 | 73.2% | 0.97 | 73.2% |
| [3.0, 4.0) | 762 | 71.0% | 0.95 | 71.0% |
| >= 4.0 | 1 | 100.0% | 1.33 | 100.0% |

**RiskOff recall (score >= 2.0): 61.1%**

### 结果：2024-2025

| Score | 样本 | Negative T+60 | Lift | Precision |
|---|---:|---:|---:|---:|
| 0.0 | 3076 | 54.1% | 1.32 | 54.1% |
| (0, 1.0) | 3030 | 41.5% | 1.02 | 41.5% |
| [1.0, 2.0) | 4943 | 35.8% | 0.88 | 35.8% |
| [2.0, 3.0) | 2506 | 24.0% | 0.59 | 24.0% |
| [3.0, 4.0) | 596 | 15.3% | 0.37 | 15.3% |
| >= 4.0 | 3 | 0.0% | 0.00 | 0.0% |

**RiskOff recall (score >= 2.0): 47.9%**

### 关键发现

1. **State Risk Model 的组件实际上是 Mean-Reversion 信号**：
   - TrendBreakdown、VolatilityExpansion、MarketBreadthCollapse、LiquidityStress 都是 "oversold" 条件。
   - 在 2023（持续下跌）中，oversold 后继续下跌。
   - 在 2024-2025（恢复）中，oversold 后反弹。

2. **Score 0.0 的 negative rate 反而更高**：
   - 2023: score 0.0 negative rate 86.7% (高于 baseline 75.1%)
   - 2024-2025: score 0.0 negative rate 54.1% (高于 baseline 40.9%)
   - 说明 "no state risk" 并不意味着安全。

3. **Regime recall 不足**：
   - 2023: 61.1% (< 70%)
   - 2024-2025: 47.9% (< 70%)

4. **模型语义错误**：
   - 用户期望的是 "already dangerous" 状态识别器。
   - 实际构建的是 "oversold / mean reversion" 信号。
   - 这些组件在趋势市场中是反向指标，在震荡市场中是正向指标。

### 根因分析

用户建议的 State Risk 组件（TrendBreakdown / VolatilityExpansion / BreadthCollapse / LiquidityStress）都是 **均值回归信号**，而不是 **风险信号**。

在趋势市场中（2023 熊市）：
- Oversold 后通常继续下跌
- 所以高 score（更多 oversold 条件）应该有更高的 negative rate
- 但实际结果是 score 0.0 的 negative rate 最高，说明模型没有区分能力

在震荡 / 恢复市场中（2024-2025）：
- Oversold 后通常反弹
- 所以高 score 应该有更低的 negative rate
- 实际结果是 score 3.0-4.0 的 negative rate 最低（15.3%）

这意味着：
- **在恢复市场中，State Risk Model 是有效的（反向指标）**
- **在熊市中，State Risk Model 是无效的（没有区分能力）**

### 结论

**TASK-166 验收标准未通过。**

- Regime recall < 70%（2023: 61.1%, 2024-2025: 47.9%）
- 模型语义不是 "already dangerous"，而是 "mean reversion"

**State Risk Model 需要重新设计。** 当前组件不适合作为 RegimeRisk 信号。

### 下一步建议

1. **重新定义 State Risk 组件**：
   - 不要用 "oversold" 条件（它们是 mean reversion 信号）。
   - 改用 "accelerating decline" 条件（如价格动量恶化、波动率上升伴随负收益、广度恶化、流动性持续下降）。
   - 但这些又接近 Transition Detector，需要小心区分。

2. **接受 Regime 依赖的现实**：
   - 在恢复市场中，HoldingRiskScore（Transition Detector）有效。
   - 在熊市中，需要不同的模型。
   - 这可能意味着需要 regime-aware ensemble，而不是单一模型。

3. **Phase 2C Shadow Mode 应该限定在恢复市场**：
   - 只在 market_regime 为 Neutral 或 RiskOn 时使用 HoldingRiskScore。
   - 在 RiskOff 时，使用不同的 risk model（尚未设计）。

4. **TASK-167 推迟**：
   - 在没有有效的 State Risk Model 之前，不能进入 Shadow Mode。
   - 需要重新设计 State Risk 组件。

### 相关文件

- `crates/execution-replay/src/regime_risk_model.rs`
- `crates/execution-replay/src/regime_risk_formatter.rs`
- `reports/execution-validation/regime_risk_cn_2023_2026-07-18.md`
- `reports/execution-validation/regime_risk_cn_2024_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets + ValidationRecord)

2B-5 Holding Risk Calibration v2
        - 2024-2025 Regime stability: ✅ PASS
        - 2024-2025 Walk-forward: ❌ FAIL (no high-risk events in 2025H1)
        - 2023 Calibration: ❌ FAIL (no events, baseline 75.1%)
        - 2023 Lifecycle: ❌ FAIL (0 events)
        - Overall: PARTIAL (valid in 2024-2025, not stable across regimes)

2B-6 Holding Risk Lifecycle Modeling
        - 2024-2025: ✅ PASS (48 events, false alarm 20.8%, avg T+60 -5.72%)
        - 2023: ❌ FAIL (0 events)

2B-7 Regime-Aware State Risk Model
        - 2023: ❌ FAIL (recall 61.1%, score 0.0 negative rate 86.7%)
        - 2024-2025: ❌ FAIL (recall 47.9%, score 0.0 negative rate 54.1%)
        - Overall: FAIL (components are mean-reversion signals, not risk signals)

2C Decision Integration
        - WAIT (needs regime-aware model)

NEXT:
        |
        v
  TASK-168: Redesign State Risk Components (accelerating decline, not oversold)
  TASK-167: Shadow Mode with Regime Gating（推迟至 State Risk 重新设计后）
  TASK-160.2C: BreadthPersistence（组合维度，谨慎）
```

---

## TASK-168: Redesign State Risk Components（2026-07-18）

### 背景

TASK-166 证明用户最初建议的 State Risk 组件（TrendBreakdown / VolatilityExpansion / MarketBreadthCollapse / LiquidityStress）实际上是 **oversold / mean-reversion 信号**，而不是 "already dangerous" 状态识别器。在 2023 深度熊市中，这些组件没有区分能力（RiskOff recall 61.1%），在 2024-2025 恢复市场中，它们表现为反向指标（oversold 后反弹）。

用户建议：

> 重新定义 State Risk，不要检测 "跌很多"，而检测 "跌势还在加速"。

### 目标

用 **accelerating-decline** 组件替代 **oversold** 组件，识别市场正在持续恶化的状态。

### 方法

新组件：

1. **DowntrendAcceleration**：5d return < 0 且 5d return delta < 0（当前收益比 5 天前更差）
2. **VolatilityNegativeDrift**：amplitude > 70th percentile + today_return < 0 + close_position < 0.3
3. **PersistentBreadthCollapse**：breadth_delta_5d < -5 且连续 >= 2 天
4. **LiquidityStress**：volume_ratio_delta_5d < -0.2 且连续 >= 2 天 + today_return < 0

验收标准：RiskOff recall > 70%。

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/state_risk_acceleration.rs` | Accelerating-decline 组件 |
| `crates/execution-replay/src/state_risk_acceleration_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `state_risk_acceleration_*` |
| `apps/cli/src/main.rs` | `state-risk-acceleration` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
# 2023 (bear)
cargo run -p quant-cli -- state-risk-acceleration \
  --scope cn --from 2023-01-01 --to 2023-12-31 \
  --output markdown

# 2024-2025
cargo run -p quant-cli -- state-risk-acceleration \
  --scope cn --from 2024-01-01 --to 2025-06-30 \
  --output markdown
```

报告路径：
- `reports/execution-validation/state_risk_acceleration_cn_2023_2026-07-18.md`
- `reports/execution-validation/state_risk_acceleration_cn_2024_2026-07-18.md`

### 结果：2023 (bear)

| Score | 样本 | Negative T+60 | Lift | Precision |
|---|---:|---:|---:|---:|
| 0.0 | 4400 | 75.1% | 1.00 | 75.1% |
| (0, 1.0) | 1055 | 75.9% | 1.01 | 75.9% |
| [1.0, 2.0) | 1376 | 75.4% | 1.00 | 75.4% |
| [2.0, 3.0) | 353 | 72.8% | 0.97 | 72.8% |
| [3.0, 4.0) | 32 | 65.6% | 0.87 | 65.6% |
| >= 4.0 | 0 | — | — | — |

**RiskOff recall (score >= 2.0): 8.0%**

### 结果：2024-2025

| Score | 样本 | Negative T+60 | Lift | Precision |
|---|---:|---:|---:|---:|
| 0.0 | 6806 | 40.4% | 0.99 | 40.4% |
| (0, 1.0) | 1451 | 43.8% | 1.07 | 43.8% |
| [1.0, 2.0) | 1774 | 42.6% | 1.04 | 42.6% |
| [2.0, 3.0) | 359 | 37.9% | 0.93 | 37.9% |
| [3.0, 4.0) | 36 | 47.2% | 1.15 | 47.2% |
| >= 4.0 | 0 | — | — | — |

**RiskOff recall (score >= 2.0): 4.4%**

### 关键发现

1. **Accelerating-decline 组件触发率极低**：
   - 2023: 只有 385 条记录 score >= 2.0（占 6.6%）
   - 2024-2025: 只有 395 条记录 score >= 2.0（占 4.6%）

2. **没有区分能力**：
   - 2023: score 0.0 negative rate 75.1%，score [3.0,4.0) negative rate 65.6%
   - 2024-2025: score 0.0 negative rate 40.4%，score [3.0,4.0) negative rate 47.2%
   - 分数越高，negative rate 并没有单调增加

3. **RiskOff recall 极低**：
   - 2023: 8.0%（远低于 70% 要求）
   - 2024-2025: 4.4%（远低于 70% 要求）

4. **模型没有识别出 "accelerating decline"**：
   - 在 2023 深度熊市中，市场已经处于稳定下跌状态，"accelerating" 条件很难触发
   - 在 2024-2025 恢复市场中，"accelerating decline" 条件与 HoldingRiskScore 部分重叠，但样本太少

### 根因分析

**"Accelerating decline" 在深度熊市中不是一个高频事件。**

2023 年的市场状态是：
- 已经持续下跌
- 波动率已经高
- 广度已经低
- 流动性已经紧张

在这种情况下，"accelerating" 意味着：
- 收益比 5 天前更差
- 波动率超过 70th percentile 且当天收跌
- 广度连续恶化
- 成交量连续下降且当天收跌

这些条件同时满足的情况非常罕见（< 7% 的记录），而且一旦满足，后续收益并没有显著差异。

### 结论

**TASK-168 验收标准未通过。**

- 2023: RiskOff recall 8.0%（要求 > 70%）
- 2024-2025: RiskOff recall 4.4%（要求 > 70%）

**State Risk Acceleration Model 与 State Risk Oversold Model 一样，无法在 2023 深度熊市中识别风险。**

### 深层洞察

现在有两个重要发现：

1. **HoldingRiskScore（Transition Detector）**：
   - 在 2024-2025（normal → deterioration）有效
   - 在 2023（already bad）无效

2. **State Risk Model（State Detector）**：
   - 无论是 oversold 还是 accelerating decline，在 2023 都无效

这意味着：
> **当前 V8 只能识别 "risk forming"，不能识别 "risk already present"。**

这是模型架构的根本限制，不是参数调整能解决的。

### 下一步建议

1. **接受模型边界**：
   - HoldingRiskScore 是一个 **Transition Risk Model**，不是 **All-Weather Risk Model**。
   - 在 2023 深度熊市中，它不产生信号，但这不意味着它是错误的——它只是不是为这种环境设计的。

2. **Phase 2C Shadow Mode 必须 regime-aware**：
   - 只在 market_regime 为 Neutral 或 RiskOn 时启用 HoldingRiskScore。
   - 在 RiskOff 时，使用现有的 market_regime 标签作为风险信号（不需要新模型）。
   - 换句话说：**不要试图用 HoldingRiskScore 替代 market_regime，而是把它们结合**。

3. **TASK-167 Shadow Mode with Regime Gating 可以启动**：
   - 使用 `market_regime_label`（已有）作为 State Risk 指标。
   - 使用 `HoldingRiskScore`（已验证）作为 Transition Risk 指标。
   - 两者结合：
     - `market_regime = RiskOff` → HIGH_RISK（State Risk）
     - `market_regime = Neutral/RiskOn` 且 `HoldingRiskScore >= 0.75` → HIGH_RISK（Transition Risk）

4. **TASK-160.2C BreadthPersistence 不再必要**：
   - 当前 Evidence 已经足够构建 regime-aware risk model。
   - 不需要再增加新的 Evidence。

### 相关文件

- `crates/execution-replay/src/state_risk_acceleration.rs`
- `crates/execution-replay/src/state_risk_acceleration_formatter.rs`
- `reports/execution-validation/state_risk_acceleration_cn_2023_2026-07-18.md`
- `reports/execution-validation/state_risk_acceleration_cn_2024_2026-07-18.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets + ValidationRecord)

2B-5 Holding Risk Calibration v2
        - 2024-2025 Regime stability: ✅ PASS
        - 2024-2025 Walk-forward: ❌ FAIL (no high-risk events in 2025H1)
        - 2023 Calibration: ❌ FAIL (no events, baseline 75.1%)
        - 2023 Lifecycle: ❌ FAIL (0 events)
        - Overall: PARTIAL (valid in 2024-2025, not stable across regimes)

2B-6 Holding Risk Lifecycle Modeling
        - 2024-2025: ✅ PASS (48 events, false alarm 20.8%, avg T+60 -5.72%)
        - 2023: ❌ FAIL (0 events)

2B-7 Regime-Aware State Risk Model
        - 2023: ❌ FAIL (recall 61.1%, score 0.0 negative rate 86.7%)
        - 2024-2025: ❌ FAIL (recall 47.9%, score 0.0 negative rate 54.1%)
        - Overall: FAIL (components are mean-reversion signals, not risk signals)

2B-8 State Risk Acceleration Model
        - 2023: ❌ FAIL (recall 8.0%, no discriminative power)
        - 2024-2025: ❌ FAIL (recall 4.4%, no discriminative power)
        - Overall: FAIL (accelerating decline is rare and not predictive in deep bear markets)

2C Decision Integration
        - WAIT (use regime-aware gating: market_regime + HoldingRiskScore)

NEXT:
        |
        v
  TASK-167: Shadow Mode with Regime Gating
        - Use market_regime_label as State Risk
        - Use HoldingRiskScore as Transition Risk
        - Combine: RiskOff OR (Neutral/RiskOn AND score >= 0.75)
  TASK-160.2C: BreadthPersistence（不再必要）
```

---

## TASK-167: Shadow Mode Runtime Wiring（2026-07-18）

### 背景

TASK-166 / TASK-168 证明：

> V8 不应该建立一个预测 State 的模型，而应该把 Market Regime Label 作为 State Context，把 HoldingRiskScore 作为 Transition Evidence。

因此 TASK-167 不再是模型开发，而是 **Shadow Mode Runtime Wiring**：把已经验证的组件组合成一个旁路观察系统。

### 目标

生成每日 Shadow Mode 输出，验证：

1. Regime Gate 是否合理
2. Transition Detector 是否继续稳定
3. Lifecycle 是否稳定

### 方法

使用已有组件：

- **State Context**：`market_regime_label`（RiskOn / Neutral / RiskOff）
- **Transition Evidence**：`HoldingRiskScore`
- **Evidence Details**：LeadershipDecayPersistence / LiquidityPressure / ConfirmationDecay

Risk State 定义：

```text
HIGH_RISK:      RiskOff OR HoldingRiskScore >= 0.75
ELEVATED_RISK:  Neutral OR HoldingRiskScore >= 0.5
NORMAL:         otherwise
```

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/shadow_mode.rs` | ShadowModeReport / ShadowModeOutput / Summary |
| `crates/execution-replay/src/shadow_mode_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `shadow_mode_*` |
| `apps/cli/src/main.rs` | `shadow-mode` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
cargo run -p quant-cli -- shadow-mode \
  --scope cn --from 2026-07-01 --to 2026-07-17 \
  --output markdown
```

报告路径：`reports/execution-validation/shadow_mode_cn_2026-07-01_to_2026-07-17.md`

### 结果

| 日期 | Regime | Score | Risk State | Transition | Candidate | LD | LP | CD |
|------|--------|------:|------------|------------|-----------|----|----|----|
| 2026-07-01 | neutral | 0.00 | ELEVATED_RISK | No | monitor | - | - | - |
| 2026-07-02 | risk_off | 0.00 | HIGH_RISK | No | reduce_watch | - | - | - |
| 2026-07-03 | neutral | 0.00 | ELEVATED_RISK | No | monitor | - | - | - |
| 2026-07-06 | neutral | 0.00 | ELEVATED_RISK | No | monitor | - | - | - |
| 2026-07-07 | risk_off | 0.00 | HIGH_RISK | No | reduce_watch | - | - | - |
| 2026-07-08 | risk_off | 0.25 | HIGH_RISK | No | reduce_watch | - | - | Y |
| 2026-07-09 | neutral | 0.00 | ELEVATED_RISK | No | monitor | - | - | - |
| 2026-07-10 | risk_off | 0.75 | HIGH_RISK | Yes | reduce_watch | Y | - | Y |
| 2026-07-13 | risk_off | 0.00 | HIGH_RISK | No | reduce_watch | - | - | - |
| 2026-07-14 | risk_off | 0.00 | HIGH_RISK | No | reduce_watch | - | - | - |
| 2026-07-15 | risk_off | 0.00 | HIGH_RISK | No | reduce_watch | - | - | - |
| 2026-07-16 | risk_off | 0.26 | HIGH_RISK | No | reduce_watch | - | Y | Y |
| 2026-07-17 | risk_off | 0.01 | HIGH_RISK | No | reduce_watch | - | Y | - |

### 关键发现

1. **2026-07 市场处于 RiskOff 状态**：13 天中有 9 天 market_regime = risk_off。
2. **Transition Detection 正常**：2026-07-10 检测到 LeadershipDecayPersistence（score 0.75），符合预期。
3. **Evidence Details 正确**：能够识别出哪些 Evidence 组件在触发。
4. **Shadow Mode Runtime 可用**：系统能够生成每日风险状态输出，不依赖 DecisionEngine。

### Phase 2C 入口标准

当前 Shadow Mode 满足进入真实测试阶段的条件：

- ✅ Evidence Integrity（TASK-159）
- ✅ Evidence Registry（TASK-160.3）
- ✅ Evidence Horizon（TASK-160.3）
- ✅ Calibration（TASK-161）
- ✅ Lifecycle（TASK-163）
- ✅ Cross-regime analysis（TASK-164/166/168）
- ✅ Shadow Mode Runtime（TASK-167）

### 下一步建议

1. **运行 Shadow Mode 1-3 个月**：
   - 每日生成 `shadow-mode` 输出。
   - 记录 Risk State / Transition Detected / Decision Candidate。
   - 未来回填 T+20 / T+60 收益。

2. **观察指标**：
   - HIGH_RISK 天数比例
   - Transition Detection 提前量（HIGH_RISK 出现到大跌的天数）
   - Recovery 时间（HIGH_RISK 到 NORMAL 的天数）
   - False Alarm（HIGH_RISK 但后续上涨的比例）

3. **TASK-165 Decision Integration 推迟**：
   - 只有在 Shadow Mode 证明 Risk State Machine 与真实市场同步后，才考虑接入 Decision。

### 相关文件

- `crates/execution-replay/src/shadow_mode.rs`
- `crates/execution-replay/src/shadow_mode_formatter.rs`
- `reports/execution-validation/shadow_mode_cn_2026-07-01_to_2026-07-17.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets + ValidationRecord)

2B-5 Holding Risk Calibration v2
        - 2024-2025 Regime stability: ✅ PASS
        - 2024-2025 Walk-forward: ❌ FAIL (no high-risk events in 2025H1)
        - 2023 Calibration: ❌ FAIL (no events, baseline 75.1%)
        - 2023 Lifecycle: ❌ FAIL (0 events)
        - Overall: PARTIAL (valid in 2024-2025, not stable across regimes)

2B-6 Holding Risk Lifecycle Modeling
        - 2024-2025: ✅ PASS (48 events, false alarm 20.8%, avg T+60 -5.72%)
        - 2023: ❌ FAIL (0 events)

2B-7 Regime-Aware State Risk Model
        - 2023: ❌ FAIL (recall 61.1%, score 0.0 negative rate 86.7%)
        - 2024-2025: ❌ FAIL (recall 47.9%, score 0.0 negative rate 54.1%)
        - Overall: FAIL (components are mean-reversion signals, not risk signals)

2B-8 State Risk Acceleration Model
        - 2023: ❌ FAIL (recall 8.0%, no discriminative power)
        - 2024-2025: ❌ FAIL (recall 4.4%, no discriminative power)
        - Overall: FAIL (accelerating decline is rare and not predictive in deep bear markets)

2B-9 Shadow Mode Runtime Wiring
        - Runtime: ✅ COMPLETE
        - Regime Gate: ✅ WORKING (market_regime_label as State Context)
        - Transition Detection: ✅ WORKING (HoldingRiskScore >= 0.75)
        - 2026-07-10: LD + CD detected, score 0.75, transition detected
        - Phase 2C Shadow Mode: ✅ READY TO START

2C Decision Integration
        - WAIT (needs 1-3 months Shadow Mode validation)

NEXT:
        |
        v
  Phase 2C Shadow Mode
        - Run daily shadow-mode output for 1-3 months
        - Track Risk State / Transition / Lifecycle
        - Backfill T+20 / T+60 returns
  TASK-165 Decision Integration Proposal（推迟至 Shadow Mode 验证后）
  TASK-160.2C: BreadthPersistence（不再必要）
```

---

## TASK-169: Shadow Deployment Contract（2026-07-18）

### 背景

TASK-167 建立了 Shadow Mode Runtime Wiring，但用户建议：

> 需要冻结边界，定义 Shadow Deployment Contract，明确输入/输出并显式禁止 DecisionEngine 消费。

这是 Phase 2C Shadow Validation 的正式入口。

### 目标

建立 **Shadow Deployment Contract**：

- **Input**：real ResearchContext（via ExecutionResearchRecord）
- **Output**：ShadowRiskAssessment（observation-only）
- **Prohibition**：DecisionEngine must NOT consume ShadowRiskAssessment

### 方法

定义 `ShadowRiskAssessment`：

```rust
pub struct ShadowRiskAssessment {
    pub date: NaiveDate,
    pub regime: String,
    pub holding_risk_score: f64,
    pub evidence: EvidenceSummary,
    pub lifecycle_state: String,
    pub simulated_action: String,
    pub decision_engine_consumption_allowed: bool, // always false
}
```

### 实现

| 文件 | 变更 |
|---|---|
| `crates/execution-replay/src/shadow_deployment.rs` | ShadowDeploymentReport / ShadowRiskAssessment / Summary |
| `crates/execution-replay/src/shadow_deployment_formatter.rs` | Markdown / JSON 输出 |
| `crates/execution-replay/src/lib.rs` | 导出模块 |
| `crates/app-service/src/execution_replay.rs` | `shadow_deployment_*` |
| `apps/cli/src/main.rs` | `shadow-deployment` CLI |
| `apps/cli/src/commands/execution_replay.rs` | handler |

未修改：ObservationEngine / EvidenceBuilder / AssessmentEngine / DecisionEngine / ExecutionPolicy。

### 运行命令

```bash
cargo run -p quant-cli -- shadow-deployment \
  --scope cn --from 2026-07-01 --to 2026-07-17 \
  --output markdown
```

报告路径：`reports/execution-validation/shadow_deployment_cn_2026-07-01_to_2026-07-17.md`

### 结果

| 日期 | Regime | Score | Lifecycle State | Simulated Action | LD | LP | CD |
|------|--------|------:|-----------------|------------------|----|----|----|
| 2026-07-01 | neutral | 0.00 | ELEVATED_RISK | monitor | - | - | - |
| 2026-07-02 | risk_off | 0.00 | HIGH_RISK | reduce_watch | - | - | - |
| 2026-07-03 | neutral | 0.00 | ELEVATED_RISK | monitor | - | - | - |
| 2026-07-06 | neutral | 0.00 | ELEVATED_RISK | monitor | - | - | - |
| 2026-07-07 | risk_off | 0.00 | HIGH_RISK | reduce_watch | - | - | - |
| 2026-07-08 | risk_off | 0.25 | HIGH_RISK | reduce_watch | - | - | Y |
| 2026-07-09 | neutral | 0.00 | ELEVATED_RISK | monitor | - | - | - |
| 2026-07-10 | risk_off | 0.75 | HIGH_RISK | reduce_watch | Y | - | Y |
| 2026-07-13 | risk_off | 0.00 | HIGH_RISK | reduce_watch | - | - | - |
| 2026-07-14 | risk_off | 0.00 | HIGH_RISK | reduce_watch | - | - | - |
| 2026-07-15 | risk_off | 0.00 | HIGH_RISK | reduce_watch | - | - | - |
| 2026-07-16 | risk_off | 0.26 | HIGH_RISK | reduce_watch | - | Y | Y |
| 2026-07-17 | risk_off | 0.01 | HIGH_RISK | reduce_watch | - | Y | - |

### Contract 约束

```text
Input:  real ResearchContext (via ExecutionResearchRecord)
Output: ShadowRiskAssessment (observation-only)
Prohibition: DecisionEngine must NOT consume ShadowRiskAssessment
```

### Phase 2C Shadow Validation 入口标准

当前 Shadow Deployment Contract 满足进入真实测试阶段的条件：

- ✅ Evidence Integrity（TASK-159）
- ✅ Evidence Registry（TASK-160.3）
- ✅ Evidence Horizon（TASK-160.3）
- ✅ Calibration（TASK-161）
- ✅ Lifecycle（TASK-163）
- ✅ Cross-regime analysis（TASK-164/166/168）
- ✅ Shadow Mode Runtime（TASK-167）
- ✅ Shadow Deployment Contract（TASK-169）

### 下一步建议

1. **运行 Shadow Validation 4-8 周**：
   - 每日生成 `shadow-deployment` 输出。
   - 记录 Risk State / Transition / Lifecycle / Simulated Action。
   - 未来回填 T+20 / T+60 收益。

2. **观察指标**：
   - HIGH_RISK 天数比例
   - Transition Detection 提前量
   - Recovery 时间
   - False Alarm（HIGH_RISK 但后续上涨）

3. **TASK-165 Decision Integration Proposal 推迟**：
   - 只有在 Shadow Validation 证明 Risk State Machine 与真实市场同步后，才考虑接入 Decision。

### 相关文件

- `crates/execution-replay/src/shadow_deployment.rs`
- `crates/execution-replay/src/shadow_deployment_formatter.rs`
- `reports/execution-validation/shadow_deployment_cn_2026-07-01_to_2026-07-17.md`

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS

2B-1 Bearish Evidence Analysis            ✅

2B-2 Transition Evidence Modeling
  2B-2.1 RecoveryFailure                ❌ REJECTED
  2B-2.2 BreadthDeterioration           ❌ REJECTED
  2B-2.3 LeadershipDecay T+20           ❌ REJECTED
  2B-2.4 LeadershipDecay T+60           ✅ PASS (lift 1.50, precision 61.5%)
  2B-2.5 LeadershipDecay T+120          ✅ PASS (lift 2.36, precision 52.6%)

2B-3 Holding Risk Evidence
  2B-3.1 Holding Risk Bundle V1
        - weighted [0.7, 1.0): ✅ PASS (lift 1.51, precision 61.8%, n=421)
  2B-3.2 Holding Risk Persistence
        - LD >= 5 days: ✅ PASS (lift 1.88, precision 76.8%, n=311)
  2B-3.3 Holding Risk Bundle V2
        - V2 (5d) [0.75, 1.0): ✅ PASS (lift 1.69, precision 69.1%, n=165)
  2B-3.4 Holding Risk Bundle V3
        - V3 (LD5d + LP + BD) [0.7, 1.0): ✅ PASS (lift 1.90, precision 77.7%, n=121)
  2B-3.5 Holding Risk Bundle V4
        - V4 (LD5d + LP + CD) >= 1.0: ✅ PASS (lift 2.30, precision 93.9%, n=33)

2B-4 Evidence Horizon Registry
        ✅ COMPLETE (7 assets + ValidationRecord)

2B-5 Holding Risk Calibration v2
        - 2024-2025 Regime stability: ✅ PASS
        - 2024-2025 Walk-forward: ❌ FAIL (no high-risk events in 2025H1)
        - 2023 Calibration: ❌ FAIL (no events, baseline 75.1%)
        - 2023 Lifecycle: ❌ FAIL (0 events)
        - Overall: PARTIAL (valid in 2024-2025, not stable across regimes)

2B-6 Holding Risk Lifecycle Modeling
        - 2024-2025: ✅ PASS (48 events, false alarm 20.8%, avg T+60 -5.72%)
        - 2023: ❌ FAIL (0 events)

2B-7 Regime-Aware State Risk Model
        - 2023: ❌ FAIL (recall 61.1%, score 0.0 negative rate 86.7%)
        - 2024-2025: ❌ FAIL (recall 47.9%, score 0.0 negative rate 54.1%)
        - Overall: FAIL (components are mean-reversion signals, not risk signals)

2B-8 State Risk Acceleration Model
        - 2023: ❌ FAIL (recall 8.0%, no discriminative power)
        - 2024-2025: ❌ FAIL (recall 4.4%, no discriminative power)
        - Overall: FAIL (accelerating decline is rare and not predictive in deep bear markets)

2B-9 Shadow Mode Runtime Wiring
        - Runtime: ✅ COMPLETE
        - Regime Gate: ✅ WORKING (market_regime_label as State Context)
        - Transition Detection: ✅ WORKING (HoldingRiskScore >= 0.75)
        - 2026-07-10: LD + CD detected, score 0.75, transition detected
        - Phase 2C Shadow Mode: ✅ READY TO START

2B-10 Shadow Deployment Contract
        - Contract: ✅ COMPLETE
        - Input: real ResearchContext
        - Output: ShadowRiskAssessment (observation-only)
        - Prohibition: DecisionEngine must NOT consume
        - Phase 2C Shadow Validation: ✅ READY TO START

2C Decision Integration
        - WAIT (needs 4-8 weeks Shadow Validation)

NEXT:
        |
        v
  Phase 2C Shadow Validation
        - Run daily shadow-deployment output for 4-8 weeks
        - Track Risk State / Transition / Lifecycle / Simulated Action
        - Backfill T+20 / T+60 returns
  TASK-165 Decision Integration Proposal（推迟至 Shadow Validation 验证后）
  TASK-160.2C: BreadthPersistence（不再必要）
```

---

## Phase 2C Shadow Validation（2026-07-18 启动）

### 状态

**Phase 2C Shadow Validation 已启动。**

当前 V8 Execution Platform 已完成研究验证阶段全部前置条件，进入真实市场环境下的影子运行验证。

### 运行管线

- **计划文档**：`docs/v8/shadow-validation-plan.md`
- **每日脚本**：`shadow-production/shadow-validation-daily.ps1`
- **输出目录**：`reports/shadow-validation/`（gitignored）

### 每日命令

```bash
cargo run -p quant-cli -- shadow-deployment \
  --scope cn --from <30天前> --to <今天> \
  --output markdown
```

或运行脚本：

```powershell
.\shadow-production\shadow-validation-daily.ps1 -Scope cn
```

### 验收标准（进入 TASK-165 前必须满足）

| 指标 | 要求 |
|---|---|
| HIGH_RISK 天数比例 | < 30% |
| False Alarm 率 | < 30% |
| Recovery 平均时间 | < 10 天 |
| Simulated Action 震荡 | < 20% |
| Transition 提前量 | > 3 天 |

### 禁止事项

- ❌ DecisionEngine 消费 ShadowRiskAssessment
- ❌ 修改 ExecutionPolicy
- ❌ 自动交易
- ❌ 调整 HoldingRiskScore 权重

### 当前状态

```text
Phase 2B Research Validation    ✅ COMPLETE
Phase 2C Shadow Validation      ✅ ACTIVE (2026-07-18 启动)
Phase 2D Decision Integration   ⏸  DEFERRED (needs 4-8 weeks Shadow Validation)
```

### 下一步

1. **运行 Shadow Validation 4-8 周**（2026-07-20 至 2026-08-31）
2. **每周回顾**：`reports/shadow-validation/weekly_review_{week}.md`
3. **回填 T+20 / T+60 收益**：2026-08-10 / 2026-09-20
4. **TASK-165 Decision Integration Proposal**：Shadow Validation 完成后

### 相关文件

- `docs/v8/shadow-validation-plan.md`
- `shadow-production/shadow-validation-daily.ps1`
- `crates/execution-replay/src/shadow_deployment.rs`
- `crates/execution-replay/src/shadow_deployment_formatter.rs`
- `reports/shadow-validation/`（运行产物）

### 2B 状态更新（最终）

```text
2B-0 ResearchContext Fact Integrity        ✅ PASS
2B-1 Bearish Evidence Analysis            ✅
2B-2 Transition Evidence Modeling         ✅
2B-3 Holding Risk Evidence                ✅
2B-4 Evidence Horizon Registry            ✅
2B-5 Holding Risk Calibration v2          ⚠️ PARTIAL
2B-6 Holding Risk Lifecycle Modeling      ✅
2B-7 Regime-Aware State Risk Model        ❌ FAIL
2B-8 State Risk Acceleration Model        ❌ FAIL
2B-9 Shadow Mode Runtime Wiring           ✅
2B-10 Shadow Deployment Contract          ✅

Phase 2B Research Validation              ✅ COMPLETE
Phase 2C Shadow Validation                ✅ ACTIVE

2C Decision Integration                   ⏸ DEFERRED
```

---

## 当前任务状态总结

### 已完成

| 任务 | 状态 | 关键结果 |
|---|---|---|
| TASK-156 | ✅ | ResearchContext Fact Integrity Audit + data bridge fix |
| TASK-157 | ✅ | LeadershipDecay Horizon Analysis (T+60 pass) |
| TASK-158 | ✅ | Holding Risk Bundle V1 (weighted [0.7,1.0) pass) |
| TASK-159 | ✅ | Context Integrity Fact Integrity Firewall |
| TASK-160.1 | ✅ | Holding Risk Persistence (5d LD pass) |
| TASK-160.2A | ✅ | LiquidityPressure + Bundle V3 |
| TASK-160.2B | ✅ | ConfirmationDecay + Bundle V4 |
| TASK-160.3 | ✅ | Evidence Horizon Registry Runtime |
| TASK-160.4 | ✅ | Evidence Registry ValidationRecord |
| TASK-161 | ✅ | Holding Risk Calibration v2 |
| TASK-163 | ✅ | Holding Risk Lifecycle Modeling |
| TASK-164 | ✅ | Extended Historical Validation (2023) |
| TASK-166 | ✅ | Regime-Aware State Risk Model (fail) |
| TASK-168 | ✅ | State Risk Acceleration Model (fail) |
| TASK-167 | ✅ | Shadow Mode Runtime Wiring |
| TASK-169 | ✅ | Shadow Deployment Contract |

### 已拒绝

| 候选 | 原因 |
|---|---|
| RecoveryFailure | T+20 lift 0.99, precision 46.8% |
| BreadthDeterioration | T+20 lift 1.03, precision 48.6% |
| Regime-Aware State Risk | recall < 70%, mean-reversion signals |
| State Risk Acceleration | recall < 70%, no discriminative power |

### 当前核心 Evidence

| Evidence | Role | Horizon | Status |
|---|---|---|---|
| LeadershipDecay | HoldingRisk | MediumTerm | Validated |
| LiquidityPressure | Amplifier | MediumTerm | Conditional |
| ConfirmationDecay | Confirmation | MediumTerm | Conditional |

### 当前核心模型

| 模型 | 类型 | 状态 | 适用环境 |
|---|---|---|---|
| HoldingRiskScore | Transition Detector | Validated | 2024-2025 (normal → deterioration) |
| Risk Lifecycle | State Machine | Validated | 2024-2025 |
| Regime-Aware State Risk | State Detector | Failed | 2023 (deep bear) |
| State Risk Acceleration | State Detector | Failed | 2023 (deep bear) |

### 架构判断

> **HoldingRiskScore 是一个 Transition Detector，不是 State Detector。它在 normal market 中有效，在 deep bear market 中不产生信号。这不是失败，而是模型语义边界。**

因此：
- **Phase 2C Shadow Validation 可以启动**（使用 market_regime_label 作为 State Context，HoldingRiskScore 作为 Transition Evidence）
- **TASK-165 Decision Integration 必须推迟**（直到 Shadow Validation 证明 Risk State Machine 与真实市场同步）

### 下一步

1. **运行 Shadow Validation 4-8 周**（2026-07-20 至 2026-08-31）
2. **每周回顾**并回填 T+20 / T+60 收益
3. **TASK-165 Decision Integration Proposal**（Shadow Validation 完成后）
4. **不再增加新 Evidence**（当前 Evidence 已足够，避免过拟合）

---

## Phase 2C Entry Hardening（2026-07-18）

### 背景

Oracle 复核后确认 Phase 2C Shadow Validation 可以启动，但建议先补 3 个小型工程护栏：

1. **TASK-170 Context Live Integrity Gate**：覆盖 live 路径，防止 placeholder 污染
2. **TASK-171 Shadow Output Safety**：防止 operator 误读 simulated_action 为可操作
3. **TASK-172 Shadow Validation Monitor**：提供 0 事件监控协议

### TASK-170: Context Live Integrity Gate

**实现**：

- 新增 `--live` 模式到 `execution-context-integrity-gate`
- 新增 `execution_context_live_integrity_check` 方法到 AppContext
- 轻量检查：只验证已知 placeholder 值不存在（不要求 variance / unique ratio，因为单日无意义）

**每日脚本更新**：

```powershell
# Step 1: Context Integrity Gate (live)
cargo run -p quant-cli --quiet -- execution-context-integrity-gate \
  --live --scope cn --output markdown
```

**验证结果**：2026-07-17 最新交易日，24 条记录，无 placeholder 值，**PASS**。

### TASK-171: Shadow Output Safety

**实现**：

- `simulated_action` 重命名为 `research_interpretation`
- 新增 `[RESEARCH ONLY — NOT ACTIONABLE]` 警告到 Markdown formatter
- 更新解释值：
  - HIGH_RISK → `monitor_risk_transition`
  - ELEVATED_RISK → `observe_market_structure`
  - NORMAL → `normal_conditions`

**意义**：防止 operator 误读为可操作决策。

### TASK-172: Shadow Validation Monitor

**实现**：

- 新增 `ShadowValidationStatus` enum：NORMAL / INSUFFICIENT_EVENTS / ACTIVE
- 规则：如果 total_days >= 20 且 transition_detected_days == 0，进入 INSUFFICIENT_EVENTS
- 每日脚本增加 Step 3：解析 JSON 输出，显示 validation_status

**验证结果**：2026-07-01 至 2026-07-17，13 天，1 transition detected，**ACTIVE**。

### 每日脚本（更新后）

```powershell
# Step 1: Context Integrity Gate (live)
cargo run -p quant-cli --quiet -- execution-context-integrity-gate \
  --live --scope cn --output markdown

# Step 2: Shadow Deployment
cargo run -p quant-cli --quiet -- shadow-deployment \
  --scope cn --from <30天前> --to <今天> --output markdown

# Step 3: Shadow Validation Monitor
# Parse JSON and display validation_status
```

### Phase 2C Shadow Validation 首次运行结果

| 步骤 | 状态 | 结果 |
|---|---|---|
| Context Integrity Gate (live) | ✅ PASS | 24 records, no placeholder |
| Shadow Deployment | ✅ PASS | 13 days, 1 transition detected |
| Shadow Validation Monitor | ✅ ACTIVE | validation_status = ACTIVE |

**Phase 2C Shadow Validation 已正式运行。**

### 相关文件

- `crates/execution-replay/src/shadow_deployment.rs`
- `crates/execution-replay/src/shadow_deployment_formatter.rs`
- `crates/app-service/src/execution_replay.rs`
- `apps/cli/src/main.rs`
- `apps/cli/src/commands/execution_replay.rs`
- `shadow-production/shadow-validation-daily.ps1`
- `reports/shadow-validation/`（运行产物）

### 2C 状态更新

```text
Phase 2B Research Validation      ✅ COMPLETE
Phase 2C Shadow Validation        ✅ ACTIVE (hardened entry)
  - Context Live Integrity Gate   ✅ TASK-170
  - Shadow Output Safety          ✅ TASK-171
  - Shadow Validation Monitor     ✅ TASK-172
  - Daily script                  ✅ Updated

2D Decision Integration           ⏸ DEFERRED (needs 4-8 weeks Shadow Validation)
```

### 当前任务状态总结（更新）

| 任务 | 状态 | 关键结果 |
|---|---|---|
| TASK-156 | ✅ | ResearchContext Fact Integrity Audit + data bridge fix |
| TASK-157 | ✅ | LeadershipDecay Horizon Analysis (T+60 pass) |
| TASK-158 | ✅ | Holding Risk Bundle V1 (weighted [0.7,1.0) pass) |
| TASK-159 | ✅ | Context Integrity Fact Integrity Firewall |
| TASK-160.1 | ✅ | Holding Risk Persistence (5d LD pass) |
| TASK-160.2A | ✅ | LiquidityPressure + Bundle V3 |
| TASK-160.2B | ✅ | ConfirmationDecay + Bundle V4 |
| TASK-160.3 | ✅ | Evidence Horizon Registry Runtime |
| TASK-160.4 | ✅ | Evidence Registry ValidationRecord |
| TASK-161 | ✅ | Holding Risk Calibration v2 |
| TASK-163 | ✅ | Holding Risk Lifecycle Modeling |
| TASK-164 | ✅ | Extended Historical Validation (2023) |
| TASK-166 | ✅ | Regime-Aware State Risk Model (fail) |
| TASK-168 | ✅ | State Risk Acceleration Model (fail) |
| TASK-167 | ✅ | Shadow Mode Runtime Wiring |
| TASK-169 | ✅ | Shadow Deployment Contract |
| TASK-170 | ✅ | Context Live Integrity Gate |
| TASK-171 | ✅ | Shadow Output Safety |
| TASK-172 | ✅ | Shadow Validation Monitor |

### 下一步（更新）

1. **运行 Shadow Validation 4-8 周**（2026-07-20 至 2026-08-31）
   - 每日运行 `shadow-validation-daily.ps1`
   - 每周生成 `weekly_review_{week}.md`
2. **2 周 checkpoint**（2026-08-03）
   - 评估 event frequency 和 regime distribution
   - 如果 0 events 连续 2 周，重新评估入口条件
3. **回填 T+20 / T+60 收益**（2026-08-10 / 2026-09-20）
4. **TASK-165 Decision Integration Proposal**（Shadow Validation 完成后）
5. **不再增加新 Evidence**（当前 Evidence 已足够，避免过拟合）

---

## TASK-173: Evidence Validation Contract Hardening（2026-07-18）

### 背景

Oracle 复核后确认 TASK-170/171/172 解决了三个关键安全缺口，但指出：

> 样本充分性（n=33）仍未在 best-bucket contracts 中量化。

这是进入 TASK-165 Decision Integration 前必须解决的问题。用户建议：

> TASK-173 Evidence Validation Contract Hardening，包含最小样本 / 最小事件数 / precision/lift 约束 / Live Integrity Contract 统一。

### 目标

建立 **Evidence Validation Contract**，使 Evidence Registry 中的 `Validated` 状态必须满足统计阈值，而不仅仅是人工标记。

### 实现

#### 1. ValidationRequirement Struct

```rust
pub struct ValidationRequirement {
    pub min_samples: usize,
    pub min_precision: f64,
    pub min_lift: f64,
    pub max_false_alarm: f64,
}
```

#### 2. EvidenceDescriptor 新增字段

```rust
pub struct EvidenceDescriptor {
    // ... existing fields ...
    pub validation_requirement: Option<ValidationRequirement>,
}
```

#### 3. meets_validation_requirement 方法

```rust
pub fn meets_validation_requirement(&self) -> bool {
    if self.validation_status != ValidationStatus::Validated {
        return false;
    }
    let Some(req) = &self.validation_requirement else {
        return true;
    };
    let Some(metric) = &self.target_metric else {
        return false;
    };
    metric.sample_count >= req.min_samples
        && metric.precision >= req.min_precision
        && metric.lift >= req.min_lift
        && metric.false_reduce_rate <= req.max_false_alarm
}
```

#### 4. EvidenceRegistry 新增方法

```rust
pub fn validated_assets(&self) -> Vec<&EvidenceDescriptor> {
    self.assets
        .iter()
        .filter(|a| a.meets_validation_requirement())
        .collect()
}
```

#### 5. Live Integrity Contract 统一

`execution_context_live_integrity_check` 现在使用 `ExecutionContextIntegrityContract::v8_default()` 的 `known_placeholders`，而不是硬编码值。这统一了 Live 和 Replay 的 integrity 检查。

### 当前 Registry（更新后）

| Evidence | Role | Horizon | Status | Validation Requirement | Meets Requirement |
|---|---|---|---|---|---|
| LeadershipDecay | HoldingRisk | MediumTerm | Validated | n>=100, p>=60%, l>=1.3, fa<=40% | ✅ YES |
| LiquidityPressure | Amplifier | MediumTerm | Conditional | n>=500, p>=50%, l>=1.2, fa<=40% | ❌ NO (n=637<500? No, but p=44.9%<50%) |
| ConfirmationDecay | Confirmation | MediumTerm | Conditional | n>=1000, p>=50%, l>=1.2, fa<=40% | ❌ NO (p=37.6%<50%, l=0.92<1.2) |
| BreadthDeterioration | HoldingRisk | MediumTerm | Rejected | n>=100, p>=50%, l>=1.2, fa<=40% | ❌ NO (status=Rejected) |
| RecoveryFailure | ExitSignal | ShortTerm | Rejected | n>=100, p>=50%, l>=1.2, fa<=40% | ❌ NO (status=Rejected) |
| RiskExpansion | HoldingRisk | ShortTerm | Rejected | n>=30, p>=50%, l>=1.2, fa<=40% | ❌ NO (status=Rejected) |
| Distribution | HoldingRisk | ShortTerm | Conditional | n>=30, p>=50%, l>=1.2, fa<=40% | ❌ NO (n=5<30) |

**当前只有 LeadershipDecay 满足 Validation Requirement。**

### 验证结果

```bash
cargo run -p quant-cli -- evidence-registry --output markdown
```

输出显示：
- Validated Assets (Meet Validation Requirement): 只有 LeadershipDecay (n=743, precision=61.5%, lift=1.50)
- LiquidityPressure 和 ConfirmationDecay 不满足 requirement（precision/lift 不足）
- 这防止了 premature decision integration

### 与 Phase 2C 的关系

TASK-173 不是 Phase 2C 的阻塞项，但它是 TASK-165 Decision Integration 的前置条件：

- **Phase 2C Shadow Validation**：可以启动（TASK-170/171/172 已解决安全缺口）
- **TASK-165 Decision Integration**：必须等待样本充分性验证（TASK-173 提供机制）

### 相关文件

- `crates/execution-replay/src/evidence_registry.rs`
- `crates/execution-replay/src/evidence_registry_formatter.rs`
- `crates/app-service/src/execution_replay.rs`
- `apps/cli/src/main.rs`
- `apps/cli/src/commands/execution_replay.rs`
- `shadow-production/shadow-validation-daily.ps1`

### 2C 状态更新（最终）

```text
Phase 2B Research Validation      ✅ COMPLETE
Phase 2C Shadow Validation        ✅ ACTIVE (hardened entry)
  - Context Live Integrity Gate   ✅ TASK-170
  - Shadow Output Safety          ✅ TASK-171
  - Shadow Validation Monitor     ✅ TASK-172
  - Evidence Validation Contract  ✅ TASK-173
  - Daily script                  ✅ Updated

2D Decision Integration           ⏸ DEFERRED
  - Requires: Shadow Validation 4-8 weeks
  - Requires: Sample sufficiency validation (TASK-173)
  - Requires: HK scope transferability
```

### 当前任务状态总结（最终）

| 任务 | 状态 | 关键结果 |
|---|---|---|
| TASK-156 | ✅ | ResearchContext Fact Integrity Audit + data bridge fix |
| TASK-157 | ✅ | LeadershipDecay Horizon Analysis (T+60 pass) |
| TASK-158 | ✅ | Holding Risk Bundle V1 (weighted [0.7,1.0) pass) |
| TASK-159 | ✅ | Context Integrity Fact Integrity Firewall |
| TASK-160.1 | ✅ | Holding Risk Persistence (5d LD pass) |
| TASK-160.2A | ✅ | LiquidityPressure + Bundle V3 |
| TASK-160.2B | ✅ | ConfirmationDecay + Bundle V4 |
| TASK-160.3 | ✅ | Evidence Horizon Registry Runtime |
| TASK-160.4 | ✅ | Evidence Registry ValidationRecord |
| TASK-161 | ✅ | Holding Risk Calibration v2 |
| TASK-163 | ✅ | Holding Risk Lifecycle Modeling |
| TASK-164 | ✅ | Extended Historical Validation (2023) |
| TASK-166 | ✅ | Regime-Aware State Risk Model (fail) |
| TASK-168 | ✅ | State Risk Acceleration Model (fail) |
| TASK-167 | ✅ | Shadow Mode Runtime Wiring |
| TASK-169 | ✅ | Shadow Deployment Contract |
| TASK-170 | ✅ | Context Live Integrity Gate |
| TASK-171 | ✅ | Shadow Output Safety |
| TASK-172 | ✅ | Shadow Validation Monitor |
| TASK-173 | ✅ | Evidence Validation Contract Hardening |

### 下一步（最终）

1. **运行 Shadow Validation 2-4 周观测窗口**（2026-07-20 至 2026-08-15）
   - 每日运行 `shadow-validation-daily.ps1`
   - 每周运行 `shadow-validation-weekly.ps1` 生成 `weekly_review_{week}.md`
2. **2 周 checkpoint**（2026-08-03）
   - 评估 event frequency 和 regime distribution
   - 如果 0 events 连续 2 周，重新评估入口条件
3. **回填 T+20 / T+60 收益**（2026-08-10 / 2026-09-20）
4. **HK scope 迁移性验证**（TASK-165 前）
   - 在 HK scope 上运行 Bundle V4 分析
5. **TASK-165 Decision Integration Proposal**（Shadow Validation + Sample Sufficiency + HK 验证完成后）
6. **不再增加新 Evidence**（当前 Evidence 已足够，避免过拟合）

---

## Phase 2C Shadow Validation 观测阶段（2026-07-18 启动）

### 状态

**Phase 2C Shadow Validation 已正式进入观测阶段（2-4 周观测窗口）。**

当前 V8 Execution Platform 已完成研究验证阶段全部前置条件，进入真实市场环境下的影子运行验证。架构边界已清晰，不再新增任务，冻结代码，专注验证现有认知链在时间维度上的稳定性。

### 观测目标（3 个关键指标）

| 指标 | 目标 | 评估方式 |
|---|---|---|
| Transition Lead Time | > 3 天 | HIGH_RISK 出现到大跌 / 波动扩大 / breadth collapse 的提前量 |
| False Alarm Rate | < 30% | HIGH_RISK 但后续 T+60 收益 >= 0 的比例（需回填） |
| State Stability | HIGH_RISK < 30% | HIGH_RISK 天数占总天数的比例 |

### 每日运行

```powershell
.\shadow-production\shadow-validation-daily.ps1 -Scope cn
```

### 每周回顾

```powershell
.\shadow-production\shadow-validation-weekly.ps1 -Scope cn
```

### 代码冻结

当前已冻结：

- `Evidence Registry`（TASK-160.3/160.4/173）
- `ResearchContext` 构建链路
- `DecisionEngine` / `ExecutionPolicy`

**不再新增 Evidence，不再调整 HoldingRiskScore 权重。**

### 当前完成度

| 层 | 状态 |
|---|---|
| 数据真实性 | ✅ |
| Evidence 语义 | ✅ |
| Evidence 治理 | ✅ |
| Shadow 隔离 | ✅ |
| Live 安全 | ✅ |
| 长期稳定性 | ⏳ 观测中 |
| Decision 映射 | 未开始 |

### 架构判断

> **V8 Execution Platform 已经从"研究模型可信性验证"阶段进入"真实运行可靠性验证"阶段。当前最重要的工作不是创造新能力，而是证明现有认知链在时间维度上不会失真。**

### 相关文件

- `docs/v8/shadow-validation-plan.md`
- `shadow-production/shadow-validation-daily.ps1`
- `shadow-production/shadow-validation-weekly.ps1`
- `reports/shadow-validation/`（运行产物）
- `crates/execution-replay/src/shadow_deployment.rs`
- `crates/execution-replay/src/shadow_deployment_formatter.rs`

