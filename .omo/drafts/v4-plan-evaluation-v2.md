# V4 规划评估报告（第二轮更新 - 结合 Anthropic 参考）

## 一、Anthropic financial-services 架构分析

### 核心结构

```
plugins/
  vertical-plugins/<vertical>/skills/     # Skill 源码（领域知识库）
  vertical-plugins/<vertical>/commands/   # 显式命令（/comps, /dcf）
  agent-plugins/<slug>/                   # Agent 插件（自包含，引用 skills）
    skills/                               # 从 vertical 同步的 skill 副本
managed-agent-cookbooks/<slug>/           # 部署配置（agent.yaml + subagents）
scripts/
  sync-agent-skills.py                    # Skill 同步工具
  orchestrate.py                          # 跨 Agent handoff 参考实现
```

### 关键设计思想

1. **Skill = Markdown + YAML front matter** — 不是代码，是"认知资产"
2. **Agent = Self-contained plugin** — bundles 所需 skills，可独立部署
3. **Vertical/Agent 分离** — Skill 在 vertical 中定义，Agent 引用并同步
4. **Everything is file-based** — 无构建步骤，纯 Markdown/YAML/JSON
5. **Agent = Workflow + Selected Skills** — 不是 giant prompt

### Anthropic 与我们系统的差异

| 维度 | Anthropic | 我们的系统 |
|------|-----------|----------|
| **目标用户** | 企业/机构（投行、PE、财富管理） | 个人量化研究者 |
| **运行环境** | Claude Cowork 平台 / Managed Agents API | 本地桌面 + CLI |
| **数据连接** | MCP servers（Daloopa, FactSet, S&P等） | 本地 ClickHouse + SQLite |
| **输出格式** | Excel, PPT, DOCX（机构工作产物） | Markdown 报告 + Dashboard |
| **编排复杂度** | Multi-agent + handoff + subagents | 单管道（ingest → ... → report）|
| **权限系统** | 企业级 ACL, audit, approval | 无（本地单用户）|

**结论**：架构思想高度同构，但实现轻量化程度应不同。

---

## 二、对 GPT 建议的修正评估

### GPT 建议 vs Anthropic 实践 vs 我们的现实

| GPT 建议 | Anthropic 实践 | 我们的现实 | 评估 |
|----------|---------------|-----------|------|
| 立即分仓 | 单仓（anthropics/financial-services） | 当前单仓 | ✅ GPT 过于激进，Anthropic 也是单仓 |
| Skill = YAML | Skill = Markdown + YAML front matter | 当前硬编码 Rust | ✅ 应采用 Markdown + YAML front matter |
| Multi-Agent P1 | Multi-Agent via Managed Agents API | 当前单管道 | ⚠️ Anthropic 是平台级能力，我们先预留接口 |
| 完全放弃 Markdown | Markdown 是核心（SKILL.md） | 当前 Markdown 报告 | ❌ GPT 错误，Anthropic 重度使用 Markdown |
| JSON-first 输出 | 输出是 Excel/PPT/DOCX | 当前 Markdown | ⚠️ 应 JSON 核心 + Markdown 渲染，非完全放弃 |
| Research OS | Cowork 插件 + Managed Agents | 本地 CLI + Desktop | ⚠️ 概念正确，但实现应轻量化 |

### 关键认知更新

**1. Anthropic 也是单仓**

GPT 建议"立即分仓"，但 Anthropic financial-services 本身就是**单仓**（`anthropics/financial-services`）。

他们通过目录结构实现逻辑分离：
- `plugins/vertical-plugins/` — 领域技能（类似我们的 "Quant Engine"）
- `plugins/agent-plugins/` — Agent 定义（类似我们的 "Research Brain"）

**启示**：V4 不需要分仓，应在当前仓库内建立清晰的 `research/` 目录。

**2. Skill 是 Markdown，不是纯 YAML**

Anthropic 的 Skill 格式：
```markdown
---
name: initiating-coverage
description: Create institutional-quality equity research initiation reports...
---

# Initiating Coverage

Create institutional-quality equity research initiation reports...
```

这是 **Markdown + YAML front matter**，不是纯 YAML。

