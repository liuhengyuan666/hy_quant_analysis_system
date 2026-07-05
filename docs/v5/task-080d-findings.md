# TASK-080D: Economic Taxonomy Discovery — Findings

**Status:** Completed  
**Date:** 2026-06-07  
**Method:** K-means clustering on factor score vectors  
**Data:** 4 factors (VIX, US10Y, Dollar, FedFunds), 1595 daily observations (2020-2024)

---

## Executive Summary

**Data supports 3-4 economic states.** Not 2, not 5+.

| # States | Multi-Factor Variance Ratio | Assessment |
|----------|---------------------------|------------|
| 2 | 0.689 | Too coarse — loses medium-risk information |
| **3** | **0.862** | **Optimal balance — captures low/medium/high risk** |
| **4** | **0.919** | **Also valid — adds "early warning" state** |
| 5 | 0.952 | Overfitting — 5th cluster is just noise |

**Recommendation:** Start with **3 states** (Favorable / Neutral / Unfavorable). Allow **optional 4th state** (Early Warning) if 3-state backtests show insufficient granularity.

---

## Methodology

1. **Data**: 4 factor score vectors (0-100, rolling 252d normalization)
2. **Clustering**: K-means with k=2,3,4,5
3. **Metric**: Between-cluster variance ratio
   - >0.6 = strong separation
   - 0.4-0.6 = moderate separation
   - <0.4 = weak separation
4. **Validation**: Per-factor + multi-factor clustering

---

## Results

### Multi-Factor Clustering (Average of 4 Factors)

This is the most important analysis — it shows how the economy clusters when all factors are considered together.

#### k=2 (Too Coarse)

| Cluster | Centroid | Range | N | % |
|---------|----------|-------|---|---|
| Low Risk | 39.2 | 2.9-57.2 | 879 | 55% |
| High Risk | 75.3 | 57.3-98.5 | 716 | 45% |

**Verdict:** Reject. Loses all medium-risk information. Cannot distinguish "slightly elevated risk" from "extreme risk."

---

#### k=3 (RECOMMENDED)

| Cluster | Centroid | Range | N | % | Label |
|---------|----------|-------|---|---|-------|
| **Favorable** | 23.9 | 2.9-36.8 | 319 | 20% | Risk assets perform well |
| **Neutral** | 49.9 | 36.9-63.8 | 661 | 41% | Mixed signals |
| **Unfavorable** | 77.7 | 63.9-98.5 | 615 | 39% | Risk assets underperform |

**Verdict:** ✅ **Optimal.**
- Captures 3 distinct economic regimes
- Favorable = 20% of time (rare but profitable)
- Neutral = 41% of time (most common)
- Unfavorable = 39% of time (frequent but not extreme)
- **Variance ratio = 0.862** (very strong separation)

---

#### k=4 (ALSO VALID)

| Cluster | Centroid | Range | N | % | Label |
|---------|----------|-------|---|---|-------|
| **Favorable** | 21.0 | 2.9-33.0 | 251 | 16% | Strong risk-on |
| **Early Warning** | 45.2 | 33.1-54.9 | 551 | 35% | Deteriorating |
| **Stressed** | 64.6 | 54.9-74.0 | 427 | 27% | Clear risk-off |
| **Crisis** | 83.8 | 74.2-98.5 | 366 | 23% | Severe risk-off |

**Verdict:** ⚠️ **Valid but more complex.**
- Adds "Early Warning" state between Neutral and Stressed
- Useful for risk management (time to de-risk)
- **Variance ratio = 0.919** (excellent separation)
- Risk: More states = harder to classify accurately

---

#### k=5 (Overfitting)

| Cluster | Centroid | Range | N | % |
|---------|----------|-------|---|---|
| 1 | 17.2 | 2.9-27.0 | 179 | 11% |
| 2 | 36.9 | 27.2-44.1 | 296 | 19% |
| 3 | 51.5 | 44.2-60.0 | 447 | 28% |
| 4 | 68.5 | 60.1-76.6 | 348 | 22% |
| 5 | 84.8 | 76.8-98.5 | 325 | 20% |

