# Current Phase
research

# Active Tasks
- [DONE] [TASK-004] P0: Regime Threshold Calibration — COMPLETED. Two calibration attempts executed and reverted. Threshold calibration alone cannot improve alignment due to feature space mismatch (macro factors vs market structure). Original thresholds retained. See docs/task-004-calibration-findings.md.
- [FROZEN] [TASK-005] P1 (GATED): Expand verifiable Skills — FROZEN pending Wave 10.
- [FROZEN] [TASK-006] P2 (GATED): Insight Quality Evaluation framework — FROZEN pending Wave 10.
- [FROZEN] [TASK-007] P3 (GATED): Allocation Layer — FROZEN pending Wave 10.
- [FROZEN] [TASK-018] Wave 7.4: External Validation — FROZEN pending Wave 10.
- [Done] [ADR-055] Factor-Dominant Regime — REJECTED.
- [Accepted] [ADR-056] Dual-Layer Architecture (State + Economic).
- [Rejected] [ADR-057] HK Liquidity-Dominant — **REJECTED**. Underlying evidence invalidated by ADR-059. HK alignment failure was caused by incorrect anchor selection (HSI instead of HSCEI), not by Liquidity factor dominance.
- [Accepted] [ADR-058] Persistence Simplification (confirmation_days = 1).
- [Accepted] [ADR-059] HK Anchor Symbol Fix (HSI → HSCEI).
- [Needs Revision] [ADR-060] Regime Ground Truth Definition — Wave 9 completed. User critique incorporated: State Layer should NOT be evaluated against Forward Return. Need State Truth Discovery first.
- [Accepted] [ADR-061] State Layer Semantic Contract — Wave 10 completed. State Layer definition FROZEN.
- [Accepted] [ADR-062] Three-Layer Evaluation Framework — ACCEPTED 2026-06-07. Separate evaluation metrics for State/Economic/Allocation layers. Alignment Gate > 0.75 officially废弃 for State Layer.

# Wave 11: Evaluation Framework Redesign + Economic Layer v2 (PROPOSED)

## Direction

正式停止 State Layer 调参。开始：

1. **ADR-062 Acceptance** — 三层分离评估框架
2. **Economic Layer v2** — 扩展特征空间，建立独立评估流程
3. **Allocation Layer 重构** — 明确输入契约，实现端到端链路

## 核心认知

> "State Layer 已经完成了它应该完成的职责，而我们过去一直拿错误的指标评估它。"

这是整个项目最有价值的认知收获。

## 三层职责分离

| Layer | 职责 | 核心问题 |
|-------|------|---------|
| **State Layer** | 描述当前宏观市场环境 | "我们现在处于什么状态？" |
| **Economic Layer** | 预测未来收益分布 | "未来N天的收益分布如何？" |
| **Allocation Layer** | 生成具体仓位决策 | "应该持有什么仓位？" |

## State Layer 已完成

- ADR-058: Persistence=1 ✅
- ADR-059: HSI→HSCEI ✅
- ADR-061: 语义定义冻结 ✅
- TASK-004: 阈值校准证明不可行 ✅

## Wave 11 执行状态

### 已完成
- **ADR-062 Accepted** — 三层分离评估框架 ✅
- **TASK-080A Completed** — Economic Feature Inventory & Architecture Design ✅
  - 13 MVP候选因子确定（含FRED可用性验证）
  - 架构修订：多维Economic Scores输出（无过早状态坍缩）
  - NFCI降级为Composite Validation Factor
- **TASK-080B Completed** — Feature Orthogonality Audit ✅
  - 从13个候选因子筛选到**10个核心因子**
  - 移除：IG Spread（冗余）、BBB Spread（冗余）、M2（低频低预测力）
  - 保留：VIX, HY Spread, Fed Funds, 10Y, 2Y, Term Spread, SOFR, Dollar, Initial Claims, NFCI(validation)
  - 方法：基于发表研究的Pearson/Spearman/MI/Predictive Orthogonality分析
  - **代码库确认**：当前生产环境仅4个FRED因子（VIXCLS, DGS10, DTWEXBGS, DFF），全部hardcoded在app-service/src/lib.rs:2076，无配置文件驱动

