# TASK-080B: Feature Orthogonality Audit

**Status:** Planned  
**Date:** 2026-06-07  
**Depends on:** TASK-080A  
**Objective:** Select final 10-12 factors from 13 MVP candidates using 4-dimensional orthogonality analysis

---

## Context

TASK-080A identified ~13 MVP candidate factors across 6 categories. Before building the Economic Layer, we must answer:

> Which factors provide **independent predictive information**?

Not just:
> Which factors are different from each other?

This is the lesson from Wave 7.5: correlation alone is insufficient. Two factors can be highly correlated but one contributes all the predictive power while the other contributes none.

---

## Methodology: Four-Table Analysis

### Table 1: Pearson Correlation Matrix

**Purpose:** Detect linear redundancy  
**Method:** Pairwise Pearson r for all factor score time series  
**Threshold:** |r| > 0.80 → flag for review  
**Action:** Consider removing the lower-predictive-power factor of each highly correlated pair

**Why Pearson first:**
- Fast to compute
- Identifies obvious linear redundancy
- Good initial filter before expensive MI computation

**Expected findings:**
- `DGS10` ↔ `DGS2`: likely high r (rate level correlation)
- `BAMLH0A0HYM2` ↔ `BAMLC0A0CM`: likely high r (credit spread correlation)
- `T10Y2Y` ↔ `DGS10`: moderate r (curve vs level)

---

### Table 2: Spearman Rank Correlation Matrix

**Purpose:** Detect monotonic redundancy (robust to outliers)  
**Method:** Pairwise Spearman ρ for all factor score time series  
**Threshold:** |ρ| > 0.80 → flag for review  
**Action:** Compare with Pearson. If Pearson ≈ Spearman → linear relationship. If Pearson << Spearman → nonlinear relationship.

**Why Spearman:**
- Financial data is often non-normal (fat tails, outliers)
- Spearman is more robust than Pearson
- Reveals rank-based relationships that Pearson misses

---

### Table 3: Mutual Information Matrix

**Purpose:** Detect **nonlinear** dependencies  
**Method:** Pairwise MI between factor score time series (using histogram/binning or k-NN estimator)  
**Threshold:** MI > 0.5 bits (or normalized MI > 0.6) → flag for review  
**Action:** If two factors have high MI but low Pearson/Spearman, they share nonlinear information

**Why MI:**
- Captures any statistical dependency, not just linear
- Essential for financial factors (relationships often nonlinear)
- Consistent with methodology used in previous Waves

**Expected findings:**
- Some factor pairs may show low Pearson but high MI (nonlinear relationship)
- VIX may have nonlinear relationships with credit spreads

---

### Table 4: Predictive Orthogonality (MOST IMPORTANT)

**Purpose:** Measure **independent predictive power** for future returns  
**Method:** For each factor, measure Information Gain vs forward returns (20d, 60d, 120d)

```
For each factor F:
    For each horizon H in [20d, 60d, 120d]:
        1. Discretize F into quintiles (or terciles)
        2. Compute forward return distribution for each bin
        3. Measure Information Gain: IG(F; Return_H)
        4. Measure Separation: distance between best/worst bin means
```

**Then test redundancy:**
```
For each pair (F1, F2):
    IG(F1; Return) = 0.12
    IG(F2; Return) = 0.08
    IG(F1+F2; Return) = 0.13  # marginal gain only 0.01
    → F2 is redundant (most of its info already in F1)
```

**Output format:**

| Factor | IG_20d | IG_60d | IG_120d | Best_Horizon | Redundant_With |
|--------|--------|--------|---------|--------------|----------------|
| VIX | 0.15 | 0.12 | 0.08 | 20d | — |
| HY_Spread | 0.14 | 0.18 | 0.22 | 120d | — |
| IG_Spread | 0.13 | 0.16 | 0.19 | 120d | HY_Spread |
| ... | ... | ... | ... | ... | ... |

**Selection rule:**
- Keep factors with IG > 0.05 (non-negligible predictive power)
- If two factors are redundant (marginal IG < 0.02), keep the one with higher standalone IG
- Ensure at least 1-2 factors per category are represented (if they pass IG threshold)

---

## Implementation Plan

### Step 1: Data Collection
- Fetch historical data for all 13 MVP factors (2020-01-01 to 2026-03-16)
- Align to common date grid (daily, forward-fill for weekly/monthly)
- Compute factor scores using same rolling min/max normalization as State Layer

### Step 2: Compute Correlation Matrices
- Pearson: `np.corrcoef()` or Rust equivalent
- Spearman: `scipy.stats.spearmanr()` or Rust equivalent
- MI: Custom k-NN estimator or histogram method

### Step 3: Compute Predictive Orthogonality
- For each factor, compute forward returns (20d, 60d, 120d)
- Discretize factor scores into quintiles
- Compute return distribution per quintile
- Compute Information Gain: `IG(F; R) = H(R) - H(R|F)`
- Compute marginal contributions for factor pairs

### Step 4: Selection
- Apply selection rules
- Document rationale for each kept/removed factor
- Produce final factor list (target: 10-12 factors)

### Step 5: Validation
- Test selected factors on out-of-sample period (if available)
- Verify no look-ahead bias
- Confirm frequency handling is correct

---

## Expected Timeline

| Step | Duration | Deliverable |
|------|----------|-------------|
| Data Collection | 2-3 hours | Aligned factor score time series |
| Correlation Matrices | 1 hour | Pearson + Spearman + MI tables |
| Predictive Orthogonality | 2-3 hours | IG table + redundancy analysis |
| Selection + Validation | 1-2 hours | Final 10-12 factor list + rationale |
| Documentation | 1 hour | This document + findings report |

**Total: ~8-10 hours**

---

## Success Criteria

1. **Coverage:** At least 4 of 6 categories represented in final list
2. **Orthogonality:** No pair with Pearson |r| > 0.85 in final list
3. **Predictive Power:** Every kept factor has IG > 0.05 for at least one horizon
4. **Redundancy Elimination:** No factor kept if marginal IG < 0.02
5. **Documentation:** Clear rationale for each kept/removed factor

---

## Output Format

### Final Deliverable

```markdown
# TASK-080B Findings

## Selected Factors (n=XX)

| Factor | Category | FRED Series | IG_20d | IG_60d | IG_120d | Rationale |
|--------|----------|-------------|--------|--------|---------|-----------|
| ... | ... | ... | ... | ... | ... | ... |

## Removed Factors

| Factor | Category | Reason |
|--------|----------|--------|
| ... | ... | ... |

## Correlation Matrix (Selected Factors Only)

[Heatmap or table]

## Key Insights

1. ...
2. ...
3. ...
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Look-ahead bias | Use only data available at time t; forward returns computed from t+1 |
| Survivorship bias | FRED data is point-in-time; no survivorship issue |
| Data mining | Use multiple horizons (20/60/120d); require consistency |
| Non-stationarity | Use rolling IG computation; report time-varying predictive power |
| Overfitting | Keep factor count low (10-12); use economic rationale |

---

## Next Step

After TASK-080B completes:
- Proceed to **TASK-080C: Economic Predictive Audit**
- Test selected factor categories (not individual factors) vs forward returns
- Measure category-level separation and information gain
