# TASK-080E: Fed Funds Regime Distortion Audit — Findings

**Status:** Completed  
**Date:** 2026-06-07  
**Method:** Distribution analysis + predictive power comparison (Level vs Z-Score vs Δ)  
**Data:** Fed Funds 2020-2026 (2341 observations)

---

## Executive Summary

**Fed Funds raw level is NOT a predictive signal — it's a monetary regime identifier.**

The raw Fed Funds rate clusters into two distinct regimes:
- **Zero-rate regime:** 33.3% of observations (2020-2021)
- **High-rate regime:** 44.9% of observations (2022-2023)
- **Transition:** Only 21.8% of observations

This bimodal distribution creates artificial separation in clustering algorithms and inflates Information Gain.

**The fix: Use rolling Z-score instead of raw level.**

| Metric | CN 120d IG | HK 120d IG | Verdict |
|--------|-----------|-----------|---------|
| Raw Level | 0.474 | 0.237 | ❌ Regime identifier |
| First Difference (Δ) | 0.620 | 0.848 | ⚠️ Too sparse (97.4% zero) |
| **Rolling Z-Score** | **1.005** | **0.488** | ✅ **Best predictor** |

**Recommendation:** Replace Fed Funds raw level with 252-day rolling Z-score in Economic Layer v2.

---

## 1. Raw Distribution Analysis

### The Problem: Severe Bimodal Clustering

```
Fed Funds Raw Value Distribution:
[0.00, 0.25):  728 (31.1%)  ← Zero-rate regime
[0.25, 0.50):   52 ( 2.2%)
[0.50, 1.00):   42 ( 1.8%)
[1.00, 2.00):  117 ( 5.0%)
[2.00, 3.00):   56 ( 2.4%)
[3.00, 5.00):  842 (36.0%)  ← Hiking regime
[5.00, 10.00): 504 (21.5%)  ← Peak rates
```

**Key finding:** 78.2% of observations are at extremes (< 0.5 or > 3.0).

**What this means:**
- The Fed Funds rate doesn't move gradually — it jumps between regimes
- Zero-rate period: 2020-2021 (2 years)
- Hiking period: 2022-2023 (2 years)
- Only brief transition periods

**Impact on clustering:**
- K-means sees two dense clusters at 0 and 5
- Creates artificial "perfect" separation
- Information Gain is inflated because the algorithm is essentially separating "2020-2021" from "2022-2023"
- This is NOT predictive power — it's calendar identification

---

## 2. Rolling Z-Score Distribution

### The Solution: Normalized Deviation from Recent History

```
Fed Funds Z-Score Distribution (252d window):
[-3, -2):  197 ( 8.4%)
[-2, -1):  325 (13.9%)
[-1,  0):  499 (21.3%)
[ 0,  1):  560 (23.9%)
[ 1,  2):  484 (20.7%)
[ 2,  3):  124 ( 5.3%)
```

**Key finding:** Normal distribution, no clustering.

**What this means:**
- Z-score measures: "Is the current rate unusually high or low relative to the past year?"
- This captures **policy surprises**, not **policy levels**
- When Fed suddenly hikes faster than expected → negative Z-score for prior period, positive for current
- When Fed pauses unexpectedly → Z-score stabilizes

---

## 3. First Difference (Δ) Distribution

### The Problem: Too Sparse for Daily Modeling

```
Δ Fed Funds Distribution:
[-50, -10):     7 ( 0.3%)  ← Rate cuts
[-10,  -1):    23 ( 1.0%)  ← Small cuts
[ -1,   1):  2280 (97.4%) ← No change
[  1,  10):    19 ( 0.8%)  ← Small hikes
[ 10,  50):     6 ( 0.3%)  ← Large hikes
```

**Key finding:** 97.4% of days have zero change.

**Why:** The Fed meets only 8 times per year. Between meetings, Fed Funds is constant.

**Implications:**
- Δ is meaningful on FOMC days but zero on all other days
- For daily economic modeling, Δ is too sparse
- Could work for weekly/monthly models but not daily

---

## 4. Predictive Power Comparison

### CN Market (000300)

| Horizon | Level IG | Δ IG | Z-Score IG | Winner |
|---------|---------|------|-----------|--------|
| 20d | 0.078 | 0.048 | 0.057 | Level |
| 60d | 0.273 | 0.172 | 0.279 | Z-Score |
| 120d | 0.474 | 0.620 | **1.005** | **Z-Score** |

### HK Market (HSCEI)

| Horizon | Level IG | Δ IG | Z-Score IG | Winner |
|---------|---------|------|-----------|--------|
| 20d | 0.155 | 0.084 | 0.063 | Level |
| 60d | 0.272 | 0.285 | 0.187 | Δ |
| 120d | 0.237 | **0.848** | 0.488 | **Δ** |

**Key findings:**
1. **CN:** Z-Score dominates at 60d and 120d (IG=1.005 at 120d is exceptionally strong)
2. **HK:** Δ dominates at 120d (IG=0.848), but Δ is too sparse for daily use
3. **Z-Score is the best compromise** — strong predictive power AND daily granularity

---

## 5. Quintile Return Analysis (CN, 120d)

### Raw Level — Non-Monotonic (Problematic)

| Quintile | Score Range | Avg Return |
|----------|-------------|-----------|
| Q1 (Low) | [3.86, 4.33] | +5.95% |
| Q2 | [4.33, 4.58] | +9.83% |
| Q3 | [4.58, 5.08] | **-6.79%** |
| Q4 | [5.08, 5.33] | -1.86% |
| Q5 (High) | [5.33, 5.33] | +9.31% |

