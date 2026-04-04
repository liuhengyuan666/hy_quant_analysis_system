# V2 Phase 1 详细技术设计：Per-Scope Regime + Environment Layer

## 1. 文档目标

本文档把 `设计规划-v2.md` 中 **Phase 1：环境层语义补全** 细化为可落地的工程设计，覆盖：

1. **per-scope regime**：让 `GLOBAL / CN / HK` 的 regime 真正按 scope 生成，而不是继续复用 global 语义。
2. **environment layer 正式化**：把 breadth / liquidity proxy / stress proxy 从“零散视图数据”提升为正式环境层输出。
3. **最小侵入演进**：在不做大规模 crate 重构的前提下，给 V2 Phase 2/3 留出稳定扩展点。

---

## 2. 设计输入与约束

## 2.1 规划来源

来自 `设计规划-v2.md` 的明确要求：

- `设计规划-v2.md:233-261`
  - `market_regime` 需要扩展为 `GLOBAL / CN / HK`
- `设计规划-v2.md:268-305`
  - breadth 要从观察项升级为环境层正式输入
- `设计规划-v2.md:440-457`
  - Phase 1 目标是：`per-scope regime + breadth environment integration + liquidity/stress proxy`

## 2.2 当前实现基线

当前代码状态：

- `crates/macro-engine/src/lib.rs`
  - `build_market_regimes(...)` 只输出 `market = "GLOBAL"`
- `crates/app-service/src/lib.rs:730-1017`
  - dashboard/report 已支持 scope，但 `dashboard_snapshot_with_scope(...)` 读取 regime 时仍使用 `fetch_latest_market_regime_on_or_before(...)`，未按 scope 过滤
- `crates/market-store/src/lib.rs:746-762`
  - `fetch_latest_market_regime_on_or_before(...)` 当前不接收 scope
- `crates/app-service/src/lib.rs:1199-1328`
  - watchlist breadth 仍是 dashboard 聚合期临时计算结果，未持久化为环境层数据

## 2.3 必须满足的工程约束

1. **不做大拆 crate**
   - `设计规划-v2.md:137-149` 已明确：V2 初期不应为了“看起来更高级”先大规模拆分 crate。
2. **不破坏 V1 主链路**
   - 现有 CLI / desktop / report / backtest 必须继续可运行。
3. **scope 语义要真正一致**
   - `dashboard/report` 的 scope 不能继续“日期 scoped、环境 global”。
4. **breadth 仍基于当前 tracked universe**
   - Phase 1 不扩展个股 universe。
   - 仍使用现有 `INDEX + ETF` 的 watchlist breadth proxy，但把它正式纳入 environment layer。

---

## 3. Phase 1 交付范围

## 3.1 In Scope

Phase 1 正式交付以下内容：

1. `market_regime` 从单一 `GLOBAL` 扩展为：
   - `GLOBAL`
   - `CN`
   - `HK`
2. 新增正式环境层快照：`environment_snapshot`
3. breadth 正式进入环境层：
   - `breadth_pct`
   - `breadth_pct_sma5`
   - `breadth_5d_delta`
   - `breadth_state`
4. 新增轻量 liquidity / stress proxy
5. dashboard / report / diagnostics 读取与展示 scope 对应的 regime + environment

## 3.2 Out of Scope

本阶段明确不做：

1. 股票 universe 扩展
2. 真正的 stock breadth
3. breadth 直接进入 `signal_snapshot.final_score`
4. 执行层仓位状态机
5. 回测规则改写
6. crate 级重构（例如新建 `environment-engine` crate）
7. strategy / signal / backtest 改成按 CN/HK scope 消费 regime

### Phase 1 兼容边界

为避免当前 V1 评分链路被“同日多条 regime”误伤，Phase 1 明确采用：

- `compute_strategy_preferences()`：继续读取 `GLOBAL` regime
- `compute_signals()`：继续读取 `GLOBAL` regime
- `run_backtest()`：继续消费现有全局 signal 流

