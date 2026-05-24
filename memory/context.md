# Current Phase
- 阶段：执行模式 - 当前目标： - Oracle 数据质量复核报告问题修复（P0-P5）已全部完成并验证。 - Dashboard 性能优化已完成并提交（commit `2a4a875`）。 - Memory 体系清理与状态同步（当前会话）。

# Active Tasks
- [Todo] [TASK-000] ✅ P0：宏观因子历史回填（`compute-macro --from 2020` → 7,149 macro 行）。
- [Todo] [TASK-001] ✅ P5：修复 `fetch_market_regimes` GLOBAL-only 过滤（`regime_missing` 17,195 → 152）。
- [Todo] [TASK-002] ✅ P1：`compute-signals` 重跑（`data_starved` 52.6% → 2.9%）。
- [Todo] [TASK-003] ✅ P2：Tencent turnover 解析（`turnover: None` → `row.get(6)`，代码已修复，存量数据待 `ingest-daily` 回填）。
- [Todo] [TASK-004] ✅ P4：注册制板块指数跳变阈值差异化（科创50/100/创业板指/50 → 22% 阈值）。
- [Todo] [TASK-005] ✅ P3：HSAHP 调研（Tencent 无 K 线，Eastmoney 不可达，待用户决策）。
- [Todo] [TASK-006] ✅ 全链路验证：`pipeline-dates` 全部对齐、`dashboard-snapshot` 正常、`export-report` 成功。
- [Todo] [TASK-007] ✅ 代码提交：P0-P5 修复 (`12b17bb`) + Dashboard 性能优化 (`2a4a875`) 已分别提交。

# Constraints
- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护。
- `TradingCalendar` 当前只覆盖 CN/HK。
- `app-service/src/lib.rs` 仍是 monolith（~796 行）。
- Eastmoney 主源从当前环境不可达，全部标的走 Tencent fallback。
- P2 turnover 修复仅影响新拉取数据，存量 ClickHouse 数据需 `ingest-daily` 回填。

