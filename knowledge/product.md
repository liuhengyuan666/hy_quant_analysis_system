# Project Product Domain Model (业务与领域模型)

> 注意：本文件由 KnowledgeGuard 增量维护，禁止写入推测性长远规划，只记录当前已实现业务事实。

## 1. 系统核心定位

本地桌面量化研究系统 V1，面向低频、趋势、长线的指数/ETF研究场景。

核心目标：
- 用 Rust 构建完整研究链路
- 用 Tauri 提供桌面端界面
- 用 ClickHouse 保存分析型时序数据
- 面向低频、趋势、长线的指数/ETF研究场景

## 2. 核心限界上下文与业务限界

* **[数据获取域]**
  * 核心规则 1: 日线行情拉取与入库（Eastmoney主源，Tencent兜底）
  * 核心规则 2: 宏观因子数据获取（FRED数据源，支持配置开关和TOML配置）
  * 核心规则 3: 统一使用前复权/qfq日线序列

* **[指标计算域]**
  * 核心规则 1: MA/EMA/MACD/RSI/ATR/VOL_MA计算
  * 核心规则 2: 宏观因子、per-scope market regime与environment layer
  * 核心规则 3: 相对强弱与轮动排名

* **[信号生成域]**
  * 核心规则 1: 四类策略偏好评分
  * 核心规则 2: 最终信号生成
  * 核心规则 3: 基础回测

 * **[报告与展示域]**
   * 核心规则 1: Markdown报告导出（日报、LLM分析、Research Quarterly Review）
   * 核心规则 2: Tauri桌面Dashboard（支持GLOBAL/CN/HK scope）
   * 核心规则 3: LLM智能报告分析（CLI与桌面端双路径，V4.5 后仅保留 5 个固定 action 的纯 Markdown 输出）
   * 核心规则 4: **Explainability Layer（可解释性层）** — 单标的归因拆解（symbol-diagnostics）和全标统一视图（symbol-scoreboard），仅解释现有决策，不创建新决策
   * 核心规则 5: **Execution Layer（执行层）** — 收盘前执行过滤（preclose-analysis），基于Pattern Library判断执行时机，不创建新投资想法
   * 核心规则 6: **V6 Reporting Platform** — 已冻结的 Stable Reporting Platform。Production Surface（DashboardSnapshot / sync-and-export / ResearchContext）稳定；新增消费者建立在平台之上，不修改平台
   * 核心规则 7: **V6 Research Surface** — 只读研究观测工具（`research-srd`、`research-stretch`、`research-analytics`、`research review`），输出 Markdown 观测报告，不进入主决策链路

## 4. 新增可解释性能力（TASK-092）

* **Explainability Layer** — 仅暴露系统判断过程，不修改判断逻辑
  * `symbol-diagnostics`：展示单个标的的信号归因拆解（Strategy 45% + Alignment 15% + Regime 20% + Rotation 20%）
  * `symbol-scoreboard`：全标的统一视图横向对比（Rotation + Signal + BestStrategy + Alignment + State + Position%）
  * 约束：不得生成新的评分、排名或决策信号，仅使用现有系统已计算的值

## 5. 外部业务依赖

- Eastmoney API（CN指数/ETF主源）
- Tencent API（CN/HK兜底源，Execution Layer实时数据源）
- FRED API（宏观因子数据源）
- ClickHouse（时序数据存储）
- SQLite（本地轻状态存储）
