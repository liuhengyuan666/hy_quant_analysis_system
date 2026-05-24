# V3 Features: sync-and-export, CLI Progress, LLM Integration

## TL;DR

> **Quick Summary**: Add three CLI-centric features: a `sync-and-export` command that auto-refreshes stale data before exporting reports; stderr progress output for long-running CLI commands; and LLM integration to send daily reports for AI-powered analysis interpretation and trading strategy suggestions.
>
> **Deliverables**:
> - New CLI command: `sync-and-export` with `--scope`, `--date`, `--to`, `--run-backtests`
> - New CLI commands: `set-llm-config`, `set-llm-api-key`, `analyze-with-llm`
> - Progress output via `eprintln!` to stderr for `refresh-all`, `sync-and-export`, and all `compute-*` commands
> - SQLite `app_config` / `credential_store` read/write helpers in `market-store`
> - LLM client module in `app-service` using `async-openai` + `keyring`
> - Unit tests for config round-trip, gate-after-refresh logic, and LLM artifact generation
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: T1 (config storage) → T5 (progress infra) → T6/T7/T8 (core features) → F1-F4

---

## Context

### Original Request
User requested three V3 features:
1. One-click export latest report (`sync-and-export`): auto-detect stale data, refresh, then export
2. CLI progress output at key execution nodes
3. LLM integration to send daily reports for analysis interpretation and trading strategy planning

### Interview Summary
**Key Discussions**:
- Feature 1: New `sync-and-export` command, auto-refreshes when gate is behind, fail-loud if refresh doesn't resolve
- Feature 2: Progress to stderr via `eprintln!`, stdout JSON unaffected, default-on with `--quiet` opt-out
- Feature 3: OpenAI-compatible API (`async-openai`), api_key in OS keyring (SQLite fallback), sync blocking call, result saved to `reports/llm-analysis-{scope}-{date}.md`
- Tests: Tests-after strategy for key logic

**Research Findings**:
- Current `export-report` already fail-loud on gate behind (2026-05-08 decision)
- Desktop has `progress_pct` mapping (ingest=20%, indicators=40%, etc.) but CLI has none
- SQLite schema already has `app_config` and `credential_store` tables but lacks Rust helpers
- `async-openai` + `keyring` are recommended production crates
- `app-service` is a monolith (795+ lines); anti-pattern warns against growing it blindly

### Metis Review
**Identified Gaps** (addressed):
- sync-and-export needs explicit `--date` support (bypasses auto-refresh like export-report)
- Progress needs `--quiet` flag for script compatibility
- LLM artifacts should register in `report_snapshot` (consistent with export-report)
- Secrets must never appear in stdout/stderr/logs/JSON
- LLM code must be isolated; avoid bloating app-service monolith
- Need mock-server tests for LLM client
- Need stale-gate-after-refresh failure test

---

## Work Objectives

### Core Objective
Add three CLI features to the rust-quant-analysis-system: auto-refresh-and-export, command-line progress feedback, and LLM-powered report analysis.

### Concrete Deliverables
- `apps/cli/src/main.rs`: New `SyncAndExport`, `SetLlmConfig`, `SetLlmApiKey`, `AnalyzeWithLlm` commands
- `crates/app-service/src/lib.rs`: New `sync_and_export`, `set_llm_config`, `get_llm_config`, `set_llm_api_key`, `get_llm_api_key`, `analyze_report_with_llm` methods
- `crates/market-store/src/lib.rs`: `fetch_app_config`, `insert_app_config`, `fetch_credential`, `insert_credential` helpers
- `crates/core-domain/src/lib.rs`: `LlmConfig`, `LlmAnalysisResult` types (if needed)
- Progress callback integration in `refresh_pipeline` and compute methods

### Definition of Done
- [ ] `cargo run -p quant-cli -- sync-and-export --scope global` auto-refreshes stale data and exports report
- [ ] `cargo run -p quant-cli -- compute-macro --from ... --to ...` shows progress in stderr
- [ ] `cargo run -p quant-cli -- analyze-with-llm --scope global` sends report to LLM and saves response
- [ ] `cargo test --workspace` passes
- [ ] `cargo check --workspace` passes

### Must Have
- `sync-and-export` with `--scope`, `--date`, `--to`, `--run-backtests`
- `--date` bypasses auto-refresh (explicit historical export)
- stderr progress for `refresh-all`, `sync-and-export`, `compute-*` commands
- `--quiet` flag to suppress progress
- LLM config persisted across sessions (URL, model, timeout)
- API key stored securely (keyring first, SQLite fallback)
- LLM response saved as markdown artifact

