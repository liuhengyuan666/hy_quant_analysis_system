# V4 最终规划：Research Skill System

## 一、V4 定位

**不是 Research OS，而是 Research Cognition Layer。**

V4 的核心目标：
> 将市场认知结构化，构建可配置、可扩展、模型无关的研究技能系统。

当前做：
```
Structured Output + Skill System + Provider Abstraction + Research Context
```

不做（V5+）：
```
Multi-Agent / Memory Graph / MCP Platform / Consensus Engine / Marketplace
```

---

## 二、核心架构

### 2.1 仓库结构（单仓）

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
│   │   │   ├── executor.rs              # Skill Executor（渲染 + 调用 LLM）
│   │   │   ├── provider.rs              # LLM Provider trait（模型无关化）
│   │   │   └── schema.rs                # Schema 验证（JSON Schema）
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
│       │   └── feature_engine.rs        # 语义特征提取
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

### 2.2 数据流

```
Market Data (ClickHouse)
  ↓
Quant Engine (indicator/macro/rotation/signal)
  ↓
DashboardSnapshot (当前已有)
  ↓
research-context/
  ├── builder.rs:     DashboardSnapshot → ResearchContext
  ├── feature_engine.rs: 提取语义特征
  └── compression.rs:     压缩为语义状态
  ↓
ResearchContext (JSON)
  ↓
research-skills/
  ├── router.rs:      根据 Trigger DSL 匹配 Skill
  ├── executor.rs:    渲染 Skill Template + 调用 LLM
  └── provider.rs:    模型无关调用
  ↓
SkillOutput (JSON)
  ↓
Report / Dashboard (JSON core + Markdown view)
```

---

## 三、核心设计

### 3.1 Skill 格式（Markdown + YAML Front Matter + Reasoning DSL）

采用 Anthropic 风格，但增强 reasoning 结构化：

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
skills:
  - liquidity-analysis
  - risk-off-detection
output_schema: market-analysis.schema.json
output_bias: neutral
priority: high
---

# Macro Analysis

## Overview
Analyze the current macro environment...

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

**关键设计**：
- `reasoning` 部分使用 YAML DSL，不是 prose
- 机器可读的研究规则（inputs → checks → outputs）
- 支持状态机（states + transitions）
- LLM 渲染时读取 reasoning graph，不是 giant prompt

### 3.2 Skill Router（Trigger DSL）

```yaml
trigger:
  all:          # 必须全部满足
    - hk_breadth < 10
    - hstech_rs60 < -5
  any:          # 至少一个满足
    - usd_strengthening == true
    - us10y_rising == true
  none:         # 必须全部不满足
    - market_holiday == true
  weight:       # 权重（用于优先级排序）
    risk: 0.8
    macro: 0.9
```

Router 逻辑：
1. 解析所有 Skill 的 trigger DSL
2. 从 `ResearchContext` 读取当前状态
3. 评估每个 trigger 条件
4. 按 weight 排序，返回匹配的 Skill 列表

### 3.3 Agent Profile（深度配置）

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

**关键设计**：
- `reasoning_style` 定义研究员人格
- `risk_tolerance` 影响输出倾向
- `priority` 定义不同维度的权重
- 不是简单的 skill list，而是完整的分析配置

### 3.4 Research Context（语义压缩层）

```rust
// crates/research-context/src/semantic_state.rs
pub struct ResearchContext {
    pub market_state: MarketState,        // risk_on / risk_off / neutral / transition
    pub breadth_condition: BreadthCondition,  // strong / weakening / collapsed
    pub rotation_state: RotationState,    // broad / concentrated / divergent
    pub liquidity_pressure: LiquidityPressure, // low / moderate / high / critical
    pub regime_transition: Option<RegimeTransition>,
    pub key_signals: Vec<KeySignal>,
}

pub struct MarketState {
    pub current: String,                  // "risk_off_transition"
    pub previous: String,                 // "risk_on"
    pub confidence: f64,
    pub drivers: Vec<String>,
}
```

Feature Engine：
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
```

### 3.5 LLM Provider（模型无关化）

```rust
// crates/research-skills/src/provider.rs
pub trait LlmProvider {
    async fn chat(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        config: &LlmConfig,
    ) -> Result<String>;
}

