# Archived Tasks

## Completed
# Archived Tasks

## 2026-06-02
- [Done] [TASK-010] ✅ V4 Research Cognition Layer 完整实现（6 Waves + Oracle D1-D4 修复）
- [Done] [TASK-001] Wave 1: ResearchContext + ContextBuilder + FeatureEngine
- [Done] [TASK-002] Wave 2: Skill 基础设施（Registry/Router/Executor/Provider）
- [Done] [TASK-003] Wave 3: market-regime-reasoning 完整链路
- [Done] [TASK-004] Wave 4: 6 个 Skills（liquidity-shock, sector-rotation, macro-linkage, factor-composite, volatility-tail）
- [Done] [TASK-005] Wave 5: AgentProfile（macro-strategist, risk-manager, technical-analyst）
- [Done] [TASK-006] Wave 6: 结构化输出 + Desktop 集成 + V3 迁移
- [Done] [TASK-007] Branch: `v4`（31 个提交，已推送 origin）
- [Superseded] [TASK-008] Wave 7.1: Ground Truth Audit & Label Redesign — inspect class distribution, transition matrix, duration persistence; redesign GT using market-state dimensions instead of future returns
  - Superseded by: ADR-053
  - Reason: Ground Truth generation redesigned with independent GT-Predictor architecture
- [Done] [TASK-000] ✅ P0：宏观因子历史回填（`compute-macro --from 2020` → 7,149 macro 行）。
- [Done] [TASK-001] ✅ P5：修复 `fetch_market_regimes` GLOBAL-only 过滤（`regime_missing` 17,195 → 152）。
- [Done] [TASK-002] ✅ P1：`compute-signals` 重跑（`data_starved` 52.6% → 2.9%）。
- [Done] [TASK-003] ✅ P2：Tencent turnover 解析（`turnover: None` → `row.get(6)`，代码已修复，存量数据待 `ingest-daily` 回填）。
- [Done] [TASK-004] ✅ P4：注册制板块指数跳变阈值差异化（科创50/100/创业板指/50 → 22% 阈值）。
- [Done] [TASK-005] ✅ P3：HSAHP 调研（Tencent 无 K 线，Eastmoney 不可达，待用户决策）。
- [Done] [TASK-006] ✅ 全链路验证：`pipeline-dates` 全部对齐、`dashboard-snapshot` 正常、`export-report` 成功。
- [Done] [TASK-007] ✅ 代码提交：P0-P5 修复 (`12b17bb`) + Dashboard 性能优化 (`2a4a875`) 已分别提交。
- [Done] [TASK-011] Phase 1: Vue 3 脚手架 + breadth-ma30 试点组件（1周）
- [Done] [TASK-012] Phase 2: 逐面板迁移 14 个 Vue 组件（3-4周）
- [Done] [TASK-013] Phase 3: 响应式网格布局 + 简约风格优化（1-2周）
- [Done] [TASK-020] Phase 0: Vue migration pre-i18n (delete dead code, extract hero, migrate 3 plain JS slices, eliminate commitRender)
- [Done] [TASK-021] Phase 1-4: Implement i18n with vue-i18n@11 (infrastructure, message extraction, language toggle, polish)
- [Done] [TASK-022] Phase 2: i18n - dashboard-utils.js fallback strings + date/number formatting locale integration
- [Done] [TASK-023] Phase 3: i18n - backend-originated strings + main.js export messages
- [Done] [TASK-024] 前端布局调整：修复顶部空白、DateSelector 全宽并排、Hero 加宽、TimeContext 独占一行、Backtest/Signals 分行、LanguageToggle 移至 Help/Usage 区域
- [Done] [TASK-025] 信号/轮动面板中文名称显示：SignalsPanel symbol_names 映射 + RotationPanel hover tooltip
- [Done] [TASK-026] Schema-evolution 修复：Oracle 评审后移除 `name` ghost field，改为 `DashboardSnapshot.symbol_names` HashMap，补充 serde(default) 文档与 AGENTS.md 策略
- [Done] [TASK-048] 前端 LLM 智能分析面板集成：3 个 Tauri 命令 + 2 个 Vue 组件 + store 扩展 + i18n + Oracle 评审后修复（路径、placeholder、XSS、DTO、死代码、事件桥接）
- [Done] [TASK-049] Oracle 二次复核：修复 i18n key mismatch、missing keys、dead import、export filename case、skill fallback i18n、unit tests（11 tests）。Commit `7bb65f1` 已推送 origin/v4。Oracle verdict: VERIFIED。

## 2026-06-05
- [Superseded] [TASK-009] Wave 7.1-A: implement inspect-ground-truth CLI command — class distribution, transition matrix, duration distribution, regime persistence, episode statistics
  - Superseded by: ADR-053
  - Reason: Ground Truth generation redesigned with independent GT-Predictor architecture

- [Superseded] [TASK-014] Wave 7.4: External Validation — compare generated regimes against VIX/HSI/HSCEI/CSI300 historical regime indicators
  - Superseded by: ADR-053
  - Reason: Ground Truth generation redesigned with independent GT-Predictor architecture

