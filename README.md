# Rust Quant Analysis System

## 使用手册

- `docs/日常操作手册.md`：适合每天快速更新、查看、导出结果
- `docs/分析使用手册.md`：适合趋势 / 长线分析时理解 MA20 / MA60 / MACD / regime / rotation / signal
- `docs/系统架构与数据流.md`：梳理系统整体架构、数据来源、数据流转路径与关键日期语义
- `docs/功能模块与处理逻辑.md`：梳理各模块职责、输入输出、数据来源与处理逻辑
- 这两份文档也已接入桌面端 UI，可通过 Dashboard 内的 **Help / Usage** 入口直接查看

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
- 宏观因子与 market regime
- 相对强弱与轮动排名
- 四类策略偏好评分
- 最终信号生成
- 基础回测
- Markdown 报告导出
- Tauri 桌面 Dashboard

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
- **宏观因子**：FRED

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

### 8.7 跑回测

```bash
cargo run -p quant-cli -- run-backtest
```

### 8.8 查看 dashboard 数据

```bash
cargo run -p quant-cli -- dashboard-dates
cargo run -p quant-cli -- dashboard-snapshot
cargo run -p quant-cli -- dashboard-snapshot --date 2026-03-16
```

说明：

- `dashboard-snapshot` 不带参数时，默认返回**最新可分析日期**
- `dashboard-dates` 返回当前可选的历史分析日期列表
- `dashboard-snapshot --date YYYY-MM-DD` 可回看某一历史日期的分析结果

### 8.9 导出日报

```bash
cargo run -p quant-cli -- export-report
cargo run -p quant-cli -- export-report --date 2026-03-06
```

说明：

- `export-report` 不带参数时，默认导出当前最新分析日期的日报
- `export-report --date YYYY-MM-DD` 可导出指定历史日期的日报

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
- Analysis date selector
- Market regime
- Top rotation
- Top signals
- Latest backtest
- Data health summary
- Report export action

---

## 11. 推荐使用流程（适合当前 V1）

如果你当前主要做：

- 趋势判断
- 长线观察
- 低频操作
- 手动 / 低频刷新

推荐日常流程：

1. 更新日线数据
2. 跑指标 / 宏观 / 轮动 / 信号
3. 跑一次数据健康检查
4. 查看 dashboard
5. 导出日报
6. 有需要时再跑回测

也就是：

```bash
cargo run -p quant-cli -- ingest-daily --from 2026-01-01 --to 2026-03-16
cargo run -p quant-cli -- compute-indicators
cargo run -p quant-cli -- compute-macro --from 2026-01-01 --to 2026-03-16
cargo run -p quant-cli -- compute-rotation
cargo run -p quant-cli -- compute-strategy-preferences
cargo run -p quant-cli -- compute-signals
cargo run -p quant-cli -- check-data-health
cargo run -p quant-cli -- dashboard-snapshot
cargo run -p quant-cli -- export-report
```

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
cargo run -p quant-cli -- check-data-health
cargo run -p quant-cli -- run-backtest
cargo run -p quant-cli -- dashboard-snapshot
cargo run -p quant-cli -- export-report
cargo run -p quant-cli -- export-data-health-report
```
