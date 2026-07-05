# PROJECT KNOWLEDGE BASE

**Generated:** 2026-07-06
**Commit:** 1136c05
**Branch:** v6

## OVERVIEW
Local desktop quant research system. The Rust workspace owns the ingestion-to-report pipeline, the Tauri desktop app is the default operator surface, and `docs/` plus `memory/` are part of the working system rather than side notes.

## ARCHITECTURE OWNERSHIP
```text
数据所有权（Data Ownership）
    ResearchDataset              ← app-service 内部，ephemeral，不暴露
    ResearchSnapshot             ← app-service 内部，computation workspace

语义所有权（Semantic Ownership）
    ResearchContext              ← 跨消费者共享的 canonical semantic contract (crates/research-context)

展示所有权（Presentation Ownership）
    ReportingSnapshot            ← 展示层 metadata + ResearchContext (crates/reporting)
    ReportInput                  ← 文档专属输入，document generation workflow 独占 (crates/report-builder)
    ReportBuilder                ← 文档组装占位 trait (crates/report-builder)
    ReportDocument               ← 渲染前文档模型 (crates/reporting)

渲染所有权（Rendering Ownership）
    Formatter                    ← Markdown / Text / JSON 渲染，无业务计算 (crates/reporting)

消费者（Consumers）
    CLI / Desktop / API / GPT / Email / PDF
```

核心规则：
- `ResearchContext` ≠ 万能 DTO，不承载 consumer-specific 字段。
- `ReportInput` 只承载 document payload，不重复 metadata（scope/date/generated_at）。
- 所有可复用的研究计算位于 `core-domain::research`。
- `ResearchDataset` 永不暴露到 `app-service` 边界之外。

