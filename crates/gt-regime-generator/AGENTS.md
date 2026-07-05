# GT-REGIME-GENERATOR KNOWLEDGE BASE

## OVERVIEW
Ground Truth four-layer regime generator. Converts `MarketStateObservation` rows into stable `RegimeLabel` rows through Candidate → Persistence filtering.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Candidate generation | `src/lib.rs::CandidateGenerator` | RiskOn/RiskOff/Neutral candidate from a single observation |
| Persistence filter | `src/lib.rs::PersistenceFilter` | state machine enforcing min_days and confirmation_days |
| Full pipeline | `src/lib.rs::RegimePipeline` | Observation → Candidate → Persistence → RegimeLabel |
| Batch convenience | `src/lib.rs::generate_regime_labels` | run pipeline over a sequence |

## CONVENTIONS
- Keep this crate pure: no fetch, no persistence, no AppContext orchestration.
- `CandidateConfig::for_scope(scope)` is the hook for per-scope tuning; defaults are shared across scopes today.
- `PersistenceConfig` defaults are data-driven (ADR-058: confirmation_days=1).
- Regime transitions require both `min_days` in current regime and `confirmation_days` in candidate.

## ANTI-PATTERNS
- Do **not** add storage or HTTP concerns here.
- Do **not** bypass the persistence filter when generating labels for downstream consumption.
- Do **not** change default thresholds without updating `regime-audit` expectations and ADR records.
- Do **not** use the deprecated `generate_with_variant`; prefer `generate_with_config`.

## NOTES
- Consumes `MarketStateObservation` from `market-state-extractor`.
- `RegimeLabel` carries both the stable `regime` and the raw `candidate` for audit.
- Tests cover candidate scoring, persistence switching, and full pipeline behavior.