也就是说：

> **Phase 1 先把 scoped regime / environment 落到 dashboard、report、diagnostics 和环境层持久化，不在本阶段扩散到策略层与回测层。**

---

## 4. 当前问题定义

## 4.1 语义错位

当前系统已存在：

- `ReportScope`
- scoped available dates
- scoped rotation / signals / breadth

但 regime 仍然只有 global 行：

- `market_regime.market = "GLOBAL"`
- scoped dashboard/report 只是把 global regime 套在 CN/HK 视图上

这会导致：

1. **CN/HK 报告的环境说明不够真实**
2. **用户会误以为 scoped 报告的 regime 已本地化**
3. **Phase 2 的策略状态机无法建立在稳定的 per-scope 环境层上**

## 4.2 环境层尚未正式存在

当前“环境”其实分散在多个位置：

- `market_regime`：粗粒度 trend / liquidity / risk
- `watchlist_breadth`：dashboard 临时视图字段
- `DataHealthSummary`：偏数据健康，不是环境信号

缺点：

1. 无统一持久化实体
2. 无法直接回溯环境层组成
3. 后续策略层无法稳定消费 environment layer

---

## 5. 核心设计决策

## 5.1 保留 `market_regime` 表，但升级为 per-scope

### 决策

继续沿用现有 `quant.market_regime` 表，不新建替代表。

### 原因

1. 现有 dashboard/report/diagnostics 已围绕该表构建
2. 变更成本最小
3. `market` 列虽然命名历史上偏“市场”，但实际可承载 `GLOBAL / CN / HK` 这类 scope 值

### 技术说明

Phase 1 不立即重命名物理列 `market`，而是：

- **逻辑语义**：把它视为 regime scope key
- **物理列名**：先保持兼容，避免大范围 SQL/serde 改动

后续若进入 Phase 4，可考虑统一重命名为 `scope`。

## 5.2 新增正式环境层表 `environment_snapshot`

### 决策

新增 ClickHouse 表 `quant.environment_snapshot` 作为环境层正式持久化输出。

### 原因

如果把 breadth / liquidity proxy / stress proxy 全部塞进 `market_regime`：

1. `market_regime` 会从“粗粒度结论表”膨胀成“环境总表”
2. dashboard/report 之外的策略层也难以清晰消费
3. 未来执行层和诊断层会继续耦合

因此 Phase 1 采用“两层输出”：

- `market_regime`：粗粒度结论层（regime）
- `environment_snapshot`：环境分解层（environment decomposition）

## 5.3 不新建 crate，只在现有模块内分层

### 决策

Phase 1 仅在现有 crate 内做模块内分层：

- `core-domain`：新增共享 scope / environment 类型
- `macro-engine`：扩展为“宏观 + regime + environment 计算入口”
- `market-store`：新增环境表读写
- `app-service`：调整编排与 dashboard/report 读取

### 原因

这与 `设计规划-v2.md:137-149` 的约束一致，先解决语义问题，再考虑 crate 拆分。

---

## 6. 领域模型设计

## 6.1 新增共享 scope 枚举

当前 `ReportScope` 只存在于 `app-service`，会造成跨 crate 的重复转换。

Phase 1 把 scope 提升到 `core-domain`：

```rust
pub enum AnalysisScope {
    Global,
    Cn,
    Hk,
}
```

### 作用

统一用于：

- regime scope
- environment scope
- dashboard/report scope
- store 层查询参数

### 兼容策略

- `app-service::ReportScope` 在 Phase 1 中删除
- CLI / Tauri 仍暴露 `global|cn|hk` 字符串接口
- 转换逻辑集中到 `core-domain` 或 `app-service` 的 thin adapter

## 6.2 扩展 `MarketRegimeSnapshot`

当前结构：

