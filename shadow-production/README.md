# Shadow Production

## Overview

This directory contains the Shadow Production logging infrastructure for the quant analysis system.

**Status:** The original 90-day observation window (Phases A/B/C below) ran 2026-06-07 → 2026-09-05 and is complete. Observation continues under `rv1_capability_consolidation`; `daily-log.ps1` still appends to `shadow-master.csv`, and the `A`/`B`/`C` phase values remain valid record-depth selectors (`A` = state layer, `B` = + economic layer, `C` = + allocation suggestion).
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

The Phase A/B/C schedule below documents the original 90-day protocol (2026-06-07 → 2026-09-05) and is retained as the reference for how each phase defined its records and columns. Continued observation under `rv1_capability_consolidation` uses the same `A`/`B`/`C` values as record-depth selectors, not as day-range gates.

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

The `T20_Return` / `T60_Return` / `T120_Return` columns in `shadow-master.csv` are broader, per-day market and state-layer observations that are filled in manually. They are **not** interchangeable with the automatic per-symbol `StrongBuy + DE_RISK` trading-bar outcomes that `quant-cli research observe` maintains in `workspace/divergence-ledger/`; that ledger matures each case independently from strictly subsequent persisted trading bars.

Where a matching persisted trading-bar series applies to a CSV row, use the Nth strictly subsequent persisted trading bar:

```powershell
# Example: Fill T20_Return for the 2026-06-07 row
# Look up the market return from that row date to the 20th strictly
# subsequent persisted trading bar (not 20 calendar days).
# Update the T20_Return column.
```

Where no such trading-bar series applies, treat the CSV columns as a manually defined broader metric and note the window basis used in the cell or in the weekly/monthly report.

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

- `docs/v5/adr-063-economic-taxonomy.md` — Economic Layer taxonomy definition
- `docs/v5/adr-061-decision-brief.md` — State Layer contract
- `docs/v5/task-080f-findings.md` — Fed Funds Z-score integration results
- `memory/decisions.md` — ADR decisions log

---

## Contact

For questions about Shadow Production protocol, refer to ADR-063 or contact the project maintainer.
