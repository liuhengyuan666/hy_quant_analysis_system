# ADR-104: ResearchContext Fact Integrity Gate

**Status:** Accepted  
**Date:** 2026-07-18  
**Scope:** V8 Execution Platform, Phase 2B-0  
**Decision Owner:** V8 Execution Platform Team

## Context

Phase 2B of the V8 Execution Platform is investigating Transition Evidence: signals that describe how market conditions are changing. The goal is to discover Exit-specific patterns before introducing any new `EvidenceKind` or modifying the Execution Pipeline.

TASK-154.2 (`BreadthDeterioration`) and TASK-154.3 (`LeadershipDecay`) produced zero samples because the underlying `ExecutionMarketView` fields are constant placeholders:

- `breadth_pct`: 50.0 for all 8,616 records
- `leadership_stability`: 0.50 for all 8,616 records

This is not a model failure. It is a **fact integrity failure**: the `ResearchContext` data consumed by the Execution Platform is not carrying real computed values, so any Evidence derived from it is degenerate.

This issue is documented in ADR-103.

## Decision

**Introduce a ResearchContext Fact Integrity Gate as Phase 2B-0.**

No Evidence Modeling work shall proceed on `ResearchContext` derived fields until those fields pass variance and provenance validation. The gate must be executed before any new Transition Evidence candidate is designed or tested.

The gate is a read-only audit tool. It does not modify any Observation, Evidence, Assessment, Decision, or Policy code. It inspects the `ExecutionResearchRecord` stream and reports whether each `ResearchContext` derived field is trustworthy.

## Requirements

### 1. Variance Check

For every numeric field that enters the Execution Platform via `ExecutionMarketView` (or future equivalent), compute:

- `unique_values`: count of distinct values
- `min`: minimum observed value
- `max`: maximum observed value
- `variance`: statistical variance
- `sample_count`: number of non-missing observations

A field is **flagged** if:
- `min == max` (constant), or
- `unique_values` is suspiciously small (e.g., fewer than 10 distinct values across thousands of records), or
- `variance` is effectively zero.

### 2. Placeholder Detection

Detect known placeholder values:

- `50.0` for percentage fields that should be variable (e.g., `breadth_pct`)
- `0.50` for stability/score fields that should be variable (e.g., `leadership_stability`)
- Any value that matches the `Default::default()` output of a struct without real computation

A field is **flagged** if it matches a known placeholder pattern or if its distribution suggests a default value rather than a computed one.

### 3. Provenance Check

The Execution Platform must be able to distinguish:

- Fields that came from a real computation over market data
- Fields that came from a placeholder / default value

For V8, this is implemented by adding a `ContextIntegrityReport` to the `ExecutionMarketView` (or equivalent) that records, for each field:

- `field_name`: the semantic field name
- `source`: the upstream computation or data source
- `is_computed`: true if the value was derived from real market data
- `placeholder_value`: the value that indicates a missing/uncomputed state, if any

## Consequences

### Accepted

- Phase 2B-0 becomes a mandatory gate before any new Evidence Modeling work.
- TASK-156: ResearchContext Fact Integrity Audit is created to implement and run the gate.
- Existing `ExecutionResearchRecord` artifacts that fail the gate must be considered polluted and should not be used for downstream Evidence validation until regenerated with fixed data.
- No new Transition Evidence candidate will be accepted for validation unless all fields it depends on pass the gate.

### Required next steps (all completed 2026-07-18)

1. ✅ Implement the `ContextIntegrityReport` audit tool in `execution-replay`.
2. ✅ Add a CLI command `execution-context-integrity-audit` to run the gate on a date range.
3. ✅ Run the gate on the CN validation window (2024-01-01 to 2025-06-30).
4. ✅ Identify all placeholder/constant fields in `ExecutionMarketView`.
5. ✅ Trace the data lineage of each flagged field to the `ResearchContext` builder.
6. ✅ Fix the bridge between `ResearchContext` and `ExecutionMarketView` in `execution_replay.rs`.
7. ✅ Regenerate `ExecutionResearchRecord` for the validation window.
8. ✅ Re-run the gate to confirm all fields pass.
9. ✅ Resume 2B-2 Transition Evidence work with real data.

### Root cause and fix

The gate initially flagged all 8 fields as constant/placeholder. The root cause was not a broken `ResearchContext` builder but a missing bridge: `crates/app-service/src/execution_replay.rs` was constructing `ExecutionMarketView` from hardcoded placeholder values instead of using `ExecutionMarketView::from_research_context`.

Fix applied:
- `build_execution_event` now loads `ResearchContext` via `AppContext::build_research_context_for_date`.
- `ExecutionMarketView` is built via `ExecutionMarketView::from_research_context(&ctx)`.
- `ResearchContext` is cached per date in range-based record loading to avoid redundant fetches.

After the fix, all 8 fields pass the gate with non-zero variance and real distributions.

### Verification

Report: `reports/execution-validation/context_integrity_audit_cn_2026-07-18.md`

All 8 fields now show PASS status.

### Schema Version Implication

Because the polluted `ExecutionResearchRecord` stream cannot be used for Evidence validation, the Execution Platform may need a schema version bump to distinguish:

- `ExecutionEvent v2.1`: market_regime_label provenance restored
- `ExecutionEvent v2.2`: ResearchContext fact integrity restored (future)

This is not required immediately, but it should be considered when the upstream fix is complete. The current fix is in the replay record generation layer, not in the `ExecutionEvent` schema itself.

## Relationship to Other ADRs

- ADR-101: Transition Evidence Modeling (blocked until this gate passes; now unblocked)
- ADR-102: Rejected `RecoveryFailure` as Exit Transition Evidence (a valid research outcome, not affected by this data issue)
- ADR-103: Transition Evidence blocked by upstream data quality (the trigger for this gate; now resolved)

## References

- `docs/v8/adr-103-transition-evidence-blocked-by-data-quality.md`
- `research/validation/execution/README.md` TASK-156 section
- `reports/execution-validation/context_integrity_audit_cn_2026-07-18.md`
- `reports/execution-validation/bearish_analysis_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_recovery_failure_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_breadth_deterioration_cn_v2_2026-07-18.md`
- `reports/execution-validation/transition_analysis_leadership_decay_cn_v2_2026-07-18.md`

---

**Update log**
- 2026-07-18: ADR-104 accepted. Fact Integrity Gate introduced as Phase 2B-0.
- 2026-07-18: Gate initially failed all 8 fields; root cause traced to `execution_replay.rs`.
- 2026-07-18: Fix applied and verified; all fields pass. 2B-1/2B-2 re-run with real data.
