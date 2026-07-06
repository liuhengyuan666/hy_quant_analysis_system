# Reporting Output Inventory

> 目标：梳理当前所有报告输出，统一命名，识别重复/冲突字段，为 V6 Reporting Layer 的 Contract 设计提供事实依据。
> 范围：CLI 命令输出、Tauri/Desktop 输出、文件导出输出。

---

## 1. 输出总览

| 输出名称 | 入口 | 当前位置 | 输出格式 | 消费者 |
|---|---|---|---|---|
| Daily Report | `export-report` / `sync-and-export` | `crates/report-engine/src/lib.rs` | Markdown | 用户、文件系统 |
| Concise Daily Report | `export-report --concise` | `crates/research-renderer/src/lib.rs` | Markdown | 用户、文件系统 |
| Dashboard Snapshot | `dashboard-snapshot` / Tauri | `crates/report-engine/src/lib.rs` | JSON | Desktop、CLI |
| Research SRD | `research srd` | `apps/cli/src/commands/research.rs` | Markdown | 用户 |
| Research Stretch | `research stretch` | `apps/cli/src/commands/research.rs` | Markdown | 用户 |
| Research Analytics | `research analytics` | `apps/cli/src/commands/research.rs` | Markdown | 用户 |
| Research Review | `research review` | `apps/cli/src/commands/research.rs` | Markdown | 用户、文件系统 |
| Rotation Ranking | `rotation-ranking` / `audit rotation-ranking` | `apps/cli/src/commands/audit.rs` | Markdown | 用户 |
| Symbol Scoreboard | `symbol-scoreboard` / `audit symbol-scoreboard` | `apps/cli/src/commands/audit.rs` | Markdown | 用户 |
| State Audit | `audit state-audit` | `apps/cli/src/commands/audit.rs` | Markdown/JSON | 用户 |
| Signal Divergence | `audit signal-divergence` | `apps/cli/src/commands/audit.rs` | Markdown | 用户 |
| Research Context | `research-context` | `crates/research-context/src/lib.rs` | JSON | GPT/LLM |
| LLM Analysis | `analyze-with-llm` | `crates/research-skills/` | Markdown | 用户、文件系统 |

---

## 2. 各输出字段映射

### 2.1 Daily Report / Dashboard Snapshot

来源：`crates/report-engine/src/lib.rs::DashboardSnapshot`

| 字段 | 类型 | 语义 | V6 归属 |
|---|---|---|---|
| `scope` | String | 分析范围 GLOBAL/CN/HK | Production Surface（冻结） |
| `report_date` | String | 报告日期 | Production Surface（冻结） |
| `latest_available_date` | String | 最新可用日期 | Production Surface（冻结） |
| `regime_as_of_date` | String | 宏观数据截止日期 | Production Surface（冻结） |
| `regime_stale_days` | i64 | 宏观数据滞后天数 | Production Surface（冻结） |
| `regime_label` | String | 当前市场状态标签 | `MarketStateSummary` |
| `trend_score` | f64 | 趋势评分 | `MarketStateSummary` |
| `liquidity_score` | f64 | 流动性评分 | `BreadthSummary` / `MarketStateSummary` |
| `risk_score` | f64 | 风险评分 | `MarketStateSummary` |
| `top_rotation` | Vec<RotationRankSnapshot> | 排名前 5 轮动 | `RotationSummary` |
| `bottom_rotation` | Vec<RotationRankSnapshot> | 排名后 5 轮动 | `RotationSummary` |
| `top_signals` | Vec<SignalSnapshot> | 排名靠前信号 | `SignalSummary` |
| `bullish_signals` | Vec<SignalSnapshot> | 看多信号 | `SignalSummary` |
| `defensive_signals` | Vec<SignalSnapshot> | 防御信号 | `SignalSummary` |
| `environment` | Option<EnvironmentSnapshot> | 市场环境层 | `BreadthSummary` |
| `strategy_state` | Option<StrategyStateSnapshot> | 策略状态 | `MarketStateSummary` |
| `trust_summary` | Option<TrustSummary> | 信任摘要 | `TrustSummary` |
| `watchlist_breadth` | Option<WatchlistBreadthSnapshot> | 观察列表广度 | `BreadthSummary` |

### 2.2 Research SRD

来源：`apps/cli/src/commands/research.rs::ResearchSnapshot`

| 字段 | 语义 | V6 归属 |
|---|---|---|
| `date` | 分析日期 | `ResearchContext.date` |
| `signals` | 当日信号列表 | `SignalSummary` |
| `state` | 当前策略状态 | `MarketStateSummary` |
| `states_history` | 历史状态序列 | `MarketStateSummary`（V6 仅保留当日） |
| `rotations` | 当日轮动排名 | `RotationSummary` |
| `env` | 当日环境快照 | `BreadthSummary` |
| `signal_history` | 历史信号序列 | `SignalSummary`（V6 仅保留当日） |