**启示**：GPT 建议的纯 YAML skill 格式过于简化，应采用 Markdown + front matter，既保持可读性又便于结构化。

**3. Agent = Workflow + Skills（正确）**

Anthropic 的 Agent 插件结构：
```
agent-plugins/pitch-agent/
  ├── agent.md              # Agent 定义（workflow + system prompt）
  ├── skills/               # 引用的 skills（从 vertical 同步）
  │   ├── comps-analysis/
  │   ├── dcf-model/
  │   └── lbo-model/
  └── ...
```

**启示**：我们的 "Agent" 应该是 `analysis-profile.yaml` + 引用的 skills，不是 giant prompt。

---

## 三、修正后的 V4 架构建议

### 核心原则

1. **单仓优先**（跟随 Anthropic 实践）
2. **Skill = Markdown + YAML front matter**（跟随 Anthropic 实践）
3. **JSON 核心 + Markdown 渲染**（不是 JSON-only）
4. **预留 Multi-Agent 接口，但不实现**（当前单 Agent 足够）
5. **轻量化**（不引入 MCP、Cowork 等重量级基础设施）

### 建议的仓库结构

```
hy-quant-analysis-system/                    # 单仓（不变）
├── apps/
│   ├── cli/                                 # CLI 入口
│   └── desktop/                             # Tauri 桌面端
├── crates/
│   ├── app-service/                         # 编排层（剥离 LLM 分析逻辑）
│   ├── core-domain/                         # 类型定义
│   ├── market-store/                        # 数据存储
│   ├── data-ingestion/                      # 数据抓取
│   ├── *-engine/                            # 计算引擎
│   └── research-skills/                     # 新增：Skill 基础设施
│       ├── src/
│       │   ├── lib.rs
│       │   ├── registry.rs                  # Skill Registry（加载、查询）
│       │   ├── router.rs                    # Skill Router（基于市场状态匹配）
│       │   ├── executor.rs                  # Skill Executor（调用 LLM）
│       │   ├── provider.rs                  # LLM Provider trait（模型无关化）
│       │   └── render.rs                    # Markdown/JSON 渲染
│       └── skills/                          # 内置 Skill 定义
│           ├── macro-analysis/
│           │   └── SKILL.md
│           ├── market-regime-reasoning/
│           │   └── SKILL.md
│           ├── rotation-analysis/
│           │   └── SKILL.md
│           └── risk-assessment/
│               └── SKILL.md
├── research/                                # 新增：研究配置（file-based）
│   ├── agents/                              # Agent / Analysis Profile 定义
│   │   └── macro-strategist.yaml
│   ├── schemas/                             # 输出 Schema（JSON Schema）
│   │   └── market-analysis.schema.json
│   └── policies/                            # 推理策略
│       └── trend-reasoning.yaml
├── config/
│   ├── universe.json
│   └── calendars/
└── ...
```

### Skill 格式（采用 Anthropic 风格）

```markdown
---
name: macro-analysis
description: Analyze macro environment and provide regime reasoning
trigger_conditions:
  - regime_changed == true
  - macro_stale_days > 2
skills:
  - liquidity-analysis
  - risk-off-detection
output_schema: market-analysis.schema.json
output_bias: neutral
priority: high
---

# Macro Analysis

## Overview

Analyze the current macro environment based on:
- Treasury yields (DGS10)
- Dollar index (DTWEXBGS)
- Risk appetite (VIXCLS)
- Fed policy (DFF)

## Reasoning Steps

1. **Liquidity Assessment**
   - Analyze yield curve shape
   - Identify duration compression risk
   
2. **Risk Regime Classification**
   - risk_on: VIX < 20, yields stable
   - risk_off: VIX > 25, yields falling
   - neutral: mixed signals

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

### Agent / Analysis Profile 格式

```yaml
# research/agents/macro-strategist.yaml
name: macro-strategist
description: Macro strategist analysis profile
skills:
  - macro-analysis
  - liquidity-analysis
  - risk-off-detection
model: gpt-4o
system_prompt: |
  You are a senior macro strategist at a quant hedge fund...
