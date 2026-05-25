# V4 规划评估报告

## 一、建议核心观点评估

### ✅ 高度认同（立即实施）

| 建议 | 评估 | 理由 |
|------|------|------|
| **Structured Output (JSON-first)** | ⭐⭐⭐⭐⭐ | 当前系统输出以 Markdown 为主，确实限制了程序化消费。改为 JSON 核心 + Markdown View Layer 是正确方向 |
| **Skill Registry 概念** | ⭐⭐⭐⭐⭐ | V3 的 LLM 分析是硬编码的，抽象为可配置 Skill 是必要的 |
| **模型无关化** | ⭐⭐⭐⭐⭐ | V3 已使用 OpenAI-compatible API，但配置和调用仍耦合在 app-service 中 |
| **不要 Prompt 巨石化** | ⭐⭐⭐⭐⭐ | 当前 V3 的 prompt 是硬编码字符串，需要结构化拆分 |

### ⚠️ 部分认同（需要调整）

| 建议 | 评估 | 调整建议 |
|------|------|---------|
| **立即分仓** | ⭐⭐⭐ | 当前系统规模（~800行 app-service）**过早分仓**会增加维护成本。建议先在单仓内建立清晰模块边界，技能数量 >20 时再分仓 |
| **Multi-Agent 架构** | ⭐⭐⭐ | 概念正确，但当前用户是个人研究者，单 Agent 已满足 80% 需求。建议 V4 先预留接口，V5 再实现 |
| **Consensus Engine** | ⭐⭐ | 需要多 Agent 才有意义，V4 阶段过早 |
| **Marketplace 开放** | ⭐⭐ | 远期愿景，当前无实际需求 |

### ❌ 不认同（当前阶段）

| 建议 | 评估 | 理由 |
|------|------|------|
| **完全放弃 Markdown** | ⭐⭐ | Markdown 对人工阅读仍不可替代。正确做法是 **JSON 核心 + Markdown 渲染**，而非完全放弃 |
| **立即构建完整 Research OS** | ⭐⭐ | 当前系统还在 V1→V3 的演进期，直接跳到 "Research OS" 会导致过度工程化 |

---

## 二、结合当前代码结构的可操作性分析

### 当前架构现状

```
hy-quant-analysis-system/
├── apps/cli              # CLI 入口（已包含 V3 LLM 命令）
├── apps/desktop          # Tauri 桌面端
├── crates/
│   ├── app-service       # 编排层（~796行 monolith）⚠️
│   ├── core-domain       # 类型定义
│   ├── market-store      # 数据存储
│   ├── data-ingestion    # 数据抓取
│   ├── *-engine          # 计算引擎（indicator/macro/rotation/signal/strategy/backtest/report）
```

### V3 现状问题（与建议的关联）

1. **app-service 已经是 monolith** - V3 的 LLM 逻辑直接写在里面，确实需要解耦
2. **输出格式单一** - 只有 Markdown，没有结构化 JSON
3. **LLM 调用硬编码** - `analyze_report_with_llm` 是固定逻辑，无法配置不同分析风格
4. **无 Skill 概念** - 所有研究逻辑都是代码级硬编码

---

## 三、V4 分阶段实施建议

### Phase 1: 基础设施（当前仓库内完成）

**目标**: 在当前仓库内建立 Skill 基础设施，不拆仓

```
crates/
├── app-service              # 保持现状，但剥离 LLM 分析逻辑
├── research-skills          # 新增：Skill Registry + Router（不是独立仓库，是 workspace crate）
│   ├── src/
│   │   ├── registry.rs      # Skill 注册表（YAML/JSON 配置加载）
│   │   ├── router.rs        # Skill Router（根据市场状态触发技能）
│   │   ├── executor.rs      # Skill 执行器（调用 LLM）
│   │   ├── schemas/         # 输出 Schema 定义
│   │   └── skills/          # 内置 Skill 定义（YAML）
│   │       ├── macro_analysis.yaml
│   │       ├── market_regime_reasoning.yaml
│   │       └── risk_assessment.yaml
│   └── skills/              # 用户可扩展的 Skill 目录
│       └── custom/
```

**具体任务**:

1. **Structured Output** (P0)
   - 在 `core-domain` 中定义 `ResearchAnalysis` JSON schema
   - 修改 `report-engine` 支持 JSON 核心输出
   - Markdown 作为 `View Layer` 保留（从 JSON 渲染）

2. **Skill Registry** (P0)
   - 创建 `crates/research-skills` crate
   - 定义 Skill YAML 格式（名称、触发条件、prompt 模板、输出 schema）
   - 从目录加载所有 Skill 配置

3. **Skill Router** (P0)
   - 根据 `DashboardSnapshot` 状态决定触发哪些 Skill
   - 例如：`hk_breadth == 0` → 触发 `hk_liquidity_risk` skill

