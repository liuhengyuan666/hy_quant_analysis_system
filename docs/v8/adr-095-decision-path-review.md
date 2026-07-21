# ADR-095: Phase 2A-4 Decision Path Review — Distribution Coverage + Decision Margin

**Status:** Accepted  
**Date:** 2026-07-17  
**Supersedes:** ADR-094  
**Tags:** v8, execution, decision-path-review, distribution-coverage, decision-margin, calibration

---

## Context

After ADR-094 (Evidence Trace & Root Cause Review) we had two candidate explanations for `Reduce = 0`:

1. **Observation layer**: `Distribution` observation count was 0, suggesting the observation condition was too strict.
2. **Decision layer**: `RiskExpansion` survived all the way to Assessment but never produced a `Reduce` decision, suggesting the decision threshold or Prior weight was the bottleneck.

The problem was that we had not proven which layer was the real bottleneck. ADR-094 therefore left the question open pending two focused reviews.

## Decision

Phase 2A-4 is renamed from "Root Cause Review" to **"Decision Path Review"** and is split into two focused sub-reviews:

- **2A-4A: Distribution Coverage Review** — does the current `Distribution` observation condition miss real distribution days? This is answered by computing feature percentiles and condition-coverage statistics, not by adjusting thresholds.
- **2A-4B: Decision Margin Review** — how does `Assessment.dominant_direction` map to the final `Decision`? This is answered by a histogram of `dominant_direction` per `EvidenceKind`, with special attention to records that cross `reduce_threshold` but still result in `Wait`.

No policy, threshold, or observation-condition modification is allowed until both reviews are complete and their findings are written into the ADR chain.

## Rationale

- We need to separate **data input correctness** (Observation) from **decision rule behavior** (Assessment → Decision). They have different fix strategies.
- **Distribution Coverage Review** prevents us from loosening `volume_ratio` or `close_position` thresholds blindly, which could make `Distribution` explode.
- **Decision Margin Review** shows whether the bottleneck is the threshold itself or upstream confidence/consensus/risk gates that suppress `Reduce` even when direction is bearish enough.
- These two reviews provide the empirical foundation for a future **Calibration Proposal** (2A-5), rather than guessing from a single summary statistic.

## Entry Criteria

- 2A-3 Evidence Trace completed (ADR-094).
- `Reduce = 0` has been narrowed to either (a) Observation condition or (b) Decision-layer behavior.

## Exit Criteria

1. 2A-4A: a Distribution Coverage Report exists, including:
   - feature percentiles for `close_position`, `volume_ratio`, and `today_return`;
   - condition-satisfaction analysis for the current `Distribution` rule;
   - clear statement of whether the condition is too strict, too loose, or correct.
2. 2A-4B: a Decision Margin Report exists, including:
   - per-`EvidenceKind` histogram of `Assessment.dominant_direction`;
   - cross-tab of `dominant_direction` vs final `Decision`;
   - count of records that cross `reduce_threshold` but still produce `Wait`.
3. No code changes to Observation, Evidence, Assessment, Decision, or Policy logic.
4. If a data bug is discovered that makes the review impossible (e.g. `prev_close` placeholder), the fix is documented but is treated as a data-pipeline correction, not a calibration decision.

## Consequences

- The CLI surface gains two new commands: `execution-distribution-coverage` and `execution-decision-margin`.
- The `execution-replay` crate gains two new domain modules: `distribution_coverage` + `distribution_coverage_formatter`, and `decision_margin` + `decision_margin_formatter`.
- The V8 validation report directory will hold additional artifacts: `distribution_coverage_cn_full_*.md` and `decision_margin_cn_full_*.md`.
- A future Calibration Proposal (2A-5) must reference the two reports and explain how it addresses the specific bottleneck they identify.

## Related ADRs

- ADR-092: Phase 2A Plan and Exit Criteria
- ADR-093: Execution Statistics Contract Freeze
- ADR-094: Evidence Trace & Root Cause Review
- ADR-082: Execution Platform V2 (market_regime_label ownership, no hardcoding)

## References

- `research/validation/execution/README.md` — Phase 2A-4 findings
- `crates/execution-replay/src/distribution_coverage.rs`
- `crates/execution-replay/src/distribution_coverage_formatter.rs`
- `crates/execution-replay/src/decision_margin.rs`
- `crates/execution-replay/src/decision_margin_formatter.rs`
- `crates/app-service/src/execution_replay.rs` — orchestration functions
- `apps/cli/src/commands/execution_replay.rs` — CLI handlers
- `apps/cli/src/main.rs` — CLI command definitions
