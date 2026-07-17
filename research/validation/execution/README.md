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
3. 保留这两个 FAIL 案例作为 State 权重校准的基准。，
