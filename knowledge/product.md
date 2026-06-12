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
  * 核心规则 2: 宏观因子数据获取（FRED数据源）
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
  * 核心规则 1: Markdown报告导出
  * 核心规则 2: Tauri桌面Dashboard（支持GLOBAL/CN/HK scope）
  * 核心规则 3: LLM智能报告分析

## 3. 外部业务依赖

- Eastmoney API（CN指数/ETF主源）
- Tencent API（CN/HK兜底源）
- FRED API（宏观因子数据源）
- ClickHouse（时序数据存储）
- SQLite（本地轻状态存储）
