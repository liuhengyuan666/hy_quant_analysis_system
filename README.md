# Rust Quant Analysis System

![Dashboard Screenshot](screen_pic/main.png)

## 使用手册

- `docs/日常操作手册.md`：适合每天快速更新、查看、导出结果
- `docs/分析使用手册.md`：适合趋势 / 长线分析时理解 MA20 / MA60 / MACD / regime / rotation / signal
- `docs/系统架构与数据流.md`：梳理系统整体架构、数据来源、数据流转路径与关键日期语义
- `docs/功能模块与处理逻辑.md`：梳理各模块职责、输入输出、数据来源与处理逻辑
- `docs/文档状态说明.md`：区分当前实现主参考、活跃设计、历史归档与运行产物
- `设计规划-rv1.md`：RV1 能力收敛设计规划
- 这些文档也已接入桌面端 UI，可通过 Dashboard 内的 **Help / Usage** 入口直接查看

本项目是一个 **Daily Portfolio Decision Assistant**（每日组合决策辅助系统），核心目标是：

- 用 Rust 构建完整研究链路
- 用 Tauri 提供桌面端界面
- 用 ClickHouse 保存分析型时序数据
- 面向 **低频、趋势、长线** 的指数 / ETF 研究场景
- 每日收盘后判断趋势健康度，决定加仓 / 持有 / 减仓 / 等待

当前已经跑通完整链路：

> 数据拉取 → 指标计算 → 宏观判断 → 轮动排序 → 策略偏好 → 最终信号 → 回测 → 报告 → 桌面展示

---

## 架构所有权（Architecture Ownership）

本项目采用分层所有权模型，每层只负责一种职责，禁止跨层泄漏：

```text
数据所有权（Data Ownership）
    ResearchDataset              ← app-service 内部，ephemeral，不暴露
    ResearchSnapshot             ← app-service 内部，computation workspace

语义所有权（Semantic Ownership）
    ResearchContext              ← 跨消费者共享的 canonical semantic contract

展示所有权（Presentation Ownership）
    ReportingSnapshot            ← 展示层 metadata + research context
    ReportInput                  ← 文档专属输入，document generation workflow 独占
    ReportBuilder                ← 文档组装（Research / Audit / Review）
    ReportDocument               ← 渲染前文档模型

渲染所有权（Rendering Ownership）
    Formatter                    ← Markdown / Text / JSON 渲染，无业务计算

消费者（Consumers）
    CLI / Desktop / API / GPT / Email / PDF
```

核心规则：
- `ResearchContext` ≠ 万能 DTO，不承载 consumer-specific 字段。
- `ReportInput` 只承载 document payload，不重复 metadata（scope/date/generated_at）。
- 所有可复用的研究计算位于 `core-domain::research`。
- `ResearchDataset` 永不暴露到 `app-service` 边界之外。

详细架构演进见 `docs/v6/adr-068-research-context-reporting-layer.md` 与 `memory/decisions.md`（ADR-068）；不可违反的分层规则见 `docs/architecture-invariants.md`（ADR-069）。

---

## 1. 当前能力概览

### 核心模块

- 日线行情拉取与入库（Eastmoney / Tencent / FRED）
- MA / EMA / MACD / RSI / ATR / VOL_MA
- 宏观因子、per-scope market regime 与 environment layer
- 相对强弱与轮动排名
- 四类策略偏好评分（ValueLeft / TrendPullback / TrendBreakout / MomentumRight）
- 最终信号生成
- 基础回测
- Markdown 报告导出
- Tauri 桌面 Dashboard（支持 `GLOBAL / CN / HK` scope）
- LLM 智能分析（OpenAI-compatible API）

### RV1 新增

- **多策略独立评分**：同一标的，四套策略各自给出独立评分和归因
- **组合操作建议**：Increase / Maintain / Reduce / Avoid（替代 BuyNow/Wait/NoChase）
- **Context Integrity Gate**：每次分析前检查数据完整性
- **Evidence Asset 状态查看**：`evidence-status` 查看 workspace 中的研究资产

### 当前适用场景

- 指数 / ETF 趋势研究
- 每日收盘后判断：加仓 / 持有 / 减仓 / 等待
- 长线 / 波段观察
- 重点看 `MA20 / MA60 / MACD`
- 用 LLM 做多视角市场解读

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
- **Vite + Vue 3**：桌面前端（Vue 3 + Composition API + vue-i18n）

