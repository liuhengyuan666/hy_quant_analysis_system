# ADR-062: Three-Layer Evaluation Framework

**Status:** Draft  
**Date:** 2026-06-07  
**Depends on:** ADR-056, ADR-061  
**Supersedes:** Alignment Gate > 0.75 (for State Layer evaluation)

---

## Context

Wave 7.5 → Wave 10 的完整实验链证明了一个核心事实：

> State Layer 与 Forward Return GT 的低相关性，不是 State Layer 的缺陷，而是评估框架的错误。

State Layer、Economic Layer、Allocation Layer 是三个不同职责的层次，但当前所有层次都被同一套指标（Alignment vs Technical GT）评估，导致：

1. State Layer 被错误地要求"预测未来收益"
2. Economic Layer 被 State Layer 的噪声掩盖
3. Allocation Layer 的决策逻辑缺乏清晰的输入契约

---

## Decision

### 原则：三层分离评估

每个层次使用与其职责匹配的独立评估框架。

---

## Layer 1: State Layer (市场环境描述)

**职责：** 描述当前宏观市场环境的状态

**核心问题：** "我们现在处于什么状态？"

**评估维度：**

| 维度 | 含义 | 当前指标 | 目标 |
|------|------|---------|------|
| **Coverage** | 状态覆盖完整性 | — | 所有交易日都有明确状态标签 |
| **Stability** | 状态持续性 | churn_rate | < 25% (避免过度翻转) |
| **Persistence** | 状态寿命合理性 | episode_duration | median ≥ 2d, 避免1d闪烁 |
| **Economic Characteristics** | 各状态经济特征分化 | forward_return_distribution | 各状态有统计学差异 |
| **Semantic Consistency** | 状态语义一致性 | — | 与 ADR-061 定义一致 |

**明确不评估：**
- ❌ Alignment vs Technical GT (Feature Space 不一致)
- ❌ Forward Return 预测准确性 (不是本层职责)
- ❌ Sharpe / CAGR (不是本层职责)

**ADR-061 已冻结 State Layer 语义：**
- RiskOn = "Momentum-favorable environment"
- RiskOff = "Uncertainty-elevated environment"
- Neutral = "Low-conviction environment"

---

## Layer 2: Economic Layer (收益分布预测)

**职责：** 基于宏观/市场因子预测未来收益分布

**核心问题：** "在当前环境下，未来N天的收益分布如何？"

**评估维度：**

| 维度 | 含义 | 指标 |
|------|------|------|
| **Separation** | 不同经济状态下的收益分布分离度 | Economic Separation Score |
| **Forward Return Distribution** | 各经济状态下的前向收益统计 | mean, std, skew, percentiles |
| **Information Gain** | 经济状态对收益预测的信息增量 | Information Score |
| **Calibration** | 预测概率与实际频率的校准度 | reliability diagram |

**输入特征（建议扩展）：**
- 现有：Liquidity (US10Y, Fed Funds)
- 新增：Credit Spread, Dollar, Term Spread, VIX
- 可选：Earnings Revision, PMI, Financial Conditions Index

**明确不评估：**
- ❌ 与 State Layer 的 Alignment (两者是不同层次)
- ❌ 单日准确率 (收益分布是概率性的)

---

## Layer 3: Allocation Layer (仓位决策)

**职责：** 基于 State + Economic 信号生成具体仓位决策

**核心问题：** "基于当前状态和收益预测，应该持有什么仓位？"

**评估维度：**

| 维度 | 含义 | 指标 |
|------|------|------|
| **CAGR** | 复合年化收益 | backtest.cagr |
| **Sharpe** | 风险调整后收益 | backtest.sharpe |
| **Max Drawdown** | 最大回撤 | backtest.max_drawdown |
| **Win Rate** | 胜率 | backtest.win_rate |
| **Turnover** | 换手率 | backtest.turnover |

**输入契约：**
- State Layer：当前市场环境状态
- Economic Layer：当前环境下的收益分布预测
- Risk Budget：用户风险承受能力

**明确不评估：**
- ❌ 单信号准确率 (决策是综合的)
- ❌ 与任何 GT 的 Alignment (决策质量由经济结果衡量)

---

## 旧指标的处置

### Alignment Score