- [Superseded] [TASK-010] Wave 7.2-A: RegimeObservation Layer — define factual dimension structs (trend/breadth/liquidity/volatility) independent from existing regime engine
  - Superseded by: ADR-053
  - Reason: Ground Truth generation redesigned with independent GT-Predictor architecture

- [Superseded] [TASK-011] Wave 7.2-B: Regime State Machine — map RegimeObservation → RegimeCandidate with explicit state rules
  - Superseded by: ADR-053
  - Reason: Ground Truth generation redesigned with independent GT-Predictor architecture

- [Superseded] [TASK-012] Wave 7.2-C: Persistence Filter — min_days + confirmation_days + transition smoothing to prevent churn
  - Superseded by: ADR-053
  - Reason: Ground Truth generation redesigned with independent GT-Predictor architecture

- [Superseded] [TASK-013] Wave 7.3: RegimeAuditReport — episode distribution, state coverage, transition matrix, stability score
  - Superseded by: ADR-053
  - Reason: Ground Truth generation redesigned with independent GT-Predictor architecture


### 2026-06-06
- [Done] [TASK-015] Wave 7.3B: Market-State Extractor — build semantic observation layer (TrendObservation, LiquidityObservation, VolatilityObservation, BreadthObservation[Optional]) from OHLCV/Indicators, completely independent from macro-engine scores

- [Done] [TASK-016] Wave 7.3C: GT Regime Generator — Candidate State → Persistence Filter two-phase state machine, map MarketStateObservation → GT Regime Label

- [Done] [TASK-017] Wave 7.3D: Regime Audit — Persistence Score (avg_episode>20d, median>15d, churn<5%, stability>0.9) + Coverage Score (imbalance_ratio<5)

- [Done] [TASK-019] Wave 7.3E: Historical Replay Audit — run complete GT chain (market-state-extractor → gt-regime-generator → regime-audit) on 2018-2026 historical data and output RegimeAuditReport


### 2026-06-17
- [Done] [TASK-090A] 量化统计最近两年每次状态切换的触发原因分布。读取 quant.market_regime + quant.environment_snapshot + quant.strategy_state，重新评估所有历史日期的 build_strategy_state 转移归因，输出：
1. 触发条件分布表（NoTrade/DeRisk/ConfirmAdd/FullTrend 各自由哪个条件触发，占比）
2. 阈值悬崖频率表（trend∈[50,65) 的日期，对应状态分布）
3. 阈值临近表（trend∈[53,57] 的 ConfirmAdd vs DeRisk 分裂比例）
4. state_score vs 实际状态混淆矩阵
零写入、零阈值改动，纯只读审计。输出为 JSON + markdown 报告到 reports/state-transition-attribution-{scope}.md。


### 2026-06-18
- [Done] [TASK-082] 基于 Oracle 复核的三个问题，完成 V5 并行化优化的修复与验证：

- [Done] [TASK-094] Phase A-E 代码级结构优化：清理 9 个编译器 warning，提取 apply_persistence 到 regime-audit/src/common.rs（6 副本），拆分 market-store 为 14 域模块，拆分 app-service 为 6 helper 模块，拆分 CLI main.rs 为 9 命令模块。A类基础设施改动，不触碰 B类禁止项。验证：cargo check 全 workspace 通过，68/68 测试通过。已推送到 v5 分支（9 commits）。

- [Done] [TASK-095] README CLI 命令文档重组：消除 section 8 与 section 13 的命令重复，新增 40 个审计/验证命令文档（5 组分类），补全 dashboard-dates / status / run-backtest 参数等缺失文档。A类基础设施改动。已推送到 v5 分支（2 commits，c8555de / 15df83c）。

- [Done] [TASK-096] app-service 已拆分为 7 个模块（core, breadth, dashboard, sync, trust, llm, config_loader），但 lib.rs 仍保留 4,083 行 AppContext 高层编排，需要进一步拆分。约束项更新：移除 'app-service/src/lib.rs 仍是 monolith（~796 行）'，改为跟踪本任务。
- [Done] [TASK-018] cargo check 全 workspace 通过


### 2026-06-25
- [Done] [TASK-123] Research Layer Refactor — delete Skill Framework, converge to 5 Prompt + Markdown. 41 files changed, 795 insertions(+), 4681 deletions(-).

- [Done] [TASK-083] Expanded universe.json from 18 to 25 symbols. Added: 000510 (中证A500), 512480 (半导体ETF), 562500 (机器人ETF), 159995 (芯片ETF), 513050 (中概互联网ETF), 515790 (光伏ETF), 000922 (中证红利). Removed duplicates: disabled 159915, 510300, 513130 (ETF duplicates of existing indices). Historical backfill: 2017-01-01 to 2026-06-17, ~23,000 rows. Tencent pagination fix (TRAP-005) and ClickHouse partition fix (TRAP-004) were discovered during this task. Full pipeline refresh completed: 25/25 symbols complete.

