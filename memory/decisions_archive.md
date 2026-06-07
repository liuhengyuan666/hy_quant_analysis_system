## ADR-057: HK Liquidity-Dominant Regime

**Status:** Rejected

### Context
Originally proposed as a solution to HK's perceived regime failure. Evidence suggested Risk was alignment-best but Liquidity was economic-best for HK, leading to a proposal for a Liquidity-Dominant threshold scheme.

### Decision
**REJECTED.** The premise was flawed. HK's "failure" was caused by:
1. `confirmation_days=10` suppressing 72% of HK episodes
2. Missing HSI bars causing `trend_score` to always default to 50.0

After fixing both issues (ADR-058 + ADR-059), HK shows:
- Alignment=0.286 (outperforms CN=0.252)
- Sharpe=1.53, CAGR=22.96%

HK does not need a separate Liquidity-Dominant regime. The standard regime works correctly.

**Tags:** hk, liquidity-dominant, rejected, adr-057