### 已完成
- **TASK-080C Completed** — Economic Predictive Audit ✅
  - 4个现有因子实证分析（VIX, 10Y Treasury, Dollar, FedFunds）
  - 6个缺失因子基于发表研究的预测力估计
  - **关键发现：**
    - VIX: 强负向预测因子（CN/HK 120d IG=0.115/0.117）
    - Dollar Index: HK强负向预测（120d IG=0.485），CN弱
    - 10Y Treasury: CN中等正向预测（60d IG=0.265）
    - FedFunds:  score聚集问题（零利率期→快速加息），信号不可靠
  - **用户关注点已回答：** Economic Layer 不会变成纯"Credit Layer"，因为VIX和Dollar有可比预测力

- **TASK-080D Completed** — Economic Taxonomy Discovery ✅
  - K-means聚类分析（k=2/3/4/5）
  - **推荐：3-State（Favorable / Neutral / Unfavorable）**
    - 多因子方差比=0.862（强分离）
    - Favorable=20%, Neutral=41%, Unfavorable=39%
  - 4-State也有效（方差比=0.919），但初期建议从3-State开始
  - **关键 caveat：** FedFunds score聚集导致当前taxonomy有偏，修复后需重新运行
  - 连续分数方案被数据拒绝（所有因子均显示离散聚类）

- **TASK-080E Completed** — Fed Funds Regime Distortion Audit ✅
  - **核心发现：Fed Funds raw level 是 regime identifier，不是 predictive signal**
  - 分布：33.3% near-zero (2020-2021), 44.9% high (2022-2023), 仅21.8% mid
  - **修复方案：使用 252d rolling Z-Score 替代 raw level**
  - Z-Score CN 120d IG = 1.005（vs raw level 0.474）
  - Δ (first difference) 太稀疏：97.4% days = 0，不适合 daily model

- **TASK-080F Completed** — Fed Funds Z-score Integration + 080C/080D Re-run ✅
  - 代码修复：`macro-engine/src/lib.rs` 中 Fed Funds 使用 252d Z-Score + ±3 capping
  - 数据迁移：2,341 行 Fed Funds scores 更新为 Z-Score（范围 5.0-95.0，无 0/100 聚集）
  - 080C Re-run：CN 120d IG 0.474 → **0.964**（+103%），HK 120d IG 0.237 → **0.524**（+121%）
  - 080D Re-run：3-State 方差比 0.862 → **0.843**（稳定），结构保持 Favorable/Neutral/Unfavorable
  - **ADR-063 推荐冻结** — Taxonomy 结构稳定，Fed Funds 不再主导聚类
  - **建议进入 Shadow Production** — 90天观察期，不实盘执行

### 待启动
- **ADR-063 Acceptance** — 3-State Economic Taxonomy 冻结（等待用户确认）
- **Shadow Production** — 90天 paper-trading 观察期（等待用户确认）
- **TASK-081** — Economic Layer v2 Prototype（ADR-063 冻结后）

# Wave 9: Ground Truth Definition (ADR-060) — COMPLETED WITH REVISION

## Phase: research

## Root Cause
TASK-035B revealed that the current Ground Truth (drawdown>20% + MA crossover) does NOT match what the regime is designed to detect (macro factor states: trend/risk/liquidity).

## Wave 9 Execution

**All tasks completed:**
- TASK-060A.1: Forward Return Distribution Audit ✅
- TASK-060A.2: 3 GT schemes designed (GT-25, GT-33, GT-10) ✅
- TASK-060B: Label sets generated ✅
- TASK-060C: Alignment computed for all variants ✅
- TASK-060D: Information Score measured ✅

## Critical User Critique (Post-Wave 9)

**Wave 9 made a logical error:**

> "Information≈0 → regime 不预测 future return → regime 的价值来自别的机制"

这个推导证据不足。

**核心问题：** Wave 9 把 `State Classification` 变成了 `Return Classification`。

按 ADR-056：
- **MarketStateRegime** = 描述市场状态
- **EconomicRegime** = 预测经济收益