## STRUCTURE
```text
rust-quant-analysis-system/
├── apps/          # CLI + desktop delivery surfaces
│   ├── cli/                     # clap-based CLI over AppContext
│   └── desktop/                 # Tauri app + Vite/Vue frontend
│       ├── frontend/            # Vite bundle, plain JS + Vue 3
│       └── src-tauri/           # Tauri native bridge
├── crates/        # contracts, engines, persistence, orchestration
│   ├── app-service/             # orchestration (lib.rs ~4,890 lines + 7 helper modules: core, trust, breadth, dashboard, llm, sync, config_loader)
│   ├── core-domain/             # shared DTOs, AnalysisScope, provenance, research::* pure computation
│   ├── market-store/            # ALL ClickHouse / SQLite IO
│   ├── macro-engine/            # factor normalization, regime rows
│   ├── report-engine/           # DashboardSnapshot, TrustSummary, markdown (legacy daily report rendering)
│   ├── report-builder/          # document-specific ReportInput + Research/Review/Audit builders (V6)
│   ├── reporting/               # presentation model: ReportingSnapshot, ReportDocument, Formatter trait (V6)
│   ├── data-ingestion/          # Eastmoney / Tencent fetchers
│   ├── indicator-engine/        # MA/EMA/MACD/RSI/ATR/VOL_MA
│   ├── rotation-engine/         # relative strength ranking
│   ├── signal-engine/           # final signal generation
│   ├── strategy-engine/         # strategy preference scoring
│   ├── backtest-engine/         # backtest execution
│   ├── gt-regime-generator/     # Ground Truth regime generation
│   ├── market-state-extractor/  # MarketStateObservation extraction
│   ├── regime-audit/            # regime alignment / sensitivity audits
│   ├── research-skills/         # LLM research actions (V4.5: 5 prompts only)
│   ├── research-context/        # canonical ResearchContext + TrustLevel semantic contract (V6)
│   ├── llm-context/             # LLM-specific 8-dimension ResearchContext builder from DashboardSnapshot
│   ├── research-renderer/       # insight/research rendering
│   ├── research-benchmark/      # research benchmarking
│   ├── research-validation/     # Ground Truth validation
│   └── task-runner/             # task execution framework
├── config/        # instrument universe + runtime inputs (llm.toml here)
├── docs/          # truth sources, operator guides, active design
│   ├── v2/        # V2 Phase 1 environment layer docs
│   ├── v3/        # V3 scoped reporting / code review docs
│   ├── v5/        # V5 execution layer, Shadow Production, State/Economic ADRs
│   └── v6/        # V6 reporting platform design docs and contracts
├── config/        # instrument universe + runtime inputs (llm.toml here)
├── infra/         # Docker / ClickHouse bootstrap
├── memory/        # durable project context, decisions, history
├── reports/       # exported artifacts; desktop open-file support stays here only
├── runtime/       # agent workflow and memory rules
└── sql/           # storage bootstrap DDL
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Agent rules + memory workflow | `memory/context.md` + `memory/decisions.md` | highest-priority collaboration rules |
| Current repo truth source | `README.md` + `docs/文档状态说明.md` | start here before old planning docs |
| Architecture invariants | `docs/architecture-invariants.md` | ADR-069; 10 non-negotiable V6 rules |
| Shadow Production playbook | `docs/shadow-production-playbook.md` | operational guidance for 90-day observation |
| Orchestration / trust / freshness guards | `crates/app-service/src/` | `AppContext` (in `lib.rs`), `build_trust_summary` (in `trust.rs`), `dashboard_bundle_with_scope`, `refresh_consistency_alerts` |
| V6 research orchestration | `crates/app-service/src/lib.rs` | `research_srd`, `research_stretch`, `research_review` |
| Shared contracts | `crates/core-domain/src/lib.rs` | `AnalysisScope`, shared snapshot DTOs, provenance fields |
| Pure research computation | `crates/core-domain/src/research/` | percentile, analytics, stretch helpers |
| Canonical semantic model | `crates/research-context/src/lib.rs` | `ResearchContext`, `TrustLevel` |
| Presentation model | `crates/reporting/src/lib.rs` | `ReportingSnapshot`, `ReportDocument`, `Formatter` |
| Document assembly | `crates/report-builder/src/lib.rs` | `ResearchReportBuilder`, `ReviewReportBuilder`, `AuditReportBuilder`, `*ReportInput` |
| Persistence boundary | `crates/market-store/src/` | 14 domain modules (`core`, `bars`, `signals`, `regime`, `environment`, `rotation`, `strategy`, `indicators`, `macro`, `backtest`, `reports`, `dates`, `instruments`, `sqlite`) via `pub use` re-exports; all ClickHouse / SQLite IO and latest-date gating |
| Pure macro regime logic | `crates/macro-engine/src/lib.rs` | factor normalization + `GLOBAL/CN/HK` regime rows |
| Report contract | `crates/report-engine/src/lib.rs` | `DashboardSnapshot`, `TrustSummary`, markdown report rendering |
| Ground Truth regime generation | `crates/gt-regime-generator/src/lib.rs` | 4-layer pipeline: Observation → Candidate → Persistence → Label |
| Market-state observations | `crates/market-state-extractor/src/lib.rs` | Trend/Volatility/Liquidity dimensions (ADR-053) |
| Pre-close execution filter | `crates/execution-engine/src/lib.rs` | V5 pattern-library execution filter |
| Research rendering | `crates/research-renderer/src/lib.rs` | insight/research rendering |
| GT validation | `crates/research-validation/src/lib.rs` | Ground Truth label generation/validation |
| LLM research actions | `crates/research-skills/src/action.rs` | 5 const prompts + `build_prompt()` |
| Desktop shell / refresh bridge | `apps/desktop/src-tauri/src/lib.rs` | command boundary, refresh coordinator, safe artifact opening |
| Frontend composition | `apps/desktop/frontend/src/main.js` | root state, scope/date flow, refresh UI, top-level render |
| Frontend feature slices | `apps/desktop/frontend/src/components/*.vue` | 20+ Vue panels |
| Environment + breadth UI | `apps/desktop/frontend/src/components/EnvironmentPanel.vue` + `BreadthPanel.vue` | paired explanation layer + proxy view |
| CLI surface | `apps/cli/src/main.rs` + `commands/` | thin dispatch over `AppContext` + 10 command modules (`config`, `pipeline`, `diagnostics`, `dashboard`, `backtest`, `research`, `llm`, `audit`, `execution`) |
| Current phase memory | `docs/阶段性更新.md` + `memory/context.md` + `memory/decisions.md` | latest intent, decisions, and next seam |

## DEVELOPER COMMANDS

### Prerequisites
- Docker Desktop with ClickHouse: `docker compose -f infra/docker/docker-compose.yml up -d`
  - ClickHouse ports: `127.0.0.1:18123` (HTTP), `19000` (native)
  - Default credentials: `quant_user` / `quant_pass`, database `quant`
- Frontend build artifacts must exist before Tauri build: `apps/desktop/frontend/dist/`
- Windows stack size increased via `.cargo/config.toml` (`/STACK:8388608`) — do not remove

### First-time setup
```bash
docker compose -f infra/docker/docker-compose.yml up -d
cargo run -p quant-cli -- init-storage
cargo run -p quant-cli -- seed-universe
```

### Rust workspace
```bash
# Run a specific crate binary
cargo run -p quant-cli -- <subcommand>
cargo run -p quant-desktop

# Check / test (sparse test coverage; no CI)
cargo check
cargo test -p <crate>

# Build desktop (frontend dist MUST exist first)
cargo build -p quant-desktop
```

### Frontend (Vue 3 + Vite)
```bash
cd apps/desktop/frontend
npm install
npm run build        # outputs to dist/, consumed by Tauri
npm run dev          # Vite dev server
```

### Recommended CLI paths
```bash
# V3 one-shot: check gate → refresh if needed → export report
cargo run -p quant-cli -- sync-and-export --scope global

# Full manual refresh (engineering / advanced path)
cargo run -p quant-cli -- refresh-all --to 2026-06-05

# Quiet mode: global flag, place BEFORE subcommand
cargo run -p quant-cli -- --quiet sync-and-export --scope global
cargo run -p quant-cli -- --quiet refresh-all --to 2026-06-05

# Pipeline diagnostics (first thing to run when freshness looks off)
cargo run -p quant-cli -- pipeline-dates
cargo run -p quant-cli -- explain-latest-gate

# Data health
cargo run -p quant-cli -- check-data-health
cargo run -p quant-cli -- export-data-health-report

# Dashboard inspection
cargo run -p quant-cli -- dashboard-snapshot --scope cn --date 2026-03-16
cargo run -p quant-cli -- export-report --scope hk --date 2026-05-07

# LLM analysis (config now lives in config/llm.toml — see ADR-032/033)
cargo run -p quant-cli -- analyze-with-llm --scope global

# V6 Research Surface
cargo run -p quant-cli -- research-srd --scope global
cargo run -p quant-cli -- research-stretch --scope cn
cargo run -p quant-cli -- research review --scope global --from 2026-04-01 --to 2026-06-30

# V5 Execution filter
cargo run -p quant-cli -- preclose-analysis --scope cn
```

### Desktop debug commands (Tauri internal)
```bash
cargo run -p quant-desktop -- get-llm-status
cargo run -p quant-desktop -- analyze-with-llm --scope global --action market_story
cargo run -p quant-desktop -- analyze-with-llm --scope cn --action explain_decision
cargo run -p quant-desktop -- analyze-with-llm --scope hk --action risk_view
```

## CONVENTIONS
- Read `memory/context.md` and `memory/decisions.md` before deep edits.
- Read `docs/architecture-invariants.md` before changing V6 reporting/data ownership.
- Desktop `Refresh data` is the default user path. CLI full-chain runs remain explicit engineering / advanced-user paths.
- Startup and scope reloads use `dashboard_bundle`; historical date changes use `dashboard_snapshot`.
- `Trust summary` is the primary trust verdict. `Pipeline freshness` and `Data health` stay as evidence / drilldown layers beneath it.
- `Recent reports` is a research-results entry point: `DAILY_REPORT*` items can reopen matching snapshots, and all artifacts can open or copy paths from desktop.
- Signal freshness / completeness guards are centralized in pipeline diagnostics alerts and enforced by `compute_signals` plus desktop refresh across `GLOBAL`, `CN`, and `HK`.
- `app-service` orchestrates, engine crates compute, and `market-store` persists. CLI, Tauri, and frontend must not own quant logic.
- Scoped dashboard/report semantics and signal/backtest provenance are intentionally explicit; read `analysis_scope`, `regime_basis_scope`, and snapshot scope together.
- Schema evolution policy: all DTOs deserialized from ClickHouse JSON must carry `#[serde(default)]` on new fields, or fetch functions must manually remap, to avoid old-row deserialization crashes.
- Sub-AGENTS.md files exist under `apps/`, `crates/`, and `docs/`. Respect the nearest one when working in those trees.
- V6 Reporting Platform is frozen; add new consumers on top of it, do not refactor the platform itself.

## ANTI-PATTERNS
- Do **not** access ClickHouse or SQLite outside `crates/market-store`.
- Do **not** move scoring, freshness, or trust logic into Tauri commands or frontend slices.
- Do **not** describe CLI full-chain execution as the default operator workflow in docs or UI.
- Do **not** collapse `dashboard_bundle` and `dashboard_snapshot` back into one path without a measured reason.
- Do **not** widen desktop artifact opening beyond canonical files under `reports/`.
- Do **not** treat the latest available date as trustworthy when diagnostics still report stale or partial decision stages.
- Do **not** add new ClickHouse JSON DTO fields without `#[serde(default)]` or manual remapping.
- Do **not** put `--quiet` after the subcommand; it is a global option and must come before.
- Do **not** expose `ResearchDataset` or `ResearchSnapshot` outside `app-service`.
- Do **not** put metadata (scope/date/generated_at) into `ReportInput` structs.
- Do **not** add artificial `ReportBuilder` implementations; the trait is pending evaluation.

## ENVIRONMENTAL GOTCHAS
- **Eastmoney is currently unreachable from this environment**; all live data fetching goes through Tencent fallback. Do not assume Eastmoney paths work for local validation.
- **Turnover backfill**: P2 turnover fix only affects newly ingested data. Stored ClickHouse bars may still lack turnover until a historical `ingest-daily` backfill is run.
- **HSAHP disabled**: ADR-029 disabled the AH premium index due to source unavailability.
- **LLM config migrated**: ADR-032/033 moved LLM configuration from opaque SQLite/keyring to transparent `config/llm.toml` (gitignored) with `${ENV_VAR}` interpolation. API key fallback chain: TOML → keyring → SQLite.
- **No CI / no formal test suite**: validation is `cargo check`, targeted `cargo test`, frontend build, and live CLI/desktop flows. Tests exist in ~12 crates but coverage is sparse.
- **Static trading calendar**: JSON calendar only covers 2024–2027 and requires manual maintenance.
- **Regime calibration frozen**: TASK-004 (threshold calibration) and downstream work (Insight, Daily Report, Allocation, Narrative) are FROZEN pending Wave 9 Ground Truth redefinition (ADR-060).

## NOTES
- `reports/`, `target/`, `apps/desktop/frontend/node_modules/`, and `apps/desktop/frontend/dist/` are generated/runtime artifacts.
- Root planning docs outside `docs/` (`设计规划-v1.md`, `数据源方案评审.md`) are archive/reference material unless `docs/文档状态说明.md` says they are current.
- `config/llm.toml` is gitignored; create it manually or via CLI `set-llm-config` / `set-llm-api-key`.
