# TASK-080A: Economic Feature Inventory & Architecture Design

**Status:** In Progress  
**Date:** 2026-06-07  
**Depends on:** ADR-062  
**Related:** TASK-080B, TASK-080C, TASK-080D

---

## Objective

Design Economic Layer v2 by:
1. Cataloging all candidate economic factors
2. Designing the architecture for multi-factor economic regime classification
3. Establishing clear contracts between State Layer, Economic Layer, and Allocation Layer

**Principle:** Design first, code second. Do not implement until architecture is validated.

---

## Part 1: Current State Inventory

### Existing FRED Factors (Production)

| Factor Name | FRED Series | Category | Invert | Frequency | Description |
|------------|-------------|----------|--------|-----------|-------------|
| vix | VIXCLS | Volatility | true | Daily | CBOE Volatility Index |
| us10y | DGS10 | Rates | true | Daily | 10-Year Treasury Yield |
| dollar_index | DTWEXBGS | Dollar | true | Daily | Trade Weighted US Dollar Index |
| fed_funds | DFF | Rates | true | Daily | Federal Funds Effective Rate |

**Current Scoring:**
- Risk Score = avg(vix_score, dollar_score)
- Liquidity Score = avg(us10y_score, fed_funds_score)
- Trend Score = price-based (MA20/MA60 cross)

**Current Regime Logic:**
- RiskOn: trend ≥ 60 AND liquidity ≥ 50 AND risk ≥ 55
- RiskOff: trend < 40 OR risk < 40
- Neutral: everything else

### Current Architecture

```
FRED CSV Fetch
    ↓
MacroFactorSeries { factor_name, source, invert_score, observations }
    ↓
build_macro_snapshots() → rolling min/max normalization → score 0-100
    ↓
build_market_regimes() → group by date → compute regime_label
    ↓
MarketRegimeSnapshot { date, market, trend_score, liquidity_score, risk_score, regime_label }
```

**Storage:** ClickHouse `macro_snapshot` table (factor_value + factor_score per factor per day)

---

## Part 2: Candidate Factor Categories

**Revised per user feedback:** Financial Conditions removed from core categories. NFCI降级为Composite Validation Factor only.

### Category A: Liquidity (现有 + 扩展)

| Factor | FRED Series | Frequency | Lead/Lag | Rationale | Status |
|--------|-------------|-----------|----------|-----------|--------|
| Fed Funds | DFF | Daily | Coincident | 现行基准 | ✅ Active |
| 10Y Treasury | DGS10 | Daily | Leading | 现行基准 | ✅ Active |
| 2Y Treasury | DGS2 | Daily | Leading | 短端利率 | ✅ Active |
| Term Spread (10Y-2Y) | T10Y2Y | Derived | Leading | 收益率曲线 | ✅ Active |
| M2 Money Supply | M2SL | Monthly | Lagging | 广义流动性 | ✅ Active |
| SOFR | SOFR | Daily | Coincident | 市场真实利率 | ✅ Active (2018+) |

### Category B: Credit (新增)

| Factor | FRED Series | Frequency | Lead/Lag | Rationale | Status |
|--------|-------------|-----------|----------|-----------|--------|
| HY Spread (OAS) | BAMLH0A0HYM2 | Daily | Leading | 高风险信用 | ✅ Active ⚠️ 3yr limit |
| IG Spread (OAS) | BAMLC0A0CM | Daily | Leading | 投资级信用 | ✅ Active ⚠️ 3yr limit |
| TED Spread | TEDRATE | Daily | Leading | 银行间信用风险 | ❌ **DISCONTINUED Jan 2022** |
| BBB Spread (OAS) | BAMLC0A4CBBB | Daily | Leading | 质量分层 | ✅ Active ⚠️ 3yr limit |

### Category C: Volatility (现有)

| Factor | FRED Series | Frequency | Lead/Lag | Rationale | Status |
|--------|-------------|-----------|----------|-----------|--------|
| VIX | VIXCLS | Daily | Leading | 股权波动率 | ✅ Active ⚠️ 3yr limit |
| MOVE Index | — | — | — | 债券波动率 | ❌ **NOT on FRED** (ICE proprietary) |

### Category D: Dollar (现有)