但 Wave 9 做的是 `State Layer vs Future Return`，然后发现不匹配。这是**预期结果**，因为本来就不是一个目标。

**典型反例（2020疫情底部）：**
```
2月: RiskOff, 流动性崩溃, VIX爆炸
↓
未来60天收益: 反而极高
```

按 Forward Return GT，RiskOff 会被标记为 RiskOn Label（因为未来收益高），导致 regime 预测正确但 GT 判定错误，Information 下降。**但这不意味着 regime 没价值。**

**RiskOff ≠ Crash**

在真实金融世界中，RiskOff 经常意味着：
- 高风险环境
- 信用利差扩大
- 流动性收缩
- 趋势恶化

但指数可能继续涨（如2018贸易战、2022加息周期）。

## Revised Conclusion

**当前证据只能证明：**
> State Layer 与「60日未来收益分位数」相关性很低。

**不能证明：**
- State Layer 没有预测能力
- State Layer 的价值来自"神秘机制"

**ADR-060 不应定义 Ground Truth = Forward Return。**

应该回到基础问题：
> **RiskOn / Neutral / RiskOff 在这个系统里究竟代表什么？**

## Impact
- **Alignment Gate**: Remains UNDEFINED.
- **TASK-004**: Remains FROZEN until State Truth is defined.
- **ADR-056 Dual-Layer**: Remains valid. State Layer and Economic Layer are separate.
- **ADR-060**: Status changed to Needs Revision.

# Wave 10: State Truth Discovery

## 方向
不是 "Mechanism Discovery"，而是 "State Truth Discovery"。

先定义清楚 **State Layer 应该预测什么**，再继续往下走。

## 执行状态

### ADR-061: State Layer Semantic Contract ✅ ACCEPTED
- State Layer 定义已冻结
- TASK-004 仍冻结（等待 TASK-071B 结果解读）

### TASK-070A: State Label Taxonomy Audit ✅ COMPLETE
- 已起草 ADR-061 State Layer Semantic Contract

### TASK-070B: State Persistence Economics ✅ COMPLETE
- CN/HK 三状态经济统计已计算
- **关键发现：RiskOff 收益最高！**

### TASK-071A: State GT Validation Demo ✅ COMPLETE
- 新 State GT 原型实现并验证
- 揭示 Regime 过度乐观（RiskOn 过度预测 73%）

### TASK-071B: State Lead/Lag Analysis ✅ COMPLETE
- **关键发现：RiskOff 是滞后指标！**
- During = 负收益, After = 强反弹

### TASK-070C: Economic Layer Target Discovery ⏸️ DEFERRED
- 等待 State Layer 时序问题理解后执行

## TASK-070B 关键发现

### CN (000300)
| State | 20d Return | 60d Return | Volatility | Max DD |
|-------|------------|------------|------------|--------|
| RiskOn | 1.36% | 3.04% | 0.97% | -6.39% |
| Neutral | 0.08% | 3.00% | 0.92% | -5.77% |
| RiskOff | **2.65%** | **5.37%** | **1.18%** | **-7.57%** |

### HK (HSCEI)
| State | 20d Return | 60d Return | Volatility | Max DD |
|-------|------------|------------|------------|--------|
| RiskOn | 2.08% | 3.51% | **1.74%** | **-12.89%** |
| Neutral | 1.08% | 5.64% | 1.42% | -9.13% |
| RiskOff | **2.98%** | **7.93%** | 1.61% | -10.18% |

### 结论

**RiskOff = High Uncertainty + High Risk Premium**

Not:
- ❌ "Market will crash"
- ❌ "Future returns will be negative"

But:
- ✅ "Environment uncertainty is elevated"
- ✅ "Risk assets carry higher risk premium"
- ✅ "Volatility is elevated"
- ✅ **"Expected returns may actually be higher"**

## TASK-071B 关键发现：RiskOff 是滞后指标

### CN RiskOff Episodes (n=39)
| Phase | Avg Return | Interpretation |
|-------|-----------|----------------|
| Before 20d | +0.56% |  modest gains before |
| Before 60d | +3.65% |  continued gains before |
| **During** | **-0.69%** | **DECLINE during episode** |
| After 20d | +2.70% |  strong rebound |
| After 60d | +4.43% |  continued rebound |

