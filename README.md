# Rust Quant Analysis System

## 使用手册

- `docs/日常操作手册.md`：适合每天快速更新、查看、导出结果
- `docs/分析使用手册.md`：适合趋势 / 长线分析时理解 MA20 / MA60 / MACD / regime / rotation / signal
- `docs/系统架构与数据流.md`：梳理系统整体架构、数据来源、数据流转路径与关键日期语义
- `docs/功能模块与处理逻辑.md`：梳理各模块职责、输入输出、数据来源与处理逻辑
- `docs/V2-Phase1-环境层详细技术设计.md`：V2 Phase 1（per-scope regime + environment layer）工程设计
- `docs/文档状态说明.md`：区分当前实现主参考、活跃设计、历史归档与运行产物
- `docs/阶段性更新-2026-04-26.md`：汇总这轮阶段性成果与当前仍待继续推进的方向
- 这些文档也已接入桌面端 UI，可通过 Dashboard 内的 **Help / Usage** 入口直接查看

本项目是一个 **本地桌面量化研究系统 V1**，核心目标是：

- 用 Rust 构建完整研究链路
- 用 Tauri 提供桌面端界面
- 用 ClickHouse 保存分析型时序数据
- 面向 **低频、趋势、长线** 的指数 / ETF 研究场景

当前已经跑通完整链路：

> 数据拉取 → 指标计算 → 宏观判断 → 轮动排序 → 策略偏好 → 最终信号 → 回测 → 报告 → 桌面展示

---

## 1. 当前能力概览

### 已实现模块

- 日线行情拉取与入库
- MA / EMA / MACD / RSI / ATR / VOL_MA
- 宏观因子、per-scope market regime 与 environment layer
- 相对强弱与轮动排名
- 四类策略偏好评分
- 最终信号生成
- 基础回测
- Markdown 报告导出
- Tauri 桌面 Dashboard（支持 `GLOBAL / CN / HK` scope）

### 当前适用场景

- 指数 / ETF 趋势研究
- 低频交易辅助判断
- 长线 / 波段观察
- 重点看 `MA20 / MA60 / MACD`

不适合作为：

- 高频执行系统
- 实盘自动交易系统
- 多用户在线量化平台

---

## 2. 技术栈

- **Rust**：核心实现
- **Tauri**：桌面端容器
- **ClickHouse**：分析型时序数据
- **SQLite**：本地轻状态
- **Vite + Plain JS**：桌面前端

---

## 3. 当前数据源策略

### 默认运行时数据源

- **CN 指数 / ETF**：Eastmoney 主源，Tencent 兜底
- **HK 指数**：Eastmoney / Tencent 低成本组合
- **宏观因子**：FRED（运行时支持已持久化 `macro_snapshot` 历史回退）

### 暂不作为默认主源

- **Yahoo Finance**：当前环境已实测出现 `403`，因此不作为港股默认主源
- **Tushare**：保留为后续可选增强源，但当前 V1 不依赖它

### 当前统一日线口径

为了让 `MA20 / MA60 / MACD` 更稳定，当前 V1 已统一为：

- **Eastmoney：`fqt=1`**
- **Tencent：`qfq`**

也就是：

> **统一使用前复权 / qfq 日线序列**

---

## 4. Universe 配置

当前标的池文件：

- `config/universe.json`

当前字段：

- `symbol`：系统内部主标识
- `name`：中文名称
- `display_symbol`：展示符号
- `instrument_type`：`INDEX` / `ETF`
- `market`：`CN` / `HK`
- `category`：标的分类
- `eastmoney_secid`：Eastmoney 拉取标识
- `tencent_symbol`：Tencent 拉取标识
- `enabled`：是否启用

说明：

- `display_symbol` 是展示元数据
- `eastmoney_secid / tencent_symbol` 是抓数元数据
- 不要把展示符号和 provider 符号混用

---

## 5. 环境要求

建议环境：

- Windows
- Rust toolchain
- Node.js / npm
- Docker Desktop

需要确保：

- Docker 可以正常启动 ClickHouse
- 本机可访问 `127.0.0.1:18123`
- 能正常执行 `cargo` 和 `npm`

---

## 6. 项目结构

```text
rust-quant-analysis-system/
├── apps/
│   ├── cli/
│   └── desktop/
├── crates/
│   ├── app-service/
│   ├── backtest-engine/
│   ├── core-domain/
│   ├── data-ingestion/
│   ├── indicator-engine/
│   ├── macro-engine/
│   ├── market-store/
│   ├── report-engine/
│   ├── rotation-engine/
│   ├── signal-engine/
│   └── strategy-engine/
├── config/
├── infra/
├── reports/
└── sql/
```

---

## 7. 初始化与首次启动

### 7.1 启动 ClickHouse

```bash
docker compose -f infra/docker/docker-compose.yml up -d
```