```rust
pub struct MarketRegimeSnapshot {
    pub date: NaiveDate,
    pub macro_as_of_date: NaiveDate,
    pub market: String,
    pub trend_score: f64,
    pub liquidity_score: f64,
    pub risk_score: f64,
    pub regime_label: String,
}
```

Phase 1 维持字段形状不大改，但逻辑语义调整为：

- `market` 的值只能是：`GLOBAL` / `CN` / `HK`
- `trend_score`：scope-aware
- `liquidity_score`：scope-aware
- `risk_score`：scope-aware

## 6.3 新增 `EnvironmentSnapshot`

建议新增到 `core-domain`：

```rust
pub struct EnvironmentSnapshot {
    pub date: NaiveDate,
    pub scope: String,
    pub regime_as_of_date: NaiveDate,
    pub breadth_as_of_date: NaiveDate,
    pub stress_as_of_date: NaiveDate,
    pub breadth_eligible_count: usize,
    pub breadth_above_count: usize,
    pub breadth_pct: f64,
    pub breadth_pct_sma5: Option<f64>,
    pub breadth_5d_delta: Option<f64>,
    pub breadth_state: String,
    pub volume_expansion_pct: Option<f64>,
    pub turnover_coverage_pct: Option<f64>,
    pub liquidity_proxy_score: f64,
    pub stress_proxy_score: f64,
    pub environment_score: f64,
    pub environment_label: String,
}
```

### 设计说明

- `scope`：`GLOBAL / CN / HK`
- `breadth_*`：仍基于 tracked universe proxy
- `volume_expansion_pct`：作为 Phase 1 的轻量 liquidity proxy 输入
- `turnover_coverage_pct`：衡量 liquidity proxy 的覆盖质量，同时也暴露 provider 限制
- `stress_proxy_score`：以 FRED 派生风险/压力分数为主
- `environment_score`：环境层综合分数，不替代 signal score
- `environment_label`：如 `supportive / mixed / fragile / stressed`

## 6.4 Dashboard 输出扩展

`report-engine::DashboardSnapshot` 建议新增：

```rust
pub environment: Option<EnvironmentSnapshotView>
```

其中 view struct 可做轻量裁剪，避免把所有内部字段都直接透给 UI。

建议视图字段：

- `scope`
- `environment_label`
- `environment_score`
- `breadth_pct`
- `breadth_pct_sma5`
- `breadth_5d_delta`
- `breadth_state`
- `liquidity_proxy_score`
- `stress_proxy_score`
- `regime_as_of_date`
- `breadth_as_of_date`

---

## 7. 存储设计

## 7.1 `market_regime` 表迁移策略

现有表：

```sql
CREATE TABLE quant.market_regime
(
    date Date,
    macro_as_of_date Date DEFAULT date,
    market LowCardinality(String),
    trend_score Float64,
    liquidity_score Float64,
    risk_score Float64,
    regime_label LowCardinality(String),
    updated_at DateTime DEFAULT now()
)
```

Phase 1 不改表结构，只改写入与查询语义：

- `market` 可写入 `GLOBAL / CN / HK`
- 删除时按 `(date range + market IN (...))` 删除
- 查询时必须显式带 scope

### 必改点

当前 `insert_market_regimes(...)` 中：

```sql
ALTER TABLE quant.market_regime DELETE WHERE market = 'GLOBAL' AND date BETWEEN ...
```

Phase 1 改为：

```sql
ALTER TABLE quant.market_regime DELETE WHERE market IN ('GLOBAL','CN','HK') AND date BETWEEN ...
```

或更稳妥：按本次要写入的 scope 集合构造删除条件。

## 7.2 新增 `environment_snapshot` 表

建议 DDL：