### HK RiskOff Episodes (n=39)
| Phase | Avg Return | Interpretation |
|-------|-----------|----------------|
| Before 20d | +0.83% |  modest gains before |
| Before 60d | +6.03% |  continued gains before |
| **During** | **-0.58%** | **DECLINE during episode** |
| After 20d | +3.16% |  strong rebound |
| After 60d | +6.79% |  continued rebound |

### Episode Duration
- RiskOff: 6.5-6.6 days (longest)
- RiskOn: 3.8-4.0 days
- Neutral: 3.1 days (shortest)

### 核心洞察

**RiskOff 不是领先指标，是滞后指标。**

Pattern:
```
Before:  市场还在涨（+0.5-0.8% in 20d）
During:  市场下跌（-0.6-0.7%）← RiskOff 出现在这里
After:   市场反弹（+2.7-3.2% in 20d）
```

**这意味着：**
- Regime 在市场已经开始下跌后才识别 RiskOff
- RiskOff 不是在"预测"危机，是在"确认"危机
- 随后 Strong rebound 证实了均值回归

### 三种解释验证

用户提出的三种可能：

**A. 均值回归** ✅ 部分正确
- After 20d: +2.7-3.2% 确实显示均值回归
- 但这解释不了 Before/During 的模式

**B. 状态滞后** ✅ 强证据支持
- RiskOff 出现在 During = 负收益
- 证明 regime 在追认风险，不是预测风险

**C. RiskOn 定义错误** ⏳ 待验证
- RiskOn Before 20d: CN +1.25%, HK +4.36%
- RiskOn During: CN +1.02%, HK +1.67%
- RiskOn 确实出现在上涨期间，但可能是"买得太晚"

## TASK-071B 完整解读：三种状态时序画像

### RiskOn："趋势跟随者"（Trend Follower）

| 阶段 | CN | HK | 模式 |
|------|-----|-----|------|
| Before 20d | +1.25% | +4.36% | 上涨已在发生 |
| During | +1.02% | +1.67% | 上涨继续 |
| After 20d | +0.64% | +1.01% | **上涨减速** |

**签名：** Before > During > After（递减）

这是**动量信号在趋势启动后入场**的经典模式。不是"预测趋势"，是"确认趋势"。

### Neutral："整理期探测器"（Consolidation Detector）

| 阶段 | CN | HK | 模式 |
|------|-----|-----|------|
| Before 60d | +4.16% | +6.55% | 大涨之后 |
| During | +0.23% | +0.09% | **横盘整理** |
| After 20d | +0.53% | +1.12% | 温和反弹 |

**签名：** Strong Before → Flat During → Modest After

Neutral 出现在**大涨后的整理期**。

### RiskOff："危机确认者"（Crisis Confirmer）

| 阶段 | CN | HK | 模式 |
|------|-----|-----|------|
| Before 20d | +0.56% | +0.83% | 市场还在涨 |
| During | **-0.69%** | **-0.58%** | **市场下跌** |
| After 20d | +2.70% | +3.16% | 强劲反弹 |

**签名：** Modest Before → Negative During → Strong Rebound After

RiskOff 识别**正在发生的不确定性**，不是预测未来的危机。

---

## 核心结论：Regime 是状态分类器（Descriptive），不是预测器（Predictive）

| 状态 | 识别什么 | 相对于市场走势 |
|------|---------|--------------|
| **RiskOn** | "我们在趋势中" | 趋势已启动后 |
| **Neutral** | "趋势已暂停" | 大涨后的整理 |
| **RiskOff** | "我们在压力中" | 下跌发生时 |

**这与 ADR-061 完全一致：**
- ADR-061 定义状态为描述性（"Uncertainty-elevated state"）
- 实现正确地识别了当前状态
- 没有矛盾，没有错误

---

## 对 TASK-004 的影响：建议方向

### 建议：接受 Descriptive Regime，开始阈值校准