pub struct OpenAiProvider;
pub struct DeepSeekProvider;
pub struct ClaudeProvider;
```

---

## 四、实施路线图

### Wave 1: 研究上下文层（3-4 周）

**目标**：构建 `crates/research-context`

- [ ] 定义 `ResearchContext` 和 `SemanticState` 类型
- [ ] 实现 `ContextBuilder`（从 DashboardSnapshot 构建）
- [ ] 实现 `FeatureEngine`（语义特征提取）
- [ ] 实现 `ContextCompression`（raw data → semantic state）
- [ ] 定义 5-10 个内置语义特征（breadth_collapse, rotation_concentration 等）
- [ ] 集成到 `app-service`，在 `dashboard_snapshot` 后生成 `ResearchContext`

**验收标准**：
```bash
cargo run -p quant-cli -- research-context --scope global
# 输出 JSON 格式的 ResearchContext
```

### Wave 2: Skill 基础设施（4-5 周）

**目标**：构建 `crates/research-skills`

- [ ] 定义 Skill Markdown + YAML front matter 格式规范
- [ ] 实现 `SkillRegistry`（加载、验证、查询 skills/ 目录）
- [ ] 实现 `TriggerDsl` 解析器（all/any/none/weight）
- [ ] 实现 `SkillRouter`（基于 ResearchContext 匹配 Skill）
- [ ] 实现 `SkillExecutor`（模板渲染 + LLM 调用）
- [ ] 实现 `LlmProvider` trait（OpenAI, DeepSeek）
- [ ] 实现 `SchemaValidator`（JSON Schema 验证）
- [ ] 定义内置 Skill YAML DSL 规范

**验收标准**：
```bash
# 列出所有可用 skill
cargo run -p quant-cli -- list-skills

# 手动执行 skill
cargo run -p quant-cli -- run-skill macro-analysis --scope global
```

### Wave 3: 内置 Skills（3-4 周）

**目标**：创建 5 个核心 Skill

- [ ] `macro-analysis` skill（宏观分析）
- [ ] `market-regime-reasoning` skill（regime 推理 + 状态机）
- [ ] `rotation-analysis` skill（轮动分析）
- [ ] `risk-assessment` skill（风险评估）
- [ ] `breadth-analysis` skill（广度分析）

每个 Skill 包含：
- SKILL.md（Markdown + front matter + reasoning DSL）
- 输出 Schema（JSON Schema）
- 测试用例

**验收标准**：
```bash
cargo run -p quant-cli -- analyze --scope global --skill macro-analysis
# 输出 JSON 格式的分析结果
```

### Wave 4: Agent Profiles（2-3 周）

**目标**：创建可配置的分析配置

- [ ] 定义 Agent Profile YAML 格式
- [ ] 创建 `macro-strategist` profile
- [ ] 创建 `risk-manager` profile
- [ ] 创建 `technical-analyst` profile
- [ ] CLI `analyze` 命令支持 `--agent <profile>`
- [ ] 支持 profile 级别的 model 选择、reasoning style、risk tolerance

**验收标准**：
```bash
cargo run -p quant-cli -- analyze --scope global --agent macro-strategist
# 使用 macro-strategist 的配置执行分析
```

### Wave 5: 结构化输出（2-3 周）

**目标**：JSON 核心 + Markdown 渲染

- [ ] 定义 `ResearchAnalysis` JSON schema
- [ ] 修改 `report-engine` 支持 JSON 输出
- [ ] 实现 Markdown 渲染器（从 JSON 生成报告）
- [ ] 修改 CLI 支持 `--format json|markdown`
- [ ] 修改桌面端支持 JSON 数据驱动渲染
- [ ] 将 V3 `analyze-with-llm` 迁移到 Skill 系统

**验收标准**：
```bash
cargo run -p quant-cli -- analyze --scope global --format json
cargo run -p quant-cli -- analyze --scope global --format markdown
```

### Wave 6: 集成验证（2 周）

- [ ] 端到端测试（Context → Router → Skill → Output）
- [ ] 性能测试（Skill 加载、Router 匹配、LLM 调用）
- [ ] 文档更新（架构文档、操作手册、Skill 开发指南）
- [ ] 示例 Skill 模板

**总计**：16-21 周（4-5 个月）

---

## 五、关键设计决策

### 决策 1: Skill 是 Markdown + YAML，不是纯代码

**原因**：
- Skill 是认知资产，不是执行逻辑
- Markdown 便于人工阅读和编辑
- YAML front matter 便于机器解析
- 与 Anthropic 实践一致

### 决策 2: Reasoning DSL 不是 Prompt

**原因**：
- Prompt 是写给 LLM 的文本，不利于机器理解
- Reasoning DSL 定义 inputs → checks → outputs 的关系
- 支持 Router、Agent、Consensus 等上层逻辑
- 未来可用于自动验证和测试

### 决策 3: 单仓优先，通过目录分离

**原因**：
- 当前 Skill 数量 = 0，过早分仓增加维护成本
- Cargo workspace 共享依赖，编译效率高
- trait 和 schema 仍在演化，单仓便于重构
- 跟随 Anthropic 实践

### 决策 4: 预留 Multi-Agent，不实现

**原因**：
- 当前需求：个人研究者，单 Agent 足够
- 先验证 Skill 系统的价值，再扩展多 Agent
- 通过 Agent Profile 预留接口（reasoning_style, priority 等）
- V5 再考虑真正的 Multi-Agent 编排

### 决策 5: Research Memory 预留 Schema，不实现

**原因**：
- 研究记忆（regime transition、历史分析）是高价值功能
- 但依赖 Skill 系统稳定运行后才能积累数据
- V4 先定义 schema（regime-transition.schema.json）
- V5 实现存储和查询

---

## 六、风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Skill YAML DSL 设计不当 | 中 | 高 | 先实现 2-3 个 Skill 验证格式，再批量创建 |
| Research Context 压缩失真 | 中 | 高 | 保留原始 DashboardSnapshot，Context 作为补充 |
| LLM Provider 抽象泄漏 | 低 | 中 | 严格限制 Provider trait 接口，仅支持 chat 方法 |
| Skill 数量增长后 Router 性能 | 低 | 中 | Router 匹配逻辑简单（条件评估），可缓存结果 |
| 与现有系统耦合 | 中 | 中 | 通过 trait 隔离，不改变现有 engine 和 app-service 核心逻辑 |

---

## 七、成功标准

### V4 完成时，系统应具备：

1. **研究上下文层**：DashboardSnapshot → ResearchContext → Semantic State
2. **Skill 系统**：5 个内置 Skill，支持 Markdown + YAML front matter + Reasoning DSL
3. **Skill Router**：基于 Trigger DSL 自动匹配 Skill
4. **模型无关**：支持 OpenAI、DeepSeek（通过 Provider trait）
5. **Agent Profile**：3 个分析配置（macro-strategist、risk-manager、technical-analyst）
6. **结构化输出**：JSON 核心 + Markdown 渲染
7. **CLI 集成**：`analyze --skill`、`analyze --agent`、`list-skills`、`run-skill`

### V4 完成时，用户应能：

```bash
# 查看当前研究上下文
cargo run -p quant-cli -- research-context --scope global