4. **模型无关化** (P0)
   - 抽象 LLM Provider 接口（OpenAI / Claude / DeepSeek 统一接口）
   - 配置驱动模型选择

### Phase 2: 多 Agent 预留（V4.5）

**目标**: 在单 Agent 架构下预留多 Agent 扩展点

- 不实现真正的 Multi-Agent，但 Skill 设计支持 "Agent-like" 的独立分析
- 每个 Skill 可以看作一个 "轻量 Agent"
- Consensus Engine 预留接口（简单的规则引擎）

### Phase 3: 分仓（V5，技能数量 >20 时）

**目标**: 当内置 + 自定义 Skill 超过 20 个时，拆分为独立仓库

```
hy-quant-analysis-system/      # 市场计算层（保持不变）
hy-research-skills/            # 研究认知层（新仓库）
```

---

## 四、与 GPT 建议的关键分歧

### 分歧 1: 分仓时机

**GPT 建议**: 立即分仓
**我的建议**: V4 先在单仓内建立 `crates/research-skills`，等技能生态成熟（>20 skills）再分仓

**理由**:
- 当前 Skill 数量 = 0（V3 是硬编码）
- 单仓开发效率更高（cargo workspace 共享依赖、统一编译）
- 过早分仓会导致：接口不稳定、版本同步痛苦、调试困难

### 分歧 2: Multi-Agent 优先级

**GPT 建议**: P1（强烈建议 V4 实现）
**我的建议**: P2（V5 再考虑）

**理由**:
- 当前用户是个人研究者，单 Agent 已满足需求
- Multi-Agent 需要：编排器、消息总线、状态同步、共识机制 —— 复杂度极高
- 应该先验证 "Skill Registry + 单 Agent 顺序执行" 的价值，再升级多 Agent

### 分歧 3: Markdown 完全废弃

**GPT 建议**: "不要 Markdown-first"
**我的建议**: "JSON 核心 + Markdown View Layer"

**理由**:
- 当前用户主要通过桌面端和报告阅读结果，Markdown 对人工阅读最友好
- JSON 适合程序化消费，但不应该完全替代 Markdown
- 正确架构：JSON 是核心数据，Markdown 是渲染视图

---

## 五、V4 实施路线图（修正版）

### Wave 1: 结构化输出（2-3 周）

- [ ] 定义 `ResearchAnalysis` JSON schema（市场状态、信号、风险、建议）
- [ ] 修改 `report-engine` 输出 JSON + Markdown（双格式）
- [ ] 修改 `apps/cli` 支持 `--format json|markdown`
- [ ] 修改桌面端支持 JSON 数据驱动渲染

### Wave 2: Skill 基础设施（2-3 周）

- [ ] 创建 `crates/research-skills` crate
- [ ] 定义 Skill YAML 格式
- [ ] 实现 Skill Registry（加载、验证、查询）
- [ ] 实现 Skill Router（基于市场状态匹配 Skill）
- [ ] 将 V3 的 `analyze_report_with_llm` 重构为第一个 Skill

### Wave 3: 模型无关化（1-2 周）

- [ ] 抽象 `LlmProvider` trait
- [ ] 实现 OpenAI provider
- [ ] 实现 DeepSeek provider（可选）
- [ ] 配置驱动模型选择

### Wave 4: 内置 Skills（2-3 周）

- [ ] `macro_analysis` skill（宏观分析）
- [ ] `market_regime_reasoning` skill（regime 推理）
- [ ] `rotation_analysis` skill（轮动分析）
- [ ] `risk_assessment` skill（风险评估）

### Wave 5: 集成与验证（1-2 周）

- [ ] CLI `analyze-with-skills` 命令（替代 V3 的 `analyze-with-llm`）
- [ ] 端到端测试
- [ ] 文档更新

**总计**: 8-13 周（2-3 个月）

---

## 六、风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 过度工程化 | 高 | 高 | 坚持 "单仓优先"，先验证 Skill 概念再扩展 |
| JSON 输出破坏现有工作流 | 中 | 中 | 保留 Markdown 输出作为默认，JSON 作为可选 |
| Skill YAML 格式频繁变更 | 中 | 中 | 先内部使用，稳定后再开放自定义 Skill |
| 多模型支持增加复杂度 | 中 | 低 | 先从 OpenAI + DeepSeek 两个 provider 开始 |

---

## 七、结论

**GPT 的建议方向正确，但实施节奏过于激进。**

V4 应该聚焦：
1. ✅ Structured Output（JSON + Markdown 双格式）
2. ✅ Skill Registry（单仓内建立，不是独立仓库）
3. ✅ 模型无关化（抽象 LLM Provider）
4. ⚠️ Multi-Agent（预留接口，不实现）
5. ❌ 分仓（V5 再考虑）

**最大价值**: 将 V3 的硬编码 LLM 分析重构为可配置、可扩展的 Skill 系统，同时保持系统简洁和可维护性。