---

## 3. 当前数据源策略

### 默认运行时数据源

- **CN 指数 / ETF**：Eastmoney 主源，Tencent 兜底
- **HK 指数**：Eastmoney / Tencent 低成本组合
- **宏观因子**：FRED（支持已持久化历史回退）

### 当前统一日线口径

- **Eastmoney：`fqt=1`** / **Tencent：`qfq`**
- **统一使用前复权 / qfq 日线序列**

---

## 4. 初始化与首次启动

### 4.1 启动 ClickHouse

```bash
docker compose -f infra/docker/docker-compose.yml up -d
```

### 4.2 初始化数据库

```bash
cargo run -p quant-cli -- init-storage
```

### 4.3 导入 universe

```bash
cargo run -p quant-cli -- seed-universe
```

---

## 5. 核心命令（RV1）

### 每日三件套

```bash
# 1. 全链路数据刷新（拉取最新数据 + 计算所有指标）
cargo run -p quant-cli -- market-refresh --to <today>

# 2. 每日分析（Integrity Gate → 信号 → 组合建议）
cargo run -p quant-cli -- daily-analysis --scope global

# 3. 导出日报
cargo run -p quant-cli -- daily-report --scope global
```

### 深度分析

```bash
# 多策略独立评分（单标的详细归因 + 场景对比）
cargo run -p quant-cli -- strategy-perspectives --symbol 000300 --scope cn --mode detail

# 多策略全市场排行（默认展示全部场景列的矩阵视图）
cargo run -p quant-cli -- strategy-perspectives --scope cn --mode scoreboard

# 聚焦单个场景
cargo run -p quant-cli -- strategy-perspectives --scope cn --mode scoreboard --scenario momentum_short

# 组合操作建议
cargo run -p quant-cli -- portfolio-decision --scope global
```

**场景配置**：`config/scenarios.toml` 定义了 `momentum_short`（短线动量）、`value_long`（长线价值）、`aggressive`（激进博弈）、`balanced`（均衡基线）四个场景。场景分仅用于展示和 LLM 上下文，不进入最终信号计算。

### 证据与验证

```bash
# 查看 Evidence 资产状态
cargo run -p quant-cli -- evidence-status

# 校准基线验证
cargo run -p quant-cli -- validation-check --scope cn

# 历史条件回放
cargo run -p quant-cli -- historical-replay --scope global
```

### LLM 分析

```bash
# 配置 LLM（只需执行一次）
cargo run -p quant-cli -- set-llm-config --base-url https://api.openai.com/v1 --model gpt-4o
cargo run -p quant-cli -- set-llm-api-key --key sk-xxxxxxxxxxxxxxxx

# 组合决策解读（解释引擎姿态 + 多策略矛盾点，日常推荐）
cargo run -p quant-cli -- llm-analyze --scope global --action portfolio_review

# 市场叙事 / 短线人格 / 长线人格
cargo run -p quant-cli -- llm-analyze --scope global --action market_story
cargo run -p quant-cli -- llm-analyze --scope cn --action short_term_trader
cargo run -p quant-cli -- llm-analyze --scope cn --action long_term_allocator
```

### 隐藏命令（`--help` 可发现）

以下命令保持可用但不进入日常推荐路径，用于需要下钻时手动查找：

```bash
cargo run -p quant-cli -- research observe --scope global    # SRD + Stretch + Analytics 聚合
cargo run -p quant-cli -- research-srd --scope global         # SRD 单独查询
cargo run -p quant-cli -- pipeline-dates                      # 管线状态诊断
cargo run -p quant-cli -- explain-latest-gate                 # gate 诊断
cargo run -p quant-cli -- data-health                         # 数据健康检查
cargo run -p quant-cli -- symbol-diagnostics --symbol 000300  # 单标的诊断
cargo run -p quant-cli -- symbol-scoreboard --scope cn        # 全市场排行榜
cargo run -p quant-cli -- rotation-ranking --scope cn         # 轮动排名
cargo run -p quant-cli -- dashboard-snapshot --scope cn       # 历史快照
cargo run -p quant-cli -- sync-and-export --scope global      # 一键同步导出
cargo run -p quant-cli -- run-backtest --scope global         # 回测
```

---

## 6. 桌面端启动方法

