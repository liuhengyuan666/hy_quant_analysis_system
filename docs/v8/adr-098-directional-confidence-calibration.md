# ADR-098: Directional Confidence Calibration Experiment

**Status:** Accepted  
**Date:** 2026-07-17  
**Tags:** v8, execution, calibration, confidence, reduce, experiment, replay

---

## Context

Risk Semantics Review (ADR-097) concluded that `RiskLevel::High` semantics were correct and that the remaining cause of `Reduce = 0` was the `confidence_threshold` of 0.6 blocking 98 bearish Reduce candidates. The hypothesis was that lowering the confidence threshold would release these candidates and produce Reduce decisions.

This ADR proposes a replay-based calibration experiment to validate that hypothesis before changing any `ExecutionPolicy` defaults.

## Decision

Implement a Calibration Framework that:

1. Re-runs the `DecisionEngine` on the same set of records using alternative confidence thresholds.
2. Tests both uniform thresholds (0.55, 0.50, 0.45) and an asymmetric threshold (buy 0.6 / reduce 0.5).
3. Measures coverage, precision, recall, F1, and opportunity cost (T+20, T+60, T+120 returns after Reduce).
4. Does not modify `DecisionEngine`, `ExecutionPolicy` defaults, or any other engine logic.

No confidence threshold is changed until the experiment shows acceptable precision (target ≥ 50%) for released Reduce actions.

## Experiments

| ID | Threshold | Description |
|---|---|---|
| baseline | 0.60 | Current default |
| c1 | 0.55 | Slightly lower uniform threshold |
| c2 | 0.50 | Moderately lower uniform threshold |
| c3 | 0.45 | Aggressively lower uniform threshold |
| asymmetric | buy 0.60 / reduce 0.50 | Directional confidence thresholds |

## Results

Run on CN 2024-01-01 to 2025-06-30 (8,616 records):

| Experiment | Reduce Candidates | Reduce Count | Avoided Loss | Missed Recovery | Precision | Recall | F1 | Avg T+20 (Reduce) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| Baseline 0.60 | 152 | 0 | 0 | 0 | N/A | 0.0% | N/A | N/A |
| C1: 0.55 | 152 | 0 | 0 | 0 | N/A | 0.0% | N/A | N/A |
| C2: 0.50 | 152 | 12 | 2 | 10 | 16.7% | 3.4% | 5.6% | +1.5% |
| C3: 0.45 | 152 | 65 | 24 | 41 | 36.9% | 40.7% | 38.7% | +2.1% |
| Asymmetric 0.60/0.50 | 152 | 12 | 2 | 10 | 16.7% | 3.4% | 5.6% | +1.5% |

## Key Findings

1. **C1 (0.55) releases no Reduce actions.** The 98 confidence-blocked candidates have confidence values mostly below 0.55, so a small threshold reduction is insufficient.
2. **C2 (0.50) and the asymmetric experiment produce only 12 Reduce actions with very low precision (16.7%).** Most released Reduce actions miss subsequent recoveries.
3. **C3 (0.45) releases 65 Reduce actions, but precision is only 36.9%** — below the 50% acceptability threshold. Over 60% of Reduce actions would be false positives.
4. **The average T+20 return of all Reduce candidates is +2.4%, and the average T+20 return after Reduce under C3 is +2.1%.** There is almost no net benefit from reducing versus waiting.

## Conclusion

**Lowering the confidence threshold alone does not produce high-quality Reduce signals.** The underlying bearish evidence is not sufficiently predictive in this dataset to distinguish between genuine distribution/risk-expansion events and short-term panic rebounds.

Therefore, **the calibration experiment does not recommend changing the confidence threshold at this time**. The next step should be to improve the quality of bearish evidence before recalibrating.

## Recommended Next Steps

1. Fix the `volume_ma20` placeholder so that `volume_ratio` reflects the actual 20-day volume average, which may change the Distribution evidence profile.
2. Re-run the entire Decision Path Review chain after the `volume_ma20` fix.
3. If evidence quality improves, repeat the calibration experiment.
4. Only promote a threshold change when precision is ≥ 50%.

## Exit Criteria

- [x] Calibration framework implemented in `execution-replay`.
- [x] CLI command `execution-calibration` exists and runs the baseline + four experiments.
- [x] Full CN dataset report generated and saved under `reports/execution-validation/`.
- [x] Coverage, precision, recall, F1, and opportunity cost metrics computed.
- [x] No changes to `ExecutionPolicy` defaults or `DecisionEngine` logic.

## Related ADRs

- ADR-092: Phase 2A Plan
- ADR-093: Execution Statistics Contract Freeze
- ADR-094: Evidence Trace & Root Cause Review
- ADR-095: Decision Path Review
- ADR-096: Decision Gate Analysis
- ADR-097: Risk Semantics Review

## References

- `research/validation/execution/README.md` — Phase 2A-5 findings
- `crates/execution-replay/src/calibration.rs`
- `crates/execution-replay/src/calibration_formatter.rs`
- `crates/app-service/src/execution_replay.rs` — `execution_calibration_from_range`
- `apps/cli/src/commands/execution_replay.rs`
- `reports/execution-validation/calibration_cn_full_2026-07-17.md`
- `reports/execution-validation/calibration_cn_full_2026-07-17.json`
