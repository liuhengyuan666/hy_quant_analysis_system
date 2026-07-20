# ADR-105: Evidence Horizon and Role Model

**Status:** Accepted  
**Date:** 2026-07-18  
**Scope:** V8 Execution Platform, Evidence Layer  
**Decision Owner:** V8 Execution Platform Team

## Context

TASK-157 (LeadershipDecay Horizon Analysis) produced a clear result: the same `LeadershipDecay` signal has no predictive power at T+5/T+20 but strong predictive power at T+60 (lift 1.50, precision 61.5%) and weaker but still positive power at T+120.

This contradicts the implicit assumption in the current Execution Pipeline: that all Evidence operates at the same decision horizon and that the same `direction` score can be aggregated into a single `dominant_direction`.

The prior evidence landscape shows the same pattern:

- `RiskExpansion` appears to act as an immediate/short-term risk signal.
- `Distribution` also appears to act as a short-term risk signal.
- `LeadershipDecay` acts as a medium-term holding risk signal.
- Fixed state evidence (`Breadth`, `Confirmation`, `Recovery`, `LeadershipRotation`) currently has no discriminating power at any horizon because it is static and present in every record.

Continuing to treat all Evidence as a single-dimensional `direction` will either force the system to ignore horizon-specific signals or misapply them (e.g., using a medium-term holding risk signal to trigger an immediate Reduce).

## Decision

**Introduce a first-class Evidence Horizon and Evidence Role model for any Evidence that enters the Execution Pipeline.**

Before any new `ObservationKind` or `EvidenceKind` is promoted into the Decision path, it must declare:

1. **Natural Horizon** (`EvidenceHorizon`): the time scale at which the signal is predictive.
2. **Evidence Role** (`EvidenceRole`): the function the signal serves in the decision process.

This is a semantic contract, not a code change to the existing DecisionEngine. Existing Evidence without explicit horizon/role metadata remains in the legacy single-horizon path. New Evidence must carry the metadata from the start.

## Proposed Model

```rust
pub enum EvidenceHorizon {
    Immediate,   // T+1 ~ T+5
    ShortTerm,   // T+5 ~ T+20
    MediumTerm,  // T+20 ~ T+60
    LongTerm,    // T+60+
}

pub enum EvidenceRole {
    EntrySignal,    // Supports opening/increasing a position
    ExitSignal,     // Supports closing/decreasing a position on short-term timing
    HoldingRisk,    // Warns that current holdings face elevated risk over a horizon
    RegimeRisk,     // Warns that the underlying market regime may be shifting
    Confirmation,   // Confirms or contradicts other signals without being primary
}

pub struct EvidenceProfile {
    pub kind: EvidenceKind,
    pub horizon: EvidenceHorizon,
    pub role: EvidenceRole,
    pub confidence: f64,
    pub direction: f64, // -1.0 to +1.0
}
```

## Example Profiles

| Evidence | Natural Horizon | Role | Rationale |
|---|---|---|---|
| `RiskExpansion` | ShortTerm | HoldingRisk / ExitSignal | Captures immediate intraday volatility expansion; may support short-term risk reduction |
| `Distribution` | ShortTerm | HoldingRisk | Intraday distribution pattern; short-term exit/holding risk |
| `LeadershipDecay` | MediumTerm | HoldingRisk | Leadership stability deterioration predicts elevated medium-term holding risk |
| `BreadthDeterioration` | MediumTerm | HoldingRisk | Market breadth deterioration predicts medium-term holding risk |
| `LiquidityDeterioration` | MediumTerm | HoldingRisk | Volume/liquidity drying up predicts medium-term holding risk |
| `MarketAcceptance` | ShortTerm | Confirmation | Price acceptance confirms trend; not a standalone exit signal |
| `MomentumExpansion` | ShortTerm | EntrySignal / Confirmation | Momentum expansion supports entry timing |

## Consequences

### Accepted

- No Evidence will be promoted into the Decision path without a declared `EvidenceHorizon` and `EvidenceRole`.
- Research-only tools (e.g., `execution-transition-analysis`, `execution-leadership-decay-horizon`) must compute and report performance at the natural horizon of the candidate, not only at T+20.
- The `AssessmentEngine` will eventually aggregate Evidence by `EvidenceRole` rather than flattening all Evidence into a single `dominant_direction`. This is a future architectural change; the ADR only establishes the contract now.
- Holding Risk Evidence is distinct from Exit Signal Evidence. The system should move toward a `Holding Risk Score` model rather than single-Evidence Reduce triggers.

### Rejected / Out of Scope

- This ADR does **not** require retrofitting all existing Evidence with horizon/role metadata immediately. Legacy Evidence continues to use the existing single-horizon aggregation path.
- This ADR does **not** change `DecisionEngine`, `ObservationEngine`, `EvidenceBuilder`, or `ExecutionPolicy` defaults.
- This ADR does **not** introduce a new `EvidenceKind` for LeadershipDecay or any other candidate.

## Relationship to Other ADRs

- ADR-100: Evidence Quality Before Decision Calibration — horizon/role is a dimension of evidence quality.
- ADR-101: Transition Evidence Modeling — transition evidence candidates must be evaluated at their natural horizon.
- ADR-104: ResearchContext Fact Integrity Gate — evidence is only meaningful if the underlying data is real; both gates must pass before promotion.
- ADR-105: Evidence Horizon and Role Model (this ADR) — evidence must declare its semantic function before calibration.

## Required Validation Before Promotion

For a candidate to be promoted from Research Asset to `ObservationKind` / `EvidenceKind`, the following must be true:

1. Fact Integrity Gate passes (ADR-104).
2. Natural horizon is identified and the candidate shows lift ≥ 1.2 and precision ≥ 50% at that horizon.
3. `EvidenceRole` is assigned and justified.
4. The candidate does not rely on a single constant/placeholder input.

## Next Steps

1. Apply this model to `LeadershipDecay` (MediumTerm, HoldingRisk).
2. Apply this model to `BreadthDeterioration` once its natural horizon is confirmed.
3. Design TASK-158: Holding Risk Evidence Bundle — combining multiple medium-term holding risk signals into a single score.
4. Implement TASK-159: Context Integrity CI Gate — prevent future fact-lineage failures.

## References

- `research/validation/execution/README.md` TASK-157 section
- `reports/execution-validation/leadership_decay_horizon_cn_2026-07-18.md`
- `docs/v8/adr-101-transition-evidence-modeling.md`
- `docs/v8/adr-104-researchcontext-fact-integrity-gate.md`
