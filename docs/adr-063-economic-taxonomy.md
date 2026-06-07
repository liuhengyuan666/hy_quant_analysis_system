# ADR-063: 3-State Economic Taxonomy

**Status:** Accepted  
**Date:** 2026-06-07  
**Author:** Sisyphus (AI Agent)  
**Reviewed by:** User (Project Owner)

---

## Context

Economic Layer v2 requires a stable taxonomy for classifying macroeconomic environments. After TASK-080A through TASK-080F, we have established:
- 10 core economic factors (4 implemented, 6 planned)
- Feature orthogonality validated
- Predictive power confirmed (after Fed Funds Z-score fix)
- Optimal number of states determined from data

This ADR freezes the Economic Layer taxonomy and defines the contract for Shadow Production.

---

## Decision

Adopt **3-State Economic Taxonomy** for Economic Layer v2:

| State | Score Range | Centroid | % Time | Economic Meaning |
|-------|-------------|----------|--------|-----------------|
| **Favorable** | 61.2 – 93.3 | 72.6 | 37.4% | Accommodative policy, low risk, supportive liquidity |
| **Neutral** | 37.5 – 61.0 | 49.8 | 40.3% | Mixed signals, transition periods, balanced risks |
| **Unfavorable** | 4.1 – 37.4 | 25.1 | 22.4% | Tight policy, elevated risk, constrained liquidity |

**Variance Ratio:** 0.843 (strong separation, > 0.6 threshold)

---

## Rationale

### Why 3 States?

K-means clustering analysis (k=2/3/4/5) on 4 factors (VIX, 10Y Treasury, Dollar, Fed Funds):

| k | Variance Ratio | Assessment |
|---|---------------|------------|
| 2 | 0.681 | Too coarse |
| **3** | **0.843** | **Optimal balance** |
| 4 | 0.908 | Over-segmentation risk |
| 5 | 0.943 | Diminishing returns |

3-state provides:
- Sufficient granularity for allocation decisions
- Stable state centroids (not sensitive to small data changes)
- Economically interpretable states
- Manageable sample sizes per state for backtesting

### Why Not Continuous?

Data rejects continuous scores:
- All 4 factors show clear multimodal distributions
- K-means variance ratio > 0.8 for k=3/4/5
- Quintile return analysis shows discrete performance regimes
- Human interpretation and logging benefit from discrete states

### Why These Boundaries?

Boundaries derived from K-means cluster centroids:
- Cluster 1 centroid: 25.1 (Unfavorable)
- Cluster 2 centroid: 49.8 (Neutral)
- Cluster 3 centroid: 72.6 (Favorable)

Boundaries at natural valleys between clusters:
- Unfavorable/Neutral: ~37.5
- Neutral/Favorable: ~61.0

---

## Factor Normalization

### Implemented Factors (4)

| Factor | Source | Normalization | Invert |
|--------|--------|--------------|--------|
| VIX | FRED (VIXCLS) | Rolling min/max (20d) | Yes |
| US 10Y Treasury | FRED (DGS10) | Rolling min/max (20d) | Yes |
| Dollar Index | FRED (DTWEXBGS) | Rolling min/max (20d) | Yes |
| **Fed Funds** | **FRED (DFF)** | **252d Z-score (±3 cap)** | **Yes** |

### Planned Factors (6)

| Factor | Source | Normalization | Status |
|--------|--------|--------------|--------|
| HY Spread | FRED (BAMLH0A0HYM2) | TBD | Pending FRED fetch |
| 2Y Treasury | FRED (DGS2) | TBD | Pending FRED fetch |
| Term Spread | Computed (10Y-2Y) | TBD | Pending FRED fetch |
| SOFR | FRED (SOFR) | TBD | Pending FRED fetch |
| Initial Claims | FRED (ICSA) | TBD | Pending FRED fetch |
| NFCI | FRED (NFCI) | Validation only | Pending FRED fetch |

**Note:** FRED data fetch currently blocked (504 Gateway Timeout). Empirical analysis for missing factors uses research-based estimates. Full integration pending network fix.

---

## Economic Layer Contract

### Input
- 4-10 macro factor scores (0-100, normalized)
- Daily granularity
- Per-scope: GLOBAL (all factors), CN (subset), HK (subset)

### Output
- **Economic State:** Favorable / Neutral / Unfavorable
- **Economic Score:** Average of factor scores (0-100 continuous)
- **State Confidence:** Based on distance to cluster centroid
- **Factor Contributions:** Per-factor deviation from state centroid

