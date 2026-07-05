# CORE-DOMAIN KNOWLEDGE BASE

## OVERVIEW
Canonical shared contracts crate. Owns enums and serializable snapshots reused across ingestion, engines, storage, reports, CLI, and desktop. Also owns pure research computation under `src/research/`.

## STRUCTURE
```text
crates/core-domain/src/
├── lib.rs              # shared DTOs, AnalysisScope, provenance fields
└── research/           # pure, reusable research computation (V6)
    ├── mod.rs
    ├── percentile.rs   # percentile_label, percentile helpers
    ├── analytics.rs    # conditional forward-return analytics
    └── stretch.rs      # market-stretch dimension scoring
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Scope contract | `src/lib.rs::AnalysisScope` | uppercase `GLOBAL/CN/HK` strings are part of downstream expectations |
| Universe model | `src/lib.rs::Instrument` | shared provider/display metadata |
| Core market series | `src/lib.rs::DailyBar` + `IndicatorSnapshot` | base time-series contracts |
| Macro/regime/environment rows | `src/lib.rs::MacroSnapshot`, `MarketRegimeSnapshot`, `EnvironmentSnapshot` | persisted analysis-layer contracts |
| Strategy/signal provenance | `src/lib.rs::StrategyPreferenceSnapshot` + `SignalSnapshot` | `analysis_scope` and `regime_basis_scope` must remain explicit |
| Pure research computation | `src/research/` | percentile, analytics, stretch helpers; no I/O |

## CONVENTIONS
- Keep this crate dependency-light: `chrono` + `serde` only unless a truly universal contract requires more.
- New shared DTOs and enums belong here before store/report/frontend invent copies.
- Prefer plain serializable data structures over helper-heavy abstractions.
- Any string form exposed here is effectively a storage/report contract.
- Research computation under `src/research/` must be pure: no DB, no HTTP, no filesystem.

## ANTI-PATTERNS
- Do **not** add I/O, DB mapping, or service orchestration here.
- Do **not** casually rename enum variants or scope strings without coordinated storage/report/frontend updates.
- Do **not** move engine-specific scoring behavior into contract types.
- Do **not** add consumer-specific fields to shared DTOs.

## NOTES
- Field or enum drift here breaks `market-store`, `report-engine`, CLI, and desktop together.
- Provenance fields on strategy/signal rows are part of the current product semantics; do not remove them as “redundant.”
- `core-domain::research` is the canonical home for reusable research math used by `app-service` V6 commands.
