# REGIME-AUDIT KNOWLEDGE BASE

## OVERVIEW
Regime label quality validation and Ground Truth audit suite. 27 modules analyzing persistence, coverage, alignment, attribution, and dual-layer validation of market regime labels.

## STRUCTURE
```text
crates/regime-audit/src/
├── lib.rs                        # Episode extraction, PersistenceScore, CoverageScore, AuditGates
├── common.rs                     # Shared audit utilities: apply_persistence (centralized from 6 modules)
├── ground_truth_audit.rs         # TASK-035B: GT definition investigation
├── persistence_mechanics.rs      # TASK-034B: persistence frontier shape analysis
├── dual_layer_validation.rs      # Dual-layer regime validation logic
├── state_alignment.rs            # State-to-signal alignment metrics
├── factor_alignment.rs           # Factor-level regime alignment
├── economic_replay.rs            # Economic regime scenario replay
├── pareto_frontier.rs            # Multi-objective regime optimization
├── wave8_revalidation.rs         # Wave 8 regime revalidation
├── state_signal_decomposition.rs # Signal-to-state decomposition
├── attribution.rs                # Regime attribution analysis
├── external_validation.rs        # External benchmark validation
└── [other audit modules]         # allocation, episode survival, forward returns, lead-lag, etc.
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add audit gate thresholds | `lib.rs::AuditGates` | Default: min_avg 20d, max_churn 5%, min_stability 70% |
| Extract episodes from labels | `lib.rs::extract_episodes` | contiguous regime runs |
| Ground Truth definition audit | `ground_truth_audit.rs` | TASK-035B: classify_raw_regime + apply_persistence |
| Persistence mechanics | `persistence_mechanics.rs` | TASK-034B: Q1/Q2/Q3 frontier shape |
| Dual-layer validation | `dual_layer_validation.rs` | largest module; cross-layer consistency |
| State alignment scoring | `state_alignment.rs` | alignment between state and downstream signals |
| Factor alignment | `factor_alignment.rs` | per-factor regime alignment |
| Economic replay | `economic_replay.rs` | scenario-based regime replay |
| Pareto frontier | `pareto_frontier.rs` | multi-objective regime optimization |

## CONVENTIONS
- Audit modules are task-numbered (TASK-034B, TASK-035B) and should include task references in headers.
- `classify_raw_regime` is intentionally duplicated across audit modules for standalone audit reproducibility; `apply_persistence` is centralized in `common.rs`.
- Audit scores are pure computation; no persistence or HTTP calls in this crate.
- Episode-based metrics (persistence, coverage) are the primary quality gates.
- Ground Truth investigations use the same `RegimeLabel` types from `gt-regime-generator`.

## ANTI-PATTERNS
- Do **not** add ClickHouse/SQLite persistence here; this crate is pure analysis.
- Do **not** add HTTP provider calls here.
- Do **not** duplicate `classify_raw_regime` logic without task reference justification.
- Do **not** bypass `AuditGates` thresholds when evaluating regime quality.
- Do **not** assume old regime labels are compatible without checking `gt-regime-generator` schema changes.

## NOTES
- `lib.rs` is 962 lines; consider extracting episode logic if it grows.
- Many audit modules (persistence_mechanics, ground_truth_audit) share the same `classify_raw_regime` / `apply_persistence` pattern.
- Wave 8 revalidation and dual-layer validation are the most complex modules.
- This crate depends on `gt-regime-generator` and `core-domain`.