**理由：**
1. ADR-061 明确定义状态为描述性
2. TASK-071B 证实实现与定义一致
3. 预测性 regime 需要根本性重新设计（超出阈值校准范围）
4. 描述性 regime 仍可为策略层提供价值（状态适配）

### RiskOff 目标时序：Synchronous（同步）

| 目标类型 | 特征 | 证据 | 判定 |
|---------|------|------|------|
| **Leading** | Before = 负收益 | Before = +0.5% | ❌ 不支持 |
| **Synchronous** | During = 负收益 | During = -0.7% | ✅ **最佳匹配** |
| **Lagging** | After = 负收益 | After = +2.7% | ❌ 不支持 |

**RiskOff 应识别"正在发生的不确定性"，不是"将要发生的不确定性"。**

### 校准方向（基于 Descriptive Regime）

**当前问题：**
- RiskOn 过度预测（111 实际 vs 64 GT，+73%）
- RiskOff 预测不足（231 实际 vs 317 GT，-27%）

**校准方向：**
- 收紧 RiskOn 条件（当前过于宽松）
- 放宽 RiskOff 条件（当前过于严格）
- 使 regime 与 State GT 对齐

---

## What We Now Know

| Question | Answer |
|----------|--------|
| Was HK "broken" due to persistence? | **Partially.** 10d made it worse, but broken trend_score was the bigger issue. |
| Is HK still broken at 1d? | **NO.** With fixed trend_score, HK Alignment=0.286 (outperforms CN=0.252). |
| Is CN threshold issue genuine? | **Yes, but less severe.** Alignment=0.252 at 1d (below 0.75 gate, but much improved). |
| Should we optimize State Layer for Sharpe? | **No.** State Layer mission is state classification, not return maximization. |
| Is the Alignment gate (0.75) appropriate? | **UNDEFINED.** Gate must be derived from valid State Truth, not imposed. |
| ADR-057 needed? | **NO.** HK is not broken. Liquidity Dominant is unnecessary. |
| What is State Layer designed to predict? | **Market states**, not future returns. Need State Truth Discovery. |
| Is current Ground Truth valid? | **NO.** Technical patterns (drawdown+MA) don't match macro factor states. |
| Is Forward Return the right GT? | **NO.** Forward Return should be Economic Layer GT, not State Layer GT. |
| Primary forward horizon? | **60d** for Economic Layer evaluation (not State Layer). |

# Wave 8: Post-Persistence Revalidation (TASK-035A)

## Root Cause
TASK-034C proved that confirmation_days=10 exceeds typical state lifetime (CN median=2d, HK median=3d).
This means Wave 7.5 entire analysis chain was run on severely distorted state sequences.
86% of CN episodes and 72% of HK episodes were swallowed at 10d.

## Decision
1. Change production confirmation_days from 10 → 1 (ADR-058 Accepted)
2. Do NOT refresh production data yet. Run shadow analysis first.
3. Freeze all State Layer economic optimization (TASK-035, TASK-036, TASK-004) until revalidation complete

## Phase 1: TASK-035A.0 ✅ COMPLETE

Baseline panel established. Production data NOT refreshed.

## Phase 2: Shadow Revalidation ✅ COMPLETE

### CN (000300) — 1d vs 10d

| Metric | 1d | 10d | Change |
|--------|-----|------|--------|
| Alignment | **0.252** | 0.113 | **+123%** |
| Information | **0.962** | 0.816 | **+18%** |
| Economic Separation | 1.3 | 3.1 | -58% |
| State Only CAGR | **21.71%** | -0.37% | **+5968%** |
| State Only Sharpe | **1.90** | -0.03 | **+6433%** |
| Baseline CAGR | **21.71%** | -0.37% | **+5968%** |
| Baseline Sharpe | **1.90** | -0.03 | **+6433%** |
| Dual Layer CAGR | **18.99%** | 1.38% | **+1276%** |
| Dual Layer Sharpe | **1.82** | 0.27 | **+574%** |

### HK (HSCEI) — 1d vs 10d (BROKEN trend_score)

**These results were computed with the broken HSI anchor (trend_score always = 50).**

