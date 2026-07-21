# ADR-102: Rejected `RecoveryFailure` as Exit Transition Evidence

**Status:** Accepted  
**Date:** 2026-07-18  
**Scope:** V8 Execution Platform, Phase 2B-2.1  
**Decision Owner:** V8 Execution Platform Team

## Context

Phase 2B-2 of the V8 Execution Platform is investigating Transition Evidence: signals that describe how market conditions are changing, rather than static market states. The goal is to discover Exit-specific patterns before introducing any new `EvidenceKind` or modifying `ObservationEngine`, `EvidenceBuilder`, `AssessmentEngine`, `DecisionEngine`, or `ExecutionPolicy`.

TASK-154.1 implemented `RecoveryFailure` as the first Transition Evidence candidate. The hypothesis was:

> After a market pressure day, if a price recovery attempt fails to restore breadth and leadership, the subsequent T+20 / T+60 return should be negative more often than the baseline.

This hypothesis was tested as a Research Asset over the CN 2024-01-01 to 2025-06-30 dataset (8,616 `ExecutionResearchRecord` samples).

## Decision

**Reject `RecoveryFailure` as a standalone Exit Transition Evidence.**

It does not meet the ADR-101 validation thresholds:

| Metric | Requirement | Observed | Result |
|---|---|---|---|
| Sample size | ≥ 30 | 2,053 | PASS |
| T+20 precision (negative rate) | ≥ 50% | 45.1% | FAIL |
| Lift vs baseline | ≥ 1.2 | 0.95 | FAIL |

Furthermore, the average T+20 return among `RecoveryFailure` samples was **+2.51%**, indicating that the signal actually selects days that tend to be followed by positive returns, not negative ones.

## Rationale

1. **The data falsifies the hypothesis.** In the tested window, a weak recovery after pressure is more consistent with normal consolidation / re-accumulation than with trend breakdown. A rejected hypothesis is a valid and valuable research outcome.
2. **The dataset context matters.** CN 2024-2025 was largely a bullish / structured market. Weak recoveries in such an environment are often temporary before continuation, not the start of exits.
3. **ADR-101 discipline works.** The candidate was validated as a Research Asset before any pipeline change. Because it failed, no `EvidenceKind`, `ObservationKind`, or policy threshold was modified. This avoids polluting the Execution Platform with a low-quality signal.
4. **Rejecting a candidate is not project failure.** It is the normal output of a rigorous Research Layer. The value of the finding is that it prevents future wasted effort on `RecoveryFailure` variants and frees the team to pursue higher-potential candidates.

## Consequences

### Accepted

- `RecoveryFailure` will not be promoted to `ObservationKind` or `EvidenceKind`.
- No `ExecutionPolicy` or `DecisionEngine` change will be motivated by `RecoveryFailure`.
- The rejection is documented in this ADR and in the execution validation README.
- The `execution-transition-analysis --candidate recovery_failure` tool remains available for future re-testing on different datasets or market regimes, but its result is not actionable until the data changes.

### Future triggers for re-evaluation

`RecoveryFailure` may be revisited only if:

- The dataset is extended to include a materially different market regime (e.g., sustained bear market or high-volatility period).
- A new definition of `RecoveryFailure` is proposed and validated independently on a different sample, with lift ≥ 1.2 and precision ≥ 50%.
- It is combined with another Transition Evidence and the combination shows additive lift in a controlled experiment.

## References

- ADR-101: Transition Evidence Modeling
- `research/validation/execution/README.md` TASK-154.1 section
- `reports/execution-validation/transition_analysis_recovery_failure_cn_2026-07-18.md`

## Next Steps

Proceed to TASK-154.2: `BreadthDeterioration` Research Module. The priority is to test whether market breadth deterioration is a more reliable Transition Evidence than price-based recovery failure.