# 列出所有可用 skill
cargo run -p quant-cli -- list-skills

# 使用特定 skill 分析
cargo run -p quant-cli -- analyze --scope global --skill macro-analysis

# 使用 agent profile 分析
cargo run -p quant-cli -- analyze --scope global --agent macro-strategist

# 输出 JSON 格式
cargo run -p quant-cli -- analyze --scope global --format json

# 输出 Markdown 格式
cargo run -p quant-cli -- analyze --scope global --format markdown
```

---

## 八、与 V3 的对比

| 能力 | V3 | V4 |
|------|-----|-----|
| LLM 分析 | 硬编码 `analyze-with-llm` | 可配置 Skill 系统 |
| 分析类型 | 单一（报告解读） | 多 Skill（宏观、regime、轮动、风险、广度） |
| 模型支持 | OpenAI-compatible | OpenAI + DeepSeek + 可扩展 |
| 输出格式 | Markdown-only | JSON core + Markdown view |
| 上下文 | Raw DashboardSnapshot | 语义压缩的 ResearchContext |
| Agent | 无 | Agent Profile（分析配置） |
| 触发方式 | 手动 CLI | Skill Router（自动匹配） |
| 扩展性 | 修改代码 | 添加 Skill 文件 |

---

## 九、长期演进路线

```
V4: Research Skill System
  └─ Structured Output + Skill System + Provider Abstraction + Research Context

V5: Research Memory
  └─ Regime transition history + Analysis memory + Skill evolution tracking

V6: Multi-Agent
  └─ Macro Agent + China Agent + Risk Agent + Orchestrator

V7: Research OS
  └─ Marketplace + Third-party skills + Consensus Engine + MCP Platform
```

---

## 十、附录

### A. Skill 目录结构

```
crates/research-skills/skills/
├── macro-analysis/
│   ├── SKILL.md                          # Skill 定义
│   ├── schema.json                       # 输出 Schema
│   └── tests/
│       └── test_cases.yaml               # 测试用例
├── market-regime-reasoning/
│   ├── SKILL.md
│   ├── schema.json
│   └── tests/
│       └── test_cases.yaml
└── ...
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
| Skill 格式 | Markdown + YAML front matter | 可读性 + 结构化 |
| Trigger DSL | YAML（all/any/none/weight） | 简单、可扩展 |
| Reasoning DSL | YAML（inputs/checks/outputs/states） | 机器可读 |
| Schema 验证 | JSON Schema | 标准、通用 |
| LLM Provider | async-trait | Rust 标准模式 |
| 模板引擎 | Handlebars / Tera | 简单、成熟 |
| 配置加载 | config crate | Rust 生态标准 |