**Verdict:** ❌ Reject.
- 5th cluster doesn't add economic meaning
- Variance ratio = 0.952 (highest, but marginal gain from k=4 is only +0.033)
- Elbow method suggests k=3 or k=4 is optimal

---

### Per-Factor Clustering

#### VIX (Volatility)

| k | Variance Ratio | Interpretation |
|---|---------------|----------------|
| 2 | 0.705 | Calm vs Fear |
| 3 | 0.859 | Calm / Elevated / Fear |
| 4 | 0.919 | Very Calm / Calm / Elevated / Fear |

**VIX naturally separates into 3-4 states:**
- Low VIX (0-46): complacency
- Medium VIX (47-78): normal uncertainty
- High VIX (79-100): fear/crisis

#### 10Y Treasury (Rates)

| k | Variance Ratio | Interpretation |
|---|---------------|----------------|
| 2 | 0.740 | Low rates / High rates |
| 3 | 0.897 | Low / Medium / High |
| 4 | 0.940 | Very low / Low / Medium / High |

**10Y naturally separates into 3-4 states:**
- Low rates (0-28): accommodative policy
- Medium rates (28-63): neutral
- High rates (63-100): tightening

#### Dollar Index

| k | Variance Ratio | Interpretation |
|---|---------------|----------------|
| 2 | 0.789 | Weak dollar / Strong dollar |
| 3 | 0.913 | Weak / Neutral / Strong |
| 4 | 0.951 | Very weak / Weak / Strong / Very strong |

**Dollar naturally separates into 3 states:**
- Weak (0-31): risk-on, EM outperformance
- Neutral (31-69): balanced
- Strong (69-100): risk-off, flight to quality

#### Fed Funds (CRITICAL ISSUE)

| k | Variance Ratio | Interpretation |
|---|---------------|----------------|
| 2 | 0.850 | Zero rate / High rate |
| 3 | 0.981 | Zero / Transition / High |
| 4 | 0.991 | Zero / Transition / High / Very high |

**CRITICAL:** Fed Funds shows extreme clustering at score=0 and score=100.
- 559 observations (35%) at score ≈ 0 (zero-rate period 2020-2022)
- 623 observations (39%) at score ≈ 100 (hiking period 2022-2023)
- Only 26% of observations in the middle range

**This is NOT natural clustering — it's a data artifact.**
- The zero-rate period created a massive cluster at the bottom
- The rapid hike cycle created a massive cluster at the top
- With longer history, we'd see more gradual distribution

**Impact on taxonomy:** Fed Funds should use first-difference (change in rate) or be excluded from state definition until score clustering is fixed.

---

## Economic Interpretation of 3-State Taxonomy

### State 1: FAVORABLE (20% of time)

**Signature:**
- VIX: Low (0-37) — calm markets
- 10Y: Low-Medium (0-45) — accommodative or neutral rates
- Dollar: Weak-Neutral (0-50) — supportive for risk assets
- Fed Funds: Low (0-50) — loose policy

**Historical periods:**
- Post-COVID recovery (2020H2-2021)
- Select periods in 2023

**Expected returns:** Highest (per TASK-080C)
- CN: +8.65% (120d) when VIX in bottom quintile
- HK: +17.42% (120d) when Dollar in bottom quintile

---

### State 2: NEUTRAL (41% of time)

**Signature:**
- VIX: Medium (38-64) — normal uncertainty
- 10Y: Medium (28-64) — neutral rates
- Dollar: Neutral (31-69) — balanced
- Fed Funds: Medium (20-67) — transitioning

**Historical periods:**
- Most of 2021
- Early 2023

**Expected returns:** Moderate
- Mixed signals; no strong directional bias
- Risk assets can go either way

---

### State 3: UNFAVORABLE (39% of time)

**Signature:**
- VIX: High (65-100) — elevated fear
- 10Y: High (65-100) — tight rates
- Dollar: Strong (70-100) — flight to quality
- Fed Funds: High (75-100) — restrictive policy

