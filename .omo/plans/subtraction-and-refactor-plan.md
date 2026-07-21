# V8.1 能力收敛方案 v2

> **定位变更**：从 "Execution Platform" 收敛为 **Daily Portfolio Decision Assistant**（每日组合决策辅助系统）
> 
> **核心原则**：你的真实场景是「每日收盘 → 判断趋势健康度 → 决定加仓/持有/减仓/等待」——不是实时交易、不是订单路由、不是执行平台。
>
> **实施顺序**：Phase 1（减法+重命名+Integrity）→ Phase 2（策略多视角）→ Phase 3（LLM增强+组合决策）

---

## 系统定位重新对齐

| 维度 | 当前 V8 | 目标 V8.1 |
|------|---------|----------|
| 系统自称 | Execution Platform | **Daily Portfolio Decision Assistant** |
| 核心产出 | ExecutionEvent, ShadowRiskAssessment | **Market Evidence + Strategy Perspectives + Portfolio Action** |
| 决策输出 | BuyNow / Wait / NoChase | **Increase / Maintain / Reduce / Avoid + 依据 + 风险提示** |
| 日常命令数 | 107 | **~10 核心 + ~9 隐藏** |
| 用户每日流程 | 记多个命令 | **3 条：market-refresh → daily-analysis → daily-report** |

---

## Phase 1：减法 + 重命名 + Integrity 集成

### 1.1 CLI 命令归类

#### 核心命令（~10 个，README 推荐，日常使用）

**每日三件套：**
```
market-refresh           ← 全链路数据刷新（替代 refresh-all）
daily-analysis           ← 一键产出：Research Evidence + 多策略评分 + 风险快照 + 组合建议
daily-report             ← 导出日报（替代 export-report）
```

**深度分析：**
```
strategy-perspectives    ← 多策略独立评分 + 场景对比 + 归因
risk-assessment          ← 持仓风险评估
portfolio-decision       ← 当日组合操作建议（替代 preclose-analysis）
```

**证据与验证：**
```
evidence-status          ← 查看 Evidence 资产状态
validation-check         ← 校准基线验证
historical-replay        ← 历史条件回放（替代 research replay）
```

**LLM：**
```
llm-analyze              ← LLM 多视角市场分析（替代 analyze-with-llm）
```

#### 隐藏命令（~9 个，`--help` 可发现，README 不推荐）

这些命令保持可用但不进入日常推荐路径，用于需要下钻时手动查找：

```
research-srd             ← SRD 单独查询
research-stretch         ← Stretch 单独查询
research-calibration     ← 校准基线检查
pipeline-dates           ← 管线状态诊断
data-health              ← 数据健康检查
symbol-diagnostics       ← 单标的诊断
symbol-scoreboard        ← 全市场排行榜
rotation-ranking         ← 轮动排名
dashboard-snapshot       ← 查看历史快照
```

### 1.2 移除的 CLI 命令（~50+ 个，底层 crate 逻辑保留）

#### 审计类（全部 ~35 个）

来自 `apps/cli/src/commands/audit.rs`（~3,578 行）和 `main.rs` 中的审计子命令。底层 `regime-audit` 和 `research-validation` crate 代码完整保留，仅移除 CLI 暴露：

`validate-regime-accuracy`, `inspect-ground-truth`, `generate-regime-labels`, `audit-gt-regime`, `audit-gt-transitions`, `audit-gt-candidates`, `validate-gt-regimes`, `audit-observation-layer`, `replay-trend-sensitivity`, `gt-sensitivity-replay`, `audit-attribution`, `audit-persistence-sensitivity`, `audit-market-structure`, `audit-regime-alignment`, `audit-factor-alignment`, `audit-false-positive-breakdown`, `audit-counterfactual-regime`, `audit-economic-replay`, `audit-economic-attribution`, `audit-pareto-frontier`, `audit-economic-regime-prototype`, `audit-dual-layer-validation`, `audit-allocation-prototype`, `audit-state-signal-decomposition`, `audit-state-transitions`, `audit-persistence-frontier`, `audit-persistence-mechanics`, `audit-episode-survival`, `audit-label-distribution`, `audit-score-distribution`, `audit-wave8-revalidation`, `audit-ground-truth`, `audit-forward-return-distribution`, `generate-ground-truth-labels`, `audit-alignment-redesign`, `audit-state-persistence-economics`, `validate-state-layer-gt`, `audit-lead-lag`

