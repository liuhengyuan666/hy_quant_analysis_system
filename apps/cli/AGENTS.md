# CLI KNOWLEDGE BASE

## OVERVIEW
Thin clap-based operational shell over `AppContext`.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Command list | `src/main.rs::Command` | authoritative CLI surface |
| Command dispatch | `src/main.rs::main` | nearly 1:1 mapping to `AppContext` |
| Runtime config source | `StorageConfig::default()` | local ClickHouse / SQLite / universe paths |

## CONVENTIONS
- Keep commands thin: parse args, call `AppContext`, print pretty JSON.
- Subcommands mirror `AppContext` method names where possible.
- Tracing subscriber is initialized in `main`; keep CLI-friendly logs there.
- Validation in this repo is often CLI-first: `cargo check`, targeted `cargo test`, then live CLI flows.

## ANTI-PATTERNS
- Do **not** fork business logic inside CLI match arms.
- Do **not** introduce CLI-only semantics when an `AppContext` method can own them.
- Do **not** hand-format output inconsistently; keep machine-readable JSON for summaries.

## COMMANDS
```bash
cargo run -p quant-cli -- status
cargo run -p quant-cli -- init-storage
cargo run -p quant-cli -- seed-universe
cargo run -p quant-cli -- pipeline-dates
cargo run -p quant-cli -- dashboard-dates
cargo run -p quant-cli -- dashboard-snapshot --date 2026-03-18
cargo run -p quant-cli -- export-report
```

## NOTES
- There is no separate CLI crate logic beyond `src/main.rs`; if complexity grows, extract flags/parsing helpers before bloating dispatch.
- `pipeline-dates` is the first diagnostic to run when dashboard freshness and stage freshness disagree.