- [Done] [TASK-120] Execution Layer Foundation (Pattern Library) Phase 1. Pre-close analysis filtering based on real-time market data. Delivered, tested, and deployed. Includes: pattern matching for StrongClose, HighVolume, GapUpOverextended, VolumeSpike, FarFromMA5; state classification (BUY_NOW, NO_CHASE, WAIT, SKIP); output to reports/execution-samples/YYYY-MM-DD.json.

- [Done] [TASK-092] P0: Add symbol-diagnostics CLI command for single-symbol signal attribution breakdown. Must display: Strategy Contribution, Alignment Contribution, Regime Contribution, Rotation Contribution, Final Score. Must also show raw strategy scores (ValueLeft, TrendPullback, TrendBreakout, MomentumRight) and rotation rank. Governance constraint: Explainability Layer may explain decisions but may NOT create decisions — no new composite scores, rankings, confidence metrics, or decision signals. Shadow Production safe: zero changes to State Layer, Signal Engine, weights, thresholds, allocation, backtest, DashboardSnapshot, or ResearchContext.


### 2026-07-04
- [Done] [TASK-099] Add --date support to `research srd` and `research stretch` commands. Default to latest date when --date is omitted. Enables historical date research output for validation and Quarterly Review generation. (Assigned TASK-099 because TASK-096 is archived for app-service modularization.)
- [Done] [TASK-097] Abstract an internal `ResearchSnapshot` model that holds SRD, Stretch, Rotation, Breadth, and Analytics results for a single (date, scope). Refactor existing research commands to build ResearchSnapshot first, then render. This unifies query logic and enables Quarterly Review aggregation. No new CLI.
- [Done] [TASK-098] Design and implement Research Quarterly Review: automatically run SRD/Stretch/Analytics over a 90-day window and generate a Markdown report (Observation Window, SRD summary, Stretch distribution, top findings, potential ADR candidates). Output to reports/research-quarterly-{scope}-{date}.md. This is the synthesis layer that turns Observation + Analytics into accumulated research assets.


### 2026-07-08
- [Done] [TASK-070] V7.1 Market Evolution Layer: implement Confirmation and Recovery in core-domain::research, extend ResearchContext, add CLI commands research-confirmation and research-recovery



### 2026-07-09
- [Done] [TASK-071A] V7.2A Market Fingerprint Foundation: create crates/market-fingerprint-engine, define MarketFingerprint and MarketFingerprintBuilder, establish stable contract

- [Done] [TASK-071B] V7.2B Similarity Engine: implement normalization, distance functions, SimilarityMatcher, OutcomeProfile, and CLI research-analogues

- [Done] [TASK-072] V7.3 Research Synthesis Layer: implement Consensus with Transition in core-domain::research, extend ResearchContext, add CLI command research-consensus

- [Done] [TASK-101] ADR-078 Research Attribution Layer: Design and implement the Failure Attribution / Regime Attribution layer on top of Research Platform 1.0. Covers Macro, Breadth, Liquidity, Theme, Crowding, Volatility attribution. Read-only, does not modify frozen State/Signal/Execution layers. Output: reports/attribution/attribution-{scope}-{condition}-{from}-{to}.md.

- [Done] [TASK-102] Historical Replay Automation Pipeline: Automate the Historical Replay workflow (research-review + research-analytics + symbol-scoreboard) to run nightly and generate Candidate Evidence reports. Output to shadow-production/historical-replay/ with dated filenames and summary JSON.

- [Done] [TASK-104] Attribution MVP Implementation (P2): Implement the first 1–2 attribution dimensions (recommended: Breadth + Liquidity) using the ADR-078 framework. Validate the full pipeline: Observation → Evidence → Attribution → Hypothesis → Confidence → Next Validation. Output: a working `research explain` command for the MVP dimensions and a sample report. Do NOT implement all six attribution dimensions at once.


### 2026-07-12
- [Done] [TASK-105] Wire real Evidence into research explain (P0): Reuse existing conditional forward-return analytics (match_srd_strong, match_stretch_extreme) to populate Evidence.occurrences, positive_ratio, median_forward_return, history_window in research explain. Remove placeholder evidence.





### 2026-07-17
- [Done] [TASK-115] Architecture Freeze: draft ADR-082 Execution Platform (main ADR covering platform boundary, pipeline, replay, LLM boundary summary, evidence summary)
- [Done] [TASK-116] Supplement ADR: draft ADR-083 Execution Evidence Model (typed payload, EvidenceKind, Observation→Evidence conversion)

- [Done] [TASK-117] Supplement ADR: draft ADR-084 LLM Boundary in Execution Platform (LLM roles and anti-roles)

- [Done] [TASK-118] DTO Freeze: define ExecutionRequest, ExecutionMarketView, ExecutionPolicy, Evidence, ExecutionAssessment, ExecutionDecision DTOs in Rust

