# PROJECT KNOWLEDGE BASE

**Generated:** 2026-07-26
**Commit:** 96ba5eb
**Branch:** main

## OVERVIEW
Local desktop quant research system — **Daily Portfolio Decision Assistant**（每日组合决策辅助系统，RV1 已合并 main）。The Rust workspace owns the ingestion-to-report pipeline, the Tauri desktop app is the default operator surface, and `docs/` plus `memory/` are part of the working system rather than side notes. RV1 完成能力收敛：CLI 精简至 ~30 命令（`market-refresh → daily-analysis → strategy-perspectives → portfolio-decision → llm-analyze` 日常闭环），Decision 语义冻结为 Increase/Maintain/Reduce/Avoid，LLM 边界（ADR-106）与前端投影边界（ADR-107/108）已冻结。

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
├── .cargo/               # Rust build config (Windows stack size /STACK:8388608)
├── .omo/                 # Agent orchestration plans & run continuations
├── .opencode/            # OpenCode skill configuration (knowledgeguard, etc.)
├── .sisyphus/            # Agent planning artifacts
├── apps/                 # CLI + desktop delivery surfaces
│   ├── cli/              # clap-based CLI over AppContext
│   └── desktop/          # Tauri app + Vite/Vue frontend
│       ├── frontend/     # Vite bundle, plain JS + Vue 3 (25+ components)
│       └── src-tauri/    # Tauri native bridge
├── config/               # instrument universe + runtime inputs (llm.toml, fred.toml here)
│   ├── calendars/        # static JSON trading calendars (2024–2027)
│   ├── llm.toml          # LLM config (gitignored, ${ENV_VAR} interpolation)
│   ├── llm.toml.example
│   ├── prompts.toml      # LLM persona 配置（ADR-106：只承载视角指令；含 market_adversarial_lens）
│   ├── scenarios.toml    # 场景权重配置（4 预设场景，RV1 策略多视角）
│   ├── fred.toml         # FRED macro config (gitignored)
│   ├── fred.toml.example
│   ├── benchmark-providers.toml
│   └── universe.json     # instrument universe
├── crates/               # contracts, engines, persistence, orchestration
│   ├── app-service/             # orchestration (lib.rs ~5,900 lines + 14 helper modules)
│   ├── backtest-engine/         # backtest execution
│   ├── core-domain/             # shared DTOs, AnalysisScope, provenance, research::* pure computation
│   ├── data-ingestion/          # Eastmoney / Tencent fetchers
│   ├── execution-engine/        # V5 pattern-library execution filter (ExecutionState: Increase/Maintain/Reduce/Avoid/Skip)
│   ├── execution-replay/        # V8 Execution Platform 证据重放（51 模块，计算+formatter 成对）
│   ├── gt-regime-generator/     # Ground Truth regime generation
│   ├── indicator-engine/        # MA/EMA/MACD/RSI/ATR/VOL_MA
│   ├── llm-context/             # LLM-specific 8-dimension ResearchContext builder
│   ├── macro-engine/            # factor normalization, regime rows
│   ├── market-fingerprint-engine/ # V7.2B historical evidence similarity matching
│   ├── market-state-extractor/  # MarketStateObservation extraction
│   ├── market-store/            # ALL ClickHouse / SQLite IO (14 domain modules)
│   ├── regime-audit/            # regime alignment / sensitivity audits (26 sub-modules)
│   ├── report-builder/          # document-specific ReportInput + builders (V6)
│   ├── report-engine/           # DashboardSnapshot, TrustSummary, legacy markdown rendering
│   ├── reporting/               # presentation model: ReportingSnapshot, ReportDocument, Formatter
│   ├── research-benchmark/      # research benchmarking harness (WIP)
│   ├── research-context/        # canonical ResearchContext + TrustLevel semantic contract
│   ├── research-renderer/       # insight/research rendering
│   ├── research-skills/         # LLM research actions (6 内置 persona + config/prompts.toml 文件 persona)
│   ├── research-validation/     # Ground Truth validation
│   ├── rotation-engine/         # relative strength ranking
│   ├── signal-engine/           # final signal generation
│   ├── strategy-engine/         # strategy preference scoring
│   └── task-runner/             # task execution framework (placeholder)
├── data/                 # runtime data (SQLite app_state.db, etc.)
├── docs/                 # truth sources, operator guides, active design
│   ├── v2/               # V2 Phase 1 environment layer docs
│   ├── v3/               # V3 scoped reporting / code review docs
│   ├── v5/               # V5 execution layer, Shadow Production, State/Economic ADRs
│   └── v6/               # V6 reporting platform design docs and contracts
├── infra/                # Docker / ClickHouse bootstrap
│   └── docker/
├── knowledge/            # KnowledgeGuard asset base (product, architecture, modules, glossary, patterns)
├── memory/               # MemGuard durable project context, decisions, history
│   ├── archive/
│   ├── history/
│   ├── tests/            # agent behavior / pressure test records
│   ├── context.md
│   ├── decisions.md
│   ├── decisions_archive.md
│   ├── glossary.md
│   ├── product.md
│   ├── structure.md
│   ├── tasks_archive.md
│   └── tech.md
├── research/             # non-code research artifacts
│   └── agents/           # agent prompt/config reference files
├── reports/              # exported artifacts; desktop open-file support stays here only
│   ├── calibration/
│   ├── consensus/
│   ├── daily/
│   └── execution-samples/
├── screen_pic/           # README / doc screenshots
├── shadow-production/    # Shadow Production run-time artifacts & ops scripts
│   ├── daily-log.ps1
│   ├── weekly-review.ps1
│   └── historical-replay/
├── sql/                  # storage bootstrap DDL
│   ├── clickhouse/
│   └── sqlite/
└── target/               # Rust build artifacts
```

> `workspace/` is a V8 runtime-generated directory (gitignored) for durable Research Assets. It does not appear in the source tree until `research analytics --save-evidence` or Historical Replay creates it.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Agent rules + memory workflow | `memory/context.md` + `memory/decisions.md` | highest-priority collaboration rules |
| Current repo truth source | `README.md` + `docs/文档状态说明.md` | start here before old planning docs |
| Architecture invariants | `docs/architecture-invariants.md` | ADR-069; 10 non-negotiable V6 rules |
| Shadow Production playbook | `docs/shadow-production-playbook.md` | operational guidance for 90-day observation |
| Orchestration / trust / freshness guards | `crates/app-service/src/` | `AppContext` (in `lib.rs`), `build_trust_summary` (in `trust.rs`), `dashboard_bundle_with_scope`, `refresh_consistency_alerts` |
| V6–V8 research orchestration | `crates/app-service/src/lib.rs` | `research_srd`, `research_stretch`, `research_review`, `research_analytics`, `research_replay`, `research_consensus` |
| V8 Research Asset workspace | `crates/app-service/src/workspace.rs` | `WorkspaceManager`, `ResearchAssetId`, `AssetKind`, lifecycle state machine |
| Shared contracts | `crates/core-domain/src/lib.rs` | `AnalysisScope`, shared snapshot DTOs, provenance fields |
| Pure research computation | `crates/core-domain/src/research/` | percentile, analytics, stretch, consensus, attribution, calibration helpers |
| Shared calendar / date logic | `crates/core-domain/src/calendar.rs` | `TradingCalendar`, trading-day helpers |
| Canonical semantic model | `crates/research-context/src/lib.rs` | `ResearchContext`, `TrustLevel` |
| Presentation model | `crates/reporting/src/lib.rs` | `ReportingSnapshot`, `ReportDocument`, `Formatter` |
| Document assembly | `crates/report-builder/src/lib.rs` | `ResearchReportBuilder`, `ReviewReportBuilder`, `AuditReportBuilder`, `*ReportInput` |
| Persistence boundary | `crates/market-store/src/` | 14 domain modules (`core`, `bars`, `signals`, `regime`, `environment`, `rotation`, `strategy`, `indicators`, `macro`, `backtest`, `reports`, `dates`, `instruments`, `sqlite`) via `pub use` re-exports; all ClickHouse / SQLite IO and latest-date gating |
| Pure macro regime logic | `crates/macro-engine/src/lib.rs` | factor normalization + `GLOBAL/CN/HK` regime rows |
| Report contract | `crates/report-engine/src/lib.rs` | `DashboardSnapshot`, `TrustSummary`, markdown report rendering |
| Historical evidence matching | `crates/market-fingerprint-engine/src/` | V7.2B similarity matching; consumed by `research analogues` |
| Ground Truth regime generation | `crates/gt-regime-generator/src/lib.rs` | 4-layer pipeline: Observation → Candidate → Persistence → Label |
| Market-state observations | `crates/market-state-extractor/src/lib.rs` | Trend/Volatility/Liquidity dimensions (ADR-053) |
| Pre-close execution filter | `crates/execution-engine/src/lib.rs` | V5 pattern-library execution filter |
| Shared adversarial context (ADR-112~114) | `crates/app-service/src/lib.rs::ensure_adversarial_context` + `crates/app-service/src/llm_history.rs` | 每 scope 每日一次前置博弈分析落盘，按 persona 分级注入；`spawn_adversarial_prewarm` 异步预热；`adversarial_context_section` 分级 + ContentPolicy 截断 |
| LLM persona resolution | `crates/app-service/src/prompts.rs` | 文件 persona（prompts.toml）→ 内置 persona 回退 |
| Strategy perspectives (RV1) | `crates/app-service/src/strategy_perspectives.rs` + `crates/app-service/src/scenarios.rs` | 四策略独立评分 scoreboard/detail + 场景加权；Tauri `strategy_scoreboard`/`strategy_attribution`；桌面「策略视角」人格卡片面板 |
| Research rendering | `crates/research-renderer/src/lib.rs` | insight/research rendering |
| GT validation | `crates/research-validation/src/lib.rs` | Ground Truth label generation/validation |
| LLM research actions | `crates/research-skills/src/action.rs` | 6 内置 persona + `build_prompt()`；文件 persona 见 `config/prompts.toml` |
| Desktop shell / refresh bridge | `apps/desktop/src-tauri/src/lib.rs` | command boundary, refresh coordinator, safe artifact opening |
| Frontend composition | `apps/desktop/frontend/src/main.js` | root state, scope/date flow, refresh UI, top-level render |
| Frontend feature slices | `apps/desktop/frontend/src/components/*.vue` | 25+ Vue panels |
| Environment + breadth UI | `apps/desktop/frontend/src/components/EnvironmentPanel.vue` + `BreadthPanel.vue` | paired explanation layer + proxy view |
| CLI surface | `apps/cli/src/main.rs` + `commands/` | thin dispatch over `AppContext` + 10 command modules (`config`, `pipeline`, `diagnostics`, `dashboard`, `backtest`, `research`, `llm`, `audit`, `execution`, `execution_replay`) |
| V7 workflow commands | `apps/cli/src/commands/research.rs` + `apps/cli/src/main.rs::Command` | `research observe`, `research replay`, `data-health`, `research explain`, `research calibration`, `research consensus` |
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

### Recommended CLI paths (RV1 surface)
```bash
# 每日闭环（日常操作员路径）
cargo run -p quant-cli -- market-refresh                          # 全链路刷新（成功后异步预生成当日博弈背景，ADR-113）
cargo run -p quant-cli -- daily-analysis                          # 30 秒看 Integrity + 信号 + 组合姿态
cargo run -p quant-cli -- research observe --scope global         # 历史语境观测（每日核心）
cargo run -p quant-cli -- strategy-perspectives                   # 四策略 × 四场景矩阵
cargo run -p quant-cli -- daily-report                            # 需要留档时导出

# 按需下钻
cargo run -p quant-cli -- strategy-perspectives --mode detail --symbol 512480   # 单标的四策略归因
cargo run -p quant-cli -- portfolio-decision --scope global       # 组合姿态（Increase/Maintain/Reduce/Avoid）
cargo run -p quant-cli -- research analogues --scope global       # 历史相似盘面

# LLM 分析（config 在 config/llm.toml，ADR-032/033；共享博弈背景默认开启，ADR-112）
cargo run -p quant-cli -- llm-analyze --action market_story --scope global
cargo run -p quant-cli -- llm-analyze --action portfolio_review --scope global
cargo run -p quant-cli -- llm-analyze --action explain_decision --adversarial compact   # 单次覆盖注入级别 full|standard|compact|none

# 周期命令
cargo run -p quant-cli -- validation-check                        # 每周：校准基线验证
cargo run -p quant-cli -- run-backtest                            # 双周/月度：信号回测
cargo run -p quant-cli -- historical-replay                       # 可脚本化：Evidence 积累
cargo run -p quant-cli -- evidence-status                         # Evidence 资产积累状态

# Quiet mode: global flag, place BEFORE subcommand
cargo run -p quant-cli -- --quiet daily-analysis

# Pipeline diagnostics (first thing to run when freshness looks off)
cargo run -p quant-cli -- pipeline-dates
cargo run -p quant-cli -- explain-latest-gate

# Engineering / advanced paths (hidden from --help, still executable)
cargo run -p quant-cli -- sync-and-export --scope global          # 旧版一键同步导出
cargo run -p quant-cli -- refresh-all --to 2026-06-05             # 旧版全量刷新（被 market-refresh 取代）
cargo run -p quant-cli -- dashboard-snapshot --scope cn --date 2026-03-16
cargo run -p quant-cli -- symbol-diagnostics --symbol 000300 --scope cn
cargo run -p quant-cli -- symbol-scoreboard --scope cn

# V6–V8 Research Surface (read-only observation)
cargo run -p quant-cli -- research-srd --scope global
cargo run -p quant-cli -- research calibration --scope global --from 2026-04-01 --to 2026-06-30
cargo run -p quant-cli -- research consensus --scope global
cargo run -p quant-cli -- research analytics --condition srd-strong --scope global --horizon 20 --save-evidence
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
- V7 Research Platform (Observation → Evolution → Historical Evidence → Synthesis) is frozen; add new Research Content, not new platform semantics.
- V8 Research Asset workspace uses `RA-XXXXXX` identity and `Draft → Verified → Published → Superseded → Archived` lifecycle; do not design Evidence Score/Weight (P3) before 1000+ assets, 30-day replay stability, and 2-cycle calibration stability.
- LLM 边界冻结（ADR-106）：数据流永远为 确定性引擎 → 决策事实 → LLM 解释；LLM 只解释不决策。共享博弈假设背景默认开启（ADR-112~114）：每 scope 每日一次前置分析落盘复用，按 persona 分级注入（full/standard/compact/none），递归防护硬编码，失败静默降级；market-refresh 成功后异步预生成。
- 前端投影边界（ADR-107/108）：UI 不拥有投资语义——Frontend: Render / Backend: Interpret / Decision: Execution Engine。禁止前端出现分数到买卖语义的映射判断；新认知层用独立展示入口，不堆进 Dashboard 首页。
- 领域枚举重命名保留 `#[serde(alias = "OLD")]` 兼容反序列化（如 ExecutionState）；alias 只管输入，所有 consumer 必须同步切换到新名。
- Rust modules use flat named files (`foo.rs`) under `src/`; the only `mod.rs` in the project is `crates/core-domain/src/research/mod.rs` (legacy holdout). New modules must not add `mod.rs` directories.
- All Rust crates use `anyhow::Result`; there are no shared `thiserror` enums. Lint suppression is exceptional: only 6 inline `#[allow(...)]` annotations exist across the workspace. No custom `rustfmt.toml` or `clippy.toml` is used; rely on Rust defaults.
- Tests are sparse and inline (`#[cfg(test)] mod tests {}`). There is no integration-test directory, no CI, and no formal test suite. Validation is `cargo check`, targeted `cargo test`, and live CLI/desktop flows.
- Windows builds require the `/STACK:8388608` linker flag in `.cargo/config.toml`; do not remove it.
- `--quiet` is a global CLI option and must appear before the subcommand, not after.

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
- Do **not** add new `mod.rs` directories; use flat named files (`foo.rs`) under `src/` for new modules.
- Do **not** derive investment semantics in the frontend (no score→stance/advice mapping in JS); frontend renders backend facts only (ADR-107/108).
- Do **not** kill `market-refresh` with a short shell timeout — an interrupted refresh can silently regress the dashboard latest gate (bars incomplete → gate falls back days); always let it run to completion and verify with `pipeline-dates` afterwards.

## HOTSPOTS
- `crates/app-service/src/lib.rs` (~5,900 lines) is the orchestration monolith. Helper modules (`trust.rs`, `workspace.rs`, `config_loader.rs`, `breadth.rs`, `core.rs`, `llm.rs`, `llm_history.rs`, `prompts.rs`, `scenarios.rs`, `strategy_perspectives.rs`, `research_evidence.rs`, `sync.rs`, `dashboard.rs`, `execution_replay.rs`) are extracted, but the main file still carries most high-level flow. Review nearby helpers before adding new orchestration logic.
- `apps/cli/src/commands/audit.rs` (~3,578 lines) is the largest CLI command module. Prefer adding new audit-like commands as a separate module or upstreaming analysis to `regime-audit` / `research-validation`.
- `crates/report-builder/src/lib.rs` (~1,145 lines) and `crates/core-domain/src/lib.rs` (~1,111 lines) are single-file hubs. Add new domain-specific DTOs in `core-domain` or new builders in `report-builder` with care; field drift here breaks downstream consumers.
- `apps/desktop/src-tauri/src/lib.rs` (~1,058 lines) contains the full Tauri command surface and refresh coordinator. Keep it thin over `app-service`.
- `crates/regime-audit/src/` has been decomposed into 26 sub-modules; use it as the model for future crate splitting.
- `crates/market-store/src/lib.rs` is a thin 133-line re-export hub after the 14-module split; this is the intended pattern for large crates.
- `crates/market-state-extractor/src/lib.rs` and `crates/gt-regime-generator/src/lib.rs` are each ~600+ lines with no sub-modules; consider decomposition if they grow.

## ENVIRONMENTAL GOTCHAS
- **Eastmoney is currently unreachable from this environment**; all live data fetching goes through Tencent fallback. Do not assume Eastmoney paths work for local validation.
- **Turnover backfill**: P2 turnover fix only affects newly ingested data. Stored ClickHouse bars may still lack turnover until a historical `ingest-daily` backfill is run.
- **HSAHP disabled**: ADR-029 disabled the AH premium index due to source unavailability.
- **LLM config migrated**: ADR-032/033 moved LLM configuration from opaque SQLite/keyring to transparent `config/llm.toml` (gitignored) with `${ENV_VAR}` interpolation. API key fallback chain: TOML → keyring → SQLite.
- **No CI / no formal test suite**: validation is `cargo check`, targeted `cargo test`, frontend build, and live CLI/desktop flows. Tests exist in ~12 crates but coverage is sparse.
- **Static trading calendar**: JSON calendar only covers 2024–2027 and requires manual maintenance.
- **Regime calibration frozen**: TASK-004 (threshold calibration) and downstream work (Insight, Daily Report, Allocation, Narrative) are FROZEN pending Wave 9 Ground Truth redefinition (ADR-060).
- **V8 workspace**: `workspace/` is a gitignored runtime directory. It is created by `research analytics --save-evidence` or Historical Replay, not by source control.

## NOTES
- `reports/`, `target/`, `apps/desktop/frontend/node_modules/`, `apps/desktop/frontend/dist/`, and `workspace/` are generated/runtime artifacts.
- Root planning docs outside `docs/` (`设计规划-v1.md`, `数据源方案评审.md`) are archive/reference material unless `docs/文档状态说明.md` says they are current.
- `config/llm.toml` is gitignored; create it manually or via CLI `set-llm-config` / `set-llm-api-key`.
