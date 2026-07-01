# Current Phase
shadow_production_observation

# Active Tasks
- [Todo] [TASK-000] [FROZEN] [TASK-004] P0: Regime Threshold Calibration — FROZEN. Will re-evaluate after Wave 9.
- [Todo] [TASK-001] [FROZEN] [TASK-005] P1 (GATED): Expand verifiable Skills — FROZEN pending Wave 9.
- [Todo] [TASK-002] [FROZEN] [TASK-006] P2 (GATED): Insight Quality Evaluation framework — FROZEN pending Wave 9.
- [Todo] [TASK-003] [FROZEN] [TASK-007] P3 (GATED): Allocation Layer — FROZEN pending Wave 9.
- [Todo] [TASK-004] [FROZEN] [TASK-018] Wave 7.4: External Validation — FROZEN pending Wave 9.
- [Todo] [TASK-006] [Accepted] [ADR-056] Dual-Layer Architecture (State + Economic).
- [Todo] [TASK-007] [Rejected] [ADR-057] HK Liquidity-Dominant — **REJECTED**. Underlying evidence invalidated by ADR-059. HK alignment failure was caused by incorrect anchor selection (HSI instead of HSCEI), not by Liquidity factor dominance.
- [Todo] [TASK-008] [Accepted] [ADR-058] Persistence Simplification (confirmation_days = 1).
- [Todo] [TASK-009] [Accepted] [ADR-059] HK Anchor Symbol Fix (HSI → HSCEI).
- [Todo] [TASK-010] [TASK-080A] 13 MVP candidate factors identified. Architecture revised to multidimensional Economic Scores output. NFCI downgraded to Composite Validation Factor.
- [Todo] [TASK-081] Integrate 6 missing factors (HY Spread, 2Y, Term Spread, SOFR, Initial Claims, NFCI). Expand from 4 to 10 factors. Re-run orthogonality and taxonomy after full factor integration. GATED until 90-day Shadow Production completes.
- [InProgress] [TASK-093] Use TASK-092 explainability tools to systematically collect and analyze divergence patterns. Primary focus: StrongBuy signal + DE_RISK state combinations. Track: Symbol, Date, Signal Score, Attribution Breakdown, State, T+20/T+60/T+120 forward returns. Goal: after 90 days, determine if State Layer is too conservative. Requires evidence before any State Layer threshold changes. Method: daily run `symbol-diagnostics` and `symbol-scoreboard`, log divergence cases, compare with future returns.
- [Done] [TASK-096] Add --date support to `research srd` and `research stretch` commands. Default to latest date when --date is omitted. Enables historical date research output for validation and Quarterly Review generation.
- [Done] [TASK-097] Abstract an internal `ResearchSnapshot` model that holds SRD, Stretch, Rotation, Breadth, and Analytics results for a single (date, scope). Refactored existing research commands to build ResearchSnapshot first, then render. No new CLI.
- [Done] [TASK-098] Implemented Research Quarterly Review: `research review` aggregates SRD/Stretch/Analytics over a 90-day window and generates a Markdown report (Observation Window, SRD summary, Stretch distribution, Conditional Forward-Return Analytics, Potential ADR Candidates). Output to reports/research-quarterly-{scope}-{date}.md. This is the synthesis layer that turns Observation + Analytics into accumulated research assets.

# Constraints
- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护。
- `TradingCalendar` 当前只覆盖 CN/HK。
- `app-service/src/lib.rs` 仍是 monolith（~796 行）。
- Eastmoney 主源从当前环境不可达，全部标的走 Tencent fallback。
- P2 turnover 修复仅影响新拉取数据，存量 ClickHouse 数据需 `ingest-daily` 回填。
- **Wave 7.5 所有结论需在 1d persistence 下重新验证，暂不基于 10d 结果做进一步决策。**
- **Production regime data refresh APPROVED for both CN and HK with confirmation_days=1.**
- **ADR-057 HK Liquidity Dominant is NOT needed. HK was never broken.**

