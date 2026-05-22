# Current Context

## 当前阶段

- 阶段：执行模式
- 当前目标：
  - Oracle 数据质量复核报告问题修复（P0-P5）已全部完成并验证。
  - 待提交代码变更（P2/P4/P5 代码修复 + PROJECT_STRUCTURE.md + memory 更新）。

## 关键任务

- ✅ P0：宏观因子历史回填（`compute-macro --from 2020` → 7,149 macro 行）。
- ✅ P5：修复 `fetch_market_regimes` GLOBAL-only 过滤（`regime_missing` 17,195 → 152）。
- ✅ P1：`compute-signals` 重跑（`data_starved` 52.6% → 2.9%）。
- ✅ P2：Tencent turnover 解析（`turnover: None` → `row.get(6)`，代码已修复，存量数据待 `ingest-daily` 回填）。
- ✅ P4：注册制板块指数跳变阈值差异化（科创50/100/创业板指/50 → 22% 阈值）。
- ✅ P3：HSAHP 调研（Tencent 无 K 线，Eastmoney 不可达，待用户决策）。
- ✅ 全链路验证：`pipeline-dates` 全部对齐、`dashboard-snapshot` 正常、`export-report` 成功。
- ⏸️ 待提交：P2/P4/P5 代码变更 + PROJECT_STRUCTURE.md + memory 更新 → git commit。

## 当前约束

- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护。
- `TradingCalendar` 当前只覆盖 CN/HK。
- `app-service/src/lib.rs` 仍是 monolith（~796 行）。
- Eastmoney 主源从当前环境不可达，全部标的走 Tencent fallback。
- P2 turnover 修复仅影响新拉取数据，存量 ClickHouse 数据需 `ingest-daily` 回填。

## 当前风险

- `config/calendars/*.json` 跨年时可能出现门控误判。
- HSAHP 数据源不可用，HK scope 仅剩 2 标的（HSCEI + HSTECH）。
- 152 个残余 `regime_missing` 为边缘日期边界效应，840 个 `rotation_missing` 待下钻。
- 长期走 Tencent fallback 时前复权算法差异（Eastmoney `fqt=1` vs Tencent `qfq`）可能引入价格序列微小断层。

## 当前发现

- **P5 为 Oracle 报告未覆盖的关键 bug**：`fetch_market_regimes` 硬编码 `WHERE market = 'GLOBAL'`，导致 CN/HK scoped 信号自 Phase 2 落地以来一直使用 regime fallback 50.0。修复后信号质量大幅提升。
- `compute-signals` 的 `fetch_market_regimes` 调用点与其他 scope-aware 路径（`fetch_latest_market_regime_on_or_before`）使用不同 fetch 函数，前者未做 scope 过滤遗漏。
- P2 修复后需重跑 `ingest-daily` 才能让存量 bar 拥有 turnover，当前所有 ETF 全 814 根 bar 缺 turnover。
- P4 修复验证有效：科创50/100/创业板指/50 的 `suspicious_jump_count` 全部归零。

## 当前已确认的功能设计方向

- Trading-Aware Partial Coverage：GLOBAL scope 默认只检查期望交易的 symbol。
- Trust summary 作为主可信度入口，non_trading 提示作为其证据层。
- 默认日报导出 fail-loud：latest gate 落后时拒绝，需显式 `--date`。
- Signal/regime scope 语义：per-scope regime 计算已落地，但信号引擎的 regime fetch 必须返回全 scope。

## 当前执行焦点

- 当前焦点：代码变更已全部就绪。下一步：
  1. （可选）`ingest-daily --from 2023-01-01` 回填 turnover 存量数据。
  2. 提交 P2/P4/P5 代码变更 + PROJECT_STRUCTURE.md + memory 更新。
  3. HSAHP 处置决策（`enabled: false` 或寻找替代源）。
  4. 840 个 rotation_missing 逐个 symbol-date 下钻。

## 当前最新进展

- `TradingCalendar` 模块已落地。
- `compute-macro --from 2020` 成功回填宏观历史。
- `fetch_market_regimes` 修复，全 scope regime 加载。
- `compute-signals` 重跑，`data_starved` 从 52.6% → 2.9%。
- `analyze_jump_metrics` 注册制板块阈值差异化。
- `fetch_tencent_daily_bars` turnover 解析。
- 全链路验证全部通过：`pipeline-dates`（9/9 阶段对齐）、`dashboard-snapshot`（report_date=2026-05-19）、`export-report`（产出 daily-report-2026-05-19.md）、`check-data-health`（P4 生效）。
- `PROJECT_STRUCTURE.md` 已创建（根目录结构说明书）。
- `memory/structure.md` 已更新至最新状态。
