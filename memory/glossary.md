# Glossary

- **AnalysisScope**：分析范围，当前包含 `GLOBAL`、`CN`、`HK`。
- **Market Regime**：市场状态判断结果，按 scope 持久化（GLOBAL/CN/HK 各自独立）。
- **Environment Snapshot**：环境层快照，按 scope 持久化与输出。
- **Dashboard Snapshot**：某个分析日期对应的桌面端聚合视图数据。
- **Pipeline Dates**：各阶段最新日期与完整性诊断信息。
- **前复权 / qfq**：当前统一日线口径，用于稳定 MA / MACD 等趋势指标。Eastmoney 使用 `fqt=1`，Tencent 使用 `qfq`。
- **Watchlist Breadth Proxy**：V1 中基于启用指数 / ETF 标的池的广度代理，不等同于全市场股票广度。
- **TrustSummary**：仪表板主可信度入口，汇总 freshness / data-health / provenance 的可用性判断。`data_health` 字段为 `Option`（可独立异步加载）。
- **TradingCalendar**：`core-domain` 模块，基于 `config/calendars/*.json` 静态日历提供 CN/HK 休市日判断，支持 Trading-Aware Partial Coverage。
- **non_trading_count**：TrustSummary 中因休市被排除的 symbol 数量提示。
- **data_starved**：信号生成时因缺失 regime 或 rotation 数据而使用 fallback 默认值的信号占比。由 `SignalBuildStats` 统计。
- **SignalBuildStats**：`signal-engine` 输出的构建统计，包含 `regime_missing` 和 `rotation_missing` 计数。
- **SignalSummary**：信号汇总 DTO，包含 `data_starved_count` 和 `data_starved_warning` 字段。
- **regime_missing**：信号生成时无法匹配到对应 scope/date 的 market_regime 而使用 fallback 50.0 的次数。
- **rotation_missing**：信号生成时无法匹配到 rotation_rank 而使用 fallback 40.0 的次数。
- **REGISTRATION_BOARD_INDICES**：注册制板块指数集合（科创50/100、创业板指/50），使用 22% 跳变阈值（区别于普通 Index 的 12%）。
- **Trading-Aware Partial Coverage**：门控逻辑只检查"该日期期望交易"的 symbol 是否有数据，休市 symbol 不计入 `expected_count`。