- [Done] [TASK-126] Milestone 1: Execution Pipeline Closed Loop (DTO → Feature → Observation → Evidence → Assessment → Decision) complete with tests and clean compilation
- [Done] [TASK-127] Architecture Review: verify no God Object, no circular deps, no DTO leakage, no ResearchContext leakage, no Presentation in Domain, no Magic Numbers, Policy boundaries intact before entering Phase 2 (Replay/Explanation/LLM)

- [Done] [TASK-128] ExecutionEvent DTO: define ExecutionEvent as the single source of truth for all downstream consumers (Replay, Research Asset, Report, LLM)

- [Done] [TASK-129] End-to-End Execution Pipeline: compose FeatureExtractor→ObservationEngine→EvidenceBuilder→AssessmentEngine→DecisionEngine into ExecutionEvent

- [Done] [TASK-134] Architecture Gate: confirm ExecutionEvent as Canonical Output, version ExecutionEvent, add Policy hash, move Replay Contract to execution-replay crate

- [Done] [TASK-135] Replay Contract: define ExecutionReplayRecord / ExecutionOutcome / ExecutionEvaluation in execution-replay crate

- [Done] [TASK-136] ADR-085: Execution Evaluation ADR documenting Event → Outcome → Evaluation → Research Asset

- [Done] [TASK-137] OutcomeResolver + EvaluationEngine: implement RuleBasedEvaluationEngine and stub/initial MarketStoreOutcomeResolver

- [Done] [TASK-139] Architecture Gate 2: ExecutionEvent Sufficiency Review + add market_regime_label + bump schema to v2.1

- [Done] [TASK-138] First Replay Validation: run one historical replay pass to verify ExecutionEvent carries enough information for Evaluation

- [Done] [TASK-141] Golden Validation Suite: 10 historical cases covering Decision Boundary + YAML loader + README

- [Done] [TASK-142] 10-Case Manual Review: run Validation CLI for each suite case and verify Acceptance Checklist

- [Done] [TASK-143] First real Golden Suite validation run: 8 PASS, 2 FAIL, root cause identified as State evidence overweight

- [Done] [TASK-144] Expanded validation report: Golden Suite + 9,083 candidate discovery records across CN/Global, identified over-conservatism and zero Reduce decisions

- [Done] [TASK-145] 2A-1 Restore Fact Lineage: wire real market_regime_label from ResearchContext.market_state.label into ExecutionEvent with Unknown fallback, 3 regression tests, ADR-082 Rule-15 added

- [Done] [TASK-146] 2A-2 Execution Statistics: implement ExecutionStatistics domain object with six frozen outputs, JSON/Markdown formatter, run Golden Suite / Representative Sample / Full Dataset

- [Done] [TASK-147] 2A-3 Evidence Trace: implement EvidenceTrace/Funnel module to identify per-stage survival of each EvidenceKind across Pipeline layers, run on full CN dataset to find where Reduce evidence dies

- [Done] [TASK-148] 2A-4 Decision Path Review: implement Distribution Coverage Review and Decision Margin Review diagnostic tools, run on full CN dataset, document findings without modifying pipeline code


### 2026-07-18
- [Done] [TASK-149] 2A-4.5 Decision Gate Analysis: implement tool to enumerate Reduce candidates and report which DecisionEngine gate blocks them, run on full CN dataset, document findings

- [Done] [TASK-150] 2A-4C Risk Semantics Review: implement analysis tool for RiskLevel::High records, output evidence composition, decision context, future outcomes, and semantic mapping proposal, run on full CN dataset, document in ADR-097

- [Done] [TASK-151] 2A-5 Directional Confidence Calibration Experiment: implement calibration framework, run baseline/C1/C2/C3/asymmetric thresholds on full CN dataset, generate coverage/precision/opportunity cost metrics, document in ADR-098

- [Done] [TASK-152] 2A-6 Restore Real Volume Context: fetch real 20-day volume MA from market-store, fix volume_ma20 placeholder, re-run full Decision Path Review chain

- [Done] [TASK-153] 2B-1 Bearish Evidence Analysis: analyze the 145 bearish candidates and their evidence composition + outcome to identify which evidence combinations distinguish true exit signals from temporary risk. Output a Bearish Candidate Matrix and hypothesis set for Holding Risk Evidence.

- [Done] [TASK-153.5] TASK-153.5 RiskExpansion Coverage Exploration: determine whether RiskExpansion is scarce alpha or under-covered due to strict observation conditions. Output: observation coverage, near-miss analysis, outcome lift for near-misses, candidate generation potential. No production logic changes.

- [Done] [TASK-156] 2B-0: Implement and run a ResearchContext Fact Integrity Gate that audits all ResearchContext-derived fields in ExecutionMarketView for variance and placeholder detection. Fix breadth_pct and leadership_stability constant-placeholder issue before resuming Transition Evidence work.

- [Done] [TASK-157] Analyze LeadershipDecay signal across T+5, T+20, T+60, T+120 horizons to determine whether it is an Immediate/Short-term Exit signal or a Medium-term Holding Risk signal. Output a LeadershipDecay Research Profile.

