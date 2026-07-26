# Project Product Domain Model (业务与领域模型)

> 注意：本文件由 KnowledgeGuard 增量维护，禁止写入推测性长远规划，只记录当前已实现业务事实。

## 1. 系统核心定位

本地桌面量化研究系统 V8，面向低频、趋势、长线的指数/ETF研究场景。

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
   * 核心规则 3: LLM智能报告分析（CLI与桌面端双路径，纯 Markdown 输出）。RV1 现状：6 个内置 action（market_story / explain_decision / preclose_review / risk_view / devils_advocate / portfolio_review）+ `config/prompts.toml` 自定义 persona（含 market_adversarial_lens 市场博弈视角、short_term_trader、long_term_allocator）；每次分析自动落盘 `workspace/llm-history/` 并在下次分析注入"前次解读"（标注为背景，非证据）；LLM 边界由 ADR-106 冻结——只解释确定性引擎产出，不决策、不评分、不输出仓位
    * 核心规则 3b: **共享博弈假设背景层（ADR-112，默认开启）** — 每次 LLM 分析前系统确保当日博弈分析已生成（同一 scope 同一日期只算一次，落盘 `workspace/llm-history/{scope}/adversarial/`），按 persona 分级注入（叙事/风控/组合类 standard 默认正文，机制解释类 compact 摘要，adversarial 自身 none 递归防护）；注入语义为"供验证或反驳的假设背景"，不是结论；可用 `--adversarial` 单次覆盖或 `[llm.adversarial]` 全局配置
   * 核心规则 4: **Explainability Layer（可解释性层）** — 单标的归因拆解（symbol-diagnostics）和全标统一视图（symbol-scoreboard），仅解释现有决策，不创建新决策
   * 核心规则 5: **Execution Layer（执行层）** — 收盘前执行过滤（preclose-analysis），基于Pattern Library判断执行时机，不创建新投资想法
    * 核心规则 6: **V6 Reporting Platform** — 已冻结的 Stable Reporting Platform。Production Surface（DashboardSnapshot / sync-and-export / ResearchContext）稳定；新增消费者建立在平台之上，不修改平台
    * 核心规则 7: **V6 Research Surface** — 只读研究观测工具（`research-srd`、`research-stretch`、`research-analytics`、`research review`），输出 Markdown 观测报告，不进入主决策链路
    * 核心规则 8: **V7 Research Platform 1.0** — 已冻结。四层研究语义：Observation (V6) + Market Evolution (V7.1: Confirmation / Recovery) + Historical Evidence (V7.2: Market Fingerprint / Historical Analogues / Outcome Profile / Calibration) + Research Synthesis (V7.3: Consensus / Evidence Aggregation)。只输出研究语言（Bias / Confidence / Evidence），不进入决策链路；新增市场内容属于 Research Content Evolution，不修改语义架构
    * 核心规则 9: **V8 Research Asset** — 研究产物（Evidence / Snapshot）以统一身份 `RA-XXXXXX` 和统一生命周期（Draft → Verified → Published → Superseded → Archived）持久化到本地 `workspace/`；Snapshot 通过 `EvidenceRef` 引用而非嵌入 Evidence；P3（Evidence Score/Weight）门控未达成前不得引入数值化权重
    * 核心规则 10: **V8 Execution Platform（Phase 2C Shadow Validation）** — 由 `execution-replay` crate 支撑的 Evidence → Risk State → Shadow Assessment 只读验证链路（`shadow-deployment`、`shadow-mode`、`execution-context-integrity-gate`、`evidence-registry`、`holding-risk-calibration`、`risk-lifecycle` 等 CLI）；只读观察不影响交易，禁止 DecisionEngine 消费、禁止新增 Evidence

## 3. V7 Research Platform 1.0 业务能力

* **[市场演化域]（V7.1，已冻结）**
  * 核心规则 1: `research confirmation` — 判断市场趋势是否被广度、参与度和风险维度确认
  * 核心规则 2: `research recovery` — 衡量市场从压力中恢复的程度

* **[历史证据域]（V7.2，已冻结）**
  * 核心规则 1: `research analogues` — 基于 Market Fingerprint 检索历史相似市场状态
  * 核心规则 2: `research calibration` — 连续运行 Confirmation / Recovery / Analogues，输出统计校准报告，建立版本化 Calibration Baseline
  * 核心规则 3: Historical Analogues 不对外暴露原始相似度百分比，仅使用 rank / 定性等级

* **[研究综合域]（V7.3，已冻结）**
  * 核心规则 1: `research consensus` — 基于 Evidence Aggregation 综合 Observation / Evolution / Historical Evidence，输出 Bias / Confidence / Supporting Evidence / Contradicting Evidence
  * 核心规则 2: Consensus 不是策略决策器，不输出买卖建议、仓位、目标价或止损
  * 核心规则 3: 权重和阈值通过 `ConsensusConfig` 配置，变更需经 Calibration 验证并可能触发版本递增

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