| Metric | 1d | 10d | Change |
|--------|-----|------|--------|
| Alignment | **0.007** | 0.000 | **IMPROVED** but still ~0 |
| Information | **0.544** | 0.282 | **+93%** |
| State Only CAGR | **15.63%** | 7.34% | **+113%** |
| State Only Sharpe | **1.41** | 0.65 | **+117%** |

### HK (HSCEI) — 1d vs 10d (FIXED trend_score from HSCEI bars)

**These results were computed with fresh trend_score from actual HSCEI bars.**

| Metric | 1d (FIXED) | 10d | Change |
|--------|-----------|-----|--------|
| **Alignment** | **0.286** | 0.226 | **+27%** |
| **Information** | **0.961** | 0.999 | High |
| **State Only CAGR** | **22.96%** | 4.12% | **+457%** |
| **State Only Sharpe** | **1.53** | 0.31 | **+394%** |
| **Baseline CAGR** | **22.96%** | 4.12% | **+457%** |
| **Baseline Sharpe** | **1.53** | 0.31 | **+394%** |
| **Dual Layer CAGR** | **20.85%** | 5.82% | **+258%** |
| **Dual Layer Sharpe** | **1.49** | 0.41 | **+263%** |

### Key Findings from Phase 2

1. **CN: EVERYTHING improves at 1d**. Alignment doubles, Sharpe goes from -0.03 to 1.90.
2. **HK was NEVER broken at 1d**. The "HK is broken" conclusion was entirely due to the missing HSI bars bug.
3. **HK with fixed trend_score OUTPERFORMS CN in Alignment** (0.286 vs 0.252).
4. **HK with fixed trend_score shows Sharpe=1.53 and CAGR=22.96%** — comparable to CN.
5. **Economic Separation is LOWER at 1d for both markets**. This confirms 10d was creating artificial separation through state suppression.
6. **Wave 7.5 core error**: "We mistook a data ingestion bug (missing HSI bars) for regime failure."

## TASK-035B: Ground Truth Audit ✅ COMPLETE

### Critical Discovery: Ground Truth Definition Mismatch

**CN Actual (Ground Truth)**:
- RiskOff (>20% drawdown): **0%** (0 days)
- RiskOn (close>MA20 && MA20>MA60): **35.7%** (189 days)
- Neutral: **64.3%** (341 days)

**CN Predicted (Regime @ 1d)**:
- RiskOff: **45.7%** (242 days)
- RiskOn: **22.1%** (117 days)
- Neutral: **32.3%** (171 days)

**Key Finding**: The regime predicts 45.7% RiskOff days, but actual drawdowns >20% occurred on 0 days. This destroys Alignment (0.252) but the regime still makes money (Sharpe=1.90).

**Root Cause**: The Ground Truth definition (drawdown>20% + uptrend via MA) does NOT match what the regime is actually detecting (macro factor states: trend/risk/liquidity scores).

**Implication**: The Alignment metric may be measuring the wrong thing. The regime is capturing economically meaningful macro states, not technical patterns.

## HK Critical Bug: FIXED ✅

**Root Cause**: `app-service/src/lib.rs:2122` hardcoded "HSI" as HK anchor, but database has NO HSI bars. `fetch_daily_bars` returned empty, causing `trend_score` to always default to 50.0.

**Fix**: Changed HK anchor from "HSI" to "HSCEI" in `app-service/src/lib.rs:2122`.

**Database verification**:
- `daily_bar` contains: HSCEI, HSTECH, HSAHP
- `daily_bar` does NOT contain: HSI

**Impact**: All HK Wave 7.5 experiments were run with broken trend_score. Must re-run HK analysis after fix.

## Phase 3: Production Refresh (Final Gate)

### CN: ✅ APPROVED
CN revalidation shows dramatic improvement. Ground Truth mismatch is a metric design issue, not a regime failure.

