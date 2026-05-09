# Current Context

## 当前阶段

- 阶段：执行模式
- 当前目标：
  - 收口默认日报导出在 latest gate 落后时的 fail-loud 行为。
  - 更新 memory / 操作文档并提交推送。
  - 保持显式 `--date` 历史导出能力不受影响。

## 关键任务

- Trading-Aware 门控和 data-starved warning 已完成并推送。
- 针对默认 `export-report` 静默导出旧日报的问题，已完成本地复现、缺失阶段重跑验证和代码修复。
- 需要提交并推送本次默认导出 fail-loud 代码与文档更新。

## 当前约束

- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护或接入半自动化源。
- `signal-engine` 的 `build_signal_snapshots` 签名已变更（返回 tuple），调用点只有 `app-service` 一处，不影响其他 crate。
- `app-service/src/lib.rs` 仍然是 monolith，后续若继续扩展 calendar 相关逻辑，应考虑内部 helper 提取。

## 当前风险

- `config/calendars/*.json` 若未及时更新，可能在跨年时出现门控误判（例如 2028 年春节）。
- `TradingCalendar` 当前只覆盖 CN/HK，若后续增加 US 等市场，需要扩展 `Market` 枚举和 JSON 配置。
- `app-service/src/lib.rs` 改动量大（795 行），monolithic 结构使 review 和后续拆分更困难。
- 默认 `export-report` 现在会在 latest gate 落后时失败；自动化脚本如果依赖“总能导出一个最新可用旧日报”的旧行为，需要改为显式传 `--date`。

## 当前发现

- `cargo check` 的增量编译缓存偶尔无法正确检测 `core-domain` 的模块暴露变化（`pub mod calendar`），需要通过 `cargo clean -p core-domain` 强制刷新。
- `TradingCalendar` 放在 `core-domain`（契约层），加载逻辑放在 `app-service`，避免了 `core-domain` 引入 I/O 依赖，保持了 dependency-light 约定。
- `signal-engine` 对缺失 regime/rotation 的 fallback 默认值（50.0 / 40.0）已被暴露为 data-starved 统计，但默认值本身未改变，以保持向后兼容。
- `analyze_gap_metrics` 的全休市过滤只检查 gap 期间是否"全部"休市；如果 gap 期间只有部分日期休市，仍可能触发告警。这是有意为之——只过滤明显因长假造成的 gap。
- 本地实测旧问题时，`daily_bar` 已到 `2026-05-07`，但 `indicator_snapshot/rotation_rank` 仅到 `2026-05-05`，`market_regime/environment/strategy/signal` 仅到 `2026-04-30`；补跑缺失阶段后全部推进到 `2026-05-07`。
- Oracle 复核确认：本次修复解决的是“默认导出静默产出旧报告”的产品错误，不自动修复上游 stale stage。

## 当前已确认的功能设计方向

- Trading-Aware Partial Coverage 是 GLOBAL scope 的默认行为：只检查期望交易的 symbol。
- Trust summary 继续作为"主可信度入口"，non_trading 提示作为其证据层的一部分。
- Data health 的 gap 告警已经过滤全休市期间，减少了噪音。
- Signal 生成阶段的 data-starved 信息现在可以被下游消费（CLI 打印、SignalSummary 字段、未来前端展示）。
- 默认日报导出代表“当前最新研究快照”；若 latest gate 未推进，应 fail-loud 而不是静默导出旧日期。

## 当前执行焦点

- 当前焦点转为：提交默认导出 fail-loud 修复与文档更新。
- 下一步（若用户需要）：进一步增强 `explain-latest-gate` 的逐日期拒绝原因、data-starved 前端展示、交易日历自动维护。

## 当前最新进展

- `TradingCalendar` 模块已落地，含 `is_trading_day`、`trading_symbols`、`non_trading_symbols_count`。
- `config/calendars/` 已包含 `cn_holidays.json` 和 `hk_holidays.json`。
- `dashboard_available_dates_for_scope` 已改为期望感知门控。
- `pipeline_date_diagnostics_for_scope` 的 completeness check 已改为 trading-aware。
- `build_trust_summary` 已增加 `non_trading_count` 和对应 note。
- `analyze_gap_metrics` 已过滤全休市期间的 gap。
- `build_signal_snapshots` 已返回 `SignalBuildStats`，`compute_signals` 已输出 data-starved warning。
- `SignalSummary` 已扩展 `data_starved_count` 和 `data_starved_warning`。
- `memory/decisions.md` 和 `docs/阶段性更新-2026-05-07.md` 已更新。
- `export_report_with_scope` 现在仅在默认导出（未传 `--date`）时检查 `explain_latest_gate`；若 dashboard latest 落后于 freshest market date，则拒绝导出并输出 gate alerts。
- 已验证 `cargo check --workspace`、`export-report`、`dashboard-snapshot`、`explain-latest-gate --scope global` 均通过，当前默认日报可导出 `2026-05-07`。