| Factor | FRED Series | Frequency | Lead/Lag | Rationale | Status |
|--------|-------------|-----------|----------|-----------|--------|
| DXY | DTWEXBGS | Daily | Coincident | 广义美元 | ✅ Active |

### Category E: Growth (新增)

| Factor | FRED Series | Frequency | Lead/Lag | Rationale | Status |
|--------|-------------|-----------|----------|-----------|--------|
| Chicago Fed NFCI | NFCI | Weekly | Leading | 综合金融条件 | ✅ Active |
| Chicago Fed Activity | CFNAI | Monthly | Leading | 85指标综合 | ✅ Active |
| ISM Manufacturing | NAPM | Monthly | Leading | 制造业PMI | ❌ **DEPRECATED 2016** |
| Initial Claims | ICSA | Weekly | Leading | 就业市场 | ✅ Active |
| Real GDP Growth | A191RL1Q225SBEA | Quarterly | Lagging | 增长基准 | ✅ Active |

### Category F: Composite Validation Factors (不进入模型)

| Factor | FRED Series | Frequency | Role | Status |
|--------|-------------|-----------|------|--------|
| Chicago Fed NFCI | NFCI | Weekly | **Validation only** — compare Economic Layer output vs NFCI | ✅ Active |
| St. Louis Fed FSI | STLFSI4 | Weekly | **Validation only** — stress index cross-check | ✅ Active |

---

## Part 3: Architecture Design

### Design Goals

1. **Modularity:** Each factor category is self-contained
2. **Extensibility:** New factors can be added without changing core logic
3. **Orthogonality:** Categories should provide independent information (validated by TASK-080B)
4. **Frequency Handling:** Daily, weekly, monthly factors coexist transparently
5. **Missing Data Resilience:** Forward-fill or interpolation for lower-frequency data

### Proposed Data Flow

```
┌─────────────────────────────────────────────────────────┐
│                    Factor Fetch Layer                    │
│  (FRED API + other sources → MacroFactorSeries)         │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                 Factor Processing Layer                  │
│  • Normalization (rolling min/max → 0-100 score)        │
│  • Frequency alignment (daily/weekly/monthly → daily)   │
│  • Missing data handling                                │
│  • Outlier detection                                    │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│              Economic Regime Engine (NEW)                │
│  • Category aggregation (Liquidity, Credit, etc.)       │
│  • Cross-category scoring                               │
│  • Economic state classification                        │
│  • Forward return distribution prediction               │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                 Economic Snapshot                        │
│  { date, economic_state, confidence,                   │
│    factor_scores: { category: score },                  │
│    predicted_return_distribution: { mean, std, pctiles }}│
└─────────────────────────────────────────────────────────┘
```

### Core Types (Proposed)

```rust
/// Economic factor with metadata
pub struct EconomicFactor {
    pub name: String,
    pub fred_series: String,
    pub category: FactorCategory,
    pub frequency: DataFrequency,
    pub invert_score: bool,
    pub availability: FactorAvailability,
}

pub enum FactorCategory {
    Liquidity,
    Credit,
    Volatility,
    Dollar,
    Growth,
    FinancialConditions,
}

pub enum DataFrequency {
    Daily,
    Weekly,
    Monthly,
}

/// Predicted return distribution for a given horizon
pub struct ReturnDistribution {
    pub horizon_days: u32,
    pub mean: f64,
    pub std: f64,
    pub skewness: f64,
    pub percentiles: BTreeMap<u8, f64>, // 5th, 25th, 50th, 75th, 95th
}

/// Economic Layer output — REVISED: multidimensional scores, no premature state collapse
/// 
/// Rationale: Do NOT collapse into a single EconomicState (Favorable/Neutral/Unfavorable)
/// until TASK-080C proves such aggregation is justified. Premature state definition 
/// risks losing information (e.g., Liquidity=Favorable + Credit=Unfavorable + Growth=Neutral).
pub struct EconomicSnapshot {
    pub date: NaiveDate,
    pub scope: AnalysisScope,
    
    // Individual category scores (0-100, higher = more favorable for returns)
    pub liquidity_score: f64,
    pub credit_score: f64,
    pub rates_score: f64,
    pub volatility_score: f64,
    pub dollar_score: f64,
    pub growth_score: f64,
    
    // Overall confidence in the economic assessment
    pub confidence: f64,
    
    // Predicted return distributions per horizon
    pub predicted_distributions: Vec<ReturnDistribution>,
    
    // Provenance for auditability
    pub provenance: EconomicProvenance,
}
```

