# Economic Layer Target Discovery — Preliminary Research

**Status:** Exploratory research (no implementation)
**Blocked on:** ADR-061 acceptance for any formal decisions
**Purpose:** Document candidate targets and tradeoffs for future decision

---

## Background

Per user directive: "Economic Layer Target Discovery" is P3 (deferred) until State Layer is defined.

This document is lightweight research to prepare for that decision. No code changes, no commitments.

---

## Candidate Targets for Economic Layer

### 1. Forward Return (Already explored in Wave 9)

**Definition:** Future returns at 20d/60d/120d horizons

**Evidence from Wave 9:**
- State Layer vs Forward Return: Information ≈ 0.006-0.086
- Conclusion: State Layer does NOT predict raw forward returns well

**But:** This doesn't mean Forward Return is wrong for Economic Layer. It means:
- State Layer ≠ Forward Return predictor
- Economic Layer MAY predict Forward Return (that's its job)

**Pros:**
- Directly measurable
- Universally understood
- Aligns with investor goals (make money)

**Cons:**
- High noise (market randomness)
- May need different features than State Layer (e.g., valuation, earnings)
- 60d horizon showed best signal but still noisy

---

### 2. Credit Cycle

**Definition:** Expansion/contraction of credit conditions

**Proxy indicators:**
- Credit spreads (IG/HY spread)
- Loan growth
- Corporate bond issuance

**Pros:**
- Leading indicator (credit leads equity)
- Strong theoretical foundation (Minsky, Ray Dalio)
- Less noisy than raw returns

**Cons:**
- Data availability for CN/HK (not standard FRED series)
- May lag in real-time (credit data released monthly)
- Harder to validate than returns

---

### 3. Liquidity Regime

**Definition:** Tight vs loose monetary conditions

**Proxy indicators:**
- Fed Funds rate trajectory
- Central bank balance sheet changes
- Interbank rates (SHIBOR, HIBOR)

**Pros:**
- Already partially captured in State Layer (US10Y, Fed Funds)
- Strong impact on equity markets
- Real-time available

**Cons:**
- CN/HK have different monetary regimes than US
- PBOC policy less transparent than Fed
- May be too correlated with State Layer (not orthogonal)

---

### 4. Volatility Regime

**Definition:** Low vol vs high vol environment

**Proxy indicators:**
- VIX level and trend
- Realized volatility
- Volatility of volatility

**Pros:**
- VIX is already in State Layer
- Clear economic impact (risk parity, vol targeting)
- Good for option strategies

**Cons:**
- May be too similar to State Layer's Risk component
- Hard to predict (vol clustering is statistical, not causal)
- Not a direct "economic" target

---

### 5. Risk-Adjusted Return (Sharpe Regime)

**Definition:** Forward return / forward volatility

**Pros:**
- Captures both return AND risk
- More stable than raw returns
- Aligns with rational investor behavior

**Cons:**
- More complex to compute
- Forward volatility harder to predict than forward return
- May introduce look-ahead bias

---

## Key Tradeoff Matrix

| Target | Predictability | Data Availability | Orthogonality to State | Actionability |
|--------|---------------|-------------------|----------------------|---------------|
| Forward Return | Low | High | Medium | High |
| Credit Cycle | Medium | Medium | High | Medium |
| Liquidity Regime | Medium | High | Low | Medium |
| Volatility Regime | Medium | High | Low | Medium |
| Risk-Adjusted Return | Low | Medium | Medium | High |

---

## Open Questions (for ADR-061 post-acceptance)

1. **Should Economic Layer be a separate model or an extension of State Layer?**
   - ADR-056 says Dual-Layer (State + Economic)
   - But if Economic Layer uses same features, why separate?

2. **What horizon is appropriate for Economic Layer?**
   - 20d: Too noisy, short-term timing
   - 60d: Sweet spot (evidence from TASK-030)
   - 120d: Better signal but fewer samples

3. **Should Economic Layer predict absolute returns or relative returns?**
   - Absolute: "Will market go up 5%?"
   - Relative: "Will RiskOn outperform RiskOff?"

4. **How to validate Economic Layer without overfitting?**
   - Out-of-sample testing
   - Walk-forward validation
   - Different markets (CN vs HK)

---

## Recommendation (for discussion)

**Wait for ADR-061 acceptance before deciding.**

But preliminary hypothesis:

**Economic Layer should predict Risk-Adjusted Return (Sharpe) at 60d horizon.**

Rationale:
- Raw returns are too noisy for State Layer to predict
- But State Layer identifies regimes with different risk profiles
- Economic Layer can then predict which regime offers best risk-adjusted returns
- This is conceptually different from State Layer (which identifies current regime)

**Example:**
```
State Layer: "We're in RiskOff"
Economic Layer: "RiskOff typically offers Sharpe 0.8 over next 60d"
Strategy: "Reduce exposure but don't exit (RiskOff has positive expected returns)"
```

---

## Next Steps (Post-ADR-061)

1. Define Economic Layer scope and target metric
2. Identify required features (beyond State Layer features)
3. Build prototype model
4. Validate against historical data
5. Integrate with strategy engine

**Note:** This document is for discussion only. No implementation until ADR-061 accepted.
