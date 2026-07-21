# ADR-101: Transition Evidence Modeling

**Status:** Accepted  
**Date:** 2026-07-18  
**Scope:** V8 Execution Platform, Phase 2B-2  
**Decision Owner:** V8 Execution Platform Team

## Context

Phase 2A isolated the Reduce=0 bottleneck to the Evidence layer: the system can detect that risk exists, but it cannot distinguish risk that requires exit from risk that resolves into recovery.

TASK-153 (Bearish Evidence Analysis) showed that current bearish evidence has low discriminating power: the baseline negative T+20 rate among bearish candidates is only 34.5%.

TASK-153.5 (RiskExpansion Coverage Exploration) disproved the hypothesis that RiskExpansion is either scarce alpha or under-covered due to a strict threshold. RiskExpansion fires on 5.11% of all records, but only 6 of 145 bearish candidates have it. Lowering the amplitude threshold increases coverage but does not produce a strong signal (lift at 0.03 threshold is only 1.11). The current threshold of 5% is above the 90th percentile of amplitude, meaning it captures extreme volatility days that are more likely to rebound.

The deeper problem is that current Evidence is **state-based**, not **transition-based**. Evidence such as `Breadth`, `Recovery`, and `LeadershipRotation` describes the current market condition, not how the market is changing. Because many of these state evidences are always present, they cannot distinguish exit-requiring deterioration from temporary stress.

## Decision

**Introduce Transition Evidence Modeling as Phase 2B-2, before any Holding Risk Evidence or Decision calibration work.**

- Transition Evidence describes **change over time**, not static state.
- The first candidate transition evidences are:
  - `BreadthDeterioration`: breadth declining over a multi-day window.
  - `RecoveryFailure`: a recovery attempt that fails to restore breadth, leadership, or liquidity.
  - `LeadershipDecay`: previously strong leadership weakening.
  - `LiquidityDeterioration`: volume/liquidity trend deteriorating.
- Each candidate must be developed and validated as a Research Asset before being wired into `EvidenceBuilder` or `DecisionEngine`.
- No DecisionEngine, Policy, or threshold changes until Transition Evidence demonstrates lift ≥ 1.2 and precision ≥ 50% in historical replay.

## Rationale

1. **State Evidence has reached its limit.** TASK-153 showed that fixed state evidences have lift 1.0 because they are always present. They can no longer improve decision quality.
2. **RiskExpansion is not the answer.** TASK-153.5 proved that simply tuning the RiskExpansion threshold does not produce a reliable exit signal.
3. **Exit decisions require transition information.** Knowing "the market is risky" is not enough; the system needs to know whether conditions are deteriorating faster than they can recover.
4. **Research Asset discipline.** ADR-100 established that evidence quality must be validated before calibration. Transition Evidence is the natural next step in that discipline.

## Consequences

### Expected

- New observation/analysis modules compute deltas and transition states from existing data.
- Each Transition Evidence candidate is evaluated against historical outcomes as a standalone Research Asset.
- Only validated candidates are promoted into the Evidence layer.
- The EvidenceKind enum may eventually be extended, but only after validation.

### Accepted

- This delays Holding Risk Evidence design until after Transition Evidence validation. That delay is intentional and required.
- Some Transition Evidence candidates will fail validation and be rejected. This is a healthy outcome, not a failure.
- The Execution Platform remains in research mode: no production decision logic changes until evidence quality is proven.

## Verification

Before any Transition Evidence is wired into `EvidenceBuilder` or `DecisionEngine`, it must produce a report showing:

- Sample count ≥ 30 (to reduce noise)
- Lift vs bearish baseline ≥ 1.2
- Negative T+20 precision ≥ 50%
- Comparison against current best combination (`Distribution + RiskExpansion`)
- No regression in false reduce rate when combined with existing evidence

## References

- ADR-100: Evidence Quality Before Decision Calibration
- TASK-153: Bearish Evidence Analysis
- TASK-153.5: RiskExpansion Coverage Exploration
- `crates/execution-replay/src/bearish_analysis.rs`
- `research/validation/execution/README.md`

## Next Steps

1. Define the first Transition Evidence candidate (recommend `RecoveryFailure` because current `Recovery` state evidence is degenerate).
2. Implement a research-only computation module that computes the transition signal from historical records.
3. Run the Bearish Evidence Analysis tool with the new signal as an additional dimension.
4. If validation passes, promote the candidate to a formal EvidenceKind; if not, iterate or reject.
