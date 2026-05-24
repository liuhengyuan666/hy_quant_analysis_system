# Oracle 数据质量复核报告

> 生成时间：2026-05-20  
> 复核范围：2023-01-01 至 2026-05-19 数据补全后的质量状态  
> 复核依据：代码实现 + 实际运行结果 + 外部数据验证

---

## 执行摘要

当前系统的核心质量缺陷是宏观因子历史仅覆盖 **2025-04-14 之后**，导致 `build_market_regimes` 对更早日期执行 `continue` 跳过，约 **50% 的信号（17,195/34,238）** 在 regime 缺失下以中性默认值 **50.0** "盲飞"。同时 Tencent fallback 硬编码 `turnover: None`（`data-ingestion` 第 287 行），系统性压低 liquidity proxy。2025-04-14 之后的最新决策链路仍可用，但 2023-2024 年历史信号与回测的可信度已被大幅削弱。

---

## 已确认问题清单（按优先级排序）

### P0 — 宏观因子历史覆盖缺口（Critical）

| 维度 | 详情 |
|------|------|
| **严重性** | Critical（结构性缺陷） |
| **影响范围** | 2023-01-01 至 2025-04-13 的全部 regime / environment / strategy_state 缺失；17,195 个 signal (50.2%) 使用 regime fallback 50.0 |
| **根因代码** | `crates/macro-engine/src/lib.rs` 第 119-148 行：`resolve_group` 使用 `history.range(..=date).next_back()` 进行 as-of 查找。若 factor_history 中无该日期之前的数据，返回 `None`，外层执行 `continue` 跳过整个日期，不生成 regime 行。 |
| **数据佐证** | `check-data-health` 显示宏观因子仅覆盖 2025-04-14 之后；`compute-signals` 报告 `regime_missing=17,195`。 |
| **修复建议** | 1. 重新执行 `compute-macro --from 2020-01-01 --to 2026-05-19`，确保 FRED 拉取并持久化完整宏观历史。  <br>2. 执行后验证 `market-store` 中 `macro_snapshot` 表的 `first_date` 是否早于 2023-01-01。  <br>3. 若 FRED 响应仍被截断，检查 `fetch_fred_series` 的网络层或 `market-store::insert_macro_snapshots` 的日期过滤逻辑。 |
| **预期效果** | regime_missing 从 17,195 降至接近 0；2023-2024 年的 regime/environment/strategy_state 正常生成。 |

---

### P1 — Data-Starved Signal 比例过高（High）

| 维度 | 详情 |
|------|------|
| **严重性** | High |
| **影响范围** | 18,035 / 34,238 信号 (52.6%) 使用 fallback 默认值；regime_missing=17,195 (主要由 P0 导致)，rotation_missing=840 |
| **根因代码** | `crates/signal-engine/src/lib.rs` 第 75-88 行：regime 缺失时 `market_regime_score = 50.0`，`stats.regime_missing += 1`；rotation 缺失时 `rotation_score = 40.0`，`stats.rotation_missing += 1`。最终信号权重为 strategy 0.45 + alignment 0.15 + regime 0.20 + rotation 0.20，regime fallback 50.0 会系统性拉低/改变最终分数。 |
| **数据佐证** | `compute-signals` 输出：`Data-starved signals detected: 18035/34238 signals used fallback defaults (regime_missing=17195, rotation_missing=840)`。 |
| **修复建议** | 1. 修复 P0 后重新运行 `compute-signals`，预计 regime_missing 大幅下降。  <br>2. 对剩余的 840 个 rotation_missing 下钻到具体 symbol-date，检查是否为特定标的日线缺口或 `rotation_engine` 最小历史长度不足。 |
| **预期效果** | data-starved 比例从 52.6% 降至 <5%（仅余正常边界缺失）。 |

---

### P2 — Turnover 系统性缺失（Medium-High）

| 维度 | 详情 |
|------|------|
| **严重性** | Medium-High |
| **影响范围** | 所有通过 Tencent fallback 获取的 bar 缺失 turnover；影响 `liquidity_proxy_score`（权重 30%），进而影响 `environment_score` 和策略状态判断 |
| **根因代码** | `crates/data-ingestion/src/lib.rs` 第 287 行：`fetch_tencent_daily_bars` 硬编码 `turnover: None`。当 Eastmoney 主源探测失败、Tencent fallback 成功时，该标的全批 turnover 缺失。  <br>`crates/macro-engine/src/lib.rs` 第 225-228 行：`liquidity_proxy_score = volume_expansion_pct * 0.7 + turnover_coverage_pct * 0.3`。turnover 缺失时 `turnover_coverage_pct` 降低，liquidity proxy 被系统性压低。 |
| **数据佐证** | `check-data-health` 显示所有 ETF 类标的（如 159611、159915、510300 等）全部 814 根 bar 缺少 turnover；指数类标的缺少 572 根。 |
| **修复建议** | 1. 调研腾讯接口是否返回成交额字段，若存在则在 `fetch_tencent_daily_bars` 中补充解析逻辑（替换第 287 行）。  <br>2. 若腾讯接口无 turnover，在 `compute_participation_point` 中对 turnover 缺失标的降低权重或标记为 estimate，避免 liquidity proxy 被系统性压低。  <br>3. 长期方案：在 `daily_bar` 表中增加 `provider_source` 字段，追踪每条 bar 的实际来源。 |
| **预期效果** | liquidity proxy 计算更准确；environment_score 不再被 Tencent fallback 日期系统性拉低。 |

---

### P3 — HSAHP（AH股溢价指数）无数据且缺乏 Fallback（Medium）

