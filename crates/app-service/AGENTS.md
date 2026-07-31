# APP-SERVICE KNOWLEDGE BASE

## OVERVIEW
Orchestration facade for the entire analysis chain. Highest-coupling crate in the workspace. Owns `AppContext`, all public workflows, trust assembly, and the V6 Research Surface commands.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Public orchestration surface | `src/lib.rs::AppContext` | all major workflows exposed here (~5,567 lines) |
| Helper modules | `src/{core,trust,breadth,dashboard,llm,sync,config_loader,research_evidence,workspace}.rs` | extracted logic; prefer adding here over growing `lib.rs` |
| Refresh planning | `build_refresh_plan` | incremental refresh window semantics |
| Dashboard/report loading | `dashboard_snapshot`, `dashboard_bundle` | hot path; keep scoped and measurable |
| Scoped dashboard/report loading | `dashboard_snapshot_with_scope`, `dashboard_bundle_with_scope`, `export_report_with_scope` | global vs CN vs HK |
| Stage freshness diagnostics | `pipeline_date_diagnostics` | compare stored stage max dates quickly |
| Breadth proxy | `compute_watchlist_breadth_snapshot` | V1 watchlist breadth logic |
| Health checks | `check_data_health` | provider/gap/jump diagnostics |
| Report export | `export_report`, `export_data_health_report` | filesystem artifact boundary |
| V6 Research Surface | `research_srd`, `research_stretch`, `research_review` | read-only observation tools; output `reports/research-*.md` |
| V7 Research Surface | `research_confirmation`, `research_recovery`, `research_analogues`, `research_calibration`, `research_consensus` | Observation → Evolution → Historical Evidence → Synthesis |
| V8 Research Asset | `research_analytics`, `research_replay`, `workspace.rs` | Evidence/Snapshot writers, registry indexes, `RA-XXXXXX` lifecycle |
| 共享博弈假设背景（ADR-112） | `lib.rs::ensure_adversarial_context` + `llm_history::adversarial_context_section` | 每 scope 每日一次前置博弈分析，按 persona 分级注入；失败静默降级 |
| Internal data workspace | `ResearchDataset` / `ResearchSnapshot` | ephemeral; must not leak outside this crate |

## CONVENTIONS
- Orchestrate only: fetch through `market-store`, compute through engine crates, render through `report-engine` / `report-builder`.
- Every public workflow returns a summary DTO or snapshot rather than leaking internal structs casually.
- Dashboard hot path must stay on scoped reads; timing instrumentation now lives in `load_metrics`.
- `ReportScope` is a thin alias over shared `AnalysisScope`; `Global`, `Cn`, and `Hk` must stay behaviorally distinct.
- Macro regime refreshes must fetch lookback history before filtering to the requested write window, otherwise forward-fill semantics break.
- Breadth proxy is derived from persisted bars + MA30 only; no provider calls here.
- V6 research commands compute document-specific inputs in `app-service` and pass them to `report-builder`; `ResearchContext` comes from `research-context`.
- V7/V8 research commands orchestrate evidence and snapshots via `core-domain::research`, `market-fingerprint-engine`, and `workspace.rs`.
- `ResearchDataset` and `ResearchSnapshot` are internal-only transient workspaces.

## ANTI-PATTERNS
- Do **not** add raw SQL or HTTP provider logic here.
- Do **not** duplicate formulas from engine crates.
- Do **not** reintroduce per-symbol full-history fetch loops on dashboard/report paths.
- Do **not** narrow macro fetches to the visible write range when regime construction depends on older FRED history.
- Do **not** let scoped exports silently fall back to global available-date selection.
- Do **not** grow `AppContext` blindly; if a new vertical becomes large, extract a focused service/module.
- Do **not** expose `ResearchDataset` or `ResearchSnapshot` in public signatures.
- Do **not** move document assembly logic into `app-service`; keep it in `report-builder`.

## NOTES
- `src/lib.rs` is ~5,900 lines of `AppContext` orchestration; 14 helper modules have been extracted (`core`, `trust`, `breadth`, `dashboard`, `llm`, `llm_history`, `prompts`, `scenarios`, `strategy_perspectives`, `sync`, `config_loader`, `research_evidence`, `execution_replay`, `workspace`). Prefer further extraction before adding more long methods.
- Timing data on `DashboardSnapshot.load_metrics` is the first place to check when dashboard performance regresses.
- `export_data_health_report` now uses the freshest checked market date for artifact naming; keep that aligned with health semantics.
- Scoped reports now read scoped regime + scoped environment. Strategy/signal/backtest computations are also scope-aware, with explicit provenance fields (analysis_scope, regime_basis_scope) in the data contract.
