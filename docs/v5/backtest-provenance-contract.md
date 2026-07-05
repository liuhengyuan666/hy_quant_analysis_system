# Backtest Provenance Contract

**Status:** Accepted  
**Date:** 2026-06-17  
**Author:** Sisyphus (AI Agent)  
**Reviewed by:** User (Project Owner)

---

## Context

After the Wave 11 milestone (Backtest Engine v1 + Historical Baseline v1), we have established:

- **State Layer v1.0** frozen (ADR-061, ADR-062, ADR-063)
- **Backtest Engine v1** frozen (DeRisk = 30%, not 0%)
- **run_version** field already deployed in `backtest_run` table and `BacktestSummary` DTO

However, `run_version` alone is insufficient for full provenance. We need a stronger contract that guarantees every backtest result can be traced back to:
- The code version that generated it
- The state machine version that governed it
- The exact point in time it was generated

This document defines the Backtest Provenance Contract for all future backtest persistence.

---

## Decision

### All backtest results MUST carry the following provenance fields

| Field | Type | Source | Default | Purpose |
|-------|------|--------|---------|---------|
| `run_version` | `String` | Hard-coded in `run_signal_backtest` | `"legacy"` | Backtest engine / state machine version (e.g. `v1`, `v2`) |
| `generated_at` | `DateTime<Utc>` | `chrono::Utc::now()` at end of `run_signal_backtest` | `now()` | Exact timestamp of generation |
| `git_commit` | `String` (optional) | Build-time env `GIT_COMMIT` or `git rev-parse HEAD` | `"unknown"` | Exact code commit hash |

### Dashboard / report layer contract

1. **Dashboard only displays `run_version = 'v1'`** (or current production version)
2. **Legacy results remain queryable** but are excluded from the default dashboard view
3. **Historical versions must NEVER be overwritten** — new versions create new rows
4. **Schema evolution**: new provenance fields must use `#[serde(default)]` or `DEFAULT` clause in ClickHouse

---

## Rationale

### Why provenance is a maturity milestone

Before this contract, the database looked like:

```text
backtest_run
├── Sharpe = 1.4
├── CAGR = 18%
└── ???
```

You could never tell whether a Sharpe change was caused by:
- **Strategy change** (new state machine thresholds)
- **Code fix** (DeRisk bug fixed)
- **Market change** (natural regime shift)

After this contract, the database looks like:

```text
backtest_run
├── run_version = v1
├── git_commit = a3f7d2e
├── generated_at = 2026-06-17 08:27:04 UTC
├── Sharpe = 0.48
├── CAGR = 8.9%
└── DeRisk = 30% (not 0%)
```

This makes it possible to distinguish:
- `v1` vs `v2` → strategy / threshold changes
- `a3f7d2e` vs `b8e1c4a` → code bug fixes
- `2026-06-17` vs `2026-05-01` → market regime changes

---

## Implementation Status

| Field | Status | Location |
|-------|--------|----------|
| `run_version` | ✅ Implemented | `BacktestSummary.run_version`, `quant.backtest_run.run_version` |
| `generated_at` | ✅ Implemented | `BacktestSummary.generated_at` (runtime UTC), `quant.backtest_run.generated_at` |
| `git_commit` | ✅ Implemented | `BacktestSummary.git_commit` (build-time via `build.rs` + `BACKTEST_GIT_COMMIT`), `quant.backtest_run.git_commit` |

---

## Consequences

### Positive
- Full traceability between code version, state machine version, and performance metrics
- Enables controlled A/B testing between backtest versions
- Prevents accidental overwrites of historical baselines
- Makes Shadow Production evaluation statistically sound

### Negative
- Slightly larger table (3 new columns)
- Build pipeline needs to inject `GIT_COMMIT` env var
- Dashboard queries must include `run_version` filter

---

## Related

- ADR-061: State Machine v1.0 Freeze
- ADR-062: Evaluation Framework
- ADR-063: Economic Taxonomy
- TRAP-003: Backtest DeRisk Bug Fix
- `shadow-production/README.md`: Phase A/B/C observation plan
