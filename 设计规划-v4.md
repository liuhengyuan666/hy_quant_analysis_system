# V4 设计规划：Research Cognition Layer

## 1. 目标概述

V4 的核心目标不是增加更多指标或更强的 prompt，而是：

> **构建市场认知系统——让"市场状态"变得可表达、可推理、可验证。**

完成从：
```
"给 LLM 喂 prompt"
```
到：
```
"构建市场认知系统"
```
的范式转换。

---

## 2. 核心架构

### 2.1 总体架构

```
Market Data (ClickHouse/SQLite)
  ↓
Quant Engine (indicator/macro/rotation/signal/backtest)
  ↓
DashboardSnapshot（当前已有）
  ↓
research-context/（新增）
  ├── builder.rs:     DashboardSnapshot → ResearchContext
  ├── feature_engine.rs: 提取语义特征（认知因子）
  └── compression.rs:     压缩为语义状态
  ↓
ResearchContext（JSON）
  ↓
research-skills/（新增）
  ├── router.rs:      根据 Trigger DSL 匹配 Skill
  ├── executor.rs:    分层渲染 + 调用 LLM
  └── provider.rs:    模型无关抽象
  ↓
SkillOutput（JSON，经语义验证）
  ↓
Report / Dashboard（JSON core + Markdown view）
```

### 2.2 仓库结构（单仓）

```
hy-quant-analysis-system/
├── apps/
│   ├── cli/                              # CLI 入口
│   └── desktop/                          # Tauri 桌面端
├── crates/
│   ├── app-service/                      # 编排层（剥离 LLM 分析逻辑）
│   ├── core-domain/                      # 类型定义
│   ├── market-store/                     # 数据存储
│   ├── data-ingestion/                   # 数据抓取
│   ├── *-engine/                         # 计算引擎
│   ├── research-skills/                  # Skill 基础设施（新增）
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── registry.rs              # Skill Registry（加载、查询、验证）
│   │   │   ├── router.rs                # Skill Router（Trigger DSL 解析）
│   │   │   ├── executor.rs              # Skill Executor（分层渲染 + LLM 调用）
│   │   │   ├── provider.rs              # LLM Provider trait（模型无关化）
│   │   │   └── schema.rs                # Schema 验证（结构 + 金融语义）
│   │   └── skills/                       # 内置 Skill 定义
│   │       ├── macro-analysis/
│   │       │   └── SKILL.md
│   │       ├── market-regime-reasoning/
│   │       │   └── SKILL.md
│   │       ├── rotation-analysis/
│   │       │   └── SKILL.md
│   │       ├── risk-assessment/
│   │       │   └── SKILL.md
│   │       └── breadth-analysis/
│   │           └── SKILL.md
│   └── research-context/                 # 研究上下文层（新增）
│       ├── src/
│       │   ├── lib.rs
│       │   ├── builder.rs               # 从 DashboardSnapshot 构建研究上下文
│       │   ├── compression.rs           # 语义压缩（raw data → semantic state）
│       │   ├── semantic_state.rs        # 语义状态定义
│       │   └── feature_engine.rs        # 语义特征提取（认知因子）
│       └── features/                     # 特征定义
│           ├── breadth_collapse.yaml
│           ├── rotation_concentration.yaml
│           └── liquidity_fragility.yaml
├── research/                             # 研究配置（file-based）
│   ├── agents/                           # Agent / Analysis Profile
│   │   └── macro-strategist.yaml
│   ├── schemas/                          # 输出 Schema（JSON Schema）
│   │   └── market-analysis.schema.json
│   ├── policies/                         # 推理策略
│   │   └── trend-reasoning.yaml
│   └── memory/                           # 研究记忆（预留 schema）
│       └── regime-transition.schema.json
├── config/
└── ...
```

---

## 3. 关键设计

### 3.1 Research Context 层（V4 灵魂）

这是整个架构最关键的升级。真正的问题从来不是"怎么让 LLM 更聪明"，而是"怎么让市场状态变得可表达"。