### 2.3 Research Stretch

来源：`apps/cli/src/commands/research.rs::ResearchSnapshot` 派生计算

| 字段 | 语义 | V6 归属 |
|---|---|---|
| Crowding Level | 前 5 动量集中度 | `RotationSummary` |
| Breadth Level | 广度水平 | `BreadthSummary` |
| Momentum Level | 动量水平 | `RotationSummary` |
| Leverage Level | 杠杆水平（当前硬编码 Normal） | `SignalSummary` / `StretchSummary`（V6 不实现） |
| Overall Level | 综合拉伸等级 | `StretchSummary`（V6 不实现，由 Section Builder 计算） |

### 2.4 Research Review

来源：`apps/cli/src/commands/research.rs`

| 字段 | 语义 | V6 归属 |
|---|---|---|
| SRD 分布 | 窗口内背离天数分布 | `DivergenceSummary` |
| Stretch 等级分布 | 窗口内拉伸等级分布 | `StretchSummary`（V6 不实现） |
| 条件前向收益 | 条件收益统计 | `AnalyticsSummary`（V6 不实现） |

### 2.5 Audit 命令输出

来源：`apps/cli/src/commands/audit.rs`

| 输出 | 字段 | V6 归属 |
|---|---|---|
| `rotation-ranking` | rank, symbol, momentum_score, rs_60, rs_120 | `RotationSummary` |
| `symbol-scoreboard` | symbol, final_score, signal_label, state, rs_60 | `SignalSummary` + `RotationSummary` |
| `state-audit` | state 分布、transition 矩阵 | `MarketStateSummary` |
| `signal-divergence` | 背离样本列表 | `DivergenceSummary` |

### 2.6 Research Context（既有 crate）

来源：`crates/research-context/src/semantic_state.rs`

| 字段 | 语义 | 备注 |
|---|---|---|
| `market.current_state` | 当前状态 | 与 `regime_label` 同义，命名不同 |
| `market.confidence` | 状态置信度 | 从 trend/liquidity/risk 分数计算 |
| `breadth.condition` | 广度条件 | Strong/Weakening/Collapsed |
| `breadth.breadth_pct` | 广度百分比 | 与 `environment.breadth_pct` 同义 |
| `rotation.state` | 轮动状态 | Broad/Concentrated/Divergent |
| `rotation.top_sectors` | 领涨板块 | 与 `top_rotation` 同义 |
| `signals.bullish_count` | 看多信号数 | 与 `bullish_signals.len()` 同义 |
| `signals.defensive_count` | 防御信号数 | 与 `defensive_signals.len()` 同义 |

---

## 3. 命名冲突与统一建议

### 3.1 同义不同名

| 概念 | 当前多个名称 | 建议统一为 |
|---|---|---|
| 市场状态 | `regime_label`, `market.current_state`, `state_label` | `market_state.label` |
| 广度百分比 | `breadth_pct`, `environment.breadth_pct`, `breadth.breadth_pct` | `breadth.pct` |
| 轮动领头 | `top_rotation`, `rotation.top_sectors` | `rotation.top` |
| 看多信号数 | `bullish_signals.len()`, `signals.bullish_count` | `signal.bullish_count` |
| 状态历史 | `states_history`, `MarketStateObservation` | V6 不纳入历史，统一用当日 |

### 3.2 建议新增的 Summary 命名

| Summary | 字段示例 | 说明 |
|---|---|---|
| `MarketStateSummary` | `label`, `trend_score`, `liquidity_score`, `risk_score`, `confidence` | 当前市场状态 |
| `BreadthSummary` | `pct`, `pct_sma5`, `delta_5d`, `condition` | 市场广度 |
| `RotationSummary` | `top`, `bottom`, `state`, `leadership_stability` | 轮动结构 |
| `SignalSummary` | `signals`, `bullish_count`, `strong_buy_count`, `average_score` | 最终信号 |
| `DivergenceSummary` | `duration`, `samples` | Signal-State 背离 |
| `TrustSummary` | `level`, `headline`, `is_data_complete` | 数据可信度 |

---

## 4. 结论与 Gate A 输入

1. **字段高度重复**：相同的概念在 DashboardSnapshot、ResearchSnapshot、ResearchContext 中以不同名称出现。
2. **核心语义层已清晰**：State / Breadth / Rotation / Signal / Divergence / Trust 是主要研究语义。
3. **V6 范围**：先统一这 6 个 Summary，不纳入历史序列（Timeline 留 V6 后续阶段）。
4. **既有冲突**：`crates/research-context` 中的命名与建议命名不同，需在 Boundary Inventory 中决定是迁移还是重命名。