### 7.2 初始化数据库

```bash
cargo run -p quant-cli -- init-storage
```

### 7.3 导入 universe

```bash
cargo run -p quant-cli -- seed-universe
```

---

## 8. 完整数据管线使用方法

### 8.1 拉取日线数据

```bash
cargo run -p quant-cli -- ingest-daily --from 2026-03-01 --to 2026-03-16
```

### 8.2 计算指标

```bash
cargo run -p quant-cli -- compute-indicators
```

### 8.3 计算宏观与市场环境

```bash
cargo run -p quant-cli -- compute-macro --from 2024-01-01 --to 2026-03-16
```

说明：

- `compute-macro` 会同时重建 `macro_snapshot / market_regime / environment_snapshot`
- 若部分 FRED 因子短时异常，系统会优先复用库里已有的 `macro_snapshot` 历史，继续按 as-of 语义构建 scoped regime / environment
- 若某次 provider 返回异常 HTML/非 CSV 响应，失败项会进入 `failed_items`，不再静默产出空结果

### 8.4 计算轮动强弱

```bash
cargo run -p quant-cli -- compute-rotation
```

### 8.5 计算策略偏好

```bash
cargo run -p quant-cli -- compute-strategy-preferences
```

### 8.6 生成最终信号

```bash
cargo run -p quant-cli -- compute-signals
```

### 8.6A 一次执行完整刷新（工程 / 高级用户路径）

```bash
cargo run -p quant-cli -- refresh-all
cargo run -p quant-cli -- refresh-all --to 2026-04-26
```

说明：

- 该命令会按当前 desktop refresh 相同顺序依次执行：
  - `ingest -> indicators -> macro -> rotation -> strategy -> signals -> backtests`
- `--scope` 用于选择**latest-date diagnostics / gate explanation** 的解释 scope，
  - 不表示只刷新某个 scope 的底层数据链路
- `--run-backtests` 当前默认为 `true`，与 desktop 完整 refresh 的语义一致
- 结束后会返回结构化 JSON，总结：
  - refresh window
  - latest daily date / latest gated dashboard date
  - refresh reason / repair window days
  - 各阶段执行结果
  - 各 scope 的 `pipeline_diagnostics`
  - default latest-date 是否推进
  - latest-gate / consistency 阻塞提示
- 当前 refresh window 不再只锚定 `latest_daily_date - 7d`；
  - 如果某个 scope 的 gated latest 落后，或仍没有 gated latest，
  - 系统会自动扩到一个保守的 repair window 来修复被 gate 卡住的较早日期
- 这条命令更适合作为 **显式工程路径 / 高级用户路径**；默认用户路径仍然优先推荐桌面端 `Refresh data`

### 8.7 跑回测

```bash
cargo run -p quant-cli -- run-backtest
```

### 8.8 查看 dashboard 数据

```bash
cargo run -p quant-cli -- dashboard-dates
cargo run -p quant-cli -- dashboard-snapshot
cargo run -p quant-cli -- dashboard-snapshot --date 2026-03-16
cargo run -p quant-cli -- explain-latest-gate
cargo run -p quant-cli -- dashboard-dates --scope cn
cargo run -p quant-cli -- dashboard-snapshot --scope hk --date 2026-03-16
```

说明：

- `dashboard-snapshot` 不带参数时，默认返回**最新可分析日期**
- `dashboard-dates` 返回当前可选的历史分析日期列表
- `dashboard-snapshot --date YYYY-MM-DD` 可回看某一历史日期的分析结果
- `--scope global|cn|hk` 可切到对应 scope 的 dashboard 语义
- `dashboard-snapshot` 现在会返回 scope 对应的 `market_regime + environment`
- `dashboard-snapshot` 还会返回 `trust_summary`，用于汇总 freshness / data-health / provenance 的可用性判断
- signal / backtest 当前应结合显式 provenance（例如 `analysis_scope`、`regime_basis_scope`、`matches current snapshot`）一起阅读，而不是只按当前 dashboard scope 直觉推断
- `explain-latest-gate` 会专门解释：为什么默认最新日期还没有推进到 freshest market date，以及卡在 signal / rotation / regime / environment 哪一层

### 8.9 导出日报

```bash
cargo run -p quant-cli -- export-report
cargo run -p quant-cli -- export-report --date 2026-04-07
cargo run -p quant-cli -- export-report --scope cn --date 2026-04-02
```

说明：

- `export-report` 不带参数时，默认导出当前最新分析日期的日报
- `export-report --date YYYY-MM-DD` 可导出指定历史日期的日报
- `export-report --scope ...` 会导出对应 scope 的日报

---

## 9. 数据健康检查

为了避免趋势系统因为缺口、fallback、异常波动而被误导，当前已补上数据健康检查。

### 检查内容