#### V8 Execution Platform 实验命令（~20 个）

Phase 2A 校准阶段的中间实验产物，底层逻辑保留在 `execution-engine` 和 `app-service`，CLI 移除：

`validate-execution-replay`, `validate-execution-suite`, `find-validation-candidates`, `execution-statistics`, `execution-evidence-trace`, `execution-distribution-coverage`, `execution-decision-margin`, `execution-decision-gate`, `execution-risk-semantics`, `execution-calibration`, `execution-context-integrity-audit`, `execution-context-integrity-gate`, `execution-bearish-analysis`, `execution-transition-analysis`, `execution-holding-risk-bundle`, `holding-risk-persistence`, `execution-holding-risk-bundle-v2`, `liquidity-pressure`, `execution-holding-risk-bundle-v3`, `confirmation-decay`, `execution-holding-risk-bundle-v4`

#### Shadow Production 形式化命令（4 个）

观察不应该是"项目"，而应该是"习惯"。日常跑 `daily-analysis` 即可覆盖。移除 CLI 暴露：

`shadow-mode`, `shadow-deployment`, `risk-lifecycle`, `holding-risk-calibration`

#### 重复/废弃命令（3 个）

`execution-leadership-decay-horizon`, `regime-risk`, `state-risk-acceleration`

### 1.3 模块三级归类

#### 一级：核心资产（提升为系统支柱）

| 模块 | 当前状态 | V8.1 动作 |
|------|---------|----------|
| **Evidence Asset System** | `workspace.rs` 里的实现 | 提升为 `daily-analysis` 输出的核心组成部分 |
| **Context Integrity Firewall** | 隐藏在 execution 命令里 | 成为 `daily-analysis` 的第一步，每次产出标注 integrity 状态 |
| **Horizon/Role Model** | 散落在 Evidence payload 里 | 合并到策略归因（Phase 2） |
| **HoldingRisk + Risk Lifecycle** | V8 execution 实验产物 | 保留核心逻辑，整合进 `risk-assessment` |

#### 二级：内部工具（保留逻辑，隐藏 CLI）

- 审计/GT 命令 → `regime-audit` + `research-validation` crate 保留
- holding-risk-bundle v1-v4 → 只保留核心逻辑，移除 CLI
- 策略独立评分（strategy-engine 已有独立分数，但 signal-engine 合并了）→ Phase 2 重构

#### 三级：归档

| 模块 | 动作 |
|------|------|
| `regime_risk_model` | 归档到 `research/archive/failed/` — 已证明不符合市场认知模型 |
| `state_risk_acceleration` | 同上 |
| `execution-replay` 作为 crate 名 | 概念已过时，保留代码但停止新功能 |

### 1.4 DecisionEngine 输出语义变更（Phase 1 即刻执行）

当前输出：
```
BuyNow / Wait / NoChase / Reduce / Skip
```

改为：
```
Increase / Maintain / Reduce / Avoid

每个 action 附带:
  - confidence: 置信度
  - evidence: 支撑证据列表
  - risk_note: 风险提示
```

涉及变更：
- `crates/execution-engine/src/lib.rs`：`ExecutionDecision` 枚举重命名
- `crates/core-domain/src/lib.rs`：DTO 字段更新（`#[serde(alias)]` 保持向后兼容）
- `crates/app-service/`：`preclose-analysis` → `portfolio-decision` 重命名

### 1.5 Context Integrity Firewall 集成（Phase 1 即刻执行）

`daily-analysis` 命令执行流程：

```
Step 0: Context Integrity Gate
  ├─ 检查关键数据源可达性
  ├─ 检查 feature 是否有变化（hash 对比）
  ├─ 检查输入数据完整性
  └─ 输出: integrity_status (PASS / DEGRADED / FAIL)

Step 1: Research Evidence（聚合 SRD + Stretch + Analytics + Health）
Step 2: Strategy Perspectives（多策略独立评分，Phase 2 完整实现，当前输出已有的四策略分数）
Step 3: Risk Assessment（持仓风险 + 市场风险）
Step 4: Portfolio Action（组合操作建议）

输出中显式标注:
  integrity: PASS → 本次分析基于完整可信数据
  integrity: DEGRADED → 部分数据源异常，以下结论置信度降低，异常项: [...]
  integrity: FAIL → 关键数据缺失，本次分析不可用于决策
```

