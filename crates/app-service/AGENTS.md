# APP-SERVICE KNOWLEDGE BASE

## OVERVIEW
Orchestration facade for the entire analysis chain. Highest-coupling crate in the workspace.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Public orchestration surface | `src/lib.rs::AppContext` | all major workflows exposed here |
| Refresh planning | `build_refresh_plan` | incremental refresh window semantics |
| Dashboard/report loading | `dashboard_snapshot`, `dashboard_bundle` | hot path; keep scoped and measurable |
| Scoped dashboard/report loading | `dashboard_snapshot_with_scope`, `dashboard_bundle_with_scope`, `export_report_with_scope` | global vs CN vs HK |
| Stage freshness diagnostics | `pipeline_date_diagnostics` | compare stored stage max dates quickly |
| Breadth proxy | `compute_watchlist_breadth_snapshot` | V1 watchlist breadth logic |
| Health checks | `check_data_health` | provider/gap/jump diagnostics |
| Report export | `export_report`, `export_data_health_report` | filesystem artifact boundary |

## CONVENTIONS
- Orchestrate only: fetch through `market-store`, compute through engine crates, render through `report-engine`.
- Every public workflow returns a summary DTO or snapshot rather than leaking internal structs casually.
- Dashboard hot path must stay on scoped reads; timing instrumentation now lives in `load_metrics`.
- `ReportScope` is a thin alias over shared `AnalysisScope`; `Global`, `Cn`, and `Hk` must stay behaviorally distinct.
- Macro regime refreshes must fetch lookback history before filtering to the requested write window, otherwise forward-fill semantics break.
- Breadth proxy is derived from persisted bars + MA30 only; no provider calls here.

## ANTI-PATTERNS
- Do **not** add raw SQL or HTTP provider logic here.
- Do **not** duplicate formulas from engine crates.
- Do **not** reintroduce per-symbol full-history fetch loops on dashboard/report paths.
- Do **not** narrow macro fetches to the visible write range when regime construction depends on older FRED history.
- Do **not** let scoped exports silently fall back to global available-date selection.
- Do **not** grow `AppContext` blindly; if a new vertical becomes large, extract a focused service/module.

## NOTES
- `src/lib.rs` is 4,083 lines of AppContext orchestration; 7 helper modules (core, trust, breadth, dashboard, llm, sync, config_loader) have been extracted. Prefer further extraction before adding more long methods.
- Timing data on `DashboardSnapshot.load_metrics` is now the first place to check when dashboard performance regresses.
- `export_data_health_report` now uses the freshest checked market date for artifact naming; keep that aligned with health semantics.
- Scoped reports now read scoped regime + scoped environment. Strategy/signal/backtest computations are also scope-aware, with explicit provenance fields (analysis_scope, regime_basis_scope) in the data contract.