```rust
// crates/research-context/src/semantic_state.rs
pub struct ResearchContext {
    pub market: MarketContext,
    pub liquidity: LiquidityContext,
    pub breadth: BreadthContext,
    pub rotation: RotationContext,
    pub regime: RegimeContext,
    pub signals: SignalsContext,
}

pub struct MarketContext {
    pub current_state: String,      // "risk_off_transition"
    pub previous_state: String,     // "risk_on"
    pub confidence: f64,            // [0, 1]
    pub drivers: Vec<String>,
    pub transition: Option<RegimeTransition>,
}

pub struct LiquidityContext {
    pub pressure: LiquidityPressure, // low / moderate / high / critical
    pub yield_curve_status: String,  // normal / flat / inverted
    pub dollar_strength: f64,
}

pub struct BreadthContext {
    pub condition: BreadthCondition, // strong / weakening / collapsed
    pub breadth_pct: f64,
    pub breadth_delta: f64,
}

pub struct RotationContext {
    pub state: RotationState,        // broad / concentrated / divergent
    pub top_sectors: Vec<SectorRotation>,
    pub leadership_stability: f64,
}

pub struct RegimeContext {
    pub current: String,
    pub confidence: f64,
    pub macro_stale_days: i32,
}

pub struct SignalsContext {
    pub bullish_count: usize,
    pub defensive_count: usize,
    pub data_starved_count: usize,
}

pub struct RegimeTransition {
    pub from: String,
    pub to: String,
    pub trigger_date: NaiveDate,
    pub confidence: f64,
}
```

**设计要点**：
- **模块化 Context**：ResearchContext 是组合体，不是全局上帝对象
- **禁止动态字段**：不允许 `HashMap<String, Value>` 或动态字段注入
- **显式 Schema**：每个子 Context 都有明确字段，便于版本控制和序列化
- 从 `DashboardSnapshot`（原始数据）提取语义状态
- 支持状态迁移追踪（`from: risk_on → to: de_risk`）
- 金融研究真正有价值的不是"当前状态"，而是"状态切换"

### 3.2 Feature Engine（认知因子）

未来会演化为 Alpha Feature Engine，提取高阶认知因子：

```yaml
# research-context/features/breadth_collapse.yaml
name: breadth_collapse
description: Detect significant breadth deterioration
inputs:
  - breadth_pct
  - breadth_5d_delta
  - breadth_pct_sma5
formula: |
  breadth_pct < 30 && breadth_5d_delta < -10
output:
  type: boolean
  labels:
    true: "breadth_collapsed"
    false: "breadth_normal"

# 未来扩展：
# - market_fragility
# - crowding
# - liquidity_exhaustion
# - gamma_pressure
# - policy_divergence
```

**设计要点**：
- Feature Engine 在 Rust 中执行，不是 DSL
- Reasoning DSL 只声明依赖关系，不执行逻辑
- 避免发明低配编程语言

### 3.3 Skill 格式（Markdown + YAML Front Matter + Reasoning DSL）

```markdown
---
name: macro-analysis
description: Analyze macro environment and provide regime reasoning
trigger:
  all:
    - macro_stale_days > 2
  any:
    - regime_changed == true
    - vix_spike > 20
  weight:
    risk: 0.8
    macro: 0.9
output_schema: market-analysis.schema.json
output_bias: neutral
priority: high
---

# Macro Analysis

## Overview
Analyze the current macro environment based on treasury yields, dollar index, and risk appetite.

## Reasoning Graph

```yaml
reasoning:
  liquidity:
    inputs:
      - dgs10
      - vix
      - dollar_index
    checks:
      - yield_curve_inversion
      - duration_compression
    outputs:
      - liquidity_state
      - risk_pressure

  regime:
    inputs:
      - liquidity_state
      - risk_pressure
    states:
      - risk_on
      - risk_off
      - neutral
      - de_risk
    transitions:
      - from: risk_on
        to: de_risk
        condition: vix > 20 && breadth_weakening
```

## Output Format
```json
{
  "regime": "risk_off",
  "confidence": 0.85,
  "key_drivers": ["yield_compression", "dollar_strength"],
  "recommendations": ["reduce_duration", "increase_quality"]
}
```
```

**设计要点**：
- Skill 本质是"认知资产"，不是代码
- Reasoning DSL 声明 inputs → checks → outputs 的依赖关系
- 支持状态机（states + transitions），捕捉状态切换
- 机器可读，支持未来验证和回测

### 3.4 Skill Router（Trigger DSL）

```yaml
trigger:
  all:          # 必须全部满足
    - macro_stale_days > 2
  any:          # 至少一个满足
    - regime_changed == true
    - vix_spike > 20
  none:         # 必须全部不满足
    - market_holiday == true
  weight:       # 权重（用于优先级排序）
    risk: 0.8
    macro: 0.9
```

