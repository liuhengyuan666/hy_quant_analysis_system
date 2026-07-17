# Execution Platform V2 Golden Validation Suite

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
