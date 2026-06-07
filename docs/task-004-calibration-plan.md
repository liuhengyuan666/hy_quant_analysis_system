# TASK-004 Calibration Plan (Post-TASK-071B)

**Status:** Ready for execution pending user approval
**Prerequisite:** TASK-071B completed — regime confirmed as **descriptive**, RiskOff = **synchronous**

---

## Current Problem

Regime predictions vs State GT (from TASK-071A):

| State | Regime Predicts | State GT | Gap |
|-------|----------------|----------|-----|
| **RiskOff** | 231 days (CN) | 317 days (CN) | **-27% under-predicted** |
| **RiskOn** | 111 days (CN) | 64 days (CN) | **+73% over-predicted** |
| **Neutral** | 168 days (CN) | 175 days (CN) | ~matched |

**Root cause:** Current thresholds are asymmetric — too easy to enter RiskOn, too hard to enter RiskOff.

---

## Calibration Strategy

### Principle: Align Regime with State GT

Since the regime is **descriptive**, calibration should make it better at **describing current conditions** (as measured by State GT), not better at **predicting future returns**.

### Target: Match Regime Distribution to State GT Distribution

**CN Target Distribution:**
- RiskOff: 57% (317 / 556 days)
- Neutral: 31% (175 / 556 days)
- RiskOn: 12% (64 / 556 days)

**Current Regime Distribution:**
- RiskOff: 42% (231 / 556 days)
- Neutral: 30% (168 / 556 days)
- RiskOn: 20% (111 / 556 days)

### Required Changes

1. **Increase RiskOff coverage:** +15 percentage points
2. **Decrease RiskOn coverage:** -8 percentage points
3. **Keep Neutral roughly same:** ±1 percentage point

---

## Specific Threshold Adjustments

### Current Thresholds (from macro-engine)

```rust
RiskOn: trend_score >= 60.0 && liquidity_score >= 50.0 && risk_score >= 55.0
RiskOff: trend_score < 40.0 || risk_score < 40.0
Neutral: everything else
```

### Problem Analysis

**RiskOn is too easy to trigger:**
- Requires: trend >= 60 (strong uptrend) AND risk >= 55 (calm) AND liquidity >= 50
- But `risk_score >= 55` is not very strict (VIX and Dollar need to be below ~55th percentile)
- Result: RiskOn captures too many days

**RiskOff is too hard to trigger:**
- Requires: trend < 40 (downtrend) OR risk < 40 (elevated fear)
- `risk_score < 40` means VIX or Dollar above ~60th percentile
- But State GT says RiskOff should trigger at VIX > 75th OR Dollar > 75th
- Current threshold is actually LESS strict than State GT, but combined with trend requirement, it under-predicts

Wait — this is contradictory. If current risk_score < 40 is LESS strict than State GT's 75th percentile, why does it under-predict?

Answer: Because the trend_score < 40 requirement is very strict (requires close < MA60, which is score 25).

Most RiskOff days in State GT are driven by VIX/Dollar, not by trend. But the current RiskOff trigger requires EITHER trend < 40 OR risk < 40. The risk < 40 alone should capture many days.

Let me re-examine... The issue might be that the risk_score is computed from VIX and Dollar, and perhaps the percentile normalization makes it harder to reach < 40 than expected.

### Proposed Adjustments

**Option 1: Adjust RiskOn (Conservative)**
```rust
RiskOn: trend_score >= 65.0 && liquidity_score >= 50.0 && risk_score >= 60.0
```
- Tighten RiskOn by +5 points on trend and risk
- Expected: Reduce RiskOn from 20% to ~12%

**Option 2: Adjust RiskOff (Aggressive)**
```rust
RiskOff: trend_score < 45.0 || risk_score < 45.0
```
- Loosen RiskOff by +5 points on trend and risk
- Expected: Increase RiskOff from 42% to ~50%

**Option 3: Combined Adjustment (Recommended)**
```rust
RiskOn: trend_score >= 65.0 && liquidity_score >= 50.0 && risk_score >= 60.0
RiskOff: trend_score < 45.0 || risk_score < 45.0
```
- Tighten RiskOn, loosen RiskOff
- Expected: RiskOff ~55%, RiskOn ~10%, Neutral ~35%
- Closer to State GT: RiskOff 57%, RiskOn 12%, Neutral 31%

---

## Validation Plan

### Step 1: Implement New Thresholds
- Modify `macro-engine/src/lib.rs` regime thresholds
- Keep old thresholds as comments for rollback

### Step 2: Regenerate Regime Labels
- Run `compute-macro` with new thresholds
- Compare new distribution vs State GT distribution

### Step 3: Measure Alignment Improvement
- Compute new Alignment against State GT
- Target: Macro F1 > 0.40 (current: 0.314)

### Step 4: Economic Metrics Check
- Verify Sharpe/CAGR remain strong
- Ensure regime still makes money

### Step 5: Rollback if Needed
- If metrics degrade, revert to old thresholds
- Try different adjustment combinations

---

## Success Criteria

| Metric | Current | Target |
|--------|---------|--------|
| RiskOff coverage | 42% | ~55% |
| RiskOn coverage | 20% | ~12% |
| Neutral coverage | 30% | ~33% |
| State GT Alignment | 0.314 | > 0.40 |
| Sharpe (CN) | 1.90 | > 1.50 |
| Sharpe (HK) | 1.53 | > 1.20 |

---

## Risk Assessment

**Risk of calibration:**
- Low. We're adjusting thresholds, not changing the model architecture.
- Rollback is easy (single line change per threshold).

**Risk of NOT calibrating:**
- Regime continues to under-predict RiskOff and over-predict RiskOn.
- Strategy engine receives biased state signals.
- Risk management decisions based on incomplete state information.

---

## Next Steps

**Awaiting user approval to execute this plan.**

If approved:
1. Implement Option 3 thresholds
2. Regenerate regime labels
3. Validate alignment and economic metrics
4. Iterate if needed

**Estimated time:** 1-2 hours for implementation + validation.