Router 逻辑：
1. 解析所有 Skill 的 trigger DSL
2. 从 `ResearchContext` 读取当前语义状态
3. 评估每个 trigger 条件：`all` 和 `any` 同时存在时为 AND 关系；`none` 为独立反向条件，与前两者为 AND 关系
4. 按 weight 排序，返回匹配的 Skill 列表

### 3.5 Skill Executor（分层渲染 + Token Budget）

**不要直接拼 giant prompt**。分层处理 + Token Budget 控制：

```
Layer 1: System Layer
  └── research style, risk tolerance, institutional perspective

Layer 2: Semantic Layer
  └── ResearchContext (JSON)

Layer 3: Reasoning Layer
  └── inputs / checks / outputs / transitions (YAML)

Layer 4: Rendering Layer
  └── final narrative (Markdown)
```

**Token Budget 控制**：

```rust
// crates/research-skills/src/token_budget.rs
pub struct TokenBudget {
    pub max_system_tokens: usize,      // 例如 1024
    pub max_context_tokens: usize,     // 例如 2048
    pub max_reasoning_tokens: usize,   // 例如 1536
    pub max_output_tokens: usize,      // 例如 2048
}

impl TokenBudget {
    /// 裁剪 context：低优先级子 Context 设为 None
    /// 优先级：market > liquidity > regime > breadth > rotation > signals
    pub fn fit_context(&self, context: &mut ResearchContext) {
        // 裁剪策略：
        // 1. 计算当前 context token 估算值
        // 2. 如果超限，按优先级从低到高将子 Context 设为 None
        // 3. 优先保留：market > liquidity > regime > breadth > rotation > signals
    }
}
```

**设计要点**：
- 每层独立渲染，独立计算 token
- Context 过长时动态裁剪（低优先级子 Context 设为 `None`）
- Reasoning graph 过大时折叠次要分支
- Token 计数使用近似字符数估算（4 chars ≈ 1 token），无需引入 tiktoken 依赖
- 防止 GPT/Kimi/Claude 出现 hallucinate / truncate / reasoning collapse

### 3.6 Agent Profile（深度配置）

```yaml
# research/agents/macro-strategist.yaml
name: macro-strategist
description: Macro strategist analysis profile

reasoning_style:
  - macro_topdown
  - institutional

risk_tolerance: conservative

output_depth: deep

output_format: json

priority:
  macro: 0.8
  technical: 0.2
  sentiment: 0.5

analysis_constraints:          # 分析约束（影响推理方向，不修改事实）
  preferred_factors:
    - macro
    - liquidity
  emphasis:
    regime_transition: high
    breadth_signal: medium
  tone: cautious                 # rendering tone：cautious / neutral / optimistic

skills:
  - macro-analysis
  - liquidity-analysis
  - risk-off-detection

model: gpt-4o
system_prompt: |
  You are a senior macro strategist at a quant hedge fund...
  Your reasoning style is {reasoning_style}.
  Your risk tolerance is {risk_tolerance}.
```

### 3.7 Schema 验证（结构 + 金融语义）

不仅验证 JSON 结构，还验证金融语义：

```json
{
  "regime": "risk_off",
  "confidence": 0.85,        // ✓ 在 [0,1] 范围内
  "key_drivers": [...],
  "recommendations": [...]   // ✓ 在允许的 taxonomy 内
}
```

语义验证规则：
- `confidence ∈ [0, 1]`
- `regime` 必须在预定义集合内
- `recommendations` 必须在允许的分类体系内
- 状态迁移必须合法（不能从 risk_off 直接到 risk_on，必须经过 neutral）

### 3.8 Skill Metadata + Versioning（认知资产标准）

每个 Skill 必须包含完整元数据 + 版本控制：

```yaml
# SKILL.md front matter
name: macro-analysis
description: Analyze macro environment and provide regime reasoning
version: "1.0.0"                    # 语义化版本
author: "quant-team"

compatibility:                       # 兼容性声明
  context: ">=1.0"                  # 依赖的 ResearchContext 版本
  schema: ">=1.0"                   # 输出 schema 版本

trigger:
  all:
    - macro_stale_days > 2
  weight:
    risk: 0.8

inputs:                        # 明确输入依赖
  - market_context.liquidity.dollar_strength
  - market_context.regime.current
  - breadth_context.condition

outputs:                       # 明确输出结构
  - regime
  - confidence
  - key_drivers
  - recommendations

dependencies:                  # 依赖其他 skill（执行顺序）
  - liquidity-analysis

confidence_model:              # 置信度模型
  base: 0.7
  factors:
    - data_freshness
    - signal_strength

failure_modes:                 # 已知失败模式
  - condition: "macro_stale_days > 5"
    action: "reduce_confidence"
    message: "Macro data severely stale"

evaluation_metrics:            # 评估指标
  - regime_accuracy
  - transition_detection_rate
  - false_positive_rate

output_schema: market-analysis.schema.json
priority: high
```