| 维度 | 详情 |
|------|------|
| **严重性** | Medium |
| **影响范围** | HK scope 的 rotation / signal / breadth 样本量减少 1/3（HK 共 3 个标的：HSCEI、HSAHP、HSTECH） |
| **根因代码** | `config/universe.json`：HSAHP 的 `tencent_symbol: null`，Eastmoney 探测失败时无 fallback。`check-data-health` 确认其 `rows=0`。 |
| **数据佐证** | `check-data-health` 显示 HSAHP 状态为 `critical`， notes: "Eastmoney 当前探测失败或无返回"。 |
| **修复建议** | 1. 验证 Eastmoney 的 `100.HSAHP` secid 是否仍有效；尝试手动访问接口确认。  <br>2. 若数据源已不可用且无替代 provider，考虑寻找新数据源（如直接计算 AH 溢价）。  <br>3. 若短期内无法修复，可将其 `enabled` 设为 `false` 以消除 noise，但需知这会进一步减少 HK scope 样本量（从 3 个降至 2 个）。 |
| **预期效果** | 消除 HSAHP 的 critical 状态；HK scope 数据完整性恢复或明确降级。 |

---

### P4 — 可疑大波动日误报（Low-Medium）

| 维度 | 详情 |
|------|------|
| **严重性** | Low-Medium（噪声问题，不阻塞核心功能） |
| **影响范围** | 数据健康检查中产生误报，分散运营注意力 |
| **根因代码** | `crates/app-service/src/lib.rs` 第 858-861 行：`analyze_jump_metrics` 对 Index 使用统一阈值 **12%**，对 ETF 使用 **15%**。未考虑科创板、创业板等成分股实行 20% 涨跌停制度的指数，其单日波动上限远高于传统 10% 板块。 |
| **数据佐证** | `check-data-health` 标记：科创50 最大 17.88%、创业板50 最大 20.00%、创业板指 17.25%、恒生科技 17.16%。  <br>**外部验证**：经 Librarian 核实，2024-09-24 政策底后，科创50 于 9/30 单日涨 17.88%、创业板50 于 10/8 单日涨 18.56%、创业板指 10/8 涨 17.25%，均为真实市场事件，非数据异常。 |
| **修复建议** | 1. 对科创50、科创100、创业板指、创业板50 等注册制板块指数，将 `analyze_jump_metrics` 阈值从 12% 上调至 **22%**。  <br>2. 保持上证50、沪深300 等宽基指数 12% 阈值不变。  <br>3. 可选：增加已知真实极端行情日期白名单，避免重复告警。 |
| **预期效果** | 消除真实极端行情下的误报，减少运营噪音。 |

---

## 潜在风险与注意事项

1. **FRED 历史可用性风险**：若 FRED 确实无法提供 2025-04-14 之前的某 series（极不可能，FRED 数据通常可回溯数十年），则需寻找替代宏观源，工作量会大幅扩大。
2. **Tencent / Eastmoney 前复权差异**：Eastmoney（`fqt=1`）与 Tencent（`qfq`）的前复权算法可能不同，混用会在价格序列中引入微小断层，长期可能影响 MA/MACD 稳定性。建议监控同一标的两种来源的价格差异。
3. **HSAHP 禁用副作用**：若直接将其 `enabled` 设为 `false`，HK scope 标的从 3 个降至 2 个，rotation 和 breadth 的代表性进一步下降。
4. ** regime_stale_days 正常性**：`regime_as_of_date` 滞后于 `report_date` 是正常的（宏观因子发布时间滞后），`trust_summary` 中的 `regime_stale_days` 会对此进行标注，不应视为数据质量问题。

---

## 修复后验证清单

执行完上述修复后，建议按以下步骤验证：

```bash
# 1. 补全宏观历史
cargo run -p quant-cli -- compute-macro --from 2020-01-01 --to 2026-05-19

# 2. 重新计算信号
cargo run -p quant-cli -- compute-signals

# 3. 检查 pipeline 完整性
cargo run -p quant-cli -- pipeline-dates
# 预期：所有 stage 的 is_latest=true 且 is_complete=true

# 4. 数据健康检查
cargo run -p quant-cli -- check-data-health
# 预期：无 critical 项，review 项显著减少

# 5. 查看最新 dashboard
cargo run -p quant-cli -- dashboard-snapshot
# 预期：regime_as_of_date 接近 report_date，trust_summary 为可信

# 6. 导出最新日报验证
cargo run -p quant-cli -- export-report
```

---

## 附录：关键代码位置速查

| 逻辑 | 文件 | 行号范围 |
|------|------|---------|
| 宏观因子 as-of forward-fill / 缺失跳过 | `crates/macro-engine/src/lib.rs` | 119-148 |
| Signal regime/rotation fallback | `crates/signal-engine/src/lib.rs` | 75-88 |
| Strategy 各项 fallback 默认值 | `crates/strategy-engine/src/lib.rs` | 36-129 |
| Tencent turnover 硬编码 None | `crates/data-ingestion/src/lib.rs` | 287 |
| Provider fallback 触发逻辑 | `crates/data-ingestion/src/lib.rs` | 293-323 |
| Gap / Jump 检测阈值 | `crates/app-service/src/lib.rs` | 833-861 |
| 健康状态分类逻辑 | `crates/app-service/src/lib.rs` | 879-904 |
| Liquidity proxy 计算公式 | `crates/macro-engine/src/lib.rs` | 225-228 |
| TradingCalendar 休市过滤 | `crates/core-domain/src/calendar.rs` | 6-34 |
| SignalBuildStats 结构定义 | `crates/core-domain/src/lib.rs` | 192-196 |

---

*报告由 Oracle 复核生成，基于代码静态分析、实际运行输出和外部市场数据验证。*