### HK: ✅ APPROVED
HK revalidation with fixed trend_score shows:
- Alignment=0.286 (outperforms CN's 0.252)
- Sharpe=1.53, CAGR=22.96%
- The "HK is broken" conclusion was entirely due to the HSI bars bug

**Both markets approved for production refresh with confirmation_days=1.**

## Production Refresh Plan (Approved)

### Pre-Refresh Checklist

- [ ] **Step 1: Backup existing regime data**
  ```sql
  -- ClickHouse backup (recommended)
  CREATE TABLE quant.market_regime_snapshot_backup_20260607 AS SELECT * FROM quant.market_regime_snapshot;
  ```
  Or export current regime data to file for rollback.

- [ ] **Step 2: Execute compute-macro**
  ```bash
  cargo run --release -p quant-cli -- compute-macro --from 2024-01-01 --to 2026-03-16
  ```
  This will:
  - Use `confirmation_days=1` (ADR-058)
  - Use HSCEI for HK trend_score (ADR-059)
  - Regenerate all regime snapshots

- [ ] **Step 3: Verify dashboard**
  ```bash
  cargo run --release -p quant-cli -- dashboard-snapshot --scope cn
  cargo run --release -p quant-cli -- dashboard-snapshot --scope hk
  ```
  Check:
  - RiskOn % is non-zero for both markets
  - Information score ≈ 0.96
  - Episode count is reasonable (not 8-12 like 10d)

- [ ] **Step 4: Compare Old vs New**
  Run `audit-label-distribution` before and after to confirm:
  - CN: RiskOn ≈ 22%, RiskOff ≈ 46%, Neutral ≈ 32%
  - HK: RiskOn appears (was 0%), RiskOff ≈ 28%, Neutral ≈ 72%

### Post-Refresh Verification

- [ ] Dashboard loads correctly for GLOBAL/CN/HK
- [ ] Trust Summary shows no critical issues
- [ ] Pipeline diagnostics pass
- [ ] Recent reports can be exported

## Next Phase Priorities

### P0: Production Refresh
Execute the approved refresh plan above.

### P1: Dashboard Verification
Confirm both CN and HK behave correctly after refresh.

### P2: ADR-060 Ground Truth Definition
- Re-evaluate what the regime is designed to predict
- Redefine Ground Truth to match regime design intent
- Re-assess Alignment Gate=0.75 appropriateness

### P3: Re-evaluate Alignment Gate
- Current gate (0.75) may be inappropriate
- Ground Truth (drawdown+MA) doesn't match regime (macro factors)
- Consider macro-factor-based Ground Truth or different metric

### P4: TASK-004 / TASK-035 (STILL FROZEN)
- Do NOT unfreeze until after production refresh stabilizes
- Requires at least one full refresh cycle before calibration
- Priority lowered because fundamental issues (persistence + anchor) are now resolved

## What We Now Know

| Question | Answer |
|----------|--------|
| Was HK "broken" due to persistence? | **Partially.** 10d made it worse, but broken trend_score was the bigger issue. |
| Is HK still broken at 1d? | **NO.** With fixed trend_score, HK Alignment=0.286 (outperforms CN=0.252). |
| Is CN threshold issue genuine? | **Yes, but less severe.** Alignment=0.252 at 1d (below 0.75 gate, but much improved). |
| Should we optimize State Layer for Sharpe? | **No.** State Layer mission is state classification, not return maximization. |
| Is the Alignment gate (0.75) appropriate? | **Questionable.** Ground Truth (drawdown+MA) may not match what regime is designed to detect. |
| ADR-057 needed? | **NO.** HK is not broken. Liquidity Dominant is unnecessary. |

# Constraints
- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护。
- `TradingCalendar` 当前只覆盖 CN/HK。
- `app-service/src/lib.rs` 仍是 monolith（~796 行）。
- Eastmoney 主源从当前环境不可达，全部标的走 Tencent fallback。
- P2 turnover 修复仅影响新拉取数据，存量 ClickHouse 数据需 `ingest-daily` 回填。
- **Wave 7.5 所有结论需在 1d persistence 下重新验证，暂不基于 10d 结果做进一步决策。**
- **Production regime data refresh APPROVED for both CN and HK with confirmation_days=1.**
- **ADR-057 HK Liquidity Dominant is NOT needed. HK was never broken.**