**Historical periods:**
- 2022 bear market
- Early 2020 COVID crash

**Expected returns:** Lowest (per TASK-080C)
- CN: -2.15% (120d) when VIX in top quintile
- HK: -4.61% (120d) when Dollar in top quintile

---

## Stability Analysis

**Question:** Are these states stable over time, or do they change with market regimes?

**Evidence:**
- The 3-state structure persists across all 4 individual factors
- VIX, 10Y, Dollar all naturally separate into 3 clusters
- Multi-factor clustering confirms 3-state structure
- **BUT:** Fed Funds clustering is artificial (zero-rate artifact)

**Risk:**
- States defined on 2020-2024 data may not generalize to future regimes
- If Fed Funds clustering is fixed, the taxonomy may shift
- Recommendation: Re-run taxonomy discovery after fixing Fed Funds

---

## Recommendation

### Option A: 3-State (RECOMMENDED)

```
Favorable    (score: 0-37)   → High expected returns
Neutral      (score: 38-64)  → Mixed expected returns  
Unfavorable  (score: 65-100) → Low expected returns
```

**Pros:**
- Simple, interpretable
- Strong empirical separation (variance ratio = 0.862)
- Matches natural clustering of individual factors
- Easy to communicate to users

**Cons:**
- May miss "early warning" transitions
- Fed Funds clustering biases the neutral state

---

### Option B: 4-State (Alternative)

```
Favorable     (score: 0-33)   → Strong risk-on
Early Warning (score: 34-55)  → Deteriorating conditions
Stressed      (score: 56-74)  → Clear risk-off
Crisis        (score: 75-100) → Severe risk-off
```

**Pros:**
- Better granularity for risk management
- "Early Warning" state provides time to adjust positions
- Higher variance ratio (0.919)

**Cons:**
- Harder to classify accurately
- More states = more classification errors
- May be overkill for initial implementation

---

### Option C: Continuous Scores (Not Recommended)

Use raw factor scores without state collapse.

**Verdict:** Reject.
- Evidence strongly supports discrete states (all variance ratios > 0.6)
- Continuous scores would lose the clear separation shown in data
- Allocation Layer needs discrete signals for decision-making

---

## Implementation Path

### Phase 1: Implement 3-State (TASK-081)
- Define thresholds based on k=3 clustering
- Generate EconomicState field in EconomicSnapshot
- Backtest 3-state vs continuous scores

### Phase 2: Test 4-State (Future)
- If 3-state backtests show insufficient granularity
- Add "Early Warning" state
- Compare performance

### Phase 3: Fix Fed Funds (Prerequisite)
- Before finalizing thresholds, fix Fed Funds score clustering
- Options: use first-difference, shorter lookback, or SOFR
- Re-run taxonomy discovery with fixed factor

---

## Critical Caveat: Fed Funds Bias

**The current 3-state taxonomy is biased by Fed Funds score clustering.**

Because Fed Funds has 35% of observations at score=0 and 39% at score=100, the multi-factor average score is pulled toward extremes.

**This means:**
- "Favorable" state may be overrepresented (includes all zero-rate periods)
- "Unfavorable" state may be overrepresented (includes all hiking periods)
- "Neutral" state may be underrepresented

**After fixing Fed Funds:**
- Expect Neutral state to grow from 41% to ~50%
- Favorable and Unfavorable may shrink to ~25% each

**Action:** Re-run TASK-080D after implementing Fed Funds fix.

---

## Summary Table

| Taxonomy | Variance Ratio | # States | Pros | Cons |
|----------|---------------|----------|------|------|
| 2-State | 0.689 | 2 | Simple | Too coarse |
| **3-State** | **0.862** | **3** | **Optimal balance** | **May miss early warning** |
| 4-State | 0.919 | 4 | More granular | Harder to classify |
| 5-State | 0.952 | 5 | Maximum separation | Overfitting |
| Continuous | N/A | ∞ | No thresholds | No separation |

**Final recommendation: Start with 3-State.**
