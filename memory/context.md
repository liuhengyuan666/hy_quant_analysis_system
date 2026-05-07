# Current Context

## 当前阶段

- 阶段：执行模式
- 当前目标：
  - 完成 Trading-Aware 门控优化与 data-starved warning 的文档同步。
  - 分批提交并推送代码。
  - 为后续可能的交易日历扩展（如 US 市场）保留清晰的接入入口。

## 关键任务

- P1-P4 已全部完成（TradingCalendar 模块、门控改造、诊断改造、trust 改造、gap 过滤、data-starved warning）。
- `cargo check --workspace` 已通过。
- 需要更新 `memory/decisions.md`、`memory/context.md`、`docs/阶段性更新-2026-05-07.md` 并分批提交。

## 当前约束

- 静态 JSON 日历覆盖 2024-2027，后续需要人工维护或接入半自动化源。
- `signal-engine` 的 `build_signal_snapshots` 签名已变更（返回 tuple），调用点只有 `app-service` 一处，不影响其他 crate。
- `app-service/src/lib.rs` 仍然是 monolith，后续若继续扩展 calendar 相关逻辑，应考虑内部 helper 提取。

## 当前风险

- `config/calendars/*.json` 若未及时更新，可能在跨年时出现门控误判（例如 2028 年春节）。
- `TradingCalendar` 当前只覆盖 CN/HK，若后续增加 US 等市场，需要扩展 `Market` 枚举和 JSON 配置。
- `app-service/src/lib.rs` 改动量大（795 行），monolithic 结构使 review 和后续拆分更困难。

## 当前发现

- `cargo check` 的增量编译缓存偶尔无法正确检测 `core-domain` 的模块暴露变化（`pub mod calendar`），需要通过 `cargo clean -p core-domain` 强制刷新。
- `TradingCalendar` 放在 `core-domain`（契约层），加载逻辑放在 `app-service`，避免了 `core-domain` 引入 I/O 依赖，保持了 dependency-light 约定。
- `signal-engine` 对缺失 regime/rotation 的 fallback 默认值（50.0 / 40.0）已被暴露为 data-starved 统计，但默认值本身未改变，以保持向后兼容。
- `analyze_gap_metrics` 的全休市过滤只检查 gap 期间是否"全部"休市；如果 gap 期间只有部分日期休市，仍可能触发告警。这是有意为之——只过滤明显因长假造成的 gap。

## 当前已确认的功能设计方向

- Trading-Aware Partial Coverage 是 GLOBAL scope 的默认行为：只检查期望交易的 symbol。
- Trust summary 继续作为"主可信度入口"，non_trading 提示作为其证据层的一部分。
- Data health 的 gap 告警已经过滤全休市期间，减少了噪音。
- Signal 生成阶段的 data-starved 信息现在可以被下游消费（CLI 打印、SignalSummary 字段、未来前端展示）。

## 当前执行焦点

- P1-P4 编码已完成，编译验证通过。
- 当前焦点转为：文档同步 → 分批提交 → 推送。
- 下一步（若用户需要）：交易日历的自动化维护、US 市场扩展、data-starved 的前端展示。

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
