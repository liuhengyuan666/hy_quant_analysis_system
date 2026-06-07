# TASK-004: Threshold Calibration Findings

**Status:** COMPLETED — Calibration attempted, reverted, findings documented  
**Date:** 2026-06-07  
**Related:** ADR-061, TASK-071B

---

## Objective

Calibrate regime thresholds to improve alignment between Regime predictions and State Ground Truth (GT).

**State GT Distribution (CN):**
- RiskOff: 317 days (57.0%)
- Neutral: 175 days (31.5%)
- RiskOn: 64 days (11.5%)

**Original Regime Distribution (CN):**
- RiskOff: 231 days (41.5%) — under-predicted by 15.5 pts
- Neutral: 168 days (30.2%) — matched
- RiskOn: 111 days (20.0%) — over-predicted by 8.5 pts

---

## Calibration Attempts

### Attempt 1: Moderate Adjustment

**Changes:**
- RiskOn: `trend >= 60.0 && risk >= 55.0` → `trend >= 65.0 && risk >= 60.0` (tightened)
- RiskOff: `trend < 40.0 || risk < 40.0` → `trend < 45.0 || risk < 45.0` (loosened)

**Results:**
- CN Distribution: RiskOff 50.2%, RiskOn 19.6%, Neutral 30.2%
- **CN Accuracy: 0.371** (down from 0.390)
- **CN Macro F1: 0.291** (down from 0.314)

**Verdict:** Distribution moved in right direction but timing alignment worsened.

---

### Attempt 2: Aggressive Adjustment

**Changes:**
- RiskOn: `trend >= 65.0 && risk >= 60.0` → `trend >= 70.0 && risk >= 65.0` (tightened further)
- RiskOff: `trend < 45.0 || risk < 45.0` → `trend < 50.0 || risk < 50.0` (loosened further)

**Results:**
- CN Distribution: RiskOff 55.6%, RiskOn 11.9%, Neutral 32.6%
- **Distribution nearly perfect match to State GT** (57% / 12% / 31%)
- **CN Accuracy: 0.374** (still below original 0.390)
- **CN Macro F1: 0.275** (down from 0.314)
- RiskOff F1: 0.466 (improved)
- RiskOn F1: 0.015 (catastrophic — only ~1 correct out of 66 predictions)

**Verdict:** Distribution matched but timing alignment degraded. RiskOn precision collapsed.

---

## Root Cause Analysis

### Why Calibration Failed

**The regime model and State GT measure different things:**

| Dimension | Regime Model | State GT |
|-----------|-------------|----------|
| **Feature Space** | Macro factors (trend_score, risk_score, liquidity_score) | Market structure (MA cross, volatility, RSI) |
| **Data Source** | VIX, Dollar, US10Y, Fed Funds, price trend | Price action, realized volatility, momentum |
| **What it captures** | Macro environment state | Technical market state |

**Threshold adjustments can match the distribution but not the timing** because:
1. The features are uncorrelated at the daily level
2. A "RiskOn" macro day (calm VIX, strong dollar, low rates) often coincides with a "Neutral" technical day (choppy price action, moderate volatility)
3. The model gets the right proportions but on the wrong days

### Per-Class Breakdown (Attempt 2)

| State | Precision | Recall | F1 | Assessment |
|-------|-----------|--------|-----|------------|
| RiskOff | 0.472 | 0.461 | 0.466 | Reasonable — macro fear often coincides with market stress |
| Neutral | 0.337 | 0.349 | 0.343 | Mediocre — catch-all class |
| RiskOn | 0.015 | 0.016 | 0.015 | **Catastrophic** — macro calm ≠ technical uptrend |

---

## Conclusion

### Threshold Calibration Alone is Insufficient

**Finding:** Changing threshold values cannot bridge a feature space mismatch.

**Implications:**
1. The original thresholds (RiskOn 60/55, RiskOff 40/40) were already near-optimal for the current feature space
2. Further threshold adjustments will not meaningfully improve alignment
3. The low alignment (0.37 accuracy) is a **feature space issue**, not a **threshold issue**

### Recommended Path Forward

**Option A: Accept Descriptive Regime as-is** ⭐ RECOMMENDED
- Regime correctly describes macro environment states
- Low alignment with market structure GT is expected
- Value comes from state-aware strategy adaptation, not prediction

**Option B: Redesign Regime Features**
- Add price-based features (volatility, RSI, momentum) to regime scoring
- This would make regime more aligned with market structure
- Requires architectural changes to macro-engine

**Option C: Build Separate Economic Layer**
- Keep State Layer as descriptive macro classifier
- Build Economic Layer with price-based features for return prediction
- This follows ADR-056 Dual-Layer architecture

---

## Action Taken

1. ✅ Attempted two calibration variants
2. ✅ Measured results against State GT
3. ✅ Documented failure mode
4. ✅ **Reverted to original thresholds** (RiskOn 60/55, RiskOff 40/40)
5. ✅ Regenerated production regime data with original thresholds

---

## Files Modified

- `crates/macro-engine/src/lib.rs` — thresholds modified and reverted
- `docs/task-004-calibration-plan.md` — original plan (superseded)
- `docs/task-004-calibration-findings.md` — this document

---

## Next Steps

1. **Do not attempt further threshold calibration** without feature space changes
2. If alignment improvement is required, pursue Option B or C above
3. Economic Layer (TASK-070C) remains the appropriate venue for return prediction
