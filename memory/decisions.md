## ADR-026: Memory 体系清理与状态同步

**Status:** active

### Context
当前 Memory 体系存在三个问题：1) glossary 术语不完整（19个）；2) decisions 状态需要确认；3) archive 目录结构需要初始化。

### Decision
执行 Memory 体系清理：补充 12 个缺失术语（available_dates_ms、pipeline_diagnostics、refresh_pipeline、dashboard_bundle 等），确认 25 条决策状态，初始化 archive 目录结构。

**Tags:** memory, maintenance, documentation

## ADR-027: ClickHouse 日期查询性能优化

**Status:** superseded

### Context
Dashboard 加载性能瓶颈：`available_dates_ms` 耗时 24 秒。根因是 `fetch_dashboard_available_dates` 查询使用 IN 子句和多个全表扫描子查询，且 `dashboard_available_dates_for_scope` 存在 N+1 查询问题。

### Decision
实施三层优化：1) 重写主查询使用 JOIN 替代 IN 子句并限制最近 90 天；2) 在 AppContext 中添加 AvailableDatesCache 内存缓存（TTL 5分钟）；3) 数据刷新后自动清除缓存。

**Tags:** performance, clickhouse, caching, dashboard

## ADR-028: rotation_missing 根因分析：历史窗口不足导致的预期行为

**Status:** active

### Context
signal-engine 中 840 个 rotation_missing 条目需要排查原因。

### Decision
rotation_missing 是预期行为，不是 bug。根因是 rotation-engine 在计算 rs_20 时需要至少 20 天历史数据（index >= 20），导致每个标的的前 20 天无法生成 rotation 排名。22 个标的 × 20 天 = 440 基础缺失，加上数据缺口和标的状态变化导致总数达到 840。

**Tags:** rotation, signal, data-quality, expected-behavior

## ADR-029: HSAHP 暂时禁用决策

**Status:** active

### Context
HSAHP（AH股溢价指数）数据源不可用：Eastmoney 从当前环境不可达，Tencent 无 K 线数据。当前 enabled: true 但 rows=0，产生 critical 状态告警。

### Decision
将 HSAHP 的 enabled 设置为 false。原因：1) 数据源短期内无法恢复；2) 消除 noise 和 critical 告警；3) HK scope 仍保留 HSCEI 和 HSTECH 两个标的。未来若找到替代数据源可重新启用。

**Tags:** HSAHP, data-source, HK-scope, disabled

## ADR-030: Turnover 存量回填待执行

**Status:** active

### Context
P2 turnover 修复（commit 12b17bb）后，新拉取的腾讯日线包含 turnover，但存量 814 根 bar 仍缺失 turnover。需要通过 ingest-daily 回填。

### Decision
Turnover 存量回填命令为 `cargo run -p quant-cli -- ingest-daily --from 2023-01-01`。当前环境 Docker 未运行，需要用户手动启动 Docker Desktop 后执行。回填后 liquidity_proxy_score 计算将更准确。

**Tags:** turnover, backfill, data-quality, manual-execution

## ADR-027: ClickHouse 日期查询性能优化（Oracle 复核修正）

**Status:** active

### Context
Dashboard 加载性能瓶颈：`available_dates_ms` 耗时 24 秒。根因是 `fetch_dashboard_available_dates` 查询使用 IN 子句导致双表全扫描。

### Decision
实施两层优化：1) 重写主查询使用 JOIN 替代 IN 子句，避免双表全扫描；2) 在 AppContext 中添加 AvailableDatesCache 内存缓存（TTL 5分钟），数据刷新后自动清除。Oracle 复核后移除了 90 天限制，因为它会破坏历史日期查询（dashboard-snapshot --date 和 export-report --date）。

**Tags:** performance, clickhouse, caching, dashboard, oracle-reviewed

## ADR-031: HSAHP 数据失效根因分析

**Status:** active

### Context
需要确认 HSAHP 数据失效的根本原因，以验证禁用决策是否正确。

### Decision
HSAHP 数据失效有两层原因：1) 当前环境无法访问 Eastmoney API（SSL/TLS 重协商失败）；2) 腾讯不提供 HSAHP 的 K 线数据（HSAHP 是衍生计算指数，非成分股指数）。测试了 hkHSAHP 和 hkHSHP 两种 Tencent symbol，均返回空数据。这验证了 ADR-029 禁用决策的正确性。

**Tags:** HSAHP, data-source, root-cause-analysis, eastmoney, tencent