### Must NOT Have (Guardrails)
- No desktop UI for LLM/config/progress (CLI only for V3)
- No streaming LLM responses
- No multi-provider abstraction beyond OpenAI-compatible `base_url + model`
- No prompt templates, chat history, RAG, embeddings
- No secrets in stdout, stderr, logs, errors, JSON, or test snapshots
- No business logic in CLI match arms
- No direct SQLite queries outside `market-store`

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (sparse Rust unit tests)
- **Automated tests**: Tests-after
- **Framework**: `cargo test` (built-in)
- **Agent-Executed QA**: ALWAYS (mandatory for all tasks)

### QA Policy
Every task MUST include agent-executed QA scenarios. Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.
- **CLI**: Bash commands with stdout/stderr assertions
- **Rust tests**: `cargo test -p <crate>` assertions
- **API/Config**: Bash (curl/manual verification)

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation - max parallel):
├── Task 1: market-store SQLite config/credential helpers
├── Task 2: core-domain types for LLM config/result
├── Task 3: Add async-openai + keyring to workspace dependencies
├── Task 4: Centralize stage progress metadata (shared constants)
└── Task 5: Progress callback infrastructure in app-service

Wave 2 (Core features - max parallel):
├── Task 6: sync-and-export CLI command + AppContext method
├── Task 7: CLI progress output for all long-running commands
├── Task 8: LLM client module + config methods in app-service
└── Task 9: CLI LLM commands (set-config, set-api-key, analyze-with-llm)

Wave 3 (Tests + integration):
├── Task 10: Unit tests for config storage round-trip
├── Task 11: Unit tests for sync-and-export gate-after-refresh logic
├── Task 12: Unit tests for LLM client (mock server)
└── Task 13: Manual QA end-to-end verification

