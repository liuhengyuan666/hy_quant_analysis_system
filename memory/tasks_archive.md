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

