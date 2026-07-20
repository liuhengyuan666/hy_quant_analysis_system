# ADR-103: Transition Evidence Blocked by Upstream `ResearchContext` Data Quality

**Status:** Accepted  
**Date:** 2026-07-18  
**Scope:** V8 Execution Platform, Phase 2B-2 / 2B-3  
**Decision Owner:** V8 Execution Platform Team

## Context

Phase 2B-2 of the V8 Execution Platform is investigating Transition Evidence: signals that describe how market conditions are changing. The goal is to discover Exit-specific patterns before introducing any new `EvidenceKind` or modifying `ObservationEngine`, `EvidenceBuilder`, `AssessmentEngine`, `DecisionEngine`, or `ExecutionPolicy`.

TASK-154.1 rejected `RecoveryFailure` as a standalone Transition Evidence because it did not meet ADR-101 thresholds (lift 0.95, precision 45.1%).

TASK-154.2 attempted to validate `BreadthDeterioration` using `ExecutionMarketView.breadth.breadth_pct` and its 5-day / 10-day deltas.

TASK-154.3 attempted to validate `LeadershipDecay` using `ExecutionMarketView.leadership_stability` and its 5-day / 10-day deltas.

## Decision

**Transition Evidence work is blocked until the upstream `ResearchContext` / `ExecutionMarketView` data is populated with real breadth and leadership values.**

- `BreadthDeterioration` cannot be validated because `breadth_pct` is a constant placeholder (50.0) across all records.
- `LeadershipDecay` cannot be validated because `leadership_stability` is a constant placeholder (0.50) across all records.
- No further Transition Evidence candidate based on `ResearchContext` derived fields will be tested until the data quality is verified and fixed.

## Evidence

CN dataset, 2024-01-01 to 2025-06-30 (8,616 `ExecutionResearchRecord`):

| Field | Min | Max | Median | Notes |
|---|---|---|---|---|
| `ExecutionMarketView.breadth.breadth_pct` | 50.0 | 50.0 | 50.0 | Constant placeholder |
| `ExecutionMarketView.breadth.delta_5d` | 0.0 | 0.0 | 0.0 | All zero |
| `ExecutionMarketView.breadth.delta_10d` (computed) | 0.0 | 0.0 | 0.0 | All zero |
| `ExecutionMarketView.leadership_stability` | 0.50 | 0.50 | 0.50 | Constant placeholder |
| `ExecutionMarketView.leadership_stability` delta_5d | 0.00 | 0.00 | 0.00 | All zero |
| `ExecutionMarketView.leadership_stability` delta_10d | 0.00 | 0.00 | 0.00 | All zero |

The `ExecutionMarketView` is built from `ResearchContext` via `ExecutionMarketView::from_research_context`. Therefore, the root cause is that `ResearchContext.breadth` and `ResearchContext.rotation.leadership_stability` are not populated with real computed values in the current pipeline.

## Rationale

1. **Transition Evidence requires real transition data.** A Transition Evidence signal is a change in some market state. If the underlying state is constant, no transition can be computed.
2. **This explains the 2B-1 finding.** All fixed ResearchContext-derived evidences (`Breadth`, `Confirmation`, `Recovery`, `LeadershipRotation`) had lift = 1.0 because they were based on the same constant placeholder values for every record. This is not a semantic design problem; it is a data quality problem.
3. **Fixing data is upstream work.** The Execution Platform must not invent breadth or leadership data. The `ResearchContext` builder (likely in `llm-context` or `app-service`) must be corrected to compute and persist these values.
4. **Continuing without fixing the data would be wasted effort.** Any new Transition Evidence candidate based on `ResearchContext` fields will produce the same "no samples" or degenerate result until the data is real.

## Consequences

### Accepted

- No new Transition Evidence candidate will be tested until the upstream data is fixed.
- No `EvidenceKind`, `ObservationKind`, `AssessmentEngine`, `DecisionEngine`, or `ExecutionPolicy` changes will be made for this reason.
- The `execution-transition-analysis` tool remains available for re-testing once the data is fixed.
- Existing reports (`transition_analysis_breadth_deterioration_cn_2026-07-18.md`, `transition_analysis_leadership_decay_cn_2026-07-18.md`) document the blockage.

### Root cause identified (2026-07-18)

The constant values were **not** caused by a broken `ResearchContext` builder. They were caused by `crates/app-service/src/execution_replay.rs` constructing `ExecutionMarketView` directly from hardcoded placeholders instead of using `ExecutionMarketView::from_research_context`.

`build_execution_event` was hardcoding:
- `confirmation.trend/participation/risk.score = 50.0`
- `breadth.breadth_pct = 50.0`
- `recovery.score = 50.0`
- `leadership_stability = 0.5`