- [Done] [TASK-159] Integrate execution-context-integrity-audit into the test/CI workflow so that any new ExecutionMarketView field must pass variance, provenance, and placeholder detection. Prevent future fact-lineage failures like the hardcoded breadth_pct/leadership_stability placeholder.

- [Done] [TASK-160.1] Validate that sustained deterioration is a stronger Holding Risk signal than single-day snapshots. Implement LeadershipDecay persistence analysis (consecutive days and velocity) as a Research Asset, run experiments on CN 2024-01-01 to 2025-06-30 at T+60 horizon, and integrate persistence into Holding Risk Bundle V2. Acceptance: sample >=300, precision >=55%, lift >=1.3, false reduce rate <40%. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.


### 2026-07-19
- [Done] [TASK-160.2A] Design LiquidityPressure as a sustained capital-pressure Research Asset (not snapshot). Definition combines turnover/volume decay, price weakness, breadth not recovering, and persistence over >=3 days. Validate at T+60 horizon on CN 2024-01-01 to 2025-06-30: sample >=30, precision >=50%, lift >=1.2. If validated, integrate into Holding Risk Bundle V3. Role: HoldingRisk, Horizon: MediumTerm. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-160.2B] Design ConfirmationDecay as a change-based (delta/velocity/persistence) Research Asset, not a snapshot. Study whether confirmation strength is continuously declining: confirmation_delta_5d, confirmation_velocity (slope), consecutive decline days, and price weakness. Validate at T+20 and T+60 on CN 2024-01-01 to 2025-06-30. If validated, integrate into Holding Risk Bundle V4 as a Confirmatory Dimension (not primary). Role: HoldingRisk/Confirmation, Horizon: ShortTerm/MediumTerm. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-160.3] Materialize EvidenceRole / EvidenceHorizon / ValidationStatus in code so Research Assets have identity. Design EvidenceDescriptor with id, role, horizon, validation_status, target_metric, dependencies, standalone_validity, decision_candidate. Register LeadershipDecay (HoldingRisk, MediumTerm, standalone), LiquidityPressure (Amplifier, MediumTerm, bundle-only), ConfirmationDecay (Confirmation, MediumTerm, requires LD+LP), BreadthDeterioration and RecoveryFailure (rejected). Add CLI to view registry and validate evidence usage. Prevent DecisionEngine from misusing non-standalone evidence. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-161] Define HoldingRiskScore (e.g., LeadershipDecay*0.5 + LiquidityPressure*0.25 + ConfirmationDecay*0.25) and validate it as a stable Research Asset at T+60. Run score bucket analysis, regime split (bullish/bearish/sideways), and walk-forward validation (train 2024, validate 2025H1). Acceptance: sample >=300, precision >=60%, lift >=1.3, cross-regime stability. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-163] Build a risk state machine around HoldingRiskScore: Risk Entry (score >= 0.75 for >= 3 days), Risk Peak (local max score), Risk Recovery (score < 0.5 for >= 3 days), Holding Period, and false alarm analysis. Validate on CN 2024-01-01 to 2025-06-30. This upgrades HoldingRiskScore from an indicator to a complete risk state machine. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-164] Validate HoldingRiskScore on 2022-2023 bear market data to confirm stability across regimes. Run calibration and risk lifecycle analysis on 2022-01-01 to 2023-12-31 CN data, then compare with 2024-2025 results. Acceptance: False Alarm < 35%, Avg T+60 Return < 0, Risk Event count >= 50, Precision decay < 30%. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-166] Design a State Risk Model that identifies 'already dangerous' market regimes, not 'deteriorating' transitions. Components: TrendBreakdown (price below MA, negative MA slope), VolatilityExpansion (ATR percentile > 70%), MarketBreadthCollapse (breadth_pct < 30%, state not delta), LiquidityStress (volume_ratio < 0.6, state not delta). Goal: classify regimes with RiskOff recall > 70% on CN 2023-01-01 to 2023-12-31. Role: RegimeRisk. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-168] Replace oversold/mean-reversion State Risk components with accelerating-decline components. New components: DowntrendAcceleration (return slope < 0 and worsening), VolatilityNegativeDrift (amplitude increase + negative return + breadth deterioration), PersistentBreadthCollapse (breadth continuously deteriorating for >= 2 days), LiquidityStress (volume continuously declining + price pressure for >= 2 days). Goal: RiskOff recall > 70% on CN 2023-01-01 to 2023-12-31. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-167] Implement Shadow Mode Runtime Wiring: use market_regime_label as State Context and HoldingRiskScore as Transition Evidence. Generate daily shadow output with date, market_regime, holding_risk_score, risk_state, transition_detected, decision_candidate, and evidence details. This is a read-only bypass; no changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy. Validate on recent dates (2026-07-01 to 2026-07-17).