**设计要点**：
- Skill 是完整的认知资产，不只是 prompt
- **版本控制**：语义化版本（semver），支持回测旧版本
- **兼容性声明**：明确依赖的 Context/Schema 版本
- 明确 inputs/outputs，支持未来编排和聚合
- confidence_model 和 failure_modes 支持运行时自适应
- evaluation_metrics 支持 benchmark 和演进

### 3.9 Deterministic Mode（可复现分析）

量化系统必须支持可复现分析：

```rust
// crates/research-skills/src/deterministic.rs
pub struct DeterministicConfig {
    pub temperature: f64,      // 0.0（完全确定）
    pub seed: u64,             // 42（固定种子）
    pub top_p: f64,            // 0.1（低随机性）
    pub max_tokens: usize,
}

impl Default for DeterministicConfig {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            seed: 42,
            top_p: 0.1,
            max_tokens: 2048,
        }
    }
}
```

**CLI 支持**：
```bash
# 默认模式（可能有轻微随机性）
cargo run -p quant-cli -- analyze --scope global --skill market-regime-reasoning

# 确定性模式（完全可复现）
cargo run -p quant-cli -- analyze --scope global --skill market-regime-reasoning --deterministic

# 指定种子
cargo run -p quant-cli -- analyze --scope global --skill market-regime-reasoning --seed 123
```

**设计要点**：
- 默认模式：temperature > 0，适合探索性分析
- 确定性模式：temperature = 0, seed 固定，适合回测和 debug
- 所有输出必须包含使用的配置（seed、temperature），便于追溯
- **注意**：主流 LLM provider 不保证 temperature=0 时的比特级可复现性。Deterministic Mode 提供最佳努力保证，实际 consistency 由 Eval Harness 的 `consistency` 指标度量。

### 3.10 Skill Evaluation Harness（评估框架）

必须建立标准化评估能力：

```
crates/research-benchmark/              # 新增：评估框架
├── src/
│   ├── lib.rs
│   ├── harness.rs                     # 评估引擎
│   ├── metrics.rs                     # 指标计算
│   └── reporters.rs                   # 报告生成
└── benchmarks/                         # 基准测试集
    ├── market-regime-reasoning/
    │   ├── snapshots/                 # 固定输入快照
    │   ├── expected/                  # 期望输出
    │   └── outputs/                   # 实际输出
    └── macro-analysis/
        ├── snapshots/
        ├── expected/
        └── outputs/
```

**评估指标**：

| 指标 | 说明 | 目标 |
|------|------|------|
| consistency | 相同输入多次运行的输出一致性 | > 95% |
| hallucination score | 输出中事实错误的比例 | < 5% |
| schema pass rate | 输出符合 JSON Schema 的比例 | 100% |
| semantic validity | 金融语义验证通过率 | > 90% |
| latency | 单次分析耗时 | < 10s |
| token cost | 平均 token 消耗 | < 4000 |

**CLI 支持**：
```bash
# 运行单个 skill 的 benchmark
cargo run -p quant-cli -- benchmark-skill market-regime-reasoning

# 跨模型比较
cargo run -p quant-cli -- benchmark-skill market-regime-reasoning --models gpt-4o,kimi-v1,deepseek-chat

# 输出详细报告
```

**设计要点**：
- 固定输入（snapshot）+ 固定 context + 多模型输出比较
- 支持版本对比（v1.0 vs v1.1 的 benchmark 差异）
- 评估结果必须量化，不能是"我感觉不错"

### 3.11 ResearchAnalysis（标准研究对象）

V4 提前定义标准研究输出结构：

