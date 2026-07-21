# CRATES KNOWLEDGE BASE

## OVERVIEW
Rust workspace implementation layer. Contracts, pure compute, persistence, orchestration, report shaping, and presentation models all live here.

## STRUCTURE
```text
crates/
├── core-domain/        # shared models + enums + research::* pure computation
├── data-ingestion/     # external market/macro fetch + canonical adjustment rules
├── indicator-engine/   # technical indicators
├── macro-engine/       # macro snapshots + per-scope regime scoring
├── rotation-engine/    # RS / momentum ranking
├── strategy-engine/    # four-strategy scoring
├── signal-engine/      # final action labels
├── backtest-engine/    # signal-driven simulation
├── report-engine/      # DashboardSnapshot + legacy daily-report markdown rendering
├── reporting/          # V6 presentation model: ReportingSnapshot, ReportDocument, Formatter
├── report-builder/     # V6 document assembly: ReportInput structs + Research/Review/Audit builders
├── market-store/       # ClickHouse + SQLite IO
├── app-service/        # orchestration facade, trust assembly, refresh guards, V6/V7/V8 research commands
├── gt-regime-generator/     # Ground Truth four-layer regime generation (Observation→Candidate→Persistence→Label)
├── market-state-extractor/  # semantic market-state observation layer (ADR-053): Trend/Volatility/Liquidity
├── market-fingerprint-engine/ # V7.2B historical evidence similarity matching
├── regime-audit/           # regime label quality validation: persistence, coverage, alignment, dual-layer validation
├── research-context/   # canonical ResearchContext + TrustLevel semantic contract (V6)
├── llm-context/        # LLM-specific 8-dimension ResearchContext builder from DashboardSnapshot
├── research-skills/    # LLM research actions (V4.5: 5 prompts only)
├── research-renderer/  # insight/research rendering
├── research-benchmark/ # skill benchmarking harness
├── research-validation/# Ground Truth validation
├── execution-engine/   # pre-close execution filter
└── task-runner/        # placeholder utility crate
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add / change shared DTOs | `core-domain/` | type first; child AGENTS file covers contract traps |
| Add reusable research computation | `core-domain/src/research/` | percentile, analytics, stretch helpers; no I/O |
| Add new factor/provider | `data-ingestion/` + engine crate | fetch outside, compute inside |
| Change pure macro scoring | `macro-engine/` | regime math only; no fetch/persist |
| Add persistence | `market-store/` | split into 14 domain modules (`core`, `bars`, `signals`, `regime`, `environment`, `rotation`, `strategy`, `indicators`, `macro`, `backtest`, `reports`, `dates`, `instruments`, `sqlite`); fetch/insert pair, enum/string mapping, scoped date helpers |
| Change trust / dashboard bootstrap / recent reports | `app-service/` + `report-engine/` | `app-service` assembles (orchestration monolith with helper modules: `core`, `trust`, `breadth`, `dashboard`, `llm`, `sync`, `config_loader`, `research_evidence`, `workspace`); `report-engine` shapes |
| Add pipeline step | `app-service/` | expose summary DTO + CLI/Tauri hook; add helper to appropriate module (trust, breadth, dashboard, workspace) |
| V6 Research Surface | `app-service/` + `core-domain::research` + `report-builder` | `research_srd`, `research_stretch`, `research_review`; `ReportInput` structs carry document-specific data |
| V7 Research Surface | `app-service/` + `core-domain::research` + `market-fingerprint-engine` | `research confirmation`, `research recovery`, `research analogues`, `research calibration`, `research consensus` |
| V8 Research Asset | `app-service/src/workspace.rs` | `WorkspaceManager`, `ResearchAssetId`, lifecycle, evidence/snapshot writers and registry indexes |
| Ground Truth regime generation | `gt-regime-generator/` | 4-layer pipeline: Observation → Candidate → Persistence → Regime Label |
| Market state observations | `market-state-extractor/` | semantic OHLCV + indicator → Trend/Volatility/Liquidity dimensions |
| Historical evidence matching | `market-fingerprint-engine/` | V7.2B similarity matching; consumed by `research analogues` |
| Regime quality validation | `regime-audit/` | persistence, coverage, alignment, dual-layer validation; pure analysis; 26 sub-modules |
| Change LLM research actions | `research-skills/src/action.rs` | 5 const prompts + build_prompt |
| Canonical ResearchContext | `research-context/` | consumer-neutral semantic model; used by `reporting`, `report-builder`, `app-service` |
| LLM-specific context building | `llm-context/` | 8-dimension ResearchContext built from DashboardSnapshot |
| Research rendering | `research-renderer/` | `DashboardInsightComposer`, `DailyReportComposer` |
| Ground Truth validation | `research-validation/` | label generator, labeler, validator, reports |
| Pre-close execution filter | `execution-engine/` | V5 pattern-library execution filter |
| Benchmark research skills | `research-benchmark/` | WIP: Wave 3 harness |
| Presentation model | `reporting/` | frozen V6: ReportingSnapshot, ReportDocument, Formatter |
| Document assembly | `report-builder/` | frozen V6: builders + ReportInput structs |

## CONVENTIONS
- `core-domain` owns serializable contracts; other crates depend inward on it.
- `core-domain::research` owns pure, reusable research computation.
- `research-context` owns the canonical `ResearchContext` / `TrustLevel` semantic contract.
- `reporting` owns the presentation model (`ReportingSnapshot`, `ReportDocument`, `Formatter`); no business logic.
- `report-builder` owns document-specific inputs and builders; it consumes `ResearchContext` from `research-context` and emits `ReportDocument` from `reporting`.
- Engine crates should stay pure whenever possible; `macro-engine` especially must not fetch or persist.
- `market-store` owns all SQL, ClickHouse/SQLite IO, and date-gating helpers.
- `report-engine` shapes dashboard/report payloads; `app-service` injects trust summary, pipeline diagnostics, and recent reports.
- Signal alignment and refresh consistency use diagnostics alerts as the single source of truth.
- Dashboard/report are scope-aware; strategy/signal/backtest computations are also scope-aware with provenance fields (analysis_scope, regime_basis_scope, signal_scope) in the data contract.
- V6 Reporting Platform is frozen; add new consumers on top of it, do not refactor the platform itself.
- V7 Research Platform (Observation → Evolution → Historical Evidence → Synthesis) is frozen; add new Research Content, not new platform semantics.
- V8 Research Asset workspace uses `RA-XXXXXX` identity and a unified lifecycle; do not design Evidence Score/Weight (P3) before 1000+ assets, 30-day replay stability, and 2-cycle calibration stability.
- Rust modules use flat named files (`foo.rs`) under `src/`; the only `mod.rs` in the project is `crates/core-domain/src/research/mod.rs` (legacy holdout). New modules must not add `mod.rs` directories.
- All crates use `anyhow::Result`; there are no shared `thiserror` enums. Lint suppression is exceptional; rely on Rust defaults.

## ANTI-PATTERNS
- Do **not** fetch HTTP data from `strategy-engine`, `signal-engine`, `backtest-engine`, or `macro-engine`.
- Do **not** duplicate snapshot DTOs outside `core-domain`.
- Do **not** add more persistence code to `app-service`.
- Do **not** let `market-store` absorb domain scoring logic.
- Do **not** put orchestration or trust assembly into `report-engine`.
- Do **not** expose `ResearchDataset` / `ResearchSnapshot` from `app-service`.
- Do **not** add metadata (scope/date/generated_at) to `ReportInput` structs.
- Do **not** add artificial `ReportBuilder` trait implementations.
- Do **not** confuse `research-context` (canonical semantic model) with `llm-context` (LLM prompt context).
- Do **not** add new `mod.rs` directories; use flat named files (`foo.rs`) under `src/` for new modules.

## HOTSPOTS
- `market-store/src/` is now split into 14 domain modules (was a 2,678-line god-module). All pub use re-exports preserve backward compatibility.
- `app-service/src/lib.rs` is still the orchestration monolith (~5,567 lines), but helper logic has been extracted into `core.rs`, `trust.rs`, `breadth.rs`, `dashboard.rs`, `llm.rs`, `sync.rs`, `config_loader.rs`, `research_evidence.rs`, `workspace.rs`. Review nearby helper modules before adding more flow logic.
- `data-ingestion/src/lib.rs` owns source semantics such as forward-adjustment and provider validation.
- `report-engine/src/lib.rs` is a shared payload contract; field drift breaks CLI, Tauri, and frontend together.
- `regime-audit/src/lib.rs` has been decomposed into 26 sub-modules; use it as the model for future crate splitting.
- `report-builder/src/lib.rs` is intentionally frozen; new document types need ADR amendment.
- `market-state-extractor/src/lib.rs` and `gt-regime-generator/src/lib.rs` are each ~600+ lines with no sub-modules; consider decomposition if they grow.

## NOTES
- `task-runner` is still placeholder-level.
- Tests are sparse and local; practical validation is cargo check/test plus live CLI/report/desktop flows.
