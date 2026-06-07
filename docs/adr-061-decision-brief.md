# ADR-061 Decision Brief

## Status: Awaiting User Acceptance

## Background

From Wave 7.5 to Wave 10, the project's core challenge shifted from implementation bugs to **conceptual definition errors**. The biggest risk is no longer "does the code work?" but "are we measuring the right thing?"

## The Discovery Chain

1. **Wave 7.5**: Alignment gate (0.75) not met. Assumed threshold problem.
2. **Wave 8**: Found persistence bug (10d → 1d) and HK anchor bug (HSI → HSCEI).
3. **Wave 9**: Discovered Ground Truth mismatch. Old GT measured technical patterns, not macro states.
4. **Wave 10**: Discovered RiskOff has HIGHEST returns. Realized State Layer ≠ Return Prediction.

## The Core Problem

```
Old Assumption:
RiskOff = "Market will crash" → Future returns negative

New Evidence:
RiskOff = "Uncertainty elevated" → Higher volatility + higher risk premium
```

**CN RiskOff 60d return: 5.37% (vs RiskOn 3.04%)**
**HK RiskOff 60d return: 7.93% (vs RiskOn 3.51%)**

## ADR-061 Proposal

### State Definitions

| State | Definition | Used For | NOT Used For |
|-------|-----------|----------|--------------|
| **RiskOff** | Uncertainty-elevated state. High VIX, strong dollar, or weak trend. | Risk management, position sizing, volatility expectation | Predicting negative returns, "sell everything" |
| **RiskOn** | Momentum-favorable state. Positive trend, calm environment. | Trend following, momentum strategies | Predicting guaranteed positive returns |
| **Neutral** | Low-conviction state. No strong directional signal. | Default allocation, rebalancing | Predicting sideways markets |

### Validation Framework

**New State Layer Ground Truth:**
- RiskOff: VIX > 75th percentile OR Dollar > 75th percentile OR close < MA60
- RiskOn: close > MA20 AND VIX < 50th AND Dollar < 50th
- Neutral: Everything else

**Key difference:** Validates current market conditions, not future returns.

## Evidence

### Implementation Alignment ✅
- Code audit confirms: macro-engine implementation matches semantic contract
- All factors (VIX, Dollar, rates) are inverted (high raw = low score = bad)
- Regime thresholds align with definitions

### State Economics ✅
- RiskOff: Highest returns, highest volatility, worst drawdowns
- RiskOn: Moderate returns, highest volatility (HK), worst drawdowns (HK)
- Neutral: Lowest returns, lowest volatility, smallest drawdowns

### GT Validation ✅
- New State GT: Accuracy 0.390 (CN), all 3 classes represented
- Old Technical GT: Accuracy 0.356 (CN), RiskOff = 0 days (artificially inflated)

## Decision Options

### Option A: Accept ADR-061

**Immediate:**
- State Layer definition FROZEN
- State Layer Ground Truth FROZEN

**Week 1-2:**
- Implement new State GT computation
- Recompute Alignment/Information with new GT

**Week 3-4:**
- Unfreeze TASK-004
- Calibrate regime thresholds (current regime over-predicts RiskOn by 73%)

**Month 2:**
- Begin TASK-070C (Economic Layer Target Discovery)

### Option B: Reject ADR-061

**Consequence:**
- State Layer remains undefined
- All metrics (Alignment, Information, Economic Separation) lack unified interpretation
- TASK-004 cannot be safely calibrated (optimizing for undefined target)
- Risk of another Wave 7.5-style false start

## Recommendation

**Accept ADR-061.**

The evidence is comprehensive, the code aligns, and the concept resolves the fundamental confusion that plagued Waves 7-9. The risk of accepting is low; the risk of not accepting is continued conceptual drift.

## Next Step

Awaiting user decision: **Accept / Modify / Reject**