```rust
// crates/research-skills/src/analysis.rs
pub struct ResearchAnalysis {
    pub meta: AnalysisMeta,
    pub thesis: Thesis,
    pub evidence: Vec<Evidence>,
    pub risks: Vec<Risk>,
    pub recommendations: Vec<Action>,
    pub confidence: ConfidenceScore,
    pub reasoning_trace: ReasoningTrace,
}

pub struct AnalysisMeta {
    pub skill_name: String,
    pub agent_profile: String,
    pub scope: ReportScope,
    pub analysis_date: NaiveDate,
    pub version: String,
}

pub struct Thesis {
    pub statement: String,
    pub conviction: f64,           // [0, 1]
    pub time_horizon: String,      // short / medium / long
}

pub struct Evidence {
    pub source: String,
    pub data_point: String,
    pub strength: f64,             // [0, 1]
}

pub struct Risk {
    pub category: String,
    pub severity: String,          // low / medium / high / critical
    pub probability: f64,          // [0, 1]
    pub mitigation: Option<String>,
}

pub struct Action {
    pub action_type: String,       // reduce_exposure / increase_quality / hedge / monitor
    pub target: String,
    pub urgency: String,           // immediate / near_term / watch
    pub rationale: String,
}

pub struct ConfidenceScore {
    pub overall: f64,              // [0, 1]
    pub data_quality: f64,
    pub model_fit: f64,
    pub market_clarity: f64,
}

pub struct ReasoningTrace {
    pub steps: Vec<ReasoningStep>,
    pub assumptions: Vec<String>,
    pub alternative_scenarios: Vec<String>,
}

pub struct ReasoningStep {
    pub step_number: usize,
    pub premise: String,
    pub conclusion: String,
    pub confidence: f64,
}
```

**设计要点**：
- 所有 Skill 输出统一的 `ResearchAnalysis` 结构
- 支持跨 Skill 聚合、比较、记忆
- `reasoning_trace` 支持可验证推理
- `confidence` 多维度分解，支持不确定性量化

---

## 4. 架构约束（Architecture Constraints）

### 4.1 ResearchContext 约束

- **禁止全局上帝对象**：ResearchContext 必须是模块化组合，不允许无限膨胀
- **禁止动态字段**：不允许 `HashMap<String, Value>` 或动态字段注入
- **显式 Schema**：所有字段必须在编译期确定，便于版本控制和序列化
- **向后兼容**：新增字段必须 Optional，不允许删除已有字段

### 4.2 Trigger DSL 约束

- **禁止表达式**：不允许 `(a && b) || (c && !d)` 等任意表达式
- **禁止嵌套**：不允许嵌套逻辑树
- **禁止函数**：不允许函数调用或动态脚本
- **禁止运行时代码执行**：Trigger DSL 不是脚本语言
- **只允许**：`field operator value` 形式（例如 `breadth_pct < 30`）
- **Router 职责**：Router 只负责 Condition Matching，不是 rule engine

### 4.3 Reasoning DSL 约束

- **只声明依赖**：Reasoning DSL 只声明 inputs → checks → outputs 的关系
- **不执行逻辑**：所有逻辑必须在 Rust Feature Engine 中执行
- **无控制流**：不允许 if/else、loop、nested execution
- **无状态修改**：Reasoning DSL 不修改任何状态

### 4.4 Skill Output 约束

- **必须结构化**：禁止 Markdown-only 输出
- **必须 JSON Core**：所有 Skill 必须输出 `ResearchAnalysis`（JSON）
- **必须语义验证**：输出必须通过结构和金融语义双重验证
- **必须可聚合**：输出结构支持跨 Skill 聚合和比较

### 4.5 Agent Profile 约束

- **不允许修改事实层**：Agent 只能影响 rendering、emphasis、prioritization
- **不允许修改市场状态**：Agent 不能修改 market state、feature output、regime detection
- **不允许引入偏见**：`analysis_constraints` 只影响分析视角，不扭曲事实
- **必须可 Benchmark**：不同 Agent Profile 在相同 Context 下的输出必须可比

### 4.6 模块依赖约束

```
research-context/      # 底层：只依赖 core-domain 和 market-store
  ↓
research-skills/       # 中层：依赖 research-context，不依赖 app-service
  ↓
app-service/           # 上层：依赖 research-skills，编排调用
```

- **禁止循环依赖**：research-skills 不能依赖 app-service
- **禁止跨越调用**：Router 不能直接调用 market-store
- **分层清晰**：Context → Skills → Orchestration

---

## 5. 实施路线图

### 核心原则：先做少，但做深

**不要立刻**：
- 写 50 个 skills
- 做 20 个 agents
- 接 10 个模型

**而是**：真正打磨 1 个完整链路，反复验证稳定性。

### Wave 1: 研究上下文层（3-4 周）

**目标**：构建 `crates/research-context`，实现语义压缩