```sql
CREATE TABLE IF NOT EXISTS quant.environment_snapshot
(
    date Date,
    scope LowCardinality(String),
    regime_as_of_date Date,
    breadth_as_of_date Date,
    stress_as_of_date Date,
    breadth_eligible_count UInt32,
    breadth_above_count UInt32,
    breadth_pct Float64,
    breadth_pct_sma5 Nullable(Float64),
    breadth_5d_delta Nullable(Float64),
    breadth_state LowCardinality(String),
    volume_expansion_pct Nullable(Float64),
    turnover_coverage_pct Nullable(Float64),
    liquidity_proxy_score Float64,
    stress_proxy_score Float64,
    environment_score Float64,
    environment_label LowCardinality(String),
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (scope, date);
```

### 设计说明

1. `scope` 单独建列，不复用 `market`
2. `breadth_as_of_date` 与 `regime_as_of_date` 分离
3. `stress_as_of_date` 单独保留，便于未来接入更多外部因子
4. `environment_label` 只存粗粒度解释结论

---

## 8. 计算逻辑设计

## 8.1 计算顺序

Phase 1 后，刷新与 CLI `compute-macro` 对应的内部顺序为：

1. 拉取 FRED 因子
2. 写入 `macro_snapshot`
3. 构建 per-scope breadth 特征
4. 构建 per-scope liquidity/stress proxy
5. 生成 per-scope `market_regime`
6. 生成 per-scope `environment_snapshot`

因此 `compute_macro_regime(from, to)` 仍保留现有命令名，但逻辑上负责 **environment layer + regime layer**。

## 8.2 Scope 定义

### GLOBAL

- 标的：全启用 universe
- anchor：CN anchor + HK anchor 聚合
- breadth：CN/HK 合并后的 tracked-universe breadth

### CN

- 标的：`instrument.market == Market::Cn`
- anchor：`000300`
- breadth：CN tracked universe breadth

### HK

- 标的：`instrument.market == Market::Hk`
- anchor：`HSI`
- breadth：HK tracked universe breadth

## 8.3 Per-scope trend score

延续当前 `macro-engine` 的 anchor 趋势评分规则：

- `close > ma20 > ma60` → 85
- `close > ma20` → 65
- `close > ma60` → 50
- 否则 → 25

但 Phase 1 改成按 scope 取值：

- `GLOBAL`：`avg(cn_anchor_trend, hk_anchor_trend)`
- `CN`：`cn_anchor_trend`
- `HK`：`hk_anchor_trend`

## 8.4 Breadth 特征正式化

### 数据来源

直接复用现有：

- `daily_bar.close`
- `indicator_snapshot.ma30`

### 计算口径

对每个 scope、每个 date：

```text
breadth_pct = above_count / eligible_count * 100
```

其中：

- `eligible_count`：当日存在 `close` 且存在 `ma30` 的已启用 scope 内标的数
- `above_count`：满足 `close > ma30` 的标的数

### 派生量

- `breadth_pct_sma5`
- `breadth_5d_delta`
- `breadth_state`

### `breadth_state` 规则

沿用 V1 watchlist breadth 的确定性标签，以保持解释连续性：

- `near_local_low`
- `near_local_high`
- `improving`
- `weakening`
- `weak`
- `neutral`
- `strong`
- `unavailable`

注意：

- Phase 1 仍然**不**使用 stock breadth 专属的 `<10% / <20%` 语义标签
- 这些标签属于未来 stock breadth 版本

## 8.5 Liquidity proxy

Phase 1 的 liquidity proxy 只用当前仓库已稳定可得的数据，不引入新 provider。

### 输入

1. `daily_bar.volume`
2. `indicator_snapshot.vol_ma20`
3. `daily_bar.turnover`

### 派生特征

#### 1）`volume_expansion_pct`

定义：

```text
在 scope 内，volume > vol_ma20 的 eligible symbols 占比
```

#### 2）`turnover_coverage_pct`

定义：

```text
在 scope 内，turnover 非空的 eligible symbols 占比
```

### Liquidity proxy score

建议一期公式：

```text
liquidity_proxy_score =
    volume_expansion_pct * 0.7
  + turnover_coverage_pct * 0.3
```