**原用途：** 衡量 Regime 与 Technical GT 的一致性  
**问题：** Technical GT (MA cross + drawdown) 与 State Layer (macro factors) 测量不同事物  
**新定位：** 
- 仅用于 **Economic Layer** 内部验证（如果 Economic GT 也是技术形态）
- 或作为 **辅助参考**，不作为 State Layer 的主要评估指标

### Information Score

**原用途：** 衡量 Regime 对 Forward Return 的信息增量  
**问题：** 对 State Layer 要求过高（要求预测收益）  
**新定位：**
- 作为 **Economic Layer** 的核心指标
- State Layer 的 Information Score 仅用于验证"状态是否有经济特征差异"

### Alignment Gate (0.75)

**原用途：** State Layer 上线的硬性门槛  
**问题：** 基于错误的 GT 定义，已被证伪  
**新定位：**
- **废弃** 作为 State Layer 的硬性门槛
- 替换为 State Layer 的多维评估框架（Coverage + Stability + Persistence + Economic Characteristics）

---

## 三层交互契约

```
┌─────────────────────────────────────────┐
│         Allocation Layer                 │
│  (仓位决策: CAGR / Sharpe / Drawdown)   │
└──────────────────┬──────────────────────┘
                   │
         ┌─────────┴──────────┐
         │                    │
┌────────▼────────┐  ┌────────▼─────────┐
│   State Layer    │  │  Economic Layer  │
│ (环境描述)        │  │ (收益分布预测)    │
└────────┬─────────┘  └────────┬─────────┘
         │                     │
         └──────────┬──────────┘
                    │
         ┌──────────▼──────────┐
         │    Raw Factors      │
         │ (Price / Macro /    │
         │  Sentiment / etc.)  │
         └─────────────────────┘
```

**契约规则：**
1. State Layer 不承诺预测收益，只承诺准确描述当前环境
2. Economic Layer 不承诺预测单日产出，只承诺收益分布的统计特征
3. Allocation Layer 综合两层信号，做出具体仓位决策
4. 评估时严禁跨层使用指标（如用 Sharpe 评估 State Layer）

---

## 实施建议

### Phase 1: 评估框架切换（立即）
1. 废弃 Alignment Gate > 0.75 作为 State Layer 上线标准
2. 建立 State Layer 多维评估面板（Coverage/Stability/Persistence/Economic Characteristics）
3. 将现有 Information Score 从 State Layer 评估中移除

### Phase 2: Economic Layer 增强（短期）
1. 扩展 Economic Layer 特征空间（Credit Spread, Term Spread, etc.）
2. 建立 Economic Layer 独立评估流程
3. 实现 Economic Layer 与 State Layer 的解耦

### Phase 3: Allocation Layer 重构（中期）
1. 明确 Allocation Layer 的输入契约（State + Economic + Risk Budget）
2. 建立三层联调测试框架
3. 实现端到端的"环境描述 → 收益预测 → 仓位决策"链路

---

## Consequences

### Positive
- 每个层次的职责清晰，避免"一刀切"评估
- State Layer 不再被错误地要求预测收益
- Economic Layer 可以独立发展，不受 State Layer 限制
- Allocation Layer 的决策逻辑有据可依

### Negative
- 需要重构现有评估工具（audit 命令）
- 需要新增 Economic Layer 模块
- 需要更新文档和操作手册
- 短期工作量增加

### Risk
- 团队可能不习惯三层分离的思维模式
- 旧指标（Alignment）可能仍被误用
- 需要严格的 code review 确保跨层评估不发生

---

## Related ADRs

- **ADR-056**: Dual-Layer Architecture (State + Economic) — 本 ADR 将其实细化为三层评估框架
- **ADR-061**: State Layer Semantic Contract — State Layer 定义已冻结，本 ADR 为其配套评估框架
- **ADR-060**: Regime Ground Truth — 本 ADR 明确为何 Forward Return GT 不适合 State Layer

---

## Notes

> "State Layer 已经完成了它应该完成的职责，而我们过去一直拿错误的指标评估它。"
> 
> — Wave 10 核心认知收获

---

## Decision Log

| Date | Event |
|------|-------|
| 2026-06-07 | ADR-062 Draft created based on Wave 7.5 → Wave 10 learnings |
| | User review and acceptance pending |
