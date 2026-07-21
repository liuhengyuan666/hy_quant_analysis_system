# 设计规划 RV1 — 能力收敛与每日组合决策助手

> **分支**：`rv1`（从 `main` 切出）
> **定位**：从 V8 "Execution Platform" 收敛为 **Daily Portfolio Decision Assistant**（每日组合决策辅助系统）
> **核心原则**：真实场景是「每日收盘 → 判断趋势健康度 → 决定加仓/持有/减仓/等待」——不是实时交易、不是订单路由、不是执行平台。

---

## 一、系统定位重新对齐

| 维度 | V8 | RV1 |
|------|----|----|
| 系统自称 | Execution Platform | **Daily Portfolio Decision Assistant** |
| 核心产出 | ExecutionEvent, ShadowRiskAssessment | **Market Evidence + Strategy Perspectives + Portfolio Action** |
| 决策输出 | BuyNow / Wait / NoChase | **Increase / Maintain / Reduce / Avoid + 依据 + 风险提示** |
| 日常命令数 | 107 | **~10 核心 + ~9 隐藏** |
| 用户每日流程 | 记多个命令 | **3 条：market-refresh → daily-analysis → daily-report** |

### 为什么不是 Execution Platform

Execution Platform 隐含订单路由、滑点、成交、盘口管理——但真实场景是场外基金收盘决策，不需要这些。V8 的 `execution-replay`、`shadow-deployment`、`holding-risk-bundle` 等模块，本质上是一个"风险研究平台"被错放在了"执行"的标签下。

### 核心资产（不可删除）

- **Evidence Asset System**：可持久化、可复现、可审计的研究证据
- **Context Integrity Firewall**：输入数据完整性校验——之前模型失败往往不是因为模型错，而是输入是假数据
- **Horizon/Role Model**：不同证据有不同的时间角色，长期风险指标不应触发短期卖出

---

## 二、实施阶段

### Phase 1：减法 + 重命名 + Integrity 集成（已完成 ✅）

（内容同前，略——见 git commit 9ca5f3b）

### Phase 1.5：工程卫生（当前）

目标：消除旧 V8 幽灵代码的认知权重，达到 workspace 零 warning。

1. **execution-replay 变体引用更新**：将 crate 内对 `ExecutionState::BuyNow/Wait/NoChase` 的引用机械更新为 `Increase/Maintain/Avoid`，随后从枚举中删除 deprecated 旧变体（serde alias 保留以保证存量 JSON 可反序列化）
2. **dead_code 处理**：`audit.rs`（45 个 warning）与 `research.rs`/`diagnostics.rs` 中因 CLI 移除而无人调用的 handler，加 `#![allow(dead_code)]` 并注明"内部保留库，不暴露 CLI"
3. **CLI 三级分类落地**：
   - 用户级（help 可见）：market-refresh / daily-analysis / daily-report / strategy-perspectives / portfolio-decision / data-health / evidence-status / validation-check / historical-replay / llm-analyze
   - 高级研究（help 可见）：research 子命令族、run-backtest
   - 工程维护（`#[command(hide = true)]`，help 不可见但可执行）：pipeline-dates / explain-latest-gate / dashboard-snapshot / dashboard-dates / symbol-diagnostics / symbol-scoreboard / rotation-ranking / sync-and-export / ingest-daily / compute-* / export-data-health-report / research-context
4. **验证**：`cargo check --workspace` 零 warning

### Phase 1.8：Domain Model Freeze ADR（Phase 2 前置门禁）

冻结四个**现有**对象为 RV1 核心领域模型（冻结 ≠ 新建）：

- `MarketRegimeSnapshot` — Market Understanding 层
- `EnvironmentSnapshot` — 环境分解层
- `Evidence` — 证据单元
- `PortfolioDecision` — 组合姿态（Increase / Maintain / Reduce / Avoid）

ADR 明确写死：

- **禁止新建 `MarketState` 同构抽象**（regime + environment 已覆盖 trend/breadth/risk/liquidity）
- **daily-analysis 输出契约永不扩展**：Integrity + Signals + Portfolio Posture，LLM 为独立后续步骤
- **Phase 2 边界**：本质是 Strategy Preference Exposure（暴露已计算但被丢弃的分数），不是策略扩张
  - 允许：消费已有策略分数、场景加权、输出解释
  - 禁止：新策略类型、新评分指标、新 Evidence（除非重走 Integrity + Validation + Registry 流程）

### Phase 2：策略多视角消费化（门禁通过后启动）

- signal-engine 不再合并四策略为单一分数，独立产出每套策略信号+归因
- 新增场景配置（`config/scenarios.toml`）：短线动量博弈 / 长线价值配置 / 激进博弈
- `strategy-perspectives` 完整实现（含场景加权）
- `SignalSnapshot` 扩展 `strategy_signals` / `scenario_scores` 字段（`#[serde(default)]`）

### Phase 3：LLM 增强 + 组合决策重构（后续）

- LLM 上下文增强：多策略矛盾点 + 历史参照 + 连续性上下文
- 对话历史持久化
- Prompt 模板化（`config/prompts.toml`）：短线交易员 / 长线配置者等分析人格
- portfolio-decision 用 LLM 替代硬编码 Pattern Library

---

## 三、预期效果

| 维度 | V8 | RV1 |
|------|----|----|
| CLI 命令数 | 107 | ~10 核心 + ~9 隐藏 |
| 策略输出 | 1 个模糊平均分 | 4 套独立评分 + 归因 + 场景加权 |
| 决策语义 | BuyNow/Wait/NoChase | Increase/Maintain/Reduce/Avoid |
| 数据可信度 | 无标注 | 每次输出标注 integrity 状态 |
| LLM 上下文 | 单一 label + rank | 多策略矛盾点 + 历史参照 + 连续性上下文 |
| 日常命令流 | 记多个命令 | 3 条：market-refresh → daily-analysis → daily-report |

---

## 四、不变的部分

- V6 Reporting Platform（冻结）
- V7 Research Platform（冻结）
- `regime-audit`、`research-validation` 等底层 crate 逻辑完整保留
- ClickHouse 存量数据不受 schema 变更影响（`#[serde(default)]` / `#[serde(alias)]`）