原因：

1. `volume > vol_ma20` 更接近当前可交易活跃度
2. `turnover` 受 provider 缺失影响较大，先降低权重

## 8.6 Stress proxy

Phase 1 的 stress proxy 直接基于已存在 FRED 因子分数构造。

### 输入因子

- `vix`
- `dollar_index`
- `us10y`
- `fed_funds`

### 派生逻辑

延续当前趋势：

- `risk_macro_score = avg(vix_score, dollar_index_score)`
- `liquidity_macro_score = avg(us10y_score, fed_funds_score)`

其中：

- `stress_proxy_score = 100 - risk_macro_score`

说明：

- 当前 `factor_score` 已对“高压力因子”做了 `invert_score` 处理
- 因此值越高越偏支持，值越低越偏压力
- 单独保留 `stress_proxy_score` 是为了 UI / diagnostics 可解释，不是为了重新发明一套宏观体系

## 8.7 Per-scope regime 公式

Phase 1 下的 regime 不再仅由 macro + anchor 决定，而是由：

- scope trend
- macro liquidity
- scope liquidity proxy
- macro stress
- scope breadth 状态

### 计算建议

```text
regime_trend_score = scope_anchor_trend_score

regime_liquidity_score =
    macro_liquidity_score * 0.7
  + liquidity_proxy_score * 0.3

regime_risk_score =
    risk_macro_score * 0.7
  + breadth_support_score * 0.3
```

其中：

- `breadth_support_score`
  - `strong` / `near_local_high` → 75
  - `improving` → 65
  - `neutral` → 50
  - `weakening` → 35
  - `weak` / `near_local_low` → 25
  - `unavailable` → 40

### regime label

一期建议：

```text
if trend >= 60 && liquidity >= 55 && risk >= 55 => risk_on
else if trend < 40 || risk < 40 => risk_off
else => neutral
```

### 设计特点

1. 公式仍然简单、规则化
2. 与 V1 regime label 兼容
3. 但输出真正受 scope 内 breadth / liquidity proxy 影响

## 8.8 Environment score / label

环境层给出一个更细的综合分数，不替代 `market_regime.regime_label`。

### 建议公式

```text
environment_score =
    regime_trend_score * 0.35
  + breadth_level_score * 0.20
  + breadth_momentum_score * 0.15
  + liquidity_proxy_score * 0.15
  + stress_proxy_score * 0.15
```

其中：

- `breadth_level_score = breadth_pct`
- `breadth_momentum_score`
  - `>= +10 pts` → 70
  - `+3 ~ +10` → 60
  - `-3 ~ +3` → 50
  - `-10 ~ -3` → 40
  - `<= -10` → 25

### `environment_label`

- `>= 70` → `supportive`
- `55 ~ 70` → `constructive`
- `40 ~ 55` → `mixed`
- `25 ~ 40` → `fragile`
- `< 25` → `stressed`

这个 label 面向：

- dashboard 环境面板
- report 文本解释
- Phase 2 状态机的环境输入

---

## 9. 模块改造设计

## 9.1 `core-domain`

### 新增

1. `AnalysisScope`
2. `EnvironmentSnapshot`
3. 可选新增：
   - `EnvironmentLabel`
   - `BreadthState`

### 原则

- scope / environment 的核心类型必须下沉到共享域模型
- 不允许继续把 scope 只定义在 `app-service`

## 9.2 `macro-engine`

### 改造目标

从“宏观因子 + global regime builder”扩展为：

- `build_macro_snapshots(...)`
- `build_scope_market_regimes(...)`
- `build_environment_snapshots(...)`

### 建议模块内文件结构

```text
crates/macro-engine/src/
├─ lib.rs
├─ macro_scores.rs
├─ scope_regime.rs
└─ environment.rs
```

### 原则

- 仍然不直接访问数据库
- 只做纯计算

## 9.3 `market-store`