- [ ] 定义 `ResearchContext`、`SemanticState`、`RegimeTransition` 类型
- [ ] 实现 `ContextBuilder`（从 DashboardSnapshot 构建）
- [ ] 实现 `FeatureEngine`（5-10 个语义特征）
- [ ] 实现 `ContextCompression`（raw data → semantic state）
- [ ] 集成到 `app-service`，在 `dashboard_snapshot` 后生成 `ResearchContext`
- [ ] CLI `research-context` 命令

**验收**：
```bash
cargo run -p quant-cli -- research-context --scope global
# 输出 JSON 格式的 ResearchContext，包含语义状态和状态迁移
```

### Wave 2: Skill 基础设施（4-5 周）

**目标**：构建 `crates/research-skills` + `crates/research-benchmark` 骨架

- [ ] 定义 Skill Markdown + YAML front matter + Reasoning DSL 规范
- [ ] 实现 `SkillRegistry`（加载、验证、查询）
- [ ] 实现 `TriggerDsl` 解析器（all/any/none/weight）
- [ ] 实现 `SkillRouter`（基于 ResearchContext 匹配 Skill）
- [ ] 实现分层 `SkillExecutor`（system → semantic → reasoning → rendering）
- [ ] 实现 `LlmProvider` trait（OpenAI, DeepSeek）
- [ ] 实现 Schema 验证（结构 + 金融语义）
- [ ] 实现 `TokenBudget`（动态裁剪 context）
- [ ] 实现 `DeterministicConfig`（可复现模式）
- [ ] **新增**：实现 `research-benchmark` 骨架（harness.rs + metrics.rs + reporters.rs）

**Provider trait 最小接口**：
```rust
pub trait LlmProvider: Send + Sync {
    async fn chat(
        &self,
        system_prompt: &str,
        messages: &[Message],
        config: &LlmConfig,
    ) -> Result<String>;

    fn token_count(&self, text: &str) -> usize;
}
```
约束：同步接口、无缓存提示词、不含密钥日志、支持 `unimplemented!()` 的 `stream` 方法预留。

**验收**：
```bash
cargo run -p quant-cli -- list-skills
cargo run -p quant-cli -- run-skill macro-analysis --scope global
```

### Wave 3: 1 个完整 Skill 链路 + Benchmark（3-4 周）

**目标**：打磨 1 个完整的 Skill，验证全链路稳定性 + Benchmark 能力

选择 `market-regime-reasoning` skill：
- [ ] 定义完整的 SKILL.md（front matter + reasoning DSL）
- [ ] 定义输出 schema（含金融语义验证规则）
- [ ] 实现状态机（risk_on → neutral → risk_off → de_risk）
- [ ] 集成 Router（trigger 条件）
- [ ] 集成 Executor（分层渲染）
- [ ] 集成 Provider（OpenAI + DeepSeek）
- [ ] **Skill Benchmarking**：测试同一 skill 在不同模型上的稳定性
- [ ] **Reasoning Validation**：验证 reasoning graph 的正确性
- [ ] **Benchmark 基准建立**：创建 snapshots/expected/outputs，跑首次 benchmark

**验收**：
```bash
cargo run -p quant-cli -- analyze --scope global --skill market-regime-reasoning --format json
cargo run -p quant-cli -- analyze --scope global --skill market-regime-reasoning --format markdown
cargo run -p quant-cli -- benchmark-skill market-regime-reasoning
```

### Wave 4: 扩展到 3-5 个 Skills（4-5 周）

- [ ] `macro-analysis` skill
- [ ] `rotation-analysis` skill
- [ ] `risk-assessment` skill
- [ ] `breadth-analysis` skill

每个 Skill 都经过完整的定义 → 测试 → benchmark 流程。

### Wave 5: Agent Profiles（2-3 周）

- [ ] 定义 Agent Profile YAML 格式（含 analysis_constraints / rendering tone）
- [ ] `macro-strategist` profile
- [ ] `risk-manager` profile
- [ ] `technical-analyst` profile
- [ ] CLI `analyze --agent <profile>`
- [ ] **Agent Benchmarking**：比较不同 profile 的输出差异

### Wave 6: 结构化输出 + Desktop 集成 + V3 迁移（3-4 周）

- [ ] 定义 `ResearchAnalysis` JSON schema
- [ ] 实现 Markdown 渲染器（从 JSON 生成报告）
- [ ] CLI `--format json|markdown`
- [ ] **Desktop 集成**：Tauri command wrapper 调用 `research-skills`，Refresh data 后自动触发 Skill 分析
- [ ] 桌面端 JSON 数据驱动渲染
- [ ] 将 V3 `analyze-with-llm` 迁移到 Skill 系统（deprecation notice → wrapper → removal）
- [ ] 端到端测试