### 1.6 涉及文件

| 文件 | 变更 |
|------|------|
| `apps/cli/src/main.rs` | 移除 50+ Command 枚举变体，新增核心命令，重命名已有命令 |
| `apps/cli/src/commands/audit.rs` | 整体移除或大幅删减（~3,578 行） |
| `apps/cli/src/commands/execution.rs` | 移除 V8 实验，保留 `portfolio-decision` |
| `crates/execution-engine/src/lib.rs` | 输出语义变更（BuyNow→Increase 等） |
| `crates/core-domain/src/lib.rs` | DTO 字段更新（`#[serde(alias)]` 向后兼容） |
| `crates/app-service/src/lib.rs` | `daily-analysis` 新增（聚合 research + strategy + risk + portfolio）；`preclose-analysis` → `portfolio-decision` 重命名 |
| `README.md` | 命令章节重写，只展示核心 ~10 个命令 |
| `docs/日常操作手册.md` | 更新每日命令流为 3 条核心命令 |

---

## Phase 2：策略引擎重构 — 多策略独立评分 + 场景化

### 2.1 现状

`strategy-engine` 已经独立计算了四套策略的分数（`value_left_score`, `trend_pullback_score` 等），但 `signal-engine` 只取最高分合并为一个总分。

### 2.2 目标

```
每个标的输出 N 套独立评分：

000300 沪深300
  ├─ [ValueLeft]       Score: 72 → Buy      | 归因: PE低估 + MA60支撑 + RSI低位
  ├─ [TrendPullback]   Score: 45 → Hold     | 归因: 未回调到 MA20-MA60 区间
  ├─ [TrendBreakout]   Score: 58 → Hold     | 归因: 收盘未突破箱体上沿
  └─ [MomentumRight]   Score: 88 → StrongBuy| 归因: RS120持续走高 + 成交量放大

  [短线动量场景] 加权: 73 | [长线价值场景] 加权: 58
```

### 2.3 实施步骤

#### Step 2A：SignalSnapshot 扩展

```rust
pub struct SignalSnapshot {
    // 保留兼容字段
    pub final_score: f64,
    pub signal_label: SignalLabel,
    
    // 新增
    pub strategy_signals: HashMap<StrategyKind, StrategySignalDetail>,
    pub scenario_scores: HashMap<String, f64>,
}

pub struct StrategySignalDetail {
    pub strategy: StrategyKind,
    pub score: f64,
    pub label: SignalLabel,
    pub attribution: StrategyAttribution,
}
```

#### Step 2B：场景配置

`config/scenarios.toml`：

```toml
[scenarios.momentum_short]
label = "短线动量博弈"
strategies = [
    { kind = "MomentumRight", weight = 0.50 },
    { kind = "TrendBreakout", weight = 0.30 },
    { kind = "TrendPullback", weight = 0.20 },
]

[scenarios.value_long]
label = "长线价值配置"
strategies = [
    { kind = "ValueLeft", weight = 0.40 },
    { kind = "TrendPullback", weight = 0.35 },
    { kind = "TrendBreakout", weight = 0.25 },
]
```

#### Step 2C：新增命令

```bash
# 多策略视角全市场排行
cargo run -p quant-cli -- strategy-perspectives --scope cn --scenario momentum_short --mode scoreboard

# 单标的多策略详细归因
cargo run -p quant-cli -- strategy-perspectives --symbol 000300 --scope cn --mode detail
```

### 2.4 涉及文件

| 文件 | 变更 |
|------|------|
| `crates/core-domain/src/lib.rs` | SignalSnapshot 扩展；新增 StrategySignalDetail, StrategyAttribution |
| `crates/signal-engine/src/lib.rs` | 重构：不再合并策略分数，独立产出 |
| `crates/strategy-engine/src/lib.rs` | 微调：新增 StrategyAttribution 构建 |
| `crates/app-service/src/lib.rs` | 新增 strategy_perspectives orchestration；加载 scenarios.toml |
| `config/scenarios.toml` | **新建** |
| `crates/report-engine/src/lib.rs` | 适配多策略输出 |
| `crates/market-store/` | SignalSnapshot schema 扩展（`#[serde(default)]`） |
| `apps/cli/src/main.rs` | `StrategyPerspectives` 命令 |
| `apps/cli/src/commands/` | 新增 `strategy.rs` |