- [Done] [TASK-169] Freeze the Shadow Mode boundary with a formal Shadow Deployment Contract. Define ShadowRiskAssessment struct with date, regime, holding_risk_score, evidence, lifecycle_state, simulated_action. Explicitly prohibit DecisionEngine from consuming ShadowRiskAssessment. This is the entry point for Phase 2C Shadow Validation (4-8 weeks real-market observation). No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-173] Define ValidationRequirement with min_samples, min_precision, min_lift, max_false_alarm for Evidence Assets. Add validation requirement check to EvidenceDescriptor so 'Validated' status requires meeting statistical thresholds. Unify Live Integrity Contract with Replay Integrity Contract (contract-driven, not hardcoded). This is required before TASK-165 Decision Integration. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.


### 2026-07-20
- [Done] [TASK-170] Add --live mode to execution-context-integrity-gate so it validates the current day's ResearchContext -> ExecutionMarketView projection, not just historical replay data. This prevents placeholder pollution during Phase 2C Shadow Validation. The live gate must run as a precondition in shadow-validation-daily.ps1 before shadow-deployment. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-171] Add explicit [RESEARCH ONLY] warning to simulated_action in Shadow Deployment output, and rename simulated_action to research_interpretation to prevent operator misreading as actionable recommendation. This is a governance fix to prevent bypassing the DecisionEngine consumption prohibition. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-172] Implement ShadowValidationStatus with NORMAL/INSUFFICIENT_EVENTS/ACTIVE states. Define explicit protocol for '0 Transition Detection events for N consecutive weeks' (e.g., after 20 trading days with 0 events, enter INSUFFICIENT_EVENTS state). This provides monitoring for the Shadow Validation phase and prevents misinterpretation of zero-event periods. No changes to ObservationEngine/EvidenceBuilder/AssessmentEngine/DecisionEngine/ExecutionPolicy.

- [Done] [TASK-154] 2B-2 Holding Risk Evidence Design: design new exit-specific evidence kinds (e.g., BreadthDeterioration, LeadershipLoss, RecoveryFailure) as Research Assets, validate against historical replay, and do not wire into DecisionEngine until precision criteria are met.

- [Done] [TASK-155] 2B-3 Calibration v2: after Holding Risk Evidence is validated, re-run the full Decision Path Review chain and Directional Confidence Calibration. Only propose threshold/policy changes if Reduce precision reaches ≥50% in replay.

- [Done] [TASK-158] Combine multiple medium-term holding risk signals (LeadershipDecay, BreadthDeterioration, LiquidityDeterioration) into a Holding Risk Evidence Bundle. Evaluate at T+60 natural horizon, not T+20. Output a Holding Risk Score profile as a Research Asset.


### 2026-07-21
- [Done] [TASK-200] RV1 Phase 1: CLI减法 + 重命名 + Integrity集成。107命令→~10核心；refresh-all→market-refresh; preclose-analysis→portfolio-decision; ExecutionState语义变更(BuyNow→Increase); 新增daily-analysis/strategy-perspectives/evidence-status/validation-check; README/操作手册重写; 设计规划-rv1.md落地

- [Done] [TASK-203] RV1 Phase 1.5: 工程卫生 — execution-replay 19文件变体引用更新；删除deprecated枚举变体；audit.rs/research.rs/diagnostics.rs dead_code处理；CLI三级分类(15个工程命令hide=true)；cargo check workspace零warning

- [Done] [TASK-204] RV1 Phase 1.8: Domain Model Freeze ADR-105 — 冻结MarketRegimeSnapshot/EnvironmentSnapshot/Evidence/PortfolioDecision四个现有对象；禁止新建MarketState；daily-analysis契约固定；Phase 2边界写死(允许消费已有分数/场景加权/归因输出，禁止新策略/新指标/新Evidence)

- [Done] [TASK-201] RV1 Phase 2: 策略引擎重构 — signal-engine不再合并四策略为单一分数，独立产出每套策略信号+归因；新增config/scenarios.toml场景配置(短线动量/长线价值/激进)；SignalSnapshot扩展strategy_signals/scenario_scores字段；strategy-perspectives完整实现

- [Done] [TASK-202] RV1 Phase 3: LLM增强 + 组合决策重构 — LLM上下文增强(多策略矛盾点+历史参照+连续性)；对话历史持久化(LlmAnalysisRecord)；config/prompts.toml可定制分析人格(短线交易员/长线配置者)；portfolio-decision用LLM替代3个硬编码Pattern


### 2026-07-24
- [Done] [TASK-205] Frontend Phase 0: ExecutionState contract sync. Fix ExecutionResultsPanel.vue (STATE_ORDER/STATE_META/i18n for INCREASE/MAINTAIN/AVOID/REDUCE/SKIP), main.js preclose notification counts (~L395), zh.json/en.json state labels + notification strings. Root cause: serde rename_all SCREAMING_SNAKE_CASE serializes new variant names; alias is deserialize-only.

- [Done] [TASK-206] Frontend Phase 1: LLM portfolio_review entry + markdown renderer upgrade. Add portfolio_review as 6th action in LlmAnalysisPanel.vue actions array + zh/en i18n keys. Replace hand-rolled renderMarkdown() with marked library (secure config: no raw HTML / sanitize). Zero backend change — analyze_with_llm already dispatches portfolio_review.



