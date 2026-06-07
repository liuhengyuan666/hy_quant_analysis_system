# Kill Criteria — Model Health Monitoring

## Overview

These criteria define when the model requires human review during the 90-day Shadow Production period.

**Rule:** If ANY criterion is triggered, STOP and review. Do not continue observation blindly.

---

## State Layer Kill Criteria

### Criterion S1: Coverage Anomaly

**Trigger:** RiskOff state > 80% for 30 consecutive days

**Rationale:** State Layer should maintain diversity. Persistent RiskOff dominance suggests either:
- Market is genuinely in prolonged crisis (rare but possible)
- Thresholds have become miscalibrated vs current market regime
- Data quality issue (e.g., stale FRED data)

**Action:**
1. Check data freshness (FRED fetch status, daily bar completeness)
2. Compare with manual market assessment
3. If data is fresh and market is NOT in crisis → threshold drift suspected → escalate

**Historical Baseline:**
- CN RiskOff: ~45% (1d persistence)
- HK RiskOff: ~28% (1d persistence)

---

### Criterion S2: State Persistence Collapse

**Trigger:** Average state duration < 1.5 days for 14 consecutive days

**Rationale:** With confirmation_days=1, states should persist 2-4 days on average. Persistent flickering suggests:
- Volatility regime has changed (high noise)
- MA crossover frequency has increased
- Indicator sensitivity too high

**Action:**
1. Check volatility regime (VIX level, ATR)
2. Compare with historical state duration distributions
3. If VIX is elevated → market condition, not model failure
4. If VIX is normal → model may need review

**Historical Baseline:**
- CN median duration: 2d
- HK median duration: 3d

---

### Criterion S3: State Transition Explosion

**Trigger:** > 50% of days show state transitions (both CN and HK) for 14 consecutive days

**Rationale:** Excessive transitions indicate the model is chasing noise rather than identifying stable regimes.

**Action:**
1. Check if transitions are synchronized (CN and HK flip together) → macro shock
2. If unsynchronized → model instability
3. Review recent threshold calibration history

---

## Economic Layer Kill Criteria

### Criterion E1: Information Gain Collapse

**Trigger:** Rolling 90-day Information Gain < 0.1 for Economic Layer

**Rationale:** Economic Layer's core value is predictive power. IG < 0.1 means the taxonomy no longer separates return distributions.

**Measurement:**
- Compute IG weekly using last 90 days of data
- Compare Favorable/Neutral/Unfavorable return distributions
- If separation disappears → model failure

**Historical Baseline:**
- CN 120d IG: 0.964 (post-Z-score fix)
- HK 120d IG: 0.524 (post-Z-score fix)

**Action:**
1. Check factor data quality (FRED fetch, forward-fill status)
2. Check if factor correlations have shifted
3. If data quality is good → taxonomy may need recalibration

---

### Criterion E2: State Distribution Drift

**Trigger:** PSI (Population Stability Index) > 0.25 between current month and training period

**Rationale:** Economic state distribution should remain roughly stable (~37/40/22). Major shifts suggest:
- Structural market change
- Factor normalization decay
- Regime change not captured by model

**PSI Thresholds:**
- PSI < 0.1: Stable
- PSI 0.1-0.25: Moderate drift (monitor)
- PSI > 0.25: Significant drift (review)

**Historical Baseline:**
- Favorable: 37.4%
- Neutral: 40.3%
- Unfavorable: 22.4%

**Action:**
1. Compute PSI weekly
2. If PSI > 0.25 → check which state shifted
3. If Favorable collapsed → check Fed Funds Z-score distribution
4. If Unfavorable exploded → check VIX and credit spreads

---

### Criterion E3: Quintile Return Inversion

**Trigger:** Bottom quintile outperforms top quintile by > 5% (120d horizon) for 60 consecutive days

**Rationale:** The taxonomy assumes higher scores → better returns. Persistent inversion means the signal has reversed.

**Action:**
1. Check if specific factor is driving inversion
2. Check market regime (value vs growth, risk-on vs risk-off)
3. If factor-level inversion → recalibrate that factor
4. If systematic inversion → taxonomy may need revision

---

## Allocation Layer Kill Criteria

### Criterion A1: Paper Portfolio Underperformance

**Trigger:** Paper portfolio Sharpe < 0 for 60 consecutive days

**Rationale:** Even a naive allocation should not consistently destroy value. Sharpe < 0 means the allocation logic is worse than cash.