---

## Phase 3：LLM 增强 + 组合决策重构

### 3.1 LLM 上下文增强

传给 LLM 的内容从「单一 label + rank order」升级为：

```
- 各标的的多策略独立评分 + 归因
  "MomentumRight: 88(StrongBuy) — RS120持续走高"
  "ValueLeft: 72(Buy) — PE低估, 但趋势偏弱"
  
- 场景评分对比
  "短线场景: 73 | 长线场景: 62 | 矛盾点: 动量强但估值不便宜"
  
- 当日 research evidence 摘要
  SRD强度 / Stretch等级 / Analytics历史参照 / Integrity状态
  
- 前次分析结论摘要（连续性上下文）
- 当前 market regime + environment
```

### 3.2 对话历史持久化

每次 LLM 分析保存 `LlmAnalysisRecord`（scope + date + 输入 hash + GPT 回复 + 摘要），下次分析自动注入"上次结论 vs 现在变化"的前置上下文。

### 3.3 Prompt 模板化

`config/prompts.toml` 替代 V4.5 的 5 个固定 action，用户可定义多个"分析人格"：

```toml
[prompts.short_term_trader]
label = "短线交易员"
system = """你是一个专注于短期动量博弈的量化交易员。
你关注：日内量价特征、技术突破点、资金流向、短期动能。
你在分析时会主动对比 MomentumRight 和 TrendBreakout 的评分差异。"""

[prompts.long_term_allocator]
label = "长线配置者"
system = """你是一个关注长期资产配置的基金经理。
你关注：估值水位、趋势结构持续性、宏观环境、跨市场比较。
你在分析时会主动对比 ValueLeft 和 TrendPullback 的评分。"""
```

### 3.4 portfolio-decision 重做

用多策略证据 + LLM 替代硬编码 Pattern Library：

```
QuoteSnapshot + 多策略独立评分 + ResearchContext + 历史类比
        ↓
  构建 PortfolioDecision 上下文
        ↓
  LLM 解读（prompt = "组合决策顾问" 模板）
        ↓
  输出: Increase/Maintain/Reduce/Avoid + 依据 + 风险提示 + 历史参照
```

### 3.5 涉及文件

| 文件 | 变更 |
|------|------|
| `crates/llm-context/src/` | 增强 build_prompt，纳入多策略评分 + 历史对话 |
| `crates/app-service/src/llm.rs` | 对话历史管理；prompt 模板加载 |
| `crates/app-service/src/lib.rs` | daily-analysis 集成 LLM；portfolio-decision 增强 |
| `config/prompts.toml` | **新建** |
| `crates/execution-engine/` | portfolio-decision LLM 路径 |
| `apps/cli/src/commands/execution.rs` | 适配新输出 |

---

## 风险与注意事项

1. **Phase 1 的 ClickHouse Schema 变更**：DecisionEngine 枚举重命名需要 `#[serde(alias)]` 保证存量数据可反序列化。

2. **Phase 2 的 Schema 扩展**：SignalSnapshot 新增字段需要 `#[serde(default)]`，遵循 ADR-047 策略。存量数据不受影响。

3. **Phase 3 的 LLM 调用成本**：增强后上下文更长，每次调用 token 消耗增加约 50-80%。建议 `daily-analysis` 默认使用"简洁模式"，LLM 深度分析通过 `llm-analyze` 单独触发。

4. **前端适配**：Phase 1-3 聚焦后端。前端面板（SignalsPanel 等）在后续单独一轮适配多策略视角。

5. **场景配置的默认值**：如果 `config/scenarios.toml` 不存在，降级为"全部四种策略等权重"的默认场景。

---

## 预期效果

| 维度 | 当前 V8 | 目标 V8.1 |
|------|---------|----------|
| CLI 命令数 | 107 | ~10 核心 + ~9 隐藏 |
| 策略输出 | 1 个模糊平均分 | 4 套独立评分 + 归因 + 场景加权 |
| 决策语义 | BuyNow/Wait/NoChase | Increase/Maintain/Reduce/Avoid |
| 数据可信度 | 无标注 | 每次输出标注 integrity 状态 |
| LLM 上下文 | 单一 label + rank | 多策略矛盾点 + 历史参照 + 连续性上下文 |
| 日常命令流 | 记多个命令 | 3 条：market-refresh → daily-analysis → daily-report |