### 2026-07-25
- [Done] [TASK-209] [market-adversarial-lens] build_snapshot_context() 增强：在 research-skills/src/action.rs 中注入 6 个已计算字段（liquidity_score, regime_stale_days, breadth_5d_delta, volume_expansion_pct, turnover_coverage_pct, bottom_rotation）到 LLM 上下文。Environment 衍生字段走 if-let guard，None 时显示 N/A。Plan: .omo/plans/market-adversarial-lens.md Task 1

- [Done] [TASK-210] [market-adversarial-lens] 新增 market_adversarial_lens persona prompt：在 config/prompts.toml 中写入完整 system + template（5 维博弈分析框架 + web search 引导词 + ADR-106 边界约束）。Plan: .omo/plans/market-adversarial-lens.md Task 2

- [Done] [TASK-211] [market-adversarial-lens] 前端按钮 + i18n：LlmAnalysisPanel.vue actions 数组新增第 7 个条目，zh.json/en.json 新增 research.marketAdversarialLens 键。Plan: .omo/plans/market-adversarial-lens.md Task 3

- [Done] [TASK-212] [market-adversarial-lens] 文档更新：research-skills/AGENTS.md action 表格新增 persona 行，根 AGENTS.md --action 参数列表追加。Plan: .omo/plans/market-adversarial-lens.md Task 4

- [Done] [TASK-213] [market-adversarial-lens] CLI QA：cargo check 零新增 warning + cargo test 三 crate 全绿 + 3 个 persona 调用验证（market_adversarial_lens/market_story/short_term_trader）。Plan: .omo/plans/market-adversarial-lens.md Task 5

- [Done] [TASK-214] [market-adversarial-lens] 前端 QA：Playwright 验证按钮渲染（7 个按钮）、中英文 locale 切换、点击触发 loading 状态。Plan: .omo/plans/market-adversarial-lens.md Task 6


### 2026-07-26
- [Done] [TASK-215] [P2-shared-adversarial] T1: llm_history.rs 新增 adversarial_context_section(record, level) 分级段落构建器（full=全文/summary=摘要）+ 假设背景头部 + 单元测试。Plan: .omo/plans/market-adversarial-shared-layer.md

- [Done] [TASK-216] [P2-shared-adversarial] T2: core-domain InjectLevel 枚举 + AdversarialSection + config_loader ResolvedLlmConfig 解析 + llm.toml.example schema。默认 auto_inject=true，默认分级映射。Plan Task 2

- [Done] [TASK-217] [P2-shared-adversarial] T3: lib.rs analyze_with_action 集成——签名加 adversarial: Option<InjectLevel>，ensure_adversarial_context() 前置逻辑（新鲜度/落盘/降级），分级注入，递归防护硬编码，返回 JSON 加 adversarial 诊断字段。Plan Task 3

- [Done] [TASK-218] [P2-shared-adversarial] T4: CLI main.rs + commands/llm.rs 新增 --adversarial <full|summary|none> flag 并透传。Plan Task 4

- [Done] [TASK-221] [P2-shared-adversarial] T7: 文档更新——README 共享层说明 + --adversarial 用法；research-skills/AGENTS.md CONVENTIONS 条目；app-service/AGENTS.md WHERE TO LOOK 行。Plan Task 7

- [Done] [TASK-220] [P2-shared-adversarial] T6: CLI QA 五场景——S1 默认开启首次注入落盘 / S2 复用无二次调用 / S3 explain_decision 收 summary 级 / S4 CLI 覆盖 none 与 full / S5 adversarial 自身无递归。证据落盘 .omo/evidence/p2-*。Plan Task 6

- [Done] [TASK-219] [P2-shared-adversarial] T5: Tauri analyze_with_llm 加可选 adversarial 参数 + tauri.js 第三参 + LlmAnalysisPanel 注入级别选择器（默认 full）+ 双语 i18n。Plan Task 5



### 2026-09-03
- [Done] [TASK-119] Feature Layer: implement QuoteSnapshot, IntradayFeatures, and FeatureExtractor

- [Done] [TASK-140] Real-data validation: run 20-50 historical cases to verify ExecutionEvent can explain decisions

- [Done] [TASK-121] Evidence Layer: implement Evidence, EvidenceKind, EvidencePayload, and EvidenceBuilder

- [Done] [TASK-130] Replay Engine: consume ExecutionEvent and record outcomes (T+20/T+60/MFE/MAE) for Research Asset calibration






- [Done] [TASK-122] Assessment Layer: implement ExecutionAssessment and AssessmentEngine

## Superseded

### 2026-07-09
- [Superseded] [TASK-071] V7.2 Historical Evidence Layer: create crates/market-fingerprint-engine, implement MarketFingerprint, Historical Analogues, and Outcome Profile, add CLI command research-analogues
  Superseded by: Task TASK-071A
  Reason: V7.2 split into V7.2A (Market Fingerprint Foundation) and V7.2B (Similarity Matcher + Outcome + CLI)