- Eastmoney 主源当前是否可达
- Tencent fallback 当前是否可达
- 存量日线是否存在大时间缺口
- 是否存在异常大波动日
- 是否存在缺失 turnover 的 bar

### 运行命令

```bash
cargo run -p quant-cli -- check-data-health
```

### 导出数据健康报告

```bash
cargo run -p quant-cli -- export-data-health-report
```

导出文件会落在：

- `reports/data-health-<date>.md`

---

## 10. 桌面端启动方法

### 前端构建

```bash
cd apps/desktop/frontend
npm install
npm run build
```

### 桌面端运行

```bash
cargo build -p quant-desktop
```

调试运行：

```bash
cargo run -p quant-desktop
```

桌面端会展示：

- App status
- Scope selector（`GLOBAL / CN / HK`）
- Analysis date selector
- Market regime
- Environment layer
- Trust summary
- Top rotation
- Top signals
- Latest backtest
- Data health summary
- Recent reports（支持回跳 matching snapshot / open artifact / copy artifact path）
- Report export action

---

## 11. 推荐使用流程（适合当前 V1）

如果你当前主要做：

- 趋势判断
- 长线观察
- 低频操作
- 手动 / 低频刷新

当前默认用户路径建议是：

> **优先使用桌面端的 `Refresh data` 作为默认刷新入口；CLI 全链路命令继续保留为显式工程/高级用户路径。**

推荐日常流程：

1. 更新日线数据
2. 跑指标 / 宏观 / 轮动 / 信号
3. 先看一次 trust summary
4. 再下钻 pipeline freshness / completeness
5. 再跑一次数据健康检查
6. 查看 dashboard
7. 导出日报
8. 有需要时再跑回测

如果你日常主要使用桌面端，更推荐的实际顺序是：

1. 打开桌面端
2. 点击 `Refresh data`
3. 先看 `Trust summary`
4. 再下钻 `Pipeline freshness` 与 `Data health`
5. 确认后继续阅读 `Environment / Rotation / Signals / Backtest`
6. 需要留档时再导出 report

也就是：

```bash
cargo run -p quant-cli -- ingest-daily --from 2026-04-01 --to 2026-05-07
cargo run -p quant-cli -- compute-indicators
cargo run -p quant-cli -- compute-macro --from 2026-04-01 --to 2026-05-07
cargo run -p quant-cli -- compute-rotation
cargo run -p quant-cli -- compute-strategy-preferences
cargo run -p quant-cli -- compute-signals
cargo run -p quant-cli -- pipeline-dates
cargo run -p quant-cli -- check-data-health
cargo run -p quant-cli -- dashboard-snapshot
cargo run -p quant-cli -- export-report
```

补充说明：

- `pipeline-dates` 用来检查每个 stage 的**最新日期**和**最新日是否全量完整**
- 如果 `strategy_preference` 已到最新，但 `signal_snapshot` 仍落后，优先单独重跑一次 `compute-signals`
- `check-data-health` 更偏向 provider 可达性、缺口、异常波动、turnover 缺失、宏观源状态
- 如果 `pipeline-dates` 显示某个 stage `is_latest=true` 但 `is_complete=false`，说明这一天**日期到了，但最新日样本不完整**
- 如果 `report_date` 是最新日期，但 `regime_as_of_date` 更早，这通常表示**宏观因子按最近可用值 forward-fill**，不代表 dashboard 出错
- `GLOBAL / CN / HK` 的 dashboard/report 现在各自读取对应 scope 的 regime 与 environment，不再复用 global regime 假装本地化

---

## 12. 当前 V1 的已知限制

- 没有正式测试套件 / CI
- `market-store` 仍然偏大
- `app-service` 仍然偏大
- 数据健康检查已上线，但还没有把 provider 来源逐 bar 持久化
- 当前更适合研究和辅助判断，不适合直接自动交易

---

## 13. 常用命令总表

```bash
cargo run -p quant-cli -- status
docker compose -f infra/docker/docker-compose.yml up -d
cargo run -p quant-cli -- init-storage
cargo run -p quant-cli -- seed-universe
cargo run -p quant-cli -- ingest-daily --from 2024-01-01 --to 2025-01-31
cargo run -p quant-cli -- compute-indicators
cargo run -p quant-cli -- compute-macro --from 2025-01-01 --to 2025-01-31
cargo run -p quant-cli -- compute-rotation
cargo run -p quant-cli -- compute-strategy-preferences
cargo run -p quant-cli -- compute-signals
cargo run -p quant-cli -- pipeline-dates
cargo run -p quant-cli -- check-data-health
cargo run -p quant-cli -- run-backtest
cargo run -p quant-cli -- dashboard-snapshot
cargo run -p quant-cli -- dashboard-snapshot --scope cn
cargo run -p quant-cli -- export-report
cargo run -p quant-cli -- export-report --scope hk
cargo run -p quant-cli -- export-data-health-report
```