**总计**：18-24 周（4.5-6 个月）

---

## 6. 边界控制（明确不做）

V4 明确不做：
- ❌ Multi-Agent 编排
- ❌ Consensus Engine
- ❌ Research Memory（预留 schema，不实现）
- ❌ MCP Platform
- ❌ Marketplace / 第三方 Skill
- ❌ 企业权限系统（ACL / audit / approval）

这些属于 V5-V7 的范畴。

---

## 7. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Reasoning DSL 过度设计 | 中 | 高 | 严格限制 DSL 只声明依赖，不执行逻辑 |
| Feature Engine 语义失真 | 中 | 高 | 保留原始 DashboardSnapshot，Context 作为补充 |
| Skill Executor prompt 爆炸 | 中 | 高 | 强制分层渲染（system → semantic → reasoning → rendering） |
| LLM Provider 抽象泄漏 | 低 | 中 | 严格限制 Provider trait 接口 |
| 与现有系统耦合 | 中 | 中 | 通过 trait 隔离，不改变现有 engine 核心逻辑 |
| 认知抽象不稳定 | 高 | 高 | **先做少做深**，1 个 skill 打磨稳定后再扩展 |

---

## 8. 成功标准

### V4 完成时，系统应具备：

1. **研究上下文层**：DashboardSnapshot → ResearchContext → Semantic State（含状态迁移）
2. **认知因子**：5-10 个语义特征（breadth_collapse, rotation_concentration 等）
3. **Skill 系统**：3-5 个内置 Skill，支持 Markdown + YAML front matter + Reasoning DSL
4. **Skill Router**：基于 Trigger DSL 自动匹配 Skill
5. **分层 Executor**：system → semantic → reasoning → rendering
6. **Token Budget**：动态裁剪 context，防止 token 爆炸
7. **模型无关**：支持 OpenAI、DeepSeek（通过 Provider trait）
8. **Agent Profile**：3 个分析配置（含 analysis_constraints）
9. **结构化输出**：JSON 核心 + Markdown 渲染
10. **金融语义验证**：confidence ∈ [0,1]、状态迁移合法性、recommendation taxonomy
11. **Skill Versioning**：语义化版本控制，支持回测旧版本
12. **Deterministic Mode**：temperature=0, seed 固定，支持可复现分析
13. **Skill Evaluation Harness**：固定输入 + 多模型比较 + 量化指标
14. **Skill Benchmarking**：consistency、hallucination score、schema pass rate

### V4 完成时，用户应能：

```bash
# 查看当前研究上下文（语义状态）
cargo run -p quant-cli -- research-context --scope global

# 列出所有可用 skill
cargo run -p quant-cli -- list-skills

# 使用特定 skill 分析
cargo run -p quant-cli -- analyze --scope global --skill market-regime-reasoning

# 使用 agent profile 分析
cargo run -p quant-cli -- analyze --scope global --agent macro-strategist

# 输出 JSON 格式
cargo run -p quant-cli -- analyze --scope global --format json

# 输出 Markdown 格式
cargo run -p quant-cli -- analyze --scope global --format markdown
```

---

## 9. 与 V3 的对比

| 能力 | V3 | V4 |
|------|-----|-----|
| LLM 分析 | 硬编码 `analyze-with-llm` | 可配置 Skill 系统 |
| 分析类型 | 单一（报告解读） | 多 Skill（宏观、regime、轮动、风险、广度） |
| 上下文 | Raw DashboardSnapshot | 语义压缩的 ResearchContext（含状态迁移） |
| Reasoning | Prose prompt | Machine-readable Reasoning DSL |
| 模型支持 | OpenAI-compatible | OpenAI + DeepSeek + 可扩展 |
| 输出格式 | Markdown-only | JSON core + Markdown view |
| Agent | 无 | Agent Profile（含 analysis_constraints / rendering tone） |
| 触发方式 | 手动 CLI | Skill Router（自动匹配） |
| 验证 | 无 | 金融语义验证（confidence、状态迁移合法性） |
| 扩展性 | 修改代码 | 添加 Skill 文件 |

---

## 10. 长期演进路线

```
V4: Research Cognition Layer
  └─ Structured Output + Skill System + Provider Abstraction + Research Context
     └─ 先做少做深：1 个完整链路打磨稳定

V5: Research Memory
  └─ Regime transition history + Analysis memory + Skill evolution tracking

V6: Multi-Agent
  └─ Macro Agent + China Agent + Risk Agent + Orchestrator

V7: Research OS
  └─ Marketplace + Third-party skills + Consensus Engine
```

