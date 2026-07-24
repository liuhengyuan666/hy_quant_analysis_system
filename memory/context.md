# Current Phase
rv1_capability_consolidation

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
- [Todo] [TASK-100] Regime Attribution Study (Failure Classification): Build a failure taxonomy for Signal/State divergences (e.g., Liquidity Trap, Crowding, Theme Rotation, Macro Shock, Breadth Collapse, Momentum Exhaustion). Track each Shadow Production divergence case and classify it. Goal is to accumulate a Failure Knowledge Base, not to improve Signal Accuracy.
- [Todo] [TASK-103] Expand Historical Replay Coverage: Run Historical Replay across more scopes, windows, and conditions after Failure Attribution framework is established. Gated by TASK-100/TASK-101. Goal is to transform additional samples into knowledge, not just increase sample count.
- [Todo] [TASK-111] V8 Research Asset Workspace foundation: implement WorkspaceManager, unified RA-XXXXXX identity (ADR-081), unified lifecycle (ADR-080), Evidence/Snapshot writers, and registry indexes (evidence-index.json, snapshot-index.json). P0 (real Evidence from research explain/analytics/review), P1 (workspace), P2 (Snapshot referencing Evidence) are complete. P3 (Evidence Score/Weight) is gated until 1000+ assets, 30-day replay stability, and 2-cycle calibration stability.
- [Todo] [TASK-114] 4-week Research Asset accumulation sprint: run Historical Replay daily for GLOBAL/CN/HK with a 90-day window, writing Evidence Assets to workspace/evidence/replay/. Target is 1000+ Research Assets before re-evaluating P3 (Evidence Score/Weight).
- [InProgress] [TASK-119] Feature Layer: implement QuoteSnapshot, IntradayFeatures, and FeatureExtractor
- [Todo] [TASK-120] Observation Layer: implement IntradayObservation and ObservationEngine
- [Todo] [TASK-121] Evidence Layer: implement Evidence, EvidenceKind, EvidencePayload, and EvidenceBuilder
- [Todo] [TASK-122] Assessment Layer: implement ExecutionAssessment and AssessmentEngine
- [Todo] [TASK-124] Execution Replay: record ExecutionRequest→Decision→Outcome and feed Research Assets
- [Todo] [TASK-125] Explanation Layer: implement ExecutionExplanation in report-engine for CLI/Desktop/PDF/LLM consumers
- [Todo] [TASK-130] Replay Engine: consume ExecutionEvent and record outcomes (T+20/T+60/MFE/MAE) for Research Asset calibration
- [Todo] [TASK-131] Research Asset Integration: write ExecutionEvent to V8 workspace as durable Research Asset
- [Todo] [TASK-132] Report Engine: build ExecutionExplanation from ExecutionEvent in report-engine (not execution-engine)
- [Todo] [TASK-133] LLM Explanation: consume ExecutionExplanation via LLM, never ExecutionEvent or raw engine internals
- [InProgress] [TASK-140] Real-data validation: run 20-50 historical cases to verify ExecutionEvent can explain decisions
- [Todo] [TASK-030] [TASK-160.4] Add EvidenceValidationRecord to EvidenceDescriptor so every Evidence Asset carries provenance: dataset scope, horizon, sample size, precision, lift, validated_at, report reference. This turns the Evidence Registry from a manual status table into a traceable Research Asset Registry.
- [Todo] [TASK-208] Frontend Phase 2 (REVISED scope per ADR-108, supersedes TASK-207): Strategy Perspectives view. GATED on RV1 Real Usage Observation Window (5-10 trading days) — observation outcome decides form: standalone panel (research-level entry, NOT Dashboard home) OR downgraded SignalDetailModal Tab. Design: persona cards (动量交易者/价值投资者 etc. with stance + score + drivers), NOT scoreboard number table. Backend: Serialize derives + named sub-structs for strategy_perspectives structs, AppContext thin wrappers. Tauri: 2 thin commands. Frontend: scoreboard list eager, attribution lazy on click (recompute cost), zero investment semantics in UI.

# Constraints
- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护。
- `TradingCalendar` 当前只覆盖 CN/HK。
- `app-service/src/lib.rs` 仍是 monolith（~4460 行），7 个 helper 模块（core, trust, breadth, dashboard, llm, sync, config_loader）已拆分，但高层编排仍集中在 lib.rs，后续可进一步拆分。
- Eastmoney 主源从当前环境不可达，全部标的走 Tencent fallback。
- P2 turnover 修复仅影响新拉取数据，存量 ClickHouse 数据需 `ingest-daily` 回填。
- **Wave 7.5 所有结论需在 1d persistence 下重新验证，暂不基于 10d 结果做进一步决策。**
- **Production regime data refresh APPROVED for both CN and HK with confirmation_days=1.**
- **ADR-057 HK Liquidity Dominant is NOT needed. HK was never broken.**
- **Shadow Production 操作指引见 `docs/shadow-production-playbook.md`**。90 天观察期；每日 `research srd/stretch` + `symbol-diagnostics`；每周 `symbol-scoreboard` + `research analytics`；每季度 `research review`。触发 kill criteria 时停止并提交 ADR review。

