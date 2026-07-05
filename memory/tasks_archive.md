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

