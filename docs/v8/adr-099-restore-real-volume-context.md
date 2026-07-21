# ADR-099: Restore Real Volume Context Before Evidence Calibration

**Status:** Accepted  
**Date:** 2026-07-18  
**Scope:** V8 Execution Platform, Phase 2A-6  
**Decision Owner:** V8 Execution Platform Team

## Context

ADR-098 (2A-5) ran a Directional Confidence Calibration Experiment and rejected the threshold-only approach. Lowering the Reduce confidence threshold released Reduce actions, but the best achievable precision was ~37% at confidence=0.45. This proved that the bottleneck is not the Decision Gate sensitivity but the quality of the bearish evidence feeding into it.

The most immediate and tractable data-quality issue in the bearish evidence path was that `ExecutionRequest.volume_ma20` was hardcoded to `1.0`. This made `volume_ratio = volume / 1.0`, which is effectively the raw absolute volume number, not a relative volume reading. As a result, the Distribution observation condition:

```text
close_position < 0.2 && volume_ratio > 1.5 && today_return < 0.0
```

was contaminated by a meaningless volume threshold. Distribution is a key bearish evidence source, so fixing its volume input is the first step toward improving bearish evidence quality before any further calibration.

## Decision

**Restore the real 20-day volume moving average in `ExecutionRequest.volume_ma20`.**

- In `app-service::execution_replay::build_execution_event`, fetch the previous 40 calendar days of daily bars for the symbol.
- Compute `prev_close` from the last bar before the event date.
- Compute `volume_ma20` from the average volume of the last 20 trading-day bars before the event date.
- If fewer than 20 prior bars exist, use the average of available prior bars; if none exist, fall back to the current bar's volume.
- Populate `ExecutionRequest.volume_ma20` with this computed value.
- **Do not modify any Observation thresholds or Decision logic.** The goal is to restore the real input, not to optimize the condition.

After the fix, re-run the entire Decision Path Review chain (Statistics → Evidence Trace → Distribution Coverage → Decision Margin → Decision Gate → Risk Semantics → Calibration) to observe how the real volume context changes the evidence layer.

## Rationale

1. **Volume is a normalized ratio, not an absolute number.** Execution platform assumes `volume_ratio` compares today's volume to a recent average. The placeholder broke that assumption.
2. **Distribution is a primary bearish evidence source.** If its volume input is wrong, downstream bearish Assessment and Decision are built on a noisy signal.
3. **Threshold calibration should only happen after upstream evidence quality is validated.** ADR-098 showed that lower thresholds produce low-quality Reduce. Improving evidence quality is the prerequisite for any successful recalibration.
4. **This is a fact restoration, not a strategy optimization.** Changing `volume_ma20` from `1.0` to the real 20-day MA restores the intended semantics of the Observation layer without changing its thresholds.

## Consequences

### Expected

- `volume_ratio` becomes a meaningful relative volume metric.
- Distribution observation count may shift materially (fewer false positives, or more true positives).
- Bearish evidence population will change, which will change the Decision Margin and Decision Gate outputs.
- Calibration results will be re-evaluated with the real volume context.

### Accepted

- Some historical replay records near the start of the dataset will have a shorter volume MA lookback, falling back to the available bar average or current volume. This is acceptable because the alternative (1.0) is worse.
- Fetching a 40-day lookback per record adds a small I/O cost to historical replay. This is acceptable for research/CLI workflows and does not affect live execution latency, which is driven by real-time quote snapshots.

## Verification

- `cargo check --workspace` passes.
- `cargo test -p execution-replay`, `cargo test -p execution-engine`, `cargo test -p app-service` pass.
- The full Decision Path Review chain produces new reports with the real volume context.
- No Observation or Decision thresholds are changed.

## References

- ADR-098: Directional Confidence Calibration Rejected Threshold-Only Approach
- `crates/app-service/src/execution_replay.rs`
- `crates/execution-replay/src/v2/observation.rs` (Distribution condition)
- `research/validation/execution/README.md`
