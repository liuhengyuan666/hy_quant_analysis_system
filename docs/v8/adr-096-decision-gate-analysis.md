# ADR-096: Decision Gate Analysis — Why Bearish Assessments Do Not Become Reduce

**Status:** Accepted  
**Date:** 2026-07-17  
**Tags:** v8, execution, decision-gate, confidence, risk, reduce, calibration

---

## Context

After ADR-095 (Decision Path Review) we knew that `Reduce = 0` was caused by the Decision layer, not the Observation layer. We had 152 records where `assessment.dominant_direction < reduce_threshold` (set to -0.3), yet none of them produced a `Reduce` decision.

The DecisionEngine checks gates in this order before the `reduce_threshold` branch:

1. `risk == Critical` → Wait
2. `risk == High` → Wait
3. `confidence < confidence_threshold` → Wait
4. `consensus < consensus_threshold` → Wait
5. `direction > buy_threshold` → BuyNow
6. `direction < reduce_threshold` → Reduce
7. else → Wait

We needed to know which gate was actually blocking the 152 bearish candidates.

## Decision

Implement a **Decision Gate Analysis** tool that reproduces the exact DecisionEngine gate order and, for every Reduce candidate, reports the first gate that blocked it. The tool also outputs per-record detail so we can inspect `confidence`, `consensus`, `risk`, and `strategy_state`.

This is a read-only diagnostic. No changes are made to Observation, Evidence, Assessment, Decision, or Policy logic.

## Rationale

- Reducing `reduce_threshold` blindly would only increase the candidate pool; it would not fix the gate that blocks them.
- We need to know whether the bottleneck is **confidence**, **consensus**, **risk**, or a combination, before writing a Calibration Proposal.
- Per-record detail lets us distinguish between a threshold issue and a risk-semantic issue (e.g., "High Risk" currently means "do nothing" rather than "exit position").

## Results

Run on CN 2024-01-01 to 2025-06-30 (8,616 records):

```
Reduce Candidates:        152
Risk Critical:              0
Risk High:                 54 (35.5%)
Confidence too low:        98 (64.5%)
Consensus too low:          0 (0.0%)
Passed all gates:           0
Final Reduce:               0
```

Key findings:

1. **Confidence is the primary blocker**: 98 of 152 candidates (64.5%) had `confidence < 0.6`. Their actual confidence values were clustered around 0.44–0.53, only 0.05–0.15 below the threshold.
2. **Risk High is the secondary blocker**: 54 candidates (35.5%) had `risk == High`. This raises a risk-semantic question: should "High Risk" suppress Reduce (as it currently does) or drive Reduce?
3. **Consensus is not a bottleneck**: no candidate was blocked by the `consensus >= 0.5` gate. Bearish evidence is already directionally aligned.
4. **All candidates had StrategyState == NoTrade**: this is expected because strategy state is scope-wide on a given date, but it means Prior evidence is uniformly bearish-conservative on these days.

## Consequences

- The next Review step (2A-4C) should be a **Risk Semantics Review** focused on the 54 `Risk High` candidates, to decide whether "High Risk" should mean "Wait" or "Reduce".
- The first Calibration Proposal (2A-5) should evaluate whether to use a lower `confidence_threshold` for `Reduce` than for `BuyNow`, rather than a single threshold for both directions.
- `volume_ma20` remains unfixed per user decision; while it distorts Distribution coverage, it does not affect the Decision Gate findings because the 152 candidates already crossed the reduce threshold.

## Exit Criteria

- [x] `DecisionGateAnalysis` module and formatter exist in `execution-replay`.
- [x] CLI command `execution-decision-gate` exists and runs on representative suite and full-population date ranges.
- [x] Full CN dataset funnel report is generated and saved under `reports/execution-validation/`.
- [x] Per-record JSON with confidence/consensus/risk/state is saved for further analysis.
- [x] No changes to Observation / Evidence / Assessment / Decision / Policy logic.

## Related ADRs

- ADR-092: Phase 2A Plan
- ADR-093: Execution Statistics Contract Freeze
- ADR-094: Evidence Trace & Root Cause Review
- ADR-095: Decision Path Review

## References

- `research/validation/execution/README.md` — Phase 2A-4.5 findings
- `crates/execution-replay/src/decision_gate.rs`
- `crates/execution-replay/src/decision_gate_formatter.rs`
- `crates/app-service/src/execution_replay.rs` — `execution_decision_gate_from_range`
- `apps/cli/src/commands/execution_replay.rs`
- `reports/execution-validation/decision_gate_cn_full_2026-07-17.md`
- `reports/execution-validation/decision_gate_cn_full_2026-07-17.json`
