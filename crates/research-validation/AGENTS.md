# RESEARCH-VALIDATION KNOWLEDGE BASE

## OVERVIEW
Ground Truth regime validation utilities. Builds labeled regimes from forward-looking returns, generates candidate labels from `RegimeObservation` rows, and validates predictions against ground truth.

## STRUCTURE
```text
crates/research-validation/src/
├── lib.rs              # re-exports
├── label_generator.rs  # RegimeLabelGenerator + PersistenceFilter + ObservationSequenceBuilder
├── labeler.rs          # forward-return ground-truth labeler
├── report.rs           # accuracy report generators
└── validator.rs        # HistoricalValidator
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Candidate labels from observations | `src/label_generator.rs::RegimeLabelGenerator` | threshold-based RiskOn/RiskOff/Neutral |
| Stable labels via persistence | `src/label_generator.rs::PersistenceFilter` | min_days + confirmation_days |
| Ground-truth labels from prices | `src/labeler.rs::RegimeLabeler` | 20-day forward return thresholds |
| Prediction validation | `src/validator.rs::HistoricalValidator` | TP/FP/FN metrics per regime |
| Accuracy reports | `src/report.rs` | `AccuracyReport`, `RegimeReport`, `ReportGenerator` |

## CONVENTIONS
- Keep this crate pure: no fetch, no persistence, no orchestration.
- `RegimeLabelGenerator` is independent from `macro-engine` scoring.
- `RegimeLabeler` uses forward returns (default 20 days) to produce ground-truth labels.
- `HistoricalValidator` matches predictions by `(date, symbol)` key.

## ANTI-PATTERNS
- Do **not** add storage or HTTP concerns here.
- Do **not** use `RegimeLabelGenerator` labels as trading signals without persistence filtering.
- Do **not** change default lookforward/threshold without updating audit and ADR records.

## NOTES
- `ObservationSequenceBuilder` constructs `RegimeObservation` rows from stored `MarketRegimeSnapshot` + `EnvironmentSnapshot`.
- Tests cover risk-on/risk-off/neutral forward-return labeling.
