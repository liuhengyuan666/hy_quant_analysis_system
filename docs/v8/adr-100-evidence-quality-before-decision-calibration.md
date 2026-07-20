# ADR-100: Evidence Quality Before Decision Calibration

**Status:** Accepted  
**Date:** 2026-07-18  
**Scope:** V8 Execution Platform, Phase 2B  
**Decision Owner:** V8 Execution Platform Team

## Context

Phase 2A (Fact Lineage → Statistics → Evidence Trace → Decision Path Review → Decision Gate Analysis → Risk Semantics Review → Directional Confidence Calibration → Real Volume Context) has isolated the Reduce=0 bottleneck.

The bottleneck is not in the Decision Gate, the confidence threshold, or the Risk Semantics. The Decision Engine is correctly behaving as a risk controller: when bearish evidence exists but its confidence or distinguishing power is insufficient, the safest output is `Wait`.

The real issue is upstream: the bearish Evidence layer does not produce enough **Exit-specific** signals. Current bearish evidences (`Distribution`, `RiskExpansion`, `MomentumFailure`) primarily detect **risk presence**, but they cannot reliably distinguish:

- **Risk that should lead to exit** (true distribution, breadth collapse, leadership loss, no recovery)
- **Risk that resolves into recovery** (panic release, temporary selloff, strong underlying breadth)

Because both cases feed into the same bearish Assessment direction, lowering the Decision confidence threshold only releases a mixed set of Reduce actions with low precision. This was empirically confirmed by 2A-5 and 2A-6.

## Decision

**Formalize the principle: Decision threshold calibration shall only happen after evidence quality has been validated.**

- No `ExecutionPolicy` threshold change or `DecisionEngine` modification may be justified solely by "Decision Gate blocking too many candidates."
- Before any calibration proposal, the evidence layer must demonstrate that the bearish signal set can distinguish exit-requiring states from temporary-risk states.
- Evidence quality is measured by replay outcome, not by coverage or frequency alone.
- Any new evidence kind or condition must be developed as a Research Asset first, validated against historical outcomes, before being wired into the Decision path.

## Rationale

1. **Thresholds cannot compensate for weak semantics.** A Decision Engine cannot reliably reduce a position if the only information it receives is "risk exists." It needs "this risk is likely to persist / worsen."
2. **Phase 2A proved the chain is clean below the Evidence layer.** Fact Lineage, Feature extraction, Observation, Assessment, and Risk Semantics all behave as intended. The only remaining gap is the semantic content of the evidence set.
3. **Calibration on weak evidence produces false positives.** The 2A-5 Calibration Experiment showed that even aggressive thresholds (0.45) produce only ~36% precision for Reduce. This is below the 50% minimum acceptable bar.
4. **Evidence Modeling is a distinct research phase.** It requires hypothesis design, historical replay, and outcome-based validation. It cannot be collapsed into a threshold tuning exercise.

## Consequences

### Expected

- The next phase (2B) focuses on designing and validating bearish evidence that is **exit-specific**, not just risk-detecting.
- Candidate evidence dimensions include: breadth deterioration, leadership loss, recovery failure, and multi-day regime breakdown.
- Each candidate will be evaluated as a Research Asset before any integration into `ExecutionPolicy` or `DecisionEngine`.
- No threshold or policy change will be proposed until Reduce precision reaches ≥50% in replay.

### Accepted

- This principle delays threshold calibration. That delay is intentional and required to avoid introducing false Reduce signals.
- New evidence kinds may require extensions to the `EvidenceKind` enum or new Observation conditions. Such changes will be treated as research artifacts, not quick fixes.

## Verification

- Any future calibration proposal must include:
  - The specific evidence conditions that distinguish exit-requiring states.
  - Historical replay precision/recall metrics on an out-of-sample or held-back period.
  - A comparison against the current baseline (no Reduce).
- `ExecutionPolicy` defaults remain unchanged until the above criteria are met.

## References

- ADR-099: Restore Real Volume Context Before Evidence Calibration
- ADR-098: Directional Confidence Calibration Rejected Threshold-Only Approach
- ADR-097: Risk Semantics Review
- ADR-096: Decision Gate Analysis
- ADR-095: Decision Path Review
- `research/validation/execution/README.md`
- `crates/execution-replay/src/v2/evidence.rs`