output_format: json
```

---

## 四、V4 实施路线图（最终版）

### Wave 1: 结构化输出（2-3 周）

- [ ] 定义 `ResearchAnalysis` JSON schema（市场状态、信号、风险、建议）
- [ ] 修改 `report-engine` 输出 JSON + Markdown（双格式）
- [ ] 修改 `apps/cli` 支持 `--format json|markdown`
- [ ] 修改桌面端支持 JSON 数据驱动渲染

### Wave 2: Skill 基础设施（3-4 周）

- [ ] 创建 `crates/research-skills` crate
- [ ] 定义 Skill Markdown + YAML front matter 格式
- [ ] 实现 Skill Registry（从 `crates/research-skills/skills/` 和 `research/skills/` 加载）
- [ ] 实现 Skill Router（基于 `DashboardSnapshot` 状态匹配 Skill）
- [ ] 实现 Skill Executor（调用 LLM，支持变量替换）
- [ ] 实现 LLM Provider trait（OpenAI, DeepSeek, Claude）

### Wave 3: 内置 Skills（2-3 周）

- [ ] `macro-analysis` skill（宏观分析）
- [ ] `market-regime-reasoning` skill（regime 推理）
- [ ] `rotation-analysis` skill（轮动分析）
- [ ] `risk-assessment` skill（风险评估）
- [ ] `breadth-analysis` skill（广度分析）

### Wave 4: Agent Profiles（1-2 周）

- [ ] 创建 `research/agents/` 目录
- [ ] 定义 Agent YAML 格式
- [ ] 创建 `macro-strategist` agent profile
- [ ] 创建 `risk-manager` agent profile
- [ ] CLI `analyze` 命令支持 `--agent <profile>`

### Wave 5: 集成与验证（1-2 周）

- [ ] 将 V3 `analyze-with-llm` 迁移到 Skill 系统
- [ ] 端到端测试
- [ ] 文档更新

**总计**: 9-14 周（2-3.5 个月）

---

## 五、与 Anthropic 的关键差异（有意为之）

| 方面 | Anthropic | 我们的 V4 | 理由 |
|------|-----------|----------|------|
| **部署方式** | Cowork 插件 / Managed Agents API | 本地 CLI + 桌面端 | 目标用户不同 |
| **数据连接** | MCP servers | 本地 ClickHouse/SQLite | 不需要外部数据平台 |
| **输出格式** | Excel/PPT/DOCX | JSON + Markdown | 研究场景 vs 生产场景 |
| **多 Agent** | 完整 handoff + subagents | 预留接口，单 Agent | 当前需求不足 |
| **权限系统** | 企业级 ACL | 无 | 本地单用户 |
| **Skill 同步** | `sync-agent-skills.py` | Cargo build 时嵌入 | 简化部署 |

---

## 六、结论

### GPT 的建议方向正确，但需要修正

**GPT 正确的地方**:
- ✅ Skill + Agent + Orchestrator 方向
- ✅ 模型无关化
- ✅ 不要将 Skill 当 Prompt
- ✅ 研究能力操作系统化

**GPT 过于激进的地方**:
- ❌ 立即分仓（Anthropic 也是单仓）
- ❌ 完全放弃 Markdown（Anthropic 重度使用 Markdown）
- ❌ V4 就实现 Multi-Agent（当前需求不足）

**Anthropic 给我们的最大启示**:
- Skill 是 Markdown + YAML front matter，不是代码或纯 YAML
- 单仓可以通过目录结构实现逻辑分离
- Agent = Workflow + Selected Skills（不是 giant prompt）
- Everything is file-based（简化部署和迭代）

### V4 核心目标

将 V3 的**硬编码 LLM 分析**重构为**可配置、可扩展的 Skill 系统**：

1. **Skill Registry** — 加载和管理 Markdown/YAML Skill 定义
2. **Skill Router** — 基于市场状态自动匹配和触发 Skill
3. **LLM Provider** — 模型无关的抽象层
4. **Structured Output** — JSON 核心 + Markdown 渲染
5. **Agent Profile** — 分析配置（Skill 组合 + 模型选择）

**不是 Research OS，而是 Research Skill System。**