### Evaluation Metric
- **Information Gain (IG)** vs Forward Return (20d/60d/120d)
- Target: IG > 0.3 for at least one horizon
- Current: CN 120d IG = 0.964 (VIX + Fed Funds combined)

### Non-Goals
- Economic Layer does NOT generate allocation decisions
- Economic Layer does NOT predict exact returns
- Economic Layer does NOT replace State Layer

---

## State Definitions

### Favorable
```
Economic Score: 61.2 - 93.3
Macro Environment: Accommodative monetary policy, low volatility, supportive liquidity
Expected Return Distribution: Positive skew, higher mean
Risk Profile: Lower tail risk
Typical Duration: TBD from Shadow Production
```

### Neutral
```
Economic Score: 37.5 - 61.0
Macro Environment: Mixed signals, policy transition, balanced risks
Expected Return Distribution: Near-zero mean, normal variance
Risk Profile: Baseline risk
Typical Duration: TBD from Shadow Production
```

### Unfavorable
```
Economic Score: 4.1 - 37.4
Macro Environment: Tight policy, elevated volatility, constrained liquidity
Expected Return Distribution: Negative skew, lower mean
Risk Profile: Elevated tail risk
Typical Duration: TBD from Shadow Production
```

---

## Fed Funds Special Handling

**Problem:** Raw Fed Funds rate clusters into two regimes (zero-rate 2020-2021 vs hiking 2022-2023), causing temporal regime leakage.

**Solution:** 252-day rolling Z-score with ±3 capping

```
z_score = (rate - rolling_mean_252d) / rolling_std_252d
capped_z = clamp(z_score, -3.0, 3.0)
score = 50.0 - capped_z * 15.0  // Inverted: high rate = tight = low score
```

**Result:**
- Eliminates 0/100 clustering
- Improves CN 120d IG from 0.474 → 0.964
- Improves HK 120d IG from 0.237 → 0.524

---

## Shadow Production Protocol

### Daily Output
```
Date: YYYY-MM-DD
CN State: {RiskOn | Neutral | RiskOff}
HK State: {RiskOn | Neutral | RiskOff}
Economic State: {Favorable | Neutral | Unfavorable}
Suggested Allocation: {Conservative | Neutral | Aggressive}
```

### Recording
- T+20 return: Record actual 20-day forward return
- T+60 return: Record actual 60-day forward return
- T+120 return: Record actual 120-day forward return
- State transitions: Log frequency and direction
- Factor scores: Daily snapshot for post-hoc analysis

### Constraints
- **NO real money execution**
- Allocation suggestions are for observation only
- Weekly human review required
- Monthly performance report

---

## Consequences

### Positive
- Stable, data-driven taxonomy for Economic Layer
- Eliminates temporal regime leakage from Fed Funds
- Enables consistent Shadow Production logging
- Provides foundation for Allocation Layer v2

### Negative
- 3 states may be too coarse for nuanced environments
- State proportions (37/40/22) may shift as more factors added
- K-means boundaries are data-dependent; require periodic re-validation
- Fed Funds Z-score requires 252-day warmup; early data less reliable

### Risks
- **Factor addition risk:** Adding 6 missing factors may change taxonomy
  - Mitigation: Re-run 080D after full factor integration
- **Regime change risk:** Future monetary policy regimes may violate historical Z-score distribution
  - Mitigation: Monitor Z-score distribution in Shadow Production
- **Lookback mismatch:** Other factors use 20d lookback, Fed Funds uses 252d
  - Mitigation: Evaluate if 20d Z-score works for Fed Funds; adjust if needed

---

## Related ADRs

- ADR-056: Dual-Layer Architecture (State + Economic)
- ADR-061: State Layer Semantic Contract (frozen)
- ADR-062: Three-Layer Evaluation Framework

---

## References

- `docs/task-080d-findings.md` — Taxonomy discovery analysis
- `docs/task-080e-findings.md` — Fed Funds distortion audit
- `docs/task-080f-findings.md` — Z-score integration results
- `crates/macro-engine/src/lib.rs` — Implementation

---

## Decision Log

| Date | Event |
|------|-------|
| 2026-06-07 | TASK-080D completed: 3-state recommended (20/41/39) |
| 2026-06-07 | TASK-080E completed: Fed Funds clustering identified |
| 2026-06-07 | TASK-080F completed: Z-score fix applied, re-run (37/40/22) |
| 2026-06-07 | **ADR-063 ACCEPTED** — Taxonomy frozen for Shadow Production |
