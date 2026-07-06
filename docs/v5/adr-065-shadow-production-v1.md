# ADR-065: Shadow Production v1 Phase Declaration

**Status:** Accepted  
**Date:** 2026-06-17  
**Author:** Sisyphus (AI Agent)  
**Reviewed by:** User (Project Owner)

---

## Context

After completing Waves 8 through 11, the project has crossed three critical milestones:

1. **Wave 8**: Fixed false conclusions (HK filtering, Liquidity Dominant fix, 10d persistence removal)
2. **Wave 9–10**: Fixed evaluation framework (ADR-062 Alignment Gate, Forward Return GT, State GT)
3. **Wave 11**: Fixed baselines and established provenance (ADR-064, DeRisk bug fix, `run_version=v1` with full `git_commit` + `generated_at` chain)

The system now has:
- **State Layer v1.0** frozen (ADR-061, ADR-062, ADR-063)
- **Backtest Engine v1** frozen (DeRisk = 30%, not 0%)
- **Historical Baseline v1** established (`bt-20260617090905`, CAGR=8.87%, Sharpe=0.48, MaxDD=30.9%)
- **Research Governance** at 9/10 maturity (run_version, git_commit, generated_at, ADR trail)

The project has transitioned from a **quantitative strategy** to a **quantitative research system** with full provenance and version control.

---

## Decision

### Phase Transition

| Phase | Status | Start Date |
|-------|--------|------------|
| Research Program | **CLOSED** | 2026-06-17 |
| Shadow Production v1 | **OPEN** | 2026-06-17 |

### Shadow Production v1 Rules

#### Allowed (Infrastructure Only)
- Monitoring, alerting, observability
- Dashboard, reporting, data visualization
- Provenance infrastructure improvements (`git_commit`, `generated_at` build injection)
- Documentation and ADR updates
- Data health checks and pipeline diagnostics

#### Forbidden (90-Day Minimum Lock)
- Threshold tuning (state machine, signal, allocation)
- Weight modifications (rotation, strategy scoring, signal combination)
- Factor additions or removals
- State machine logic changes (transitions, state definitions)
- Economic taxonomy changes (ADR-063 3-State boundaries)
- Backtest execution semantics (slippage, fee modeling, drawdown logic)

#### Exception: Kill Criteria

If any of the following triggers activate, the lock may be broken with a **new ADR** documenting the violation:

| Category | Criteria | Description |
|----------|----------|-------------|
| **S1** | State Layer instability | `RiskOn`/`RiskOff`/`Neutral` transitions exceed 3σ of historical frequency |
| **S2** | State persistence collapse | `NO_TRADE` or `DE_RISK` exceeds 30% consecutive days without macro justification |
| **S3** | State signal contradiction | State Layer recommends `PROCEED` but 100% of signals are `Hold`/`Watch` for >5 days |
| **E1** | Economic Layer misalignment | Forward return distribution by Economic State violates ADR-063 variance ratio (<0.6) |
| **E2** | Regime macro disconnect | `Favorable` economic state but `RiskOff` market regime for >10 days |
| **E3** | Economic factor degradation | >2 core factors become permanently unavailable or show structural breaks |
| **A1** | Allocation drawthrough | Position sizing violates state recommendations by >20% in backtest |
| **A2** | Drawdown breach | Live portfolio drawdown exceeds 1.5× backtest MaxDD for same window |
| **A3** | Strategy drift | Best strategy changes from `MomentumRight` to `ValueLeft` for >30 days without macro shift |
| **D1** | Data pipeline failure | >2 consecutive days of missing market data for >50% of universe |
| **D2** | Provider structural change | Eastmoney / Tencent API changes permanently breaking ingestion |

---

## Rationale

### Why stop now?

The optimization trap is the most common failure mode for individual quantitative projects:

```text
发现一个问题
↓
加一个指标
↓
指标变好
↓
再发现一个问题
↓
再加一个指标
↓
...
最终 Sharpe: 1.9 → 2.3 → 2.7 → 3.1
实盘: 0.6
```

We have reached the point where **further code changes have lower expected value than observing real market behavior**. The system has:
- Corrected false conclusions (Wave 8)
- Fixed evaluation framework (Wave 9–10)
- Established provenance and baselines (Wave 11)

The next 90 days of market data (2026-06-17 onward) are the most valuable data we can collect.

### What distinguishes this from "giving up"?

This is a **disciplined pause with explicit re-entry conditions**. The system is not abandoned; it is:
- Frozen in a known-good state
- Under observation with defined metrics
- Protected from researcher-initiated drift
- Ready to resume research if kill criteria activate

---

## Baseline for Observation

| Component | Frozen Version | Baseline Run ID |
|-----------|---------------|-----------------|
| State Layer | v1.0 | ADR-061/062/063 |
| Backtest Engine | v1 | `bt-20260617090905` |
| Provenance Chain | v1 | `run_version=v1`, `git_commit=df9904edc26af440a88ecf83cbe50cbd6e763cd7`, `generated_at=2026-06-17 09:09:05 UTC` |
| Data | 2026-06-16 | All pipeline stages complete |

### Shadow Production Metrics to Track

| Phase | Days | Focus | Key Metric |
|-------|------|-------|------------|
| **A** | 1–30 | State Layer | Daily state stability, transition frequency, NO_TRADE% |
| **B** | 31–60 | Economic Layer | State-to-forward-return alignment, 3-State distribution |
| **C** | 61–90 | Allocation Layer | Position sizing vs backtest, drawdown tracking, strategy drift |

---

## Consequences

### Positive
- Prevents overfitting to historical data
- Forces validation of research assumptions in real market conditions
- Creates a disciplined framework for evaluating future changes
- Protects sunk cost of Waves 8–11 from being eroded by incremental tweaks

### Negative
- 90-day delay before any research improvements can be deployed
- Requires daily manual observation (or automated alerting) during Shadow Production
- Kill criteria may never activate, making the lock feel arbitrary in hindsight
- Risk of missing a genuine improvement opportunity during the lock period

### Mitigation
- Kill criteria are explicitly defined and can be triggered by data, not just intuition
- Infrastructure improvements (monitoring, reporting) are still allowed and should be invested in during the lock period
- After 90 days, a formal review ADR will evaluate whether to extend, lift, or modify the lock

---

## Related

- ADR-061: State Machine v1.0 Freeze
- ADR-062: Evaluation Framework
- ADR-063: Economic Taxonomy
- [Backtest Provenance Contract](./backtest-provenance-contract.md)
- TRAP-003: Backtest DeRisk Bug Fix
- `shadow-production/README.md`: Phase A/B/C observation plan
- `reports/state-transition-attribution-global.md`: TASK-090A audit baseline
