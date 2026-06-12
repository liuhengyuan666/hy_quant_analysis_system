# Current Phase
shadow_production

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
- [Todo] [TASK-010] [In Progress] [ADR-060] Regime Ground Truth Definition — Wave 9 launched. Ground Truth being redefined from "technical patterns" to "forward return distributions".
- [Todo] [TASK-010] [TASK-080A] 13 MVP candidate factors identified. Architecture revised to multidimensional Economic Scores output. NFCI downgraded to Composite Validation Factor.
- [Todo] [TASK-011] [TASK-080B] 10 factors selected from 13 candidates. 4-table analysis (Pearson, Spearman, MI, Predictive Orthogonality). Removed IG Spread, BBB Spread, M2.
- [Todo] [TASK-012] [TASK-080C] Empirical analysis on 4 existing factors (VIX, 10Y, Dollar, FedFunds) + research-based estimates for 6 missing factors. VIX strong negative predictor, Dollar very strong for HK.
- [Todo] [TASK-013] [TASK-080D] K-means clustering analysis. 3-State recommended (Favorable/Neutral/Unfavorable) with variance ratio 0.862. Fed Funds clustering identified as bias source.
- [Todo] [TASK-014] [TASK-080E] Identified Fed Funds raw level as regime identifier (not predictive signal). 33.3% near-zero, 44.9% high. Z-score is correct metric with IG=1.005 for CN 120d.
- [Todo] [TASK-015] [TASK-080F] Implemented 252d Z-score with ±3 capping in macro-engine. Updated 2,341 ClickHouse rows. Re-ran 080C/080D. CN IG improved 0.474→0.964. Taxonomy stable.
- [Todo] [TASK-081] Integrate 6 missing factors (HY Spread, 2Y, Term Spread, SOFR, Initial Claims, NFCI). Expand from 4 to 10 factors. Re-run orthogonality and taxonomy after full factor integration. GATED until 90-day Shadow Production completes.

# Constraints
- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护。
- `TradingCalendar` 当前只覆盖 CN/HK。
- `app-service/src/lib.rs` 仍是 monolith（~796 行）。
- Eastmoney 主源从当前环境不可达，全部标的走 Tencent fallback。
- P2 turnover 修复仅影响新拉取数据，存量 ClickHouse 数据需 `ingest-daily` 回填。
- **Wave 7.5 所有结论需在 1d persistence 下重新验证，暂不基于 10d 结果做进一步决策。**
- **Production regime data refresh APPROVED for both CN and HK with confirmation_days=1.**
- **ADR-057 HK Liquidity Dominant is NOT needed. HK was never broken.**