### 2026-07-12
- [Superseded] [TASK-106] Elevate Evidence Index to unified Research Layer asset (P1): Create research/evidence/ directory structure with replay/, calibration/, attribution/, validation/ subdirectories. Update run-historical-replay.ps1 to write replay evidence to research/evidence/replay/. Create a unified evidence-index.json aggregator at research/evidence/evidence-index.json.
  Superseded by: Task TASK-111
  Reason: Unified workspace.rs approach replaced the standalone evidence directory structure; identity/lifecycle/indexing are now handled by WorkspaceManager.

- [Superseded] [TASK-107] Add Evidence Strength and Evidence Weight to ADR-078 and implementation (P2): Define EvidenceStrength (Weak/Moderate/Strong/Verified) based on historical occurrence count. Define EvidenceWeight placeholder for multi-source aggregation (Replay/Calibration/Manual Review). Update ADR-078, core-domain, report-builder, and research explain output.
  Superseded by: ADR ADR-079
  Reason: P2 evolved from Evidence Strength/Weight into Snapshot structure with EvidenceRef. Evidence weighting is deferred to P3 after asset accumulation.

- [Superseded] [TASK-108] Research Snapshot Replay (P3, long-term): Design and implement saving/replaying full Research Snapshots (Observation + Evolution + Evidence + Consensus) per date. Historical Replay becomes a producer of Research Snapshots, not just condition analytics. Output design doc or ADR-079.
  Superseded by: ADR ADR-079
  Reason: Snapshot structure is now P2 (ADR-079). Research Snapshot Replay as P3 is delayed until 1000+ assets, 30-day replay stability, and 2-cycle calibration stability.
























































### 2026-07-24
- [Superseded] [TASK-207] Frontend Phase 2 (GATED until Phase 0+1 real-usage observation): StrategyPerspectivesPanel. Backend: add Serialize derives to StrategyPerspectiveEntry/StrategyAttributionView/StrategyPerspectiveDetail + AppContext thin wrappers (strategy_scoreboard, strategy_attribution). Tauri: 2 thin commands. Frontend: persona-card UI (per-strategy cards with score + drivers, not scoreboard table), scoreboard list loads first, attribution fetched lazily on click (recomputation cost). Keep SignalsPanel unchanged.
  Superseded by: Task TASK-208
  Reason: 范围按 ADR-108 修订：增加观察窗门控、禁止 Dashboard 首页入口、允许降级为 SignalDetailModal Tab















### 2026-07-26
- [Superseded] [TASK-208] Frontend Phase 2 (REVISED scope per ADR-108, supersedes TASK-207): Strategy Perspectives view. GATED on RV1 Real Usage Observation Window (5-10 trading days) — observation outcome decides form: standalone panel (research-level entry, NOT Dashboard home) OR downgraded SignalDetailModal Tab. Design: persona cards (动量交易者/价值投资者 etc. with stance + score + drivers), NOT scoreboard number table. Backend: Serialize derives + named sub-structs for strategy_perspectives structs, AppContext thin wrappers. Tauri: 2 thin commands. Frontend: scoreboard list eager, attribution lazy on click (recompute cost), zero investment semantics in UI.
  Superseded by: Task TASK-209
  Reason: 观察窗结论落地：用户高频需要四策略视角，按 ADR-108 规则定为独立 research 级面板；同时纳入 ADR-112 共享博弈层的边界说明






### 2026-09-03
- [Superseded] [TASK-124] Execution Replay: record ExecutionRequest→Decision→Outcome and feed Research Assets
  Superseded by: Task TASK-222
  Reason: ExecutionRequest→Decision→Outcome recording and replay are already implemented in execution-engine v2/execution-replay; only the gated WorkspaceManager persistence residual remains, now tracked narrowly by TASK-222 under RV1 boundaries.

- [Superseded] [TASK-131] Research Asset Integration: write ExecutionEvent to V8 workspace as durable Research Asset
  Superseded by: Task TASK-222
  Reason: Replaced by the uniquely identified, RV1-scoped and explicitly gated ExecutionEvent-to-Research-Asset persistence task TASK-222.

- [Superseded] [TASK-125] Explanation Layer: implement ExecutionExplanation in report-engine for CLI/Desktop/PDF/LLM consumers
  Superseded by: ADR ADR-106
  Reason: RV1 abandoned the report-engine ExecutionExplanation object in favor of deterministic Decision Facts consumed only by rightmost LLM explanation personas.

- [Superseded] [TASK-132] Report Engine: build ExecutionExplanation from ExecutionEvent in report-engine (not execution-engine)
  Superseded by: ADR ADR-106
  Reason: RV1 no longer requires report-engine to build ExecutionExplanation from ExecutionEvent; explanation consumes frozen deterministic facts under ADR-106.

- [Superseded] [TASK-133] LLM Explanation: consume ExecutionExplanation via LLM, never ExecutionEvent or raw engine internals
  Superseded by: ADR ADR-106
  Reason: The V8 ExecutionExplanation consumer design was replaced by RV1 llm-analyze personas that explain deterministic Decision Facts and never decide.