### 前端构建

```bash
cd apps/desktop/frontend
npm install
npm run build
```

### 桌面端运行

```bash
cargo build -p quant-desktop
cargo run -p quant-desktop
```

桌面端展示：Dashboard 总览、Scope 选择器、历史日期选择、Market Regime、Environment Layer、Trust Summary、Top Rotation、Top Signals、Latest Backtest、Data Health、Recent Reports、LLM 智能分析面板（5 个按钮）。

---

## 7. 推荐使用流程

### 每日推荐 CLI 工作流（收盘后）

```bash
# 1. 全链路刷新（拉取最新数据）
cargo run -p quant-cli -- market-refresh --to <today>

# 2. 每日分析（Integrity Gate + 信号 + 组合姿态）
cargo run -p quant-cli -- daily-analysis --scope global

# 3. 导出日报
cargo run -p quant-cli -- daily-report --scope global
```

**当 daily-analysis 出现"信号强但姿态 Maintain/Avoid"的张力时，继续下钻：**

```bash
# 4a. 看该标的四套策略各自给多少分、为什么
cargo run -p quant-cli -- strategy-perspectives --symbol 512480 --scope cn --mode detail

# 4b. 或直接让 LLM 结合多策略视角解读引擎姿态（自动携带全部上下文）
cargo run -p quant-cli -- llm-analyze --scope global --action portfolio_review
```

### 桌面端工作流

1. 打开桌面端 → 点击 `Refresh data`
2. 先看 `Trust summary` → 再下钻 `Pipeline freshness` 与 `Data health`
3. 确认后继续阅读 `Environment / Rotation / Signals / Backtest`
4. 需要 LLM 解读时切换到 LLM 分析面板
5. 需要留档时再导出 report

---

## 8. 高级参考：分步执行

> **这组命令必须按顺序执行**，不能倒序或跳过中间阶段，否则 `daily-report` 会因 latest gate 落后而被拒绝。

```bash
# 1. 拉取日线数据
cargo run -p quant-cli -- ingest-daily --from 2026-05-19 --to 2026-05-20

# 2. 计算宏观与市场环境
cargo run -p quant-cli -- compute-macro --from 2024-01-01 --to 2026-03-16

# 3. 计算指标 / 轮动 / 策略 / 信号（通常由 market-refresh 自动覆盖）
cargo run -p quant-cli -- compute-indicators
cargo run -p quant-cli -- compute-rotation
cargo run -p quant-cli -- compute-strategy-preferences
cargo run -p quant-cli -- compute-signals
```

---

## 9. LLM 智能分析

```bash
# 配置 LLM
cargo run -p quant-cli -- set-llm-config --base-url https://api.openai.com/v1 --model gpt-4o
cargo run -p quant-cli -- set-llm-api-key --key sk-xxxxxxxxxxxxxxxx

# LLM 分析
cargo run -p quant-cli -- llm-analyze --scope global --action market_story
```

可用的 `action` 参数：
- `market_story` — 市场叙事（今天发生了什么）
- `explain_decision` — 解释决策（为什么系统这样决定）
- `preclose_review` — 收盘前复核（组合操作建议解读）
- `risk_view` — 风险视角（风控总监视角）
- `devils_advocate` — 唱反调（质疑系统结论）
- `portfolio_review` — 组合决策解读（解释确定性引擎的组合姿态 + 多策略矛盾点）

**自定义分析人格**：`config/prompts.toml` 支持自定义 persona（已内置 `short_term_trader` 短线交易员、`long_term_allocator` 长线配置者），`--action` 直接使用 persona key 即可。LLM 每次分析会自动携带多策略评分、场景对比、数据完整性状态与前次解读（标注为非证据背景）。

---

## 10. 回测（Backtest）

```bash
cargo run -p quant-cli -- run-backtest --scope global --use-state-sizing --max-drawdown 0.15
```

---

## 附录. 已知限制

- 没有正式测试套件 / CI
- 桌面端 LLM 仅支持 OpenAI-compatible API，不支持流式输出
- 前端面板仍为单信号视图，多策略视角目前仅在 CLI（`strategy-perspectives`）与 LLM 上下文中呈现，前端适配待后续
- Evidence 权重设计（P3）延迟，直到积累 1000+ 资产、30 天 Replay 稳定、2 周期 Calibration 稳定
