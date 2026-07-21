# CLI KNOWLEDGE BASE

## OVERVIEW
Thin clap-based operational shell over `AppContext`. Dispatches to orchestration methods; keeps almost no business logic.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Command list | `src/main.rs::Command` | authoritative CLI surface |
| Command dispatch | `src/main.rs::main` | nearly 1:1 mapping to `AppContext` |
| Runtime config source | `StorageConfig::default()` | local ClickHouse / SQLite / universe paths |
| Command modules | `src/commands/*.rs` | 10 modules: config, pipeline, diagnostics, dashboard, backtest, research, llm, audit, execution |
| Research commands | `src/commands/research.rs` | V6: `research-srd`, `research-stretch`, `research review`; V7/V8: `research observe`, `research analytics`, `research replay`, `research consensus`, `data-health` |
| Execution commands | `src/commands/execution.rs` | `preclose-analysis` |
| Audit monolith | `src/commands/audit.rs` | ~3,578 lines; contains many GT/audit/experiment subcommands |

## CONVENTIONS
- Keep commands thin: parse args, call `AppContext`, print pretty JSON.
- Subcommands mirror `AppContext` method names where possible.
- Tracing subscriber is initialized in `main`; keep CLI-friendly logs there.
- Validation in this repo is often CLI-first: `cargo check`, targeted `cargo test`, then live CLI flows.
- Scope-aware dashboard/report commands are the fastest smoke test for Phase 1 semantics.
- Research Surface commands (`research-srd`, `research-stretch`, `research review`, `research observe`, `research analytics`, `research replay`, `research consensus`, `data-health`) are read-only observation tools.
- `--quiet` is a global option and must appear before the subcommand, not after.

## ANTI-PATTERNS
- Do **not** fork business logic inside CLI match arms.
- Do **not** introduce CLI-only semantics when an `AppContext` method can own them.
- Do **not** hand-format output inconsistently; keep machine-readable JSON for summaries.
- Do **not** add new audit subcommands to `audit.rs` without considering decomposition; the file is already a monolith (~3,578 lines).
- Do **not** put `--quiet` after the subcommand.

## COMMANDS
```bash
cargo run -p quant-cli -- status
cargo run -p quant-cli -- init-storage
cargo run -p quant-cli -- seed-universe
cargo run -p quant-cli -- compute-macro --from 2025-12-01 --to 2026-04-01
cargo run -p quant-cli -- pipeline-dates
cargo run -p quant-cli -- dashboard-dates --scope cn
cargo run -p quant-cli -- dashboard-snapshot --scope hk --date 2026-03-30
cargo run -p quant-cli -- export-report --scope cn

# V6 Research Surface
cargo run -p quant-cli -- research-srd --scope global
cargo run -p quant-cli -- research-stretch --scope cn
cargo run -p quant-cli -- research review --scope global --from 2026-04-01 --to 2026-06-30

# V7 Research Surface
cargo run -p quant-cli -- research observe --scope global
cargo run -p quant-cli -- research confirmation --scope global
cargo run -p quant-cli -- research recovery --scope global
cargo run -p quant-cli -- research analogues --scope global
cargo run -p quant-cli -- research calibration --scope global --from 2026-04-01 --to 2026-06-30
cargo run -p quant-cli -- research consensus --scope global
cargo run -p quant-cli -- research replay --scope global --from 2026-04-01 --to 2026-06-30

# V7 workflow: data health + report
cargo run -p quant-cli -- data-health

# V8 Research Asset (save Evidence)
cargo run -p quant-cli -- research analytics --condition srd-strong --scope global --horizon 20 --save-evidence

# V5 Execution filter
cargo run -p quant-cli -- preclose-analysis --scope cn

# Shadow Production symbol diagnostics & scoreboard
cargo run -p quant-cli -- symbol-diagnostics --symbol 000300 --scope cn
cargo run -p quant-cli -- symbol-scoreboard --scope cn
```

## NOTES
- There is no separate CLI crate logic beyond `src/main.rs` and `src/commands/`; if complexity grows, extract flags/parsing helpers before bloating dispatch.
- `pipeline-dates` is the first diagnostic to run when dashboard freshness and stage freshness disagree.
- `dashboard-dates`, `dashboard-snapshot`, and `export-report` now accept explicit `--scope global|cn|hk`; default remains global.
- `export-report` now fails loudly when latest gate is behind; use `--date YYYY-MM-DD` for historical exports.
- `compute-macro` now rebuilds scoped regime/environment and may report provider-specific `failed_items`; treat that summary as user-facing diagnostic output, not noise.
- `data-health` merges `check-data-health` and `export-data-health-report` into one command.
- `audit.rs` is the largest command module; prefer adding new audit-like commands as a new module or upstreaming analysis to `regime-audit` / `research-validation`.