The `ResearchContext` builder (`build_research_context_from_dataset`) already computes real values from `environment_snapshot` (breadth) and rotation data (leadership). The bridge between `ResearchContext` and `ExecutionMarketView` was missing.

### Fix applied (2026-07-18)

1. `build_execution_event` now loads `ResearchContext` for the date/scope via `AppContext::build_research_context_for_date`.
2. `ExecutionMarketView` is now built with `ExecutionMarketView::from_research_context(&ctx)`.
3. `ResearchContext` is cached per date in `load_records_from_range` and `find_validation_candidates` to avoid redundant dataset fetches.
4. All call sites in `execution_replay.rs` were updated to pass the pre-built `ResearchContext`.

### Verification after fix (2026-07-18)

The `execution-context-integrity-audit` tool now reports all 8 fields as PASS:

| Field | Status | Unique | Min | Max | Variance |
|---|---|---:|---:|---:|---:|
| `confirmation.trend.score` | PASS | 359 | 35.54 | 84.38 | 106.15 |
| `confirmation.participation.score` | PASS | 150 | 20.00 | 100.00 | 457.38 |
| `confirmation.risk.score` | PASS | 358 | 14.23 | 87.52 | 246.05 |
| `breadth.breadth_pct` | PASS | 25 | 0.00 | 100.00 | 1255.29 |
| `breadth.delta_5d` | PASS | 39 | -83.33 | 87.50 | 836.50 |
| `breadth.sma5` | PASS | 110 | 3.33 | 100.00 | 1105.40 |
| `recovery.score` | PASS | 180 | 22.00 | 96.40 | 375.85 |
| `leadership_stability` | PASS | 359 | 0.06 | 1.00 | 0.05 |

### Re-run results after fix (2026-07-18)

After the fix, the 2B-1 and 2B-2 analyses were re-run with real data:

- `execution-bearish-analysis`: Fixed evidence still shows lift=1.00 within bearish candidates, but RiskExpansion at low thresholds shows lift=1.50. Data fix alone does not create exit signals; it only makes the data trustworthy.
- `execution-transition-analysis --candidate recovery_failure`: lift=0.99, precision=46.8% — still fails ADR-101.
- `execution-transition-analysis --candidate breadth_deterioration`: lift=1.03, precision=48.6% — fails ADR-101.
- `execution-transition-analysis --candidate leadership_decay`: lift=0.90 (T+20), precision=42.6% (T+20), but lift=1.51 (T+60), negative T+60=61.6% — interesting medium-term signal but fails T+20 ADR-101 thresholds.

### Required upstream work

1. ✅ Verify how `ResearchContext.breadth` is computed. (Already correct; bridge was missing.)
2. ✅ Verify how `ResearchContext.rotation.leadership_stability` is computed. (Already correct; bridge was missing.)
3. ✅ Re-generate the `ExecutionResearchRecord` stream for the validation window after fixing the data.
4. ✅ Re-run `execution-bearish-analysis` and `execution-transition-analysis` to confirm that fixed-state evidence now has lift variation and that Transition Evidence candidates can produce non-zero samples.

## References

- ADR-101: Transition Evidence Modeling
- ADR-102: Rejected `RecoveryFailure` as Exit Transition Evidence
- ADR-104: ResearchContext Fact Integrity Gate
- `research/validation/execution/README.md` TASK-154.2 / TASK-154.3 / TASK-156 sections
- `reports/execution-validation/context_integrity_audit_cn_2026-07-18.md`
- `reports/execution-validation/bearish_analysis_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_recovery_failure_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_breadth_deterioration_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_leadership_decay_cn_v2_2026-07-18.md`

## Next Steps

1. ✅ Fix the placeholder bridge in `execution_replay.rs` (completed).
2. ✅ Re-run the full 2B analysis chain (completed).
3. Iterate on Transition Evidence detection logic:
   - Tighten `BreadthDeterioration` conditions or add price/leadership filters.
   - Explore `LeadershipDecay` at T+60 horizon as a medium-term Holding Risk signal.
   - Try combination conditions (e.g., `BreadthDeterioration + LeadershipDecay`).
4. Only promote validated Transition Evidence to `ObservationKind` / `EvidenceKind` after ADR-101 thresholds are met.

---

**Update log**
- 2026-07-18: ADR-103 originally accepted. Transition Evidence blocked by constant placeholders.
- 2026-07-18: Root cause identified as hardcoded `ExecutionMarketView` construction in `execution_replay.rs`.
- 2026-07-18: Fix applied and verified. All 8 ResearchContext-derived fields pass the Fact Integrity Gate. 2B-1/2B-2 re-run with real data.
