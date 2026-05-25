# Learnings - v4 Research Cognition Layer

## 2026-05-25: research-context crate bootstrapped
- Workspace Cargo.toml uses version.workspace / edition.workspace / license.workspace / authors.workspace for all crates
- Crate ordering in workspace members is alphabetical
- core-domain crate is dependency-light: only chrono + serde
- FeatureEngine trait uses Box<dyn FeatureComputer> via builtin_features() factory
- All types must derive Serialize + Deserialize for JSON support
- Compilation clean: 0 LSP diagnostics, cargo check passes

## 2026-05-25: research-skills Wave 2.4 - SkillExecutor + provider/token/deterministic
- `async-trait` crate required for `LlmProvider` trait with async methods (workspace uses stable Rust, no nightly)
- `SkillExecutor` renders prompts in 4 layers: system → semantic (JSON) → reasoning (YAML) → final rendered
- `research_context::ResearchContext` is the context type used across router and executor - derives Serialize/Deserialize
- `Skill` struct already has `definition`, `overview`, `reasoning`, `output_format` - maps cleanly to executor layers
- `cargo check -p research-skills` triggers cascading rebuilds of `core-domain → backtest-engine → report-engine → research-context → research-skills` due to dependency chain
- 0 LSP diagnostics, 0 compiler warnings after cleanup

## 2026-05-25: Oracle C2 - Wired SkillExecutor into analyze_with_skill
- Rust does NOT allow struct/enum definitions or trait impls inside `impl AppContext` blocks - must be at module level
- `PlaceholderProvider` placed before `impl AppContext` to keep code organized
- Pattern for bridging sync→async in sync-only contexts (CLI): `tokio::runtime::Runtime::new()` + `block_on()`
- app-service already had `tokio.workspace = true` in deps, but CLI needed `tokio` added explicitly
- `analyze_with_skill` uses full module paths for non-re-exported types: `research_skills::token_budget::TokenBudget`, `research_skills::deterministic::DeterministicConfig`, `research_skills::executor::SkillExecutor`
- Re-exported types use short paths: `research_skills::RegimeStateMachine`, `research_skills::registry::SkillRegistry`, `research_skills::router::SkillRouter`
- `async-trait` added to app-service Cargo.toml (already present in research-skills)

## 2026-05-25: liquidity-shock skill created (Wave 2.5)
- Skill YAML front matter requires: name, description, version, author, trigger (all/any/none/weight), inputs, outputs, dependencies, confidence_model (base + factors array), failure_modes (condition/action/message), evaluation_metrics, output_schema, priority
- Reasoning Graph uses YAML fenced block with steps containing: inputs, checks, outputs (and optionally states/transitions for state machines)
- schema.json uses draft-07 JSON Schema with required fields array and enum-constrained string properties
- Error Handling section uses a markdown table with Condition/Action/Result columns
- Dependencies section cross-references other skills (market-regime-reasoning, breadth-analysis)
- The `previous_state` field in schema is NOT marked required (unlike regime-state which includes it in the required array) - the market-regime-reasoning schema does use previous_state as optional

## 2026-05-25: sector-rotation skill created
- Followed exact same format as market-regime-reasoning (YAML front matter + reasoning graph + execution instructions + output format + error handling + dependencies)
- Reasoning graph has 4 steps: momentum_analysis → factor_analysis → rotation_detection → sector_allocation
- Rotation types: momentum_rotation, value_rotation, quality_rotation, defensive_rotation, no_rotation
- Transition rules map between rotation types based on factor thresholds
- Schema uses nested objects for leading_sectors/lagging_sectors (each with sector + score) and factor_analysis (dominant_style + strength/alignment enums + crowding_alert)
- Dependencies section is informational only (no hard dependencies) but cross-references market-regime-reasoning and liquidity-shock as commonly paired skills

## 2026-05-25: macro-linkage skill created (海外宏观联动)
- Followed same pattern as market-regime-reasoning: YAML front matter → Overview → Reasoning Graph (fenced YAML) → Execution Instructions → Output Format → Error Handling → Dependencies
- Reasoning Graph has 3 steps: spread_analysis (spread_10y + dxy_index), flow_analysis (foreign_flow + vix), linkage_detection (composite alignment)
- schema.json required fields: linkage_signal, spread_analysis, dxy_analysis, flow_analysis, recommendation, confidence
- flow_analysis.schema has nested supporting_factors for vix_signal/vix_value
- Error Handling covers: stale data (>5 days), null spread/dxy (abort), null flow (partial analysis), null VIX (substitute ~20)
- Dependencies section cross-references market-regime-reasoning and liquidity-shock

## 2026-05-25: factor-composite skill created (量化因子复合信号)
- Followed same pattern as market-regime-reasoning: YAML front matter → Overview → Reasoning Graph (fenced YAML) → Execution Instructions → Output Format → Error Handling → Dependencies
- Reasoning Graph has 3 steps: factor_normalization (4 raw factors → normalized), weight_calculation (inverse volatility → risk parity weights), composite_signal (blend → signal + contributions)
- Trigger section uses both `all` (regime.confidence >= 0.6) and `none` (breadth.breadth_pct < 20) guards
- Crowding factor is inverted: lower crowding → higher normalized score
- Weight cap: no single factor exceeds 40% of total weight
- Recommendations enum: overweight_risk, neutral, defensive, high_quality, avoid_crowded
- Dependencies: market-regime-reasoning (regime gating), rotation-engine (raw factor inputs), breadth analysis (extreme condition guard)