### Factor Scoring Logic

**Level 1: Raw Factor Score**
- Same as current: rolling min/max normalization → 0-100
- Lookback period: configurable (default 252 trading days = 1 year)

**Level 2: Category Score**
- Aggregate all factors within a category
- Methods: simple average, weighted average, or PCA first component
- Missing factors: reduce weight or exclude

**Level 3: Economic Score Vector (TEMPORARY — no state collapse yet)**
- Output individual category scores as a vector: `(liquidity, credit, rates, vol, dollar, growth)`
- **Do NOT collapse into a single state** (Favorable/Neutral/Unfavorable)
- Rationale: Mixed states are common (e.g., Liquidity=Favorable + Credit=Unfavorable). Premature collapse loses information.
- State collapse (if any) will be decided by TASK-080D based on data, not preset by architecture

### Frequency Handling Strategy

| Frequency | Handling | Example |
|-----------|----------|---------|
| Daily | Direct use | VIX, Fed Funds |
| Weekly | Forward-fill from last available | M2, NFCI |
| Monthly | Forward-fill + linear interpolation | PMI |

**Rule:** Always use the most recent available observation. Never extrapolate beyond the observation date.

---

## Part 4: Integration with Existing Layers

### State Layer (Existing)

**Role:** Describe current market environment  
**Input:** Price action + macro factors  
**Output:** RiskOn / Neutral / RiskOff

**Relationship with Economic Layer:**
- State Layer answers: "What is the market doing?"
- Economic Layer answers: "What is the economy doing?"
- Both can coexist; they measure different things

### Economic Layer (NEW)

**Role:** Predict future return distribution  
**Input:** Economic factors (liquidity, credit, rates, vol, dollar, growth)  
**Output:** EconomicSnapshot (category scores + return distributions)

**Relationship with State Layer:**
- Economic Layer does NOT replace State Layer
- Economic Layer is a separate signal source
- Allocation Layer combines both

**Key Design Decision:** Economic Layer outputs a **score vector**, not a single state.
- `(liquidity=75, credit=35, rates=60, vol=45, dollar=70, growth=55)`
- This preserves all information for Allocation Layer to use
- State collapse (if justified by data) happens in TASK-080D, not here

### Allocation Layer (Future)

**Role:** Generate position sizing decisions  
**Input:** State Layer + Economic Layer + Risk Budget  
**Output:** Position percentage, stop loss, target

**Decision Logic Example (using category scores):**
```
IF State = RiskOn AND liquidity > 60 AND growth > 60:
    position = 100%  # strong trend + favorable liquidity/growth
ELIF State = RiskOn AND credit < 40:
    position = 50%   # trend following but credit stress
ELIF State = RiskOff AND liquidity > 60 AND credit > 60:
    position = 30%   # buying dip (liquidity ample, credit OK)
ELIF State = RiskOff AND credit < 40 AND growth < 40:
    position = 0%    # risk off (credit + growth both stressed)
```

---

## Part 5: FRED Availability Findings

Based on librarian agent research (bg_ad48cc93):

### Critical Gaps

| Factor | Issue | Mitigation |
|--------|-------|------------|
| **MOVE Index** | ❌ NOT on FRED (ICE proprietary) | Source from ICE/Bloomberg, or use VIX as bond vol proxy |
| **TED Spread** | ❌ DISCONTINUED Jan 2022 (LIBOR cessation) | Use `SOFR - DTB3` spread, or omit |
| **ISM PMI (NAPM)** | ❌ DEPRECATED on FRED 2016 | Source from ISM directly, or use CFNAI |
| **Earnings Revisions** | ❌ NOT on FRED | Source from FactSet/Bloomberg, or omit |

### FRED Limits (April 2026)

**ICE BofA series now have 3-year observation limits on FRED:**
- BAMLH0A0HYM2 (HY Spread)
- BAMLC0A0CM (IG Spread)  
- BAMLC0A4CBBB (BBB Spread)
- VIXCLS (VIX)

**Impact:** For lookback normalization requiring >3 years of history, must source directly from ICE or use shorter lookback periods.

### Recommended MVP Factor Set

