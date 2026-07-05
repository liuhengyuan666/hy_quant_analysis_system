# RESEARCH-SKILLS KNOWLEDGE BASE

## OVERVIEW
Research Layer — 冻结量化引擎之上的只读叙事层。V4.5 架构：没有 Skill Framework，没有 Agent Registry，没有 Router。只有 5 个 Prompt 常量 + 一个 `build_prompt` 函数。

## STRUCTURE (V4.5)
```text
crates/research-skills/src/
├── lib.rs               # 模块声明 + re-exports (action, provider, openai_provider, inference)
├── action.rs            # 5 个 const PROMPT + build_prompt() + build_snapshot_context()
├── provider.rs          # LlmProvider trait + LlmCallConfig
├── openai_provider.rs   # OpenAI-compatible API implementation
└── inference.rs         # 推理配置 + 调用编排

# 以下文件为 V4 遗留死代码，待清理：
├── skill.rs             # [DEPRECATED] V4 SkillDefinition
├── agent_profile.rs     # [DELETED] V4 AgentProfile
├── trigger.rs           # [DELETED] V4 trigger conditions
├── reasoning.rs         # [DELETED] V4 ReasoningGraph
├── registry.rs          # [DELETED] V4 skill registry
├── router.rs            # [DELETED] V4 skill router
├── executor.rs          # [DELETED] V4 executor
├── schema.rs            # [DELETED] V4 output schema
├── token_budget.rs      # [DEPRECATED] V4 token counting
├── deterministic.rs     # [DEPRECATED] V4 deterministic mode
├── analysis.rs          # [DEPRECATED] V4 ResearchAnalysis (ConfidenceScore, probability, conviction)
└── renderer.rs          # [DEPRECATED] V4 render_analysis_markdown
└── regime_state_machine.rs  # [DEPRECATED] V4 regime transition tracking
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| 修改/新增研究动作 | `action.rs` | 编辑 5 个 const PROMPT 或新增 prompt；需要 ADR-074 修正案 |
| 修改 LLM 调用 | `openai_provider.rs` | async-openai crate |
| 修改 provider 抽象 | `provider.rs` | LlmProvider trait |
| 修改推理配置 | `inference.rs` | InferenceConfig |
| 修改传给 LLM 的 context | `action.rs::build_snapshot_context()` | 控制哪些数据喂给 LLM |

## 5 个研究动作 (Research Actions)
| Action | 中文名 | 角色 | 任务 |
|--------|--------|------|------|
| `market_story` | 市场叙事 | 资深研究员 | 今天市场发生了什么 |
| `explain_decision` | 解释决策 | 系统分析师 | 为什么系统做出这样的决策 |
| `preclose_review` | 收盘前复核 | 执行分析师 | 为什么 Execution 给出这样的建议 |
| `risk_view` | 风险视角 | 风控总监 | 我最担心什么 |
| `devils_advocate` | 唱反调 | 质疑者 | 系统可能错在哪里 |

## CONVENTIONS
- **只读叙事层**：解释、质疑、提供上下文、讲述历史。禁止创建信号、评分、排序、覆盖决策。
- **输出 Markdown 纯文本**：禁止 JSON schema、confidence、score、ranking、probability。
- **不传评分数据给 LLM**：`build_snapshot_context` 只给 symbol + label + rank order，不给 score/RS/CAGR/Sharpe/MaxDD。
- 没有 enum。没有 registry。没有 router。只有 `build_prompt(action, snapshot)` 函数。
- 系统 prompt 按 action 分化（研究员 / 风控总监 / 质疑者），但没有 "agent 选择" UI。
- 90 天内禁止新增 Pattern、禁止前端面板、禁止定时调度、禁止 Backtest v2、禁止参数优化、禁止性能声称。

## ANTI-PATTERNS
- Do **not** add new research actions without ADR-074 修正案。
- Do **not** feed signal scores, RS scores, or backtest metrics to LLM prompts。
- Do **not** add structured output schema, confidence metrics, or probability estimates。
- Do **not** reintroduce agent profiles, skill registry, or reasoning graphs。
- Do **not** let LLM output be treated as a decision signal。
- Do **not** add persistence logic here; this crate is pure analysis。

## NOTES
- 旧 V4 Skill 文本保留在 `skills/` 目录（仅作为文本参考，不作为代码框架调用）。
- 死代码文件（skill.rs, renderer.rs, analysis.rs, deterministic.rs, token_budget.rs, regime_state_machine.rs）已标记 `#[allow(dead_code)]` 或不再被 lib.rs 使用，待后续清理。
- `async-openai` is the LLM client library.
- Depends on `core-domain` for `DashboardSnapshot` and `LlmConfig`.
- V6 canonical `ResearchContext` lives in `crates/research-context`; LLM-specific context building lives in `crates/llm-context`.
