# Shadow Production — 90-Day Observation

## Overview

This directory contains the Shadow Production logging infrastructure for the quant analysis system.

**Status:** Phase A (State Layer observation only)  
**Start Date:** 2026-06-07  
**End Date:** 2026-09-05 (90 days)  
**Rule:** NO REAL MONEY EXECUTION

---

## Directory Structure

```
shadow-production/
├── daily-log.ps1          # Daily logging script
├── shadow-master.csv      # Master log (auto-generated)
├── shadow-log-YYYY-MM-DD.json  # Daily detailed logs (auto-generated)
├── reports/               # Weekly/monthly reports (manual)
└── README.md              # This file
```

---

## Phases

### Phase A: State Layer Observation (Days 1-30)

**Goal:** Validate State Layer stability in live market conditions.

**Daily action:**
```powershell
.\daily-log.ps1 -Phase A
```

**Records:**
- CN State (RiskOn / Neutral / RiskOff)
- HK State (RiskOn / Neutral / RiskOff)
- State transitions

**Weekly review:**
- State persistence (average duration)
- Coverage (% of days with valid state)
- Compare with ADR-061 State Truth

### Phase B: Economic Layer Integration (Days 31-60)

**Goal:** Validate Economic Layer taxonomy and predictive power.

**Daily action:**
```powershell
.\daily-log.ps1 -Phase B
```

**Records:**
- Economic State (Favorable / Neutral / Unfavorable)
- Economic Score (0-100)
- Factor contributions

**Weekly review:**
- Information Gain vs Forward Return
- State distribution stability (should remain ~37/40/22)
- Quintile return monotonicity

### Phase C: Allocation Prototyping (Days 61-90)

**Goal:** Begin paper-trading signals (no execution).

**Daily action:**
```powershell
.\daily-log.ps1 -Phase C
```

**Records:**
- Suggested Allocation (Conservative / Neutral / Aggressive)
- Paper portfolio P&L (hypothetical)

**Weekly review:**
- Paper portfolio Sharpe vs buy-and-hold
- Signal accuracy
- Turnover frequency

---

## Forward Return Tracking

After T+20/60/120 days, fill in the returns in `shadow-master.csv`:

```powershell
# Example: Fill T+20 return for 2026-06-07
# Look up market return from 2026-06-07 to 2026-06-27
# Update the T20_Return column
```

---

## Monthly Report Template

Create `reports/monthly-YYYY-MM.md` with:

1. **State Summary**
   - Days in each state (CN/HK)
   - State transition frequency
   - Average state duration

2. **Economic Layer Performance**
   - Information Gain (20d/60d/120d)
   - Quintile return spread
   - State distribution

3. **Paper Portfolio (Phase C only)**
   - Hypothetical P&L
   - Sharpe ratio
   - Max drawdown
   - Turnover

4. **Observations & Anomalies**
   - Any unexpected state assignments
   - Data quality issues
   - Model behavior notes

---

## Constraints

- **NO real money execution**
- Allocation suggestions are for observation only
- Human judgment required for all decisions
- Weekly review mandatory
- Monthly report mandatory

---

## Related Documents

- `docs/adr-063-economic-taxonomy.md` — Economic Layer taxonomy definition
- `docs/adr-061-state-semantic-contract.md` — State Layer contract
- `docs/task-080f-findings.md` — Fed Funds Z-score integration results
- `memory/decisions.md` — ADR decisions log

---

## Contact

For questions about Shadow Production protocol, refer to ADR-063 or contact the project maintainer.
