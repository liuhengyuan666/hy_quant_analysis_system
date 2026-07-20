# Phase 2C Shadow Validation Plan

## 目标

在真实市场环境中运行 V8 Execution Platform 的 Shadow Deployment，验证：

1. **数据链稳定**：每日 `ResearchContext` → `ExecutionMarketView` → `Evidence` 是否 100% 通过 Integrity Gate
2. **Evidence 行为稳定**：`LeadershipDecay` 是否继续表现为 Transition Detector
3. **Risk Lifecycle 稳定**：`Entry / Peak / Recovery` 是否出现长期不恢复或频繁震荡

## 运行周期

**2-4 周观测窗口**（2026-07-20 至 2026-08-15）

## 每日运行

### 命令

```powershell
.\shadow-production\shadow-validation-daily.ps1 -Scope cn
```

### 输出

每日生成：

- `reports/shadow-validation/shadow_deployment_cn_{date}.md`
- `reports/shadow-validation/shadow_deployment_cn_{date}.json`
- `reports/shadow-validation/context_integrity_cn_{date}.md`

### 内容

| 字段 | 说明 |
|---|---|
| date | 日期 |
| regime | market_regime_label (RiskOn / Neutral / RiskOff) |
| holding_risk_score | 0.0 - 1.0 |
| lifecycle_state | HIGH_RISK / ELEVATED_RISK / NORMAL |
| research_interpretation | monitor_risk_transition / observe_market_structure / normal_conditions |
| evidence.leadership_decay_persistence | LD >= 5 days |
| evidence.liquidity_pressure | LP >= 3 days |
| evidence.confirmation_decay | CD >= 2 days |
| decision_engine_consumption_allowed | false（禁止） |

## 观察指标（3 个关键指标）

### 1. Transition Lead Time（目标：>3 天）

**定义**：HIGH_RISK 出现到大跌 / 波动扩大 / breadth collapse 的提前量。

**评估**：

- HIGH_RISK 出现到 T+5 负收益的提前量
- HIGH_RISK 出现到 amplitude > 3% 的提前量
- HIGH_RISK 出现到 breadth < 30% 的提前量

**目标**：平均提前量 > 3 天

### 2. False Alarm Rate（目标：<30%）

**定义**：HIGH_RISK 但后续 T+60 收益 >= 0 的比例。

**评估**：

- T+20 回填：2026-08-10 回填 2026-07-20 的 T+20 收益
- T+60 回填：2026-09-20 回填 2026-07-20 的 T+60 收益

**目标**：False Alarm 率 < 30%

### 3. State Stability（目标：HIGH_RISK <30%）

**定义**：HIGH_RISK 天数占总天数的比例。

**评估**：

- 每日统计 HIGH_RISK 天数
- 每周汇总 HIGH_RISK 天数比例

**目标**：HIGH_RISK 比例 < 30%

## 每周回顾

每周五运行：

```powershell
.\shadow-production\shadow-validation-weekly.ps1 -Scope cn
```

生成 `reports/shadow-validation/weekly_review_{week}.md`，包含：

- 本周 HIGH_RISK 天数
- 本周 Transition Detection 事件
- 本周 Lifecycle 事件（Entry / Peak / Recovery）
- 本周 False Alarm 事件
- 3 个关键指标状态

## 2 周 Checkpoint（2026-08-03）

评估：

- Event frequency 是否符合预期
- Regime distribution 是否稳定
- 如果 0 events 连续 2 周，重新评估入口条件

## 验收标准（进入 TASK-165 前必须满足）

| 指标 | 要求 |
|---|---|
| HIGH_RISK 天数比例 | < 30% |
| False Alarm 率 | < 30% |
| Recovery 平均时间 | < 10 天 |
| Transition 提前量 | > 3 天 |
| Integrity Gate 通过率 | 100% |

## 禁止事项

- ❌ 不允许 DecisionEngine 消费 ShadowRiskAssessment
- ❌ 不允许修改 ExecutionPolicy
- ❌ 不允许自动交易
- ❌ 不允许宣称模型已经跨周期完成验证
- ❌ 不允许在 Shadow Validation 期间调整 HoldingRiskScore 权重
- ❌ 不允许新增 Evidence（当前 Evidence 已足够，避免过拟合）

## 相关文件

- `crates/execution-replay/src/shadow_deployment.rs`
- `crates/execution-replay/src/shadow_deployment_formatter.rs`
- `reports/shadow-validation/`（运行产物，gitignored）
- `docs/v8/adr-105-evidence-horizon-and-role-model.md`
- `research/validation/execution/README.md`
- `shadow-production/shadow-validation-daily.ps1`
- `shadow-production/shadow-validation-weekly.ps1`

## 下一阶段

Shadow Validation 完成后，进入 **TASK-165 Decision Integration Proposal**：

1. 分析 2-4 周的 Shadow Validation 数据
2. 确定 Risk State Machine 与真实市场的同步性
3. 决定是否将 ShadowRiskAssessment 升级为 DecisionEvidence
4. 如果升级，定义 DecisionEngine 消费方式（advisory weight 从 0 开始）
5. **前提**：HK scope 迁移性验证 + Sample Sufficiency 验证（TASK-173）
- 与上周对比

## 回填机制

- **T+20 回填**：2026-08-10 回填 2026-07-20 的 T+20 收益
- **T+60 回填**：2026-09-20 回填 2026-07-20 的 T+60 收益
- 使用 `execution-statistics` 或 `risk-lifecycle` 命令重新分析回填后的数据

## 验收标准

4-8 周 Shadow Validation 后，满足以下条件才进入 TASK-165 Decision Integration Proposal：

| 指标 | 要求 |
|---|---|
| HIGH_RISK 天数比例 | < 30%（避免过度敏感） |
| False Alarm 率 | < 30%（HIGH_RISK 但后续上涨） |
| Recovery 平均时间 | < 10 天（避免滞后） |
| Simulated Action 震荡 | < 20%（避免频繁切换） |
| Transition 提前量 | > 3 天（HIGH_RISK 到大跌） |

## 禁止事项

- ❌ 不允许 DecisionEngine 消费 ShadowRiskAssessment
- ❌ 不允许修改 ExecutionPolicy
- ❌ 不允许自动交易
- ❌ 不允许宣称模型已经跨周期完成验证
- ❌ 不允许在 Shadow Validation 期间调整 HoldingRiskScore 权重

## 相关文件

- `crates/execution-replay/src/shadow_deployment.rs`
- `crates/execution-replay/src/shadow_deployment_formatter.rs`
- `reports/shadow-validation/`（运行产物，gitignored）
- `docs/v8/adr-105-evidence-horizon-and-role-model.md`
- `research/validation/execution/README.md`

## 下一阶段

Shadow Validation 完成后，进入 **TASK-165 Decision Integration Proposal**：

1. 分析 4-8 周的 Shadow Validation 数据
2. 确定 Risk State Machine 与真实市场的同步性
3. 决定是否将 ShadowRiskAssessment 升级为 DecisionEvidence
4. 如果升级，定义 DecisionEngine 消费方式（advisory weight 逐步增加，从 0 开始）
