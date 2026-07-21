# ADR-097: Risk Semantics Review — Entry Risk vs Holding Risk

**Status:** Accepted  
**Date:** 2026-07-17  
**Tags:** v8, execution, risk-semantics, domain-modeling, entry-risk, holding-risk, reduce

---

## Context

Decision Gate Analysis (ADR-096) found that 54 of 152 bearish Reduce candidates were blocked by `RiskLevel::High`. The question was whether this was a domain-modeling error: the current semantics treat `RiskHigh` as "do not trade" rather than "exit position".

The user proposed that RiskLevel might be conflating two distinct concepts:

- **Entry Risk**: conditions that make opening a position dangerous (e.g., chasing, gap, overextension).
- **Holding Risk**: conditions that suggest an existing position should be reduced (e.g., Distribution, RiskExpansion, MomentumFailure).

If RiskHigh were mostly composed of Holding Risk evidence, then the current `RiskHigh -> Wait` rule would be suppressing necessary Reduce actions.

## Decision

Implement a Risk Semantics Review that:

1. Reports the evidence composition of `RiskLevel::High` records.
2. Reports the decision context (direction, confidence, consensus) for RiskHigh records.
3. Reports the **actual forward outcomes** of RiskHigh + Wait records, especially the bearish candidates blocked from Reduce.
4. Proposes a semantic mapping of Entry Risk vs Holding Risk evidence kinds, but **does not change any code**.

No modifications to `RiskLevel`, `DecisionEngine`, or `ExecutionPolicy` are allowed until the review is complete.

## Results

Run on CN 2024-01-01 to 2025-06-30 (8,616 records):

### Risk Distribution

| Level | Count | % |
|---|---|---|
| Low | 288 | 3.3% |
| Medium | 7,420 | 86.1% |
| High | 908 | 10.5% |

### RiskHigh Evidence Composition

| Evidence | Count in High Risk | % of High Risk | Proposed Category |
|---|---|---|---|
| Breadth | 908 | 100.0% | Ambiguous |
| Confirmation | 908 | 100.0% | Ambiguous |
| LeadershipRotation | 908 | 100.0% | Entry Risk |
| Recovery | 908 | 100.0% | Ambiguous |
| SignalStrength | 908 | 100.0% | Ambiguous |
| StrategyState | 908 | 100.0% | Ambiguous |
| Distribution | 774 | 85.2% | Holding Risk |
| RiskExpansion | 211 | 23.2% | Holding Risk |
| MomentumExpansion | 116 | 12.8% | Entry Risk |
| MarketAcceptance | 64 | 7.0% | Entry Risk |
| TrendParticipation | 61 | 6.7% | Ambiguous |
| MomentumFailure | 3 | 0.3% | Holding Risk |

### RiskHigh Decision Context

- All 908 High Risk records resulted in `Wait`.
- Direction: mean=-0.105, p50=-0.133, min=-0.508, max=0.512
- Confidence: mean=0.498, p50=0.481, p75=0.532
- Consensus: mean=0.621, p50=0.665, p75=0.701

### Future Outcomes

| Group | Count | T+20 Mean | T+60 Mean | T+120 Mean | Negative T+20 % |
|---|---|---:|---:|---:|---:|
| High Risk | 908 | 4.72% | 7.30% | 16.32% | 40.1% |
| High Risk + Wait | 908 | 4.72% | 7.30% | 16.32% | 40.1% |
| **RiskHigh + Bearish + Wait (blocked Reduce)** | **54** | **6.25%** | **4.86%** | **2.91%** | **29.6%** |
| Medium Risk | 7,420 | 1.83% | 7.51% | 16.47% | 47.2% |
| Low Risk | 288 | -2.41% | -1.34% | 10.35% | 69.8% |

## Key Finding

The 54 bearish RiskHigh + Wait candidates have an **average T+20 return of +6.25%** and a **negative T+20 ratio of only 29.6%**. This is better than Medium Risk and Low Risk groups.

Therefore, **the current `RiskHigh -> Wait` semantics are empirically justified for this dataset**. Changing them to `RiskHigh -> Reduce` would likely have underperformed by missing subsequent rebounds.

## Consequences

- The RiskHigh blocker is **not** a domain-modeling error that needs to be fixed before calibration.
- The dominant remaining cause of `Reduce = 0` is the **Confidence threshold** (98 of 152 candidates blocked by `confidence < 0.6`).
- The 2A-5 Calibration Proposal should focus on asymmetric confidence thresholds (e.g., lower threshold for Reduce than for BuyNow) rather than risk semantics.
- `volume_ma20` remains unfixed per user decision; it does not affect the Decision-layer findings because the candidates already crossed the reduce threshold.

## Exit Criteria

- [x] `RiskSemanticsReview` module and formatter exist in `execution-replay`.
- [x] CLI command `execution-risk-semantics` exists.
- [x] Full CN dataset report generated and saved under `reports/execution-validation/`.
- [x] Evidence composition, decision context, and forward outcome tables produced.
- [x] Semantic mapping proposal documented.
- [x] No changes to `RiskLevel`, `DecisionEngine`, or `ExecutionPolicy`.

## Related ADRs

- ADR-092: Phase 2A Plan
- ADR-093: Execution Statistics Contract Freeze
- ADR-094: Evidence Trace & Root Cause Review
- ADR-095: Decision Path Review
- ADR-096: Decision Gate Analysis

## References

- `research/validation/execution/README.md` — Phase 2A-4C findings
- `crates/execution-replay/src/risk_semantics.rs`
- `crates/execution-replay/src/risk_semantics_formatter.rs`
- `crates/app-service/src/execution_replay.rs` — `execution_risk_semantics_from_range`
- `apps/cli/src/commands/execution_replay.rs`
- `reports/execution-validation/risk_semantics_cn_full_2026-07-17.md`
- `reports/execution-validation/risk_semantics_cn_full_2026-07-17.json`