---

## 11. 附录

### A. 完整模块结构

```
crates/
├── research-context/                   # 研究上下文层
│   ├── src/
│   │   ├── lib.rs
│   │   ├── builder.rs
│   │   ├── compression.rs
│   │   ├── semantic_state.rs
│   │   └── feature_engine.rs
│   └── features/
│       ├── breadth_collapse.yaml
│       ├── rotation_concentration.yaml
│       └── liquidity_fragility.yaml
│
├── research-skills/                    # Skill 基础设施
│   ├── src/
│   │   ├── lib.rs
│   │   ├── registry.rs
│   │   ├── router.rs
│   │   ├── executor.rs
│   │   ├── provider.rs
│   │   ├── schema.rs
│   │   ├── token_budget.rs            # Token 预算控制
│   │   └── deterministic.rs           # 确定性模式
│   └── skills/
│       ├── macro-analysis/
│       │   ├── SKILL.md
│       │   ├── schema.json
│       │   └── tests/
│       └── market-regime-reasoning/
│           ├── SKILL.md
│           ├── schema.json
│           └── tests/
│
├── research-benchmark/                 # 评估框架（新增）
│   ├── src/
│   │   ├── lib.rs
│   │   ├── harness.rs
│   │   ├── metrics.rs
│   │   └── reporters.rs
│   └── benchmarks/
│       ├── market-regime-reasoning/
│       │   ├── snapshots/             # 固定输入
│       │   ├── expected/              # 期望输出
│       │   └── outputs/               # 实际输出
│       └── macro-analysis/
│           ├── snapshots/
│           ├── expected/
│           └── outputs/
```

### B. Agent Profile 目录结构

```
research/agents/
├── macro-strategist.yaml
├── risk-manager.yaml
├── technical-analyst.yaml
└── custom/                               # 用户自定义
    └── my-profile.yaml
```

### C. 关键技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| Skill 格式 | Markdown + YAML front matter | 可读性 + 结构化（跟随 Anthropic 实践） |
| Trigger DSL | YAML（all/any/none/weight） | 简单、声明式、可扩展 |
| Reasoning DSL | YAML（inputs/checks/outputs/states/transitions） | 机器可读、支持状态机 |
| Schema 验证 | JSON Schema + 自定义语义验证 | 结构验证 + 金融语义验证 |
| LLM Provider | async-trait | Rust 标准模式 |
| 模板引擎 | Handlebars / Tera | 简单、成熟 |
| 配置加载 | config crate | Rust 生态标准 |
| Token Budget | 自定义（基于 tiktoken 估算） | 防止 context 溢出 |
| 版本控制 | SemVer | 支持回测和兼容性检查 |
| 评估框架 | 自定义 benchmark harness | 固定输入 + 多模型比较 |
| 确定性模式 | LLM temperature=0 + seed | 可复现分析 |

### D. 核心原则

1. **Reasoning DSL 只声明依赖，不执行逻辑** — 逻辑在 Rust Feature Engine
2. **分层渲染，不拼 giant prompt** — system → semantic → reasoning → rendering
3. **先做少，但做深** — 1 个完整链路打磨稳定后再扩展
4. **单仓优先** — 通过目录结构实现逻辑分离
5. **JSON 核心 + Markdown 渲染** — 不是 JSON-only，也不是 Markdown-only

### E. Architecture Constraints 速查

```yaml
# 必须遵守的架构约束
dos:
  - ResearchContext 模块化（market/liquidity/breadth/rotation/regime/signals）
  - Trigger DSL 只支持 field operator value
  - Reasoning DSL 只声明依赖关系
  - Skill 必须输出 ResearchAnalysis（JSON）
  - Agent 只影响 rendering，不修改事实层
  - Token Budget 动态裁剪 context
  - Skill 必须语义化版本控制
  - 支持 Deterministic Mode（temperature=0, seed 固定）
  - 建立 Evaluation Harness（固定输入 + 多模型比较）
donts:
  - ResearchContext 成为全局上帝对象
  - Trigger DSL 支持任意表达式或嵌套逻辑
  - Reasoning DSL 执行逻辑或控制流
  - Skill 输出 Markdown-only
  - Agent 修改 market state 或 feature output
  - Context 全部塞入 prompt（必须裁剪）
  - Skill 无版本控制直接修改
  - 分析结果不可复现（无 deterministic 模式）
  - 评估凭感觉（必须量化指标）