### 新增能力

1. `insert_environment_snapshots(...)`
2. `fetch_latest_environment_on_or_before(scope, date)`
3. `fetch_environment_snapshots_in_range(scope, from, to)`
4. `fetch_latest_market_regime_on_or_before(scope, date)`
5. `fetch_dashboard_available_dates(scope)` 的 regime 过滤改为带 scope

### 必改点

当前：

- `fetch_latest_market_regime_on_or_before(report_date)` 无 scope
- `fetch_dashboard_available_dates()` 只要求存在任意 `market_regime`

Phase 1 后：

- dashboard available dates 必须基于 **对应 scope 的 regime**
- CN/HK 不得再被 global regime“托底”

## 9.4 `app-service`

### 编排目标

`compute_macro_regime(from, to)` 在 Phase 1 后承担：

1. 生成 macro rows
2. 生成 per-scope regime rows
3. 生成 per-scope environment rows
4. 分别写入 `market_regime` / `environment_snapshot`

### dashboard/report 改造点

1. `dashboard_snapshot_with_scope(...)`
   - regime 查询必须按 scope
   - environment 查询必须按 scope
2. `dashboard_available_dates_for_scope(...)`
   - 必须要求 scoped regime 可用
3. `pipeline_date_diagnostics_for_scope(...)`
   - `market_regime` completeness 需按 scope 检查，不再按“当天有任意 regime 行”视为满足
4. `export_report_with_scope(...)`
   - report 中要带对应 environment 摘要

## 9.5 `report-engine`

### 新增职责

1. 承载 `EnvironmentSnapshotView`
2. 在 markdown 报告中新增 `Environment Layer` 小节
3. 保留 `Market Regime` 小节，但强调其为 environment 的粗粒度总结

### report 建议结构

```text
## Market Regime
## Environment Layer
### Breadth
### Liquidity / Participation
### Stress
```

## 9.6 `apps/desktop`

### Tauri

- 原则上不新增 command
- 继续沿用 `dashboard_bundle` / `dashboard_snapshot`

### Frontend

新增独立的 environment 面板，位置建议：

1. `Market regime`
2. `Environment layer`
3. `Watchlist breadth`（可合并进 environment 区块，避免重复）

更推荐把当前 breadth 面板并入 environment 面板，而不是长期分离。

---

## 10. CLI 与外部接口设计

## 10.1 保持现有命令名

保留：

```bash
cargo run -p quant-cli -- compute-macro --from YYYY-MM-DD --to YYYY-MM-DD
```

原因：

- 兼容已有脚本与 README
- Phase 1 是语义升级，不是命令面重做

## 10.2 可选新增调试命令

为了便于 Phase 1 验证，建议新增但不强制暴露到 README 的调试命令：

```bash
cargo run -p quant-cli -- environment-snapshot --scope cn --date 2026-04-01
```

如果不想扩 CLI 面，也可以仅依赖：

- `dashboard-snapshot --scope ...`
- ClickHouse 查询

Phase 1 最小可交付允许不新增该命令。

---

## 11. 数据迁移与兼容策略

## 11.1 迁移步骤

1. 新增 `environment_snapshot` DDL
2. 更新 `market_regime` 写入逻辑，允许 `GLOBAL / CN / HK`
3. 更新读路径，全部改成按 scope 查询
4. 重跑：
   - `compute-macro`
   - `dashboard-snapshot --scope global/cn/hk`

## 11.2 兼容原则

### 旧数据兼容

如果库里只有旧 `GLOBAL` regime 行：

- scoped dashboard 不应继续静默复用 global regime
- 应返回“对应 scope regime 不可用”或引导用户重跑 `compute-macro`

### 原因

这正是本阶段要修正的语义近似，不能再靠 fallback 把错误语义掩盖掉。

---

## 12. 验收标准

## 12.1 功能验收

### 1）Per-scope regime