**Measurement:**
- Compute paper portfolio returns daily
- Rolling 60-day Sharpe
- Compare with buy-and-hold benchmark

**Action:**
1. Check if underperformance is due to allocation logic or market conditions
2. Compare with Economic Layer IG (if IG is good but allocation is bad → allocation logic issue)
3. If IG also collapsed → Economic Layer issue, not Allocation Layer

---

### Criterion A2: Excessive Turnover

**Trigger:** Allocation changes > 30% of days (more than 2x per week) for 30 consecutive days

**Rationale:** High turnover increases transaction costs and suggests the allocation is chasing noise.

**Action:**
1. Check if turnover is driven by State Layer or Economic Layer
2. If State Layer → check S2 (persistence collapse)
3. If Economic Layer → check E2 (distribution drift)

---

### Criterion A3: Drawdown Breach

**Trigger:** Paper portfolio max drawdown > 20% from peak

**Rationale:** Risk management boundary. Even in paper trading, excessive drawdown suggests the model is not respecting risk limits.

**Action:**
1. Check if drawdown is market-wide or model-specific
2. Compare with benchmark drawdown
3. If model drawdown > 1.5x benchmark → allocation logic too aggressive

---

## Data Quality Kill Criteria

### Criterion D1: FRED Data Outage

**Trigger:** > 2 FRED factors fail to fetch for 7 consecutive days

**Rationale:** Economic Layer depends on FRED data. Persistent outage means Economic Layer is running on stale forward-filled data.

**Action:**
1. Check network connectivity to FRED
2. Check if FRED API has changed
3. If unresolvable → Economic Layer observation paused until data restored

---

### Criterion D2: Daily Bar Gaps

**Trigger:** > 5% of universe shows data gaps > 3 days for 7 consecutive days

**Rationale:** State Layer depends on daily bars. Persistent gaps mean trend scores are computed on incomplete data.

**Action:**
1. Check data provider status (Eastmoney, Tencent)
2. Run `check-data-health` diagnostics
3. If provider issue → wait for resolution
4. If ingestion bug → fix and backfill

---

## Kill Criteria Summary Table

| Layer | Criterion | Trigger | Action |
|-------|-----------|---------|--------|
| State | S1: Coverage Anomaly | RiskOff > 80% for 30d | Check data + manual assessment |
| State | S2: Persistence Collapse | Avg duration < 1.5d for 14d | Check volatility regime |
| State | S3: Transition Explosion | > 50% transition days for 14d | Check synchronization |
| Economic | E1: IG Collapse | 90d IG < 0.1 | Check factor quality |
| Economic | E2: Distribution Drift | PSI > 0.25 | Check structural shift |
| Economic | E3: Quintile Inversion | Bottom > Top by 5% for 60d | Check factor-level reversal |
| Allocation | A1: Underperformance | Sharpe < 0 for 60d | Check logic vs market |
| Allocation | A2: Excessive Turnover | Changes > 30% of days for 30d | Check driver layer |
| Allocation | A3: Drawdown Breach | Max DD > 20% | Check risk limits |
| Data | D1: FRED Outage | > 2 factors fail for 7d | Pause Economic Layer |
| Data | D2: Bar Gaps | > 5% universe gaps > 3d for 7d | Fix ingestion |

---

## Weekly Review Checklist

Every week, check:

- [ ] S1: RiskOff coverage < 80%?
- [ ] S2: Average persistence > 1.5d?
- [ ] E1: Rolling 90d IG > 0.1?
- [ ] E2: PSI < 0.25?
- [ ] A1: Paper Sharpe > 0?
- [ ] D1: FRED data fresh?
- [ ] D2: No bar gaps?

If ANY check fails → trigger review.

---

## Review Process

When a Kill Criterion is triggered:

1. **Document:** Record the trigger, date, and current market conditions
2. **Diagnose:** Run diagnostics (`check-data-health`, `pipeline-dates`, manual inspection)
3. **Classify:**
   - Data issue → Fix data, continue observation
   - Market regime change → Expected, continue observation
   - Model degradation → Escalate to review committee
4. **Decide:**
   - Continue observation (with monitoring)
   - Pause observation (fix first)
   - End Shadow Production (model fundamentally broken)

---

## Related Documents

- `shadow-production/README.md` — Shadow Production protocol
- `docs/adr-063-economic-taxonomy.md` — Economic Layer taxonomy
- `docs/task-080f-findings.md` — Baseline metrics and thresholds

---

**Effective Date:** 2026-06-07  
**Review Cycle:** Weekly  
**Next Review:** 2026-06-14