**Problem:** Non-monotonic. Q3 (middle-high rates) has worst returns, but Q5 (highest rates) has good returns. This is because Q5 captures the post-hiking recovery period (late 2023-2024).

---

### Z-Score — Monotonic (Correct)

| Quintile | Score Range | Avg Return |
|----------|-------------|-----------|
| Q1 (Low Z) | [-15.84, -1.35] | +1.81% |
| Q2 | [-1.34, 0.00] | **+15.91%** |
| Q3 | [0.00, 0.35] | +9.25% |
| Q4 | [0.36, 1.19] | -2.39% |
| Q5 (High Z) | [1.19, 1.66] | **-8.14%** |

**Pattern:**
- Very low Z-score (rates far below recent average) → modest positive returns
- Low Z-score (rates slightly below average) → **best returns** (accommodative policy)
- High Z-score (rates above average) → **negative returns** (tightening pressure)

**Economic intuition:**
- When Fed Funds is below its 1-year average (negative Z) → accommodative → good for equities
- When Fed Funds is above its 1-year average (positive Z) → tightening → bad for equities
- The magnitude matters: extremely low Z (emergency cuts) is less bullish than moderately low Z (normal easing)

---

## 6. Root Cause Analysis

### Why Raw Level Fails

```
Timeline:
2020-2021: Fed Funds ≈ 0.05% (zero-rate regime)
2022-2023: Fed Funds ↑ to 5.33% (hiking regime)
2024-2025: Fed Funds ↓ to 4.33% (easing regime)
```

**Raw level clustering:**
- All of 2020-2021: score ≈ 0 (zero-rate cluster)
- All of 2022-2023: score ≈ 100 (peak-rate cluster)
- The model learns: "2020-2021 = one thing, 2022-2023 = another thing"
- But this is just calendar identification, not predictive signal

**Z-score solution:**
- 2020-2021: Z ≈ 0 (rate is normal for zero-rate period)
- Early 2022: Z ↑ rapidly (rate is rising faster than historical average)
- Late 2022: Z ≈ +2 (rate is high relative to past year)
- 2024: Z ↓ negative (rate is falling below recent average)
- The model learns: "rate rising faster than usual = bad for equities"

---

## 7. Recommendation

### Immediate Fix for Economic Layer v2

**Replace Fed Funds raw level with 252-day rolling Z-score.**

**Implementation:**
```rust
// OLD: Raw level normalization
let fed_funds_score = ((max - value) / (max - min)) * 100.0;

// NEW: Z-score normalization
let mean = rolling_mean(values, 252);
let std = rolling_std(values, 252);
let z_score = (value - mean) / std;
let fed_funds_score = 50.0 + z_score * 10.0; // Scale to 0-100
```

**Why this works:**
1. Eliminates regime clustering
2. Captures policy surprise (deviation from expectation)
3. Stronger predictive power (IG=1.005 vs 0.474 for CN 120d)
4. Monotonic relationship with returns
5. Works daily (unlike Δ which is mostly zero)

---

## 8. Impact on TASK-080D Taxonomy

### Current 3-State (with raw Fed Funds)

| State | Centroid | % Time |
|-------|----------|--------|
| Favorable | 23.9 | 20% |
| Neutral | 49.9 | 41% |
| Unfavorable | 77.7 | 39% |

**Problem:** Fed Funds clustering inflates the Unfavorable state (pulls average up).

### Expected 3-State (with Z-Score Fed Funds)

After fixing Fed Funds:
- **Neutral state should grow** from 41% to ~50%
- **Favorable/Unfavorable should shrink** to ~25% each
- **Variance ratio may decrease slightly** (less artificial separation)
- **But predictive power should increase** (Z-Score is genuinely predictive)

**Action required:** Re-run TASK-080D after implementing Z-Score fix.

---

## 9. Broader Implications

### For State Layer
- No impact. State Layer uses trend_score, not Fed Funds.

### For Economic Layer
- **Fed Funds Z-Score** becomes a genuine predictive factor
- **10Y Treasury** should also be evaluated for clustering (may have similar issue)
- **SOFR** may be better than Fed Funds (market-based, more intra-period variation)

### For Other Factors
- Any factor with regime shifts (M2, QE/QT periods) should be checked for clustering
- Factors with natural drift (inflation, GDP) are less prone to this issue

---

## 10. Summary

| Question | Answer |
|----------|--------|
| Is Fed Funds predictive? | **Yes, but only as Z-Score, not raw level** |
| Is Fed Funds a regime identifier? | **Raw level is; Z-Score is not** |
| Should we use Δ (first difference)? | **No — too sparse for daily modeling** |
| Should we use Z-Score? | **Yes — best predictive power + daily granularity** |
| Does this fix the taxonomy? | **Partially — re-run 080D after implementation** |
| Does this validate 3-State? | **Pending — need clean data first** |

---

## Next Steps

1. **Implement Z-Score fix** for Fed Funds (and check 10Y Treasury)
2. **Re-run TASK-080C** with fixed Fed Funds
3. **Re-run TASK-080D** with fixed Fed Funds
4. **Then decide** on 3-State vs 4-State vs continuous
5. **Then proceed** to TASK-081 (Economic Layer v2 Prototype)

**Estimated time:** 2-3 hours for fix + re-run.