```bash
cargo run -p quant-cli -- dashboard-snapshot --scope global
cargo run -p quant-cli -- dashboard-snapshot --scope cn
cargo run -p quant-cli -- dashboard-snapshot --scope hk
```

预期：

- 三者都能返回 regime
- `scope=cn` 与 `scope=hk` 不再只是复用 global regime

### 2）Environment layer

预期：

- dashboard snapshot 中存在正式 environment 字段
- report 导出中存在 environment section

### 3）Diagnostics

预期：

- scoped pipeline diagnostics 对 `market_regime` 的检查按 scope 生效
- CN/HK 缺本 scope regime 时不能误判为完整

## 12.2 数据验收

ClickHouse 预期：

- `quant.market_regime` 在同一交易日可出现三行：`GLOBAL / CN / HK`
- `quant.environment_snapshot` 在同一交易日可出现三行：`GLOBAL / CN / HK`

## 12.3 语义验收

必须满足：

1. CN/HK 报告中不再隐式使用 global regime
2. breadth 明确仍为 tracked-universe proxy，不伪装成真实 stock breadth
3. environment layer 成为正式输出，而不是 dashboard 临时拼装字段

---

## 13. 风险与缓解

## 13.1 风险：scope 与 market 语义混淆

### 现象

- `Instrument.market` 表示标的市场
- `market_regime.market` 在 Phase 1 后实际上表示环境 scope

### 缓解

- 代码层统一引入 `AnalysisScope`
- 注释与文档明确说明：`market_regime.market` 是历史列名

## 13.2 风险：breadth / environment 重复展示

### 现象

如果继续保留旧 breadth 面板，同时新增 environment 面板，UI 可能冗余。

### 缓解

- Phase 1 UI 直接把 breadth 纳入 environment 面板
- 不再把 breadth 作为完全独立的大面板长期保留

## 13.3 风险：turnover 缺失影响 liquidity proxy

### 现象

- Tencent fallback 条目经常没有 turnover

### 缓解

- Phase 1 liquidity proxy 以 `volume_expansion_pct` 为主
- `turnover_coverage_pct` 只做辅助权重与解释字段

## 13.4 风险：环境层过早进入 signal 导致重复计分

### 缓解

- Phase 1 不把 environment_score 并入 signal 总分
- 仅作为 regime 与 dashboard/report 的正式上游

## 13.5 风险：外部宏观源短时异常导致 environment 全链路断流

### 缓解

- `compute-macro` 优先使用最新抓取结果
- 若部分因子抓取失败，则回退到库里已持久化的 `macro_snapshot` 历史
- 仅覆盖本次成功抓到的因子，不再按整段日期误删其他宏观因子
- 若风险组或流动性组缺少实时数据但仍有历史数据，则继续按 as-of 语义构建 regime / environment

---

## 14. 推荐实施顺序

### Step 1：共享类型收敛

- `core-domain` 新增 `AnalysisScope`
- 清理 `app-service::ReportScope`

### Step 2：存储层升级

- 新增 `environment_snapshot` 表
- market-store 新增 scope-aware 查询

### Step 3：计算层升级

- `macro-engine` 输出 per-scope regime
- 构建 environment snapshot

### Step 4：编排层升级

- `compute_macro_regime(...)` 改写为完整 environment pipeline
- dashboard/report 改为读取 scoped regime + environment

### Step 5：展示与文档同步

- dashboard 新增 environment 面板
- report 新增 environment section
- 更新架构文档与使用说明

---

## 15. 一句话结论

Phase 1 的正确落地方式不是“给当前 global regime 多加几个 if”，而是：

> **保留 `market_regime` 作为结论层，新增 `environment_snapshot` 作为正式环境层，把 scope、breadth、liquidity、stress 全部收敛到统一语义下。**

这样做既能修正当前 scoped report 的语义近似，也能为 Phase 2 的状态机与 Phase 3 的执行层留出稳定接口。