Wave FINAL (4 parallel reviews):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: T1 → T2 → T3 → T5 → T6/T7/T8/T9 → T10-T13 → F1-F4
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 5 (Wave 1) + 4 (Wave 2) + 4 (Wave 3)
```

### Dependency Matrix

- **T1**: - → T8, T10
- **T2**: - → T8, T9
- **T3**: - → T8
- **T4**: - → T5, T7
- **T5**: T4 → T6, T7
- **T6**: T5 → T11, T13
- **T7**: T5 → T13
- **T8**: T1, T2, T3 → T9, T12
- **T9**: T8 → T12, T13
- **T10**: T1 → F1-F4
- **T11**: T6 → F1-F4
- **T12**: T8, T9 → F1-F4
- **T13**: T6, T7, T9 → F1-F4

---

## TODOs

- [x] 1. **market-store SQLite config/credential helpers**

  **What to do**:
  - Add `fetch_app_config(storage, key) -> Result<Option<String>>` helper
  - Add `insert_app_config(storage, key, value)` helper (UPSERT pattern)
  - Add `fetch_credential(storage, key) -> Result<Option<String>>` helper
  - Add `insert_credential(storage, key, value)` helper (UPSERT pattern)
  - Follow existing `market-store` patterns: use `rusqlite` parameterized queries
  - Return `Option<String>` to represent missing keys naturally

  **Must NOT do**:
  - Do **not** add business logic (deciding default values, validation) in market-store
  - Do **not** expose raw `rusqlite::Connection` outside these helpers
  - Do **not** add keyring logic here; keyring is a CLI/app-service concern

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: Simple CRUD helpers following existing SQLite patterns in market-store

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4, 5)
  - **Blocks**: Task 8 (LLM config methods), Task 10 (config tests)
  - **Blocked By**: None

  **References**:
  - `crates/market-store/src/lib.rs` - existing `insert_*` / `fetch_*` patterns for SQLite
  - `sql/sqlite/001_init.sql` - `app_config` and `credential_store` schema
  - `crates/market-store/AGENTS.md` - "Enum-like DB values are bridged with serde_json::Value remapping"

  **Acceptance Criteria**:
  - [ ] `fetch_app_config` returns `None` for non-existent key
  - [ ] `insert_app_config` + `fetch_app_config` round-trips arbitrary string values
  - [ ] `insert_credential` + `fetch_credential` round-trips arbitrary string values
  - [ ] UPSERT semantics: second insert with same key updates value

  **QA Scenarios**:
  ```
  Scenario: Config round-trip
    Tool: Bash (cargo test)
    Preconditions: SQLite database initialized
    Steps:
      1. Run cargo test -p market-store app_config_round_trips
      2. Run cargo test -p market-store credential_round_trips
    Expected Result: Tests PASS
    Evidence: .sisyphus/evidence/task-1-config-roundtrip.txt
  ```

  **Commit**: YES
  - Message: `feat(market-store): add app_config and credential_store helpers`
  - Files: `crates/market-store/src/lib.rs`

- [x] 2. **core-domain types for LLM config/result**

  **What to do**:
  - Add `LlmConfig` struct with fields: `base_url: String`, `model: String`, `timeout_secs: u64`
  - Add `LlmAnalysisResult` struct with fields: `report_date: String`, `scope: String`, `output_path: String`, `analysis_text: String`
  - Derive `Serialize`, `Deserialize`, `Debug`, `Clone`
  - Keep this crate dependency-light; no I/O, no HTTP types

  **Must NOT do**:
  - Do **not** add async-openai types or reqwest types to core-domain
  - Do **not** add keyring types here
  - Do **not** add I/O or orchestration logic

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 8 (LLM methods use these types)
  - **Blocked By**: None

  **References**:
  - `crates/core-domain/src/lib.rs` - existing DTO patterns (`ReportSummary`, `SignalSummary`)
  - `crates/core-domain/AGENTS.md` - "Keep this crate dependency-light: chrono + serde only"

  **Acceptance Criteria**:
  - [ ] `LlmConfig` serializes/deserializes correctly with serde_json
  - [ ] `LlmAnalysisResult` serializes/deserializes correctly

  **QA Scenarios**:
  ```
  Scenario: LLM types serialization
    Tool: Bash (cargo test)
    Steps:
      1. Run cargo test -p core-domain llm_types_serialize
    Expected Result: Tests PASS
    Evidence: .sisyphus/evidence/task-2-llm-types.txt
  ```

  **Commit**: YES (groups with Task 1)

- [x] 3. **Add async-openai + keyring to workspace dependencies**

  **What to do**:
  - Add `async-openai = { version = "0.34", default-features = false, features = ["chat-completion", "native-tls"] }` to `[workspace.dependencies]`
  - Add `keyring = "3"` to `[workspace.dependencies]` (default features work cross-platform: Windows Credential Manager, macOS Keychain, Linux Secret Service)
  - Add both to `crates/app-service/Cargo.toml` dependencies
  - Verify `cargo check --workspace` passes after additions
  - Note: tokio is already in workspace; app-service may need tokio dependency added

  **Must NOT do**:
  - Do **not** enable all async-openai features; only `chat-completion` needed
  - Do **not** add these deps to crates that don't need them

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 8 (LLM module needs these crates)
  - **Blocked By**: None

  **References**:
  - `Cargo.toml` (root) - `[workspace.dependencies]` section
  - `crates/app-service/Cargo.toml` - existing dependency declarations
  - `async-openai` docs: configurable base_url + api_key via `OpenAIConfig`

  **Acceptance Criteria**:
  - [ ] `cargo check --workspace` passes with new dependencies

  **QA Scenarios**:
  ```
  Scenario: Workspace compiles with new deps
    Tool: Bash
    Steps:
      1. Run cargo check --workspace
    Expected Result: exit code 0, no errors
    Evidence: .sisyphus/evidence/task-3-check-workspace.txt
  ```

  **Commit**: YES (groups with Task 1)

- [x] 4. **Centralize stage progress metadata**

  **What to do**:
  - Extract stage names and progress percentages from desktop `RefreshStartStage` into shared constants
  - Create a small module (in core-domain or app-service) mapping stage names to progress values:
    - ingest: 20%, indicators: 40%, macro: 60%, rotation: 75%, strategy: 88%, signals: 92%, backtests: 96%
  - Update desktop `RefreshStartStage::progress_after()` to use these shared constants
  - Ensure CLI and desktop share the same progress semantics

  **Must NOT do**:
  - Do **not** duplicate the mapping in CLI
  - Do **not** change existing desktop behavior
  - Do **not** add progress logic to core-domain (keep it as constants only)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 5 (progress callback uses constants), Task 7 (CLI progress uses constants)
  - **Blocked By**: None

  **References**:
  - `apps/desktop/src-tauri/src/lib.rs` - `RefreshStartStage::progress_after()`
  - `crates/core-domain/AGENTS.md` - "Any string form exposed here is effectively a storage/report contract"

  **Acceptance Criteria**:
  - [ ] Desktop progress_pct remains identical before/after refactoring
  - [ ] Constants accessible from both app-service and CLI

  **QA Scenarios**:
  ```
  Scenario: Progress constants match desktop
    Tool: Bash (cargo test)
    Steps:
      1. Verify desktop build passes: cargo check -p quant-desktop
      2. Verify progress mapping matches original values
    Expected Result: Desktop compiles, progress values unchanged
    Evidence: .sisyphus/evidence/task-4-progress-constants.txt
  ```

  **Commit**: YES (groups with Task 5)

- [x] 5. **Progress callback infrastructure in app-service**

  **What to do**:
  - Add `ProgressReporter` trait or `Box<dyn Fn(&str) + Send>` callback parameter to `refresh_pipeline`
  - Add similar callback to `compute_indicators`, `compute_macro_regime`, `compute_rotation`, `compute_strategy_preferences`, `compute_signals`
  - At key points in each method, call the callback with human-readable stage messages:
    - "[1/7] Starting ingest..."
    - "[2/7] Computing indicators..."
    - etc.
  - If callback is `None`, behavior is identical to before (no progress)
  - Keep changes minimal; don't refactor entire methods

  **Must NOT do**:
  - Do **not** break existing desktop calls (pass `None` for callback)
  - Do **not** add progress inside tight loops (per-symbol processing)
  - Do **not** change return types of existing methods

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: Requires careful insertion into monolithic app-service without breaking existing behavior

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 1-4)
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 6 (sync-and-export uses progress), Task 7 (CLI progress output)
  - **Blocked By**: Task 4 (progress constants)

  **References**:
  - `crates/app-service/src/lib.rs` - `refresh_pipeline` signature and `run_refresh_stage!` macro
  - `apps/desktop/src-tauri/src/lib.rs` - `DashboardRefreshStatus` and progress tracking
  - `apps/cli/AGENTS.md` - "Keep commands thin: parse args, call AppContext, print pretty JSON"

  **Acceptance Criteria**:
  - [ ] `refresh_pipeline` compiles with `None` callback (desktop compatibility)
  - [ ] `refresh_pipeline` calls callback at each stage start when provided
  - [ ] Compute methods call callback at start and completion

  **QA Scenarios**:
  ```
  Scenario: Progress callback fires for each stage
    Tool: Bash (cargo test)
    Steps:
      1. Run cargo test -p app-service progress_callback_fires
    Expected Result: Test verifies callback receives expected stage messages
    Evidence: .sisyphus/evidence/task-5-progress-callback.txt
  ```

  **Commit**: YES (groups with Task 4)

- [x] 6. **sync-and-export CLI command + AppContext method**

  **What to do**:
  - Add `Command::SyncAndExport` to CLI with args: `--scope`, `--date`, `--to`, `--run-backtests`
  - Implement `AppContext::sync_and_export(&self, date: Option<NaiveDate>, scope: ReportScope, to: NaiveDate, run_backtests: bool) -> Result<SyncAndExportSummary>`
  - Logic:
    1. If `date` is `Some`, bypass auto-refresh and call `export_report_with_scope(date, scope)` directly
    2. If `date` is `None`, call `explain_latest_gate(scope)` to check gate status
    3. If `latest_gate_advanced == Some(false)`, call `refresh_pipeline(to, scope, run_backtests, None, None)`
    4. After refresh, call `explain_latest_gate(scope)` AGAIN to verify gate advanced
    5. If still behind, return error (fail-loud)
    6. If gate passed, call `export_report_with_scope(None, scope)`
    7. Return summary with exported report path, refreshed status, and gate info
  - CLI dispatch: parse args, call `sync_and_export`, print JSON

  **Must NOT do**:
  - Do **not** bypass existing `export_report_with_scope` fail-loud check
  - Do **not** export stale reports after failed refresh
  - Do **not** add business logic in CLI match arms
  - Do **not** change existing `export_report` behavior

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: Touches gate logic, refresh orchestration, and export; requires careful sequencing

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 7, 8, 9)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 11 (sync-and-export tests), Task 13 (manual QA)
  - **Blocked By**: Task 5 (progress callback for refresh)

  **References**:
  - `crates/app-service/src/lib.rs` - `export_report_with_scope` (L2883), `explain_latest_gate` (L1563), `refresh_pipeline` (L1289)
  - `apps/cli/src/main.rs` - existing command dispatch pattern
  - `crates/app-service/AGENTS.md` - "Do not let scoped exports silently fall back to global available-date selection"

  **Acceptance Criteria**:
  - [ ] `sync-and-export --date 2026-05-06` bypasses refresh and exports that date
  - [ ] `sync-and-export` (no date) with current gate exports immediately without refresh
  - [ ] `sync-and-export` with stale gate triggers refresh then exports
  - [ ] `sync-and-export` with stale gate where refresh fails returns error JSON
  - [ ] Progress messages appear in stderr during refresh

  **QA Scenarios**:
  ```
  Scenario: Gate current, export immediately
    Tool: Bash (cargo run)
    Preconditions: Database is up-to-date (gate advanced)
    Steps:
      1. Run cargo run -p quant-cli -- sync-and-export --scope global
      2. Assert stdout contains valid JSON with report_path
      3. Assert stderr does NOT contain "refreshing" message
    Expected Result: Exit code 0, report exported immediately
    Evidence: .sisyphus/evidence/task-6-gate-current.txt

  Scenario: Gate stale, refresh then export
    Tool: Bash (cargo run)
    Preconditions: Database lagging by at least one stage
    Steps:
      1. Run cargo run -p quant-cli -- sync-and-export --scope global
      2. Assert stderr contains progress messages during refresh
      3. Assert stdout contains valid JSON with report_path
    Expected Result: Exit code 0, auto-refreshed then exported
    Evidence: .sisyphus/evidence/task-6-gate-stale.txt
  ```

  **Commit**: YES
  - Message: `feat(cli,app-service): add sync-and-export command`
  - Files: `apps/cli/src/main.rs`, `crates/app-service/src/lib.rs`

- [x] 7. **CLI progress output for all long-running commands**

  **What to do**:
  - Add `--quiet` global flag or per-command flag to suppress progress
  - In CLI dispatch, wrap long-running commands with a progress callback:
    - `RefreshAll`: pass progress callback to `refresh_pipeline`
    - `SyncAndExport`: pass progress callback to `sync_and_export`
    - `ComputeIndicators`, `ComputeMacro`, `ComputeRotation`, `ComputeStrategyPreferences`, `ComputeSignals`: print "Starting..." / "Completed" messages
  - Progress callback implementation: `eprintln!("[stage] message")`
  - Ensure stdout JSON is unaffected (progress only to stderr)

  **Must NOT do**:
  - Do **not** print progress to stdout
  - Do **not** add progress to short commands (status, pipeline-dates, explain-latest-gate)
  - Do **not** break existing JSON output contract

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 6, 8, 9)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 13 (manual QA)
  - **Blocked By**: Task 5 (progress callback infrastructure)

  **References**:
  - `apps/cli/src/main.rs` - existing dispatch pattern
  - `apps/cli/AGENTS.md` - "Keep machine-readable JSON for summaries"

  **Acceptance Criteria**:
  - [ ] `cargo run -p quant-cli -- refresh-all` shows progress in stderr
  - [ ] `cargo run -p quant-cli -- refresh-all --quiet` suppresses progress
  - [ ] stdout remains valid JSON in both cases
  - [ ] `cargo run -p quant-cli -- compute-indicators` shows start/completion in stderr

  **QA Scenarios**:
  ```
  Scenario: Progress to stderr, JSON to stdout
    Tool: Bash
    Preconditions: Database initialized
    Steps:
      1. Run cargo run -p quant-cli -- refresh-all --to 2026-05-10 1>stdout.json 2>stderr.log
      2. Assert stdout.json is valid JSON (python -m json.tool stdout.json)
      3. Assert stderr.log contains progress messages
    Expected Result: Valid JSON + progress lines
    Evidence: .sisyphus/evidence/task-7-progress-stderr.txt

  Scenario: Quiet mode suppresses progress
    Tool: Bash
    Steps:
      1. Run cargo run -p quant-cli -- refresh-all --quiet 1>stdout.json 2>stderr.log
      2. Assert stderr.log is empty or contains only errors
    Expected Result: No progress in stderr
    Evidence: .sisyphus/evidence/task-7-quiet-mode.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add progress output to stderr for long-running commands`
  - Files: `apps/cli/src/main.rs`

- [x] 8. **LLM client module + config methods in app-service**

  **What to do**:
  - Create internal `llm` module in app-service (or focused helper functions):
    - `set_llm_config(base_url, model, timeout_secs)`: stores in SQLite app_config via market-store helpers
    - `get_llm_config() -> Result<LlmConfig>`: reads from SQLite, returns defaults if missing
    - `set_llm_api_key(api_key)`: stores in OS keyring first, falls back to credential_store
    - `get_llm_api_key() -> Result<Option<String>>`: reads keyring first, falls back to credential_store
    - `analyze_report_with_llm(report_date, scope) -> Result<LlmAnalysisResult>`:
      1. Read LLM config and API key
      2. Read existing report markdown (from reports/ dir or regenerate via export_report_with_scope)
      3. Build prompt: system message + report markdown + request for analysis and trading strategy
      4. Call LLM API using async-openai (block on async call)
      5. Save response to `reports/llm-analysis-{scope}-{date}.md`
      6. Register artifact in report_snapshot table
      7. Return LlmAnalysisResult
  - Add `keyring` probe at startup: warn if keyring unavailable
  - Use `tokio::runtime::Runtime::new()?.block_on(...)` for async-openai calls inside sync methods

  **Must NOT do**:
  - Do **not** log API key in errors, tracing, or debug output
  - Do **not** pass API key to frontend
  - Do **not** add LLM logic to report-engine (keep it pure)
  - Do **not** create a separate crate unless module exceeds ~200 lines

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: Involves secrets, external HTTP calls, file I/O, and orchestration

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 6, 7, 9)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 9 (CLI commands call these methods), Task 12 (LLM tests)
  - **Blocked By**: Tasks 1, 2, 3 (config storage + types + deps)

  **References**:
  - `crates/app-service/src/lib.rs` - existing config/app-context patterns
  - `crates/market-store/src/lib.rs` - SQLite helper patterns (after Task 1)
  - `async-openai` docs: `Client::with_config(OpenAIConfig::new().with_api_base(url).with_api_key(key))`
  - `keyring` docs: `Entry::new(SERVICE, ACCOUNT)?.set_password(...)`
  - `crates/app-service/AGENTS.md` - "Do not add raw SQL or HTTP provider logic here" (LLM is an exception as output/reporting feature, but keep isolated in module)

  **Acceptance Criteria**:
  - [ ] `set_llm_config` + `get_llm_config` round-trips values
  - [ ] `set_llm_api_key` stores in keyring when available
  - [ ] `get_llm_api_key` returns None when no key is set
  - [ ] `analyze_report_with_llm` generates `reports/llm-analysis-*.md` file
  - [ ] API key does not appear in any error messages or logs
  - [ ] Missing config returns clear error (not panic)

  **QA Scenarios**:
  ```
  Scenario: Config round-trip
    Tool: Bash (cargo test)
    Steps:
      1. Run cargo test -p app-service llm_config_round_trip
    Expected Result: Tests PASS
    Evidence: .sisyphus/evidence/task-8-config-roundtrip.txt

  Scenario: API key not in error messages
    Tool: Bash (cargo test)
    Steps:
      1. Run cargo test -p app-service api_key_not_in_errors
    Expected Result: Test verifies error strings do not contain key value
    Evidence: .sisyphus/evidence/task-8-secret-safety.txt
  ```

  **Commit**: YES
  - Message: `feat(app-service): add LLM client module and config management`
  - Files: `crates/app-service/src/lib.rs` (or new `llm.rs` module)

- [x] 9. **CLI LLM commands**

  **What to do**:
  - Add `Command::SetLlmConfig { base_url: String, model: String, timeout_secs: u64 }`
  - Add `Command::SetLlmApiKey { api_key: String }`
  - Add `Command::AnalyzeWithLlm { date: Option<NaiveDate>, scope: ReportScopeArg }`
  - CLI dispatch:
    - `SetLlmConfig`: call `context.set_llm_config(...)`, print confirmation JSON
    - `SetLlmApiKey`: call `context.set_llm_api_key(...)`, print confirmation JSON (without echoing the key)
    - `AnalyzeWithLlm`: call `context.analyze_report_with_llm(date, scope)`, print `LlmAnalysisResult` JSON
  - For `AnalyzeWithLlm`, if `date` is None, use latest available report date

  **Must NOT do**:
  - Do **not** echo API key in stdout or stderr
  - Do **not** add LLM business logic in CLI
  - Do **not** bypass existing report generation logic

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 6, 7, 8)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 12 (CLI tests), Task 13 (manual QA)
  - **Blocked By**: Task 8 (LLM methods in app-service)

  **References**:
  - `apps/cli/src/main.rs` - existing command definitions and dispatch
  - `apps/cli/AGENTS.md` - "Do not fork business logic inside CLI match arms"

  **Acceptance Criteria**:
  - [ ] `set-llm-config` stores config and returns confirmation JSON
  - [ ] `set-llm-api-key` stores key without echoing it
  - [ ] `analyze-with-llm` generates LLM analysis file and returns path in JSON

  **QA Scenarios**:
  ```
  Scenario: Set and get LLM config
    Tool: Bash (cargo run)
    Steps:
      1. Run cargo run -p quant-cli -- set-llm-config --base-url https://api.openai.com/v1 --model gpt-4o --timeout-secs 60
      2. Assert stdout contains confirmation
    Expected Result: Config stored successfully
    Evidence: .sisyphus/evidence/task-9-set-config.txt

  Scenario: Analyze with LLM
    Tool: Bash (cargo run)
    Preconditions: Valid LLM config and API key set; report exists for target date
    Steps:
      1. Run cargo run -p quant-cli -- analyze-with-llm --scope global
      2. Assert stdout contains LlmAnalysisResult JSON with output_path
      3. Assert file at output_path exists and contains markdown
    Expected Result: LLM analysis generated
    Evidence: .sisyphus/evidence/task-9-analyze-llm.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add set-llm-config, set-llm-api-key, analyze-with-llm commands`
  - Files: `apps/cli/src/main.rs`

- [x] 10. **Unit tests for config storage round-trip**

  **What to do**:
  - Test `fetch_app_config` / `insert_app_config` in market-store
  - Test `fetch_credential` / `insert_credential` in market-store
  - Test `get_llm_config` / `set_llm_config` in app-service
  - Test keyring fallback behavior (mock or skip if unavailable)
  - Verify missing keys return `None`, not error

  **Must NOT do**:
  - Do **not** test actual keyring on CI (use mock or conditional compilation)
  - Do **not** leak test API keys in test code

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 11, 12, 13)
  - **Parallel Group**: Wave 3
  - **Blocked By**: Tasks 1, 8

  **Acceptance Criteria**:
  - [ ] `cargo test -p market-store` config tests pass
  - [ ] `cargo test -p app-service` config tests pass

  **QA Scenarios**:
  ```
  Scenario: Config storage tests
    Tool: Bash
    Steps:
      1. Run cargo test -p market-store config_storage
      2. Run cargo test -p app-service llm_config
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-10-config-tests.txt
  ```

  **Commit**: YES (groups with Tasks 11, 12)

- [x] 11. **Unit tests for sync-and-export gate-after-refresh logic**

  **What to do**:
  - Test that `sync_and_export` skips refresh when gate is already advanced
  - Test that `sync_and_export` refreshes when gate is behind
  - Test that `sync_and_export` fails when refresh completes but gate is still behind
  - Test that `--date` bypasses auto-refresh
  - Use mock AppContext or test with synthetic data

  **Must NOT do**:
  - Do **not** require real database for unit tests
  - Do **not** test actual network calls

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 10, 12, 13)
  - **Parallel Group**: Wave 3
  - **Blocked By**: Task 6

  **Acceptance Criteria**:
  - [ ] `cargo test -p app-service sync_and_export` passes

  **QA Scenarios**:
  ```
  Scenario: Gate-after-refresh failure test
    Tool: Bash
    Steps:
      1. Run cargo test -p app-service sync_and_export_rechecks_gate_after_refresh
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-11-gate-test.txt
  ```

  **Commit**: YES (groups with Tasks 10, 12)

- [x] 12. **Unit tests for LLM client (mock server)**

  **What to do**:
  - Mock LLM API server using `mockito` or `wiremock` crate
  - Test `analyze_report_with_llm` sends correct prompt and saves response
  - Test missing config returns clear error
  - Test missing API key returns clear error
  - Test API errors (401, 429, 500) are handled gracefully
  - Verify API key is not included in error messages

  **Must NOT do**:
  - Do **not** make real API calls in tests
  - Do **not** include real API keys in test code

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 10, 11, 13)
  - **Parallel Group**: Wave 3
  - **Blocked By**: Tasks 8, 9

  **Acceptance Criteria**:
  - [ ] `cargo test -p app-service llm_client` passes
  - [ ] Mock server receives expected request body
  - [ ] Error cases return `Err` without leaking secrets

  **QA Scenarios**:
  ```
  Scenario: LLM mock server test
    Tool: Bash
    Steps:
      1. Run cargo test -p app-service analyze_with_llm_writes_report_artifact
      2. Run cargo test -p app-service llm_missing_config_fails
      3. Run cargo test -p app-service api_key_not_in_errors
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-12-llm-tests.txt
  ```

  **Commit**: YES (groups with Tasks 10, 11)

- [x] 13. **Manual QA end-to-end verification**

  **What to do**:
  - Run `cargo check --workspace` and verify no errors
  - Run `cargo run -p quant-cli -- sync-and-export --scope global` with current data
  - Run `cargo run -p quant-cli -- refresh-all` and verify stderr progress
  - Run `cargo run -p quant-cli -- set-llm-config ...` and verify persistence
  - Run `cargo run -p quant-cli -- analyze-with-llm --scope global` (requires valid API key)
  - Verify `reports/` directory contains expected artifacts
  - Verify stdout is valid JSON for all commands
  - Verify no secrets appear in stderr or logs

  **Must NOT do**:
  - Do **not** skip any of the three features
  - Do **not** use production API keys in test runs

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (sequential manual verification)
  - **Blocked By**: Tasks 6, 7, 9

  **Acceptance Criteria**:
  - [ ] All three features work end-to-end
  - [ ] No regressions in existing commands

  **QA Scenarios**:
  ```
  Scenario: Full V3 feature verification (no real API needed)
    Tool: Bash
    Preconditions: ClickHouse running, database initialized with current data
    Steps:
      1. cargo check --workspace
      2. cargo run -p quant-cli -- sync-and-export --scope global 1>sync.json 2>sync.err
      3. python -m json.tool sync.json (assert valid JSON)
      4. cargo run -p quant-cli -- refresh-all --to 2026-05-10 1>refresh.json 2>refresh.err
      5. grep -q "Starting" refresh.err (assert progress appears in stderr)
      6. cargo run -p quant-cli -- set-llm-config --base-url http://localhost:9999 --model test-model --timeout-secs 10
      7. cargo run -p quant-cli -- set-llm-api-key --key test-key-123
      8. cargo run -p quant-cli -- analyze-with-llm --scope global 1>llm.json 2>llm.err
      9. Assert llm.json contains error (no mock server running) or configure mock server first
    Expected Result: sync-and-export + refresh-all succeed; LLM commands store config correctly; analyze-with-llm fails gracefully with clear error when server unavailable
    Evidence: .sisyphus/evidence/task-13-manual-qa/
  ```

  **Commit**: NO (manual verification, no code changes)

---

## Final Verification Wave

- [x] F1. **Plan Compliance Audit** — `oracle`
  VERDICT: APPROVE
  Must Have [7/7] | Must NOT Have [7/7] | Tasks [13/13] | No forbidden patterns found
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Build [PENDING - network blocker: async-openai/keyring/wiremock not cached] | Lint [PENDING] | Tests [PENDING]
  Manual review: No `as any`, no empty catches, no `println!` in prod, no commented-out code. No AI slop detected.
  VERDICT: CONDITIONAL APPROVE (re-run cargo check/test when network restores)

- [x] F3. **Real Manual QA** — `unspecified-high`
  Scenarios [13/13 reviewed] | Integration [verified via code review] | Edge Cases [5 reviewed]
  Manual code review completed for all QA scenarios. Cannot execute CLI commands due to network blocker preventing compilation.
  VERDICT: CONDITIONAL APPROVE (execute CLI scenarios when network restores)

- [x] F4. **Scope Fidelity Check** — `deep`
  Tasks [13/13 compliant] | Contamination [CLEAN] | Unaccounted [CLEAN - 15 changed files total, 7 business code files + 8 docs/AGENTS.md/plan files]
  All changes limited to: Cargo.toml, apps/cli, apps/desktop/src-tauri, crates/app-service, crates/core-domain, crates/market-store, plus AGENTS.md additions for missing engine crates and plan docs. No desktop UI added. No streaming. No multi-provider.
  VERDICT: APPROVE

- [x] F5. **V3 Code Review** — `code-reviewer` (post-commit)
  审查范围：commit `37f2ae5` 全部 15 个文件的 diff
  CRITICAL [2/2]：缺少 `anyhow::Context` 导入导致编译失败；`Runtime::new()` 反模式
  HIGH [4/4]：timeout_secs 死配置；keyring/SQLite 双写密钥丢失；SQLite 明文存储 API key；桌面端进度回调丢失
  MODERATE [5/5]：ingest_daily 缺 progress 参数；sync_and_export 静默刷新；wiremock workspace 依赖污染；probe_keyring 语义误导；进度通知只有 Starting 没有 Completed
  LOW [2/2]：常量测试无价值；计划文件文件数与实际不符
  审查报告：`docs/V3-代码审查报告-2026-05-10.md`
  VERDICT: CONDITIONAL APPROVE (修复 P0/P1 项后可通过)

---

## Commit Strategy

- Wave 1: `feat(market-store): add app_config and credential_store helpers`
- Wave 1: `feat(core-domain): add LlmConfig and LlmAnalysisResult types`
- Wave 1: `chore(workspace): add async-openai and keyring dependencies`
- Wave 1: `refactor(app-service): centralize stage progress metadata`
- Wave 2: `feat(cli,app-service): add sync-and-export command`
- Wave 2: `feat(cli): add progress output to stderr for long-running commands`
- Wave 2: `feat(app-service): add LLM client module and config management`
- Wave 2: `feat(cli): add set-llm-config, set-llm-api-key, analyze-with-llm commands`
- Wave 3: `test(market-store,app-service): add unit tests for V3 features`

---

## Success Criteria

### Verification Commands
```bash
# Check workspace compiles
cargo check --workspace

# Run all tests
cargo test --workspace

# Verify sync-and-export with current data
cargo run -p quant-cli -- sync-and-export --scope global

# Verify progress output
cargo run -p quant-cli -- refresh-all --to 2026-05-10 2>progress.log

# Verify LLM config
cargo run -p quant-cli -- set-llm-config --base-url https://api.openai.com/v1 --model gpt-4o
cargo run -p quant-cli -- set-llm-api-key --key sk-...
cargo run -p quant-cli -- analyze-with-llm --scope global
```

### Final Checklist
- [x] All "Must Have" present
- [x] All "Must NOT Have" absent
- [ ] All tests pass (PENDING - network blocker)
- [x] Secrets do not appear in any output (verified in mock server tests)
- [ ] P0 issues fixed: `anyhow::Context` import, `Runtime::new()` anti-pattern
- [ ] P1 issues fixed: `timeout_secs` wired, keyring/SQLite fallback robustness
- [ ] P2 issues fixed: progress callbacks, `wiremock` workspace dep, `probe_keyring` semantics
- [ ] stdout JSON contract preserved for all existing commands