**Daily (auto-ingest):**
- `DFF` — Fed Funds (existing)
- `DGS10` — 10Y Treasury (existing)
- `DGS2` — 2Y Treasury
- `T10Y2Y` — Term Spread
- `SOFR` — Secured Overnight Financing Rate
- `VIXCLS` — VIX (existing)
- `DTWEXBGS` — Dollar Index (existing)
- `BAMLH0A0HYM2` — HY Spread
- `BAMLC0A0CM` — IG Spread

**Weekly/Monthly (manual handling):**
- `NFCI` — Financial Conditions (validation only)
- `M2SL` — M2 Money Supply
- `WRESBAL` — Bank Reserves
- `CFNAI` — Chicago Fed Activity Index
- `ICSA` — Initial Claims

**Total MVP: ~13 factors** (down from 19 candidates)

---

## Part 6: Implementation Roadmap

### Phase 1: Foundation (TASK-080A — Current) ✅
- [x] Catalog existing factors
- [x] Design architecture
- [x] Research FRED series availability
- [x] Define MVP factor list (13 factors)

### Phase 2: Orthogonality Audit (TASK-080B)
- [ ] Fetch historical data for all 13 MVP factors
- [ ] Build 4 analysis tables: Pearson, Spearman, MI, Predictive Orthogonality
- [ ] Select final 10-12 factors based on orthogonality + predictive power
- [ ] Document factor selection rationale

### Phase 3: Economic Predictive Audit (TASK-080C)
- [ ] Collect forward returns (20d, 60d, 120d)
- [ ] Test each category vs forward returns
- [ ] Measure separation, information gain
- [ ] Identify which categories actually predict returns

### Phase 4: Taxonomy Discovery (TASK-080D)
- [ ] Let data reveal natural clusters (3-class? 4-class? continuous?)
- [ ] Test multiple state definitions against predictive power
- [ ] Select optimal taxonomy based on evidence
- [ ] **Only NOW define EconomicState if justified**

### Phase 5: Prototype (TASK-081)
- [ ] Implement Economic Layer v2 with selected factors
- [ ] Generate Economic Snapshots
- [ ] Backtest in isolation
- [ ] Measure Economic Layer metrics

### Phase 6: Integration
- [ ] Connect Economic Layer to Allocation Layer
- [ ] Three-layer backtest (State + Economic + Allocation)
- [ ] Compare vs State-Only baseline
- [ ] Compare vs State-Only baseline

---

## Part 6: Risk Assessment

### Data Risks
- **FRED series discontinued:** Some series may be discontinued (e.g., LIBOR-based)
- **Missing data:** Weekly/monthly factors create gaps
- **Look-ahead bias:** Monthly data released with lag; must use as-of dates

### Model Risks
- **Overfitting:** Too many factors may overfit historical data
- **Non-stationarity:** Economic relationships change over time
- **Multicollinearity:** Factors within a category may be highly correlated

### Mitigation
- TASK-080B explicitly tests orthogonality
- Use rolling window for normalization (adapts to regime changes)
- Start with fewer factors, expand based on evidence

---

## Next Steps

1. **Validate this architecture** — Review with user
2. **Research FRED series IDs** — Librarian agent working on this
3. **Check factor availability** — Test fetch for all candidate series
4. **Proceed to TASK-080B** — Orthogonality audit

---

## Appendix: Full Candidate Factor List

### Liquidity (6 factors)
- DFF (Fed Funds)
- DGS10 (10Y Treasury)
- DGS2 (2Y Treasury)
- T10Y2Y (Term Spread)
- M2SL (M2 Money Supply)
- SOFR (Secured Overnight Financing Rate)

### Credit (4 factors)
- BAMLH0A0HYM2 (HY Spread)
- BAMLH0A3HYCEY (IG Spread)
- TEDRATE (TED Spread)
- BAMLH0A3HYCEY (CCC Spread — verify series ID)

### Volatility (2 factors)
- VIXCLS (VIX)
- MOVE (MOVE Index)

### Dollar (2 factors)
- DTWEXBGS (DXY)
- DEXUSEU (EUR/USD)

### Growth (4 factors)
- NAPM (PMI)
- ICSA (Initial Claims)
- CCSA (Continuing Claims)
- USPHCI (Philadelphia Fed Leading Index)

### Financial Conditions (1 factor)
- NFCI (Chicago Fed National Financial Conditions Index)

**Total: 19 candidate factors**
