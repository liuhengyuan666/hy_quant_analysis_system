# Current Phase
shadow_production_v1

# Phase Declaration
- **Research Program**: CLOSED (2026-06-17)
- **Shadow Production v1**: OPEN (2026-06-17)
- **Frozen Baseline**: bt-20260617090905 (run_version=v1, git_commit=df9904e..., generated_at=2026-06-17 09:09:05 UTC)
- **Lock Expiry**: 2026-09-17 (90 days minimum)

# Shadow Production Rules

## A类: Allowed (Infrastructure Only)
- Monitoring, alerting, observability
- Dashboard, reporting, documentation
- Provenance infrastructure improvements
- Data health checks and pipeline diagnostics
- ADR updates and governance documentation

## B类: Forbidden (Behavior Changes)
- New factors, new weights, new thresholds
- New state machines, new classification logic, new taxonomy
- Threshold tuning, weight tuning, factor tuning
- State machine changes, economic taxonomy changes
- Backtest execution semantics changes

**Exception**: Kill Criteria activation only (S1-S3, E1-E3, A1-A3, D1-D2)

# 90-Day Observation Plan
- **Phase A (Days 1-30)**: State Layer - observe stability, transition frequency, NO_TRADE%
- **Phase B (Days 31-60)**: Economic Layer - observe 3-State distribution, Forward Return alignment, PSI
- **Phase C (Days 61-90)**: Allocation Layer - observe position sizing vs backtest, turnover, drawdown tracking

# Goal
**Not to prove the system is correct, but to find evidence of system failure.**

# Kill Criteria
| Code | Category | Description |
|------|----------|-------------|
| S1 | State | RiskOn/RiskOff/Neutral transition frequency exceeds historical 3σ |
| S2 | State | NO_TRADE or DE_RISK exceeds 30% consecutive days without macro justification |
| S3 | State | State recommends PROCEED but 100% signals are Hold/Watch for >5 days |
| E1 | Economic | Forward Return distribution violates ADR-063 variance ratio (<0.6) |
| E2 | Economic | Favorable economic state but RiskOff market regime for >10 days |
| E3 | Economic | >2 core factors become permanently unavailable or show structural breaks |
| A1 | Allocation | Position sizing violates state recommendations by >20% in backtest |
| A2 | Allocation | Live drawdown exceeds 1.5× backtest MaxDD for same window |
| A3 | Allocation | Best strategy changes from MomentumRight to ValueLeft for >30 days without macro shift |
| D1 | Data | >2 consecutive days missing market data for >50% of universe |
| D2 | Data | Eastmoney/Tencent API changes permanently breaking ingestion |

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
- [Todo] [TASK-081] [GATED] Integrate 6 missing factors (HY Spread, 2Y, Term Spread, SOFR, Initial Claims, NFCI). Expand from 4 to 10 factors. Re-run orthogonality and taxonomy after full factor integration. GATED until 90-day Shadow Production completes.
- [Todo] [TASK-082] [COMPLETED] Based on Oracle review, completed V5 parallelization fixes and verification:
  - cargo check full workspace passed
  - cargo test -p indicator-engine passed
  - cargo test -p rotation-engine passed
  - Two consecutive refresh-all --to 2026-06-16 successful
- [Todo] [TASK-090A] [COMPLETED] State Machine Attribution Audit completed (620 trading days, 2024-01-01 ~ 2026-06-16). DeRisk 50.3%, risk>60 41.8%, stress>70 33.1%, trend<55 19.7%. NoTrade(fallback) 10.8% (67 days) as only observation metric.
- [Todo] [TASK-090B] [PENDING] Shadow Production Phase A: State Layer observation (Days 1-30)
- [Todo] [TASK-090C] [PENDING] Shadow Production Phase B: Economic Layer observation (Days 31-60)
- [Todo] [TASK-090D] [PENDING] Shadow Production Phase C: Allocation Layer observation (Days 61-90)

# Constraints
- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护。
- `TradingCalendar` 当前只覆盖 CN/HK。
- `app-service/src/lib.rs` 仍是 monolith（~796 行）。
- Eastmoney 主源从当前环境不可达，全部标的走 Tencent fallback。
- P2 turnover 修复仅影响新拉取数据，存量 ClickHouse 数据需 `ingest-daily` 回填。
- **Wave 7.5 所有结论需在 1d persistence 下重新验证，暂不基于 10d 结果做进一步决策。**
- **Production regime data refresh APPROVED for both CN and HK with confirmation_days=1.**
- **ADR-057 HK Liquidity Dominant is NOT needed. HK was never broken.**
- **State Layer v1.0 FROZEN**: All thresholds and state transition logic frozen. Only implementation bug fixes allowed. No behavioral optimization.
- **Economic Layer 3-State Taxonomy FROZEN**: Favorable/Neutral/Unfavorable boundaries locked (ADR-063). Variance ratio 0.843.
- **Backtest Engine v1 FROZEN**: DeRisk=30% (not 0%), run_version=v1, git_commit tracked, generated_at tracked.
- **Kill Criteria**: S1-S3, E1-E3, A1-A3, D1-D2. Only way to break 90-day lock.

