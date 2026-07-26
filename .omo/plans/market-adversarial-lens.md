# Market Adversarial Lens — 市场博弈视角 Persona 接入

## TL;DR

> **Quick Summary**: 新增一个文件 persona `market_adversarial_lens`（市场博弈视角），增强 LLM 上下文注入 6 个已计算但未喂给 LLM 的字段，在 prompt 中加入 web search 引导词弥补缺失的结构化数据。零引擎变更，纯 prompt + 上下文增强。
>
> **Deliverables**:
> - `config/prompts.toml` 新增 `[prompts.market_adversarial_lens]` persona
> - `crates/research-skills/src/action.rs` 的 `build_snapshot_context()` 增强（6 字段注入）
> - `apps/desktop/frontend/src/components/LlmAnalysisPanel.vue` 新增按钮
> - `apps/desktop/frontend/src/locales/zh.json` + `en.json` 新增 i18n 键
> - `crates/research-skills/AGENTS.md` 文档更新
>
> **Estimated Effort**: Quick
> **Parallel Execution**: YES — 2 waves
> **Critical Path**: Task 1 → Task 4

---

## Context

### Original Request
用户要求将徐翔"技术翔"时期的二级市场博弈分析框架提炼为量化系统 LLM 层的一个 skill，融入现有 persona 体系。经过四轮讨论后确定为：以文件 persona 形式加入 `config/prompts.toml`，作为独立的"市场博弈视角"；同时增强 LLM 上下文注入 6 个系统已计算但未喂给 LLM 的字段；对缺失的结构化数据（融资余额、ETF 申赎、持仓成本分布等）通过 prompt 中的 web search 引导词尝试补偿。不涉及引擎层变更。

### Interview Summary
**Key Discussions**:
- 徐翔框架的价值不在具体技巧而在博弈结构认知——"看懂市场里谁被套住了比看懂市场要往哪走更有预测力"
- persona 定位为前置共享分析层 vs 独立 persona —— 先作为独立 persona 落地验证
- 命名从 `xuxiang_lens` 改为 `market_adversarial_lens`，中文标签"市场博弈视角"
- 5 维分析框架：资金角色冲突、强制卖盘与流动性、被套资金与筹码、预期差（核心 web search 场景）、信号生命周期
- 三个 prompt 规则：减少"阶段"语言、"证据优先"约束、"反事实检查"
- 排除引擎层变更，缺失的结构化数据靠 web search prompt 引导词补偿

**Research Findings**:
- `volume_expansion_pct`、`turnover_coverage_pct`、`breadth_5d_delta` 已在 `EnvironmentSnapshot` 中计算好，但未在 `build_snapshot_context()` 中注入
- `bottom_rotation`、`regime_stale_days`、`liquidity_score` 已在 `DashboardSnapshot` 顶层，也未注入
- `liquidity_score` 使用 `DashboardSnapshot.liquidity_score`（regime 级系统性流动性），非 `EnvironmentSnapshot.liquidity_proxy_score`
- Environment 衍生字段需通过 `snapshot.environment`（`Option<EnvironmentSnapshot>`）访问，必须 guard
- 现有两个文件 persona（`short_term_trader`、`long_term_allocator`）在 `LlmAnalysisPanel.vue` 中无按钮——前端按钮是新增模式

### Metis Review
**Identified Gaps** (addressed):
- Q1（哪个 liquidity_score？）→ 用 `DashboardSnapshot.liquidity_score`（always present, regime-level）
- Q2（None 字段格式化？）→ 遵循现有 `"N/A"` 惯例
- Q3（web search 机制？）→ 纯 prompt 引导词，不建立后端搜索基础设施
- A1（嵌套字段路径？）→ 放 `if let Some(ref env)` guard 内
- Edge #1（environment 为 None？）→ guard 内处理，None 时整段省略
- G3（字段重复？）→ 已验证 6 字段均为 LLM 上下文中的新增信息
- G4（上下文预算？）→ 6 字段约 15-20 行，控制在内

---

## Work Objectives

### Core Objective
为量化系统 LLM 分析层新增一个"市场博弈视角"分析 persona，使其能够从市场参与者行为、资金角色冲突、流动性压力等维度解读已有系统数据，填补现有 8 个 persona 在"谁在交易、为什么价格被推动"这一维度上的空白。

### Concrete Deliverables
- `config/prompts.toml` — 新增 `[prompts.market_adversarial_lens]` section（system + template）
- `crates/research-skills/src/action.rs` — `build_snapshot_context()` 函数增强
- `apps/desktop/frontend/src/components/LlmAnalysisPanel.vue` — actions 数组新增条目
- `apps/desktop/frontend/src/locales/zh.json` — `research.marketAdversarialLens` 键
- `apps/desktop/frontend/src/locales/en.json` — `research.marketAdversarialLens` 键
- `crates/research-skills/AGENTS.md` — action 表格更新

### Definition of Done
- [ ] `cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global` 返回有效 JSON（含 `persona_label`、`markdown` 字段，无 "unknown action" 错误）
- [ ] `cargo check` 零新增 warning
- [ ] `cargo test -p report-engine -p research-skills -p app-service` 全绿
- [ ] 新建 persona 的 `system` 和 `template` 无阈值、无 if/then、无 BUY/SELL/HOLD、无仓位百分比
- [ ] 桌面端 LlmAnalysisPanel 显示"市场博弈视角"按钮，点击后正常发起请求并渲染 markdown

### Must Have
- persona `system` 和 `template` 不含任何阈值规则、if/then 决策逻辑、BUY/SELL/HOLD 语言、仓位百分比
- web search 引导词为纯 prompt 文本，不建立后端搜索基础设施
- 零引擎 crate 变更（`macro-engine` / `indicator-engine` / `signal-engine` / `strategy-engine` / `rotation-engine` / `execution-engine` / `backtest-engine` / `data-ingestion` / `market-store` / `core-domain` / `llm-context`）
- `builtin_persona()` 不修改

### Must NOT Have (Guardrails)
- 不创建结构化输出（JSON schema、枚举标签、confidence 字段）
- 不输出 `market_phase` / `Leverage State` 等分类标签（阶段分类由引擎负责）
- 不引入新的 Rust 依赖或 API 集成
- 不引入下拉菜单/分组/UI 重构
- 不修改 `crates/llm-context/`
- 不将 web search 引导词实现为后端功能

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES（sparse test coverage in report-engine, research-skills, app-service）
- **Automated tests**: None（no new automated tests — follow existing project convention）
- **Framework**: N/A

### QA Policy
所有任务使用 Agent-Executed QA：CLI 验证用 PowerShell，前端验证用 Playwright。

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — all independent):
├── Task 1: build_snapshot_context() 增强 [quick]
├── Task 2: market_adversarial_lens persona prompt [writing]
├── Task 3: 前端按钮 + i18n [visual-engineering]
└── Task 4: 文档更新 [quick]

Wave 2 (After Wave 1 — integration verification):
├── Task 5: CLI QA — persona 解析与 cargo 检查 [quick]
└── Task 6: 前端 QA — 按钮渲染与交互 [visual-engineering]

Wave FINAL (After ALL tasks):
├── Task F1: Plan compliance audit [oracle]
├── Task F2: Code quality review [unspecified-high]
├── Task F3: Real manual QA [unspecified-high]
└── Task F4: Scope fidelity check [deep]
```

Critical Path: Task 1 → Task 5 — none（Task 1 和 Task 2/3/4 完全独立）
Parallel Speedup: ~100% faster than sequential (Wave 1 fully parallel)

---

## TODOs

- [ ] 1. `build_snapshot_context()` 增强 —— 注入 6 个已计算字段

  **What to do**:
  - 在 `crates/research-skills/src/action.rs` 的 `build_snapshot_context()` 函数中（约 line 214-254），新增以下格式化输出：
    1. `snapshot.liquidity_score` → `**流动性评分**: {value}`（regime 级系统性流动性）
    2. `snapshot.regime_stale_days` → `**Regime 数据新鲜度**: {value} 天`（使用 `snapshot.regime_stale_days.max(0)` 防负值）
    3. **在现有 `if let Some(ref env) = snapshot.environment` guard 块内添加**：
       - `env.breadth_5d_delta` → `**广度 5 日变化**: {value}`（None 时显示 `N/A`）
       - `env.volume_expansion_pct` → `**成交量扩张**: {value}`（None 时显示 `N/A`）
       - `env.turnover_coverage_pct` → `**换手率覆盖**: {value}`（None 时显示 `N/A`）
    4. **在 guard 块外**，`bottom_rotation` 单独处理 —— 新增一段：
       - 如果 `!snapshot.bottom_rotation.is_empty()`，添加 `**轮动末位（退潮方向）**:` 后跟 top-5 symbol 名（仅名称，不加 RS 分数，镜像 top_rotation 格式）
       - 如果为空，整段省略
  - 更新函数注释：`/// Build snapshot context for LLM prompts. Includes regime, environment, rotation, signals, and microstructure fields.`
  - 最终 `build_snapshot_context()` 长度控制在 ~60 行以内（当前约 40 行，新增约 15-20 行）

  **Must NOT do**:
  - 不添加 `liquidity_proxy_score`（那是 breadth-derived proxy，易混淆）
  - 不添加 `trend_score` / `risk_score`（已由 regime_label 覆盖，重复）
  - 不添加 `bullish_signals` / `defensive_signals`（已由 top_signals 覆盖）
  - 不在 guard 外直接访问 `snapshot.environment.unwrap()`（会 panic）
  - 不修改函数签名

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单文件单函数修改，约 15-20 行增量，纯文本格式化
  - **Skills**: None required

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 Tasks 2, 3, 4 并行）
  - **Blocks**: Task 5（CLI QA 依赖此变更）
  - **Blocked By**: None

  **References** (CRITICAL):
  - `crates/research-skills/src/action.rs:214-254` — 当前 `build_snapshot_context()` 实现，理解现有格式化风格（bold 标签、简单数值、无推导评注）
  - `crates/report-engine/src/lib.rs:25` — `DashboardSnapshot.liquidity_score` 字段定义（`f64`, always present）
  - `crates/report-engine/src/lib.rs:22` — `DashboardSnapshot.regime_stale_days` 字段定义（`i64`）
  - `crates/report-engine/src/lib.rs:28` — `DashboardSnapshot.bottom_rotation` 字段定义（`Vec<RotationRankSnapshot>`）
  - `crates/core-domain/src/lib.rs:203-206` — `EnvironmentSnapshot` 的 `breadth_5d_delta`、`volume_expansion_pct`、`turnover_coverage_pct` 定义（均为 `Option<f64>`）
  - `crates/core-domain/src/lib.rs:216-224` — `RotationRankSnapshot` 结构（含 `symbol`、`rs_20`、`rs_60`、`rs_120`、`rank`）
  - `crates/app-service/src/trust.rs:358` — `unwrap_or("N/A")` 惯例，作为 None 格式化参考

  **Acceptance Criteria**:
  - `cargo check -p research-skills` 通过，零新增 warning
  - `cargo test -p research-skills` 全绿（现有测试不应受格式化变更影响）
  - 新增的 `bottom_rotation` 段仅包含 symbol 名（如 `1. 000300`），不包含 RS 分数

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: 六字段注入后 build_snapshot_context() 产出完整上下文
    Tool: Bash (cargo run)
    Preconditions: Dashboard data available for global scope
    Steps:
      1. Run: cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global 2>&1
      2. Parse the JSON output, extract the "markdown" field
      3. Verify the LLM response references concepts matching the 6 new fields (e.g., mentions liquidity score, regime staleness, breadth 5-day delta, volume expansion, turnover coverage, rotation bottom names)
    Expected Result: LLM output shows awareness of at least 4 of the 6 newly injected data points
    Failure Indicators: LLM response shows no reference to any of the 6 new data dimensions
    Evidence: .omo/evidence/task-1-context-injection.md (captured CLI output)
  ```

  **Evidence to Capture**:
  - [ ] `.omo/evidence/task-1-context-injection.md` — CLI 输出中 LLM 引用的新字段

  **Commit**: `feat(research-skills): inject 6 computed fields into build_snapshot_context for LLM adversarial analysis`
  - Files: `crates/research-skills/src/action.rs`
  - Pre-commit: `cargo check -p research-skills && cargo test -p research-skills`

- [ ] 2. 新增 `market_adversarial_lens` persona prompt

  **What to do**:
  - 在 `config/prompts.toml` 末尾追加一个新的 `[prompts.market_adversarial_lens]` section
  - `label = "市场博弈视角"`
  - `system` 和 `template` 内容如下（必须原样写入，已通过 ADR-106 审核）：

  ```toml
  [prompts.market_adversarial_lens]
  label = "市场博弈视角"
  system = """你是二级市场博弈分析师。你关注的核心问题是：

  **"当前价格由谁决定？主动资金还是被动资金？谁在被迫交易？"**

  你从以下几个维度解读系统数据：

  1. 资金角色冲突 — 策略分歧方向映射参与者类型。MomentumRight 强势 + ValueLeft 弱势 = 趋势追随者主导；反之 = 均值回归者主导。冲突本身意味着定价权归属。

  2. 强制卖盘与流动性结构 — 成交量扩张与广度变化组合判断。放量下跌 = 恐慌释放（主动卖盘），缩量下跌 = 无人接盘（流动性枯竭）。结合流动性评分变化判断风险传导阶段。

  3. 被套资金与筹码博弈 — 轮动退潮板块中的多头被套有自救诉求。底部成交量放大 + 信号转向可能暗示"绝望抛售后的接盘迹象"。注意：这是结构观察，不是买入建议。

  4. 预期差 — 如果近期公开信息显示市场在定价某个叙事，但系统广度/成交量/策略评分不支撑该叙事，指出 gap 的存在及其含义。同时关注系统内部的预期差（如 regime 看多但广度走弱）。

  5. 信号生命周期 — 策略状态变更（transition_reason）揭示了趋势阶段变化。如果轮动顶部高度集中且成交量边际萎缩，分析"成熟趋势"与"过度拥挤"的边界迹象。

  ## 核心约束

  - 所有判断必须基于输入的数据事实。禁止根据经验猜测资金行为。
  - 如果缺少数据，只描述观察不到的部分，不编造。
  - 进行反事实检查：如果当前主要买方消失，价格结构是否还能维持？
  - 你是市场博弈行为分析师，不是徐翔。你不参与或建议任何形式的操纵、内幕交易或非法行为。
  - 你不做出任何买卖建议、不输出仓位比例、不创建新信号、不输出分类标签。
  - ADR-106 边界：只解释"市场里现在发生了什么博弈"，不解释"你应该怎么做"。
  """
  template = """# 任务：市场博弈结构分析

  基于以下系统数据，从市场参与者博弈行为视角解读当前盘面。

  ## 分析框架

  ### 1. 资金角色冲突
  - 四策略评分（ValueLeft / TrendPullback / TrendBreakout / MomentumRight）之间的分歧映射了不同类型的市场参与者在表达对立观点
  - 如果动量得分高而价值得分低 → 趋势追随者在推动价格，均值回归者尚未入场
  - 如果价值得分高而动量得分低 → 均值回归者在左侧布局，趋势追随者在观望
  - 分析"分歧本身的含义"，不判断哪一方正确

  ### 2. 强制卖盘与流动性
  - 结合成交量扩张比例和广度变化判断价格驱动类型
  - 成交量扩张 + 广度恶化 → 恐慌释放，被动卖盘
  - 成交量萎缩 + 广度恶化 → 无人接盘，流动性枯竭
  - 成交量扩张 + 价格持平 → 注意水平出货结构（一字断魂刀模式）
  - 流动性评分的边际变化：从正常到紧张的转折点在哪里

  ### 3. 被套资金与筹码博弈
  - 轮动末位榜单显示退潮方向 → 这些板块中的多头有自救诉求
  - 信号从 StrongBuy 转为 Watch/Reduce 的标的 → 前期多头被套
  - 换手率覆盖升高 + 退潮板块振幅扩大 → 多空分歧加剧，可能是"绝望中接盘"的信号
  - 再次强调：这是结构观察，不是买入建议

  ### 4. 预期差
  - 系统内部预期差：regime 判断 vs 广度现实的 gap、信号强度 vs 环境强度的 gap
  - 市场外部预期差：如果近期公开信息显示市场在交易某个叙事，系统数据是否支持？
  - 如果模型支持网络搜索，请检索以下公开信息：
    1. 当前 scope 对应市场的近期政策动态（央行/证监会/发改委近一周公告）
    2. 轮动 Top-3 板块的最新行业新闻与共识方向
    3. 信号最强标的的近期重大公告
  - 如果无法搜索，请明确声明"当前无法获取公开信息背景，仅基于系统数据进行内部预期差分析"，然后继续

  ### 5. 信号生命周期
  - strategy_state.transition_reason 揭示了引擎判断的趋势阶段变化
  - 轮动排名持续高位但成交量边际萎缩 → "成熟趋势"的特征
  - 轮动集中度 + 策略分歧度 + 成交量趋势组合判断趋势的边际强化或弱化方向
  - 描述趋势是否出现边际强化或边际弱化迹象，而不是给趋势贴分类标签

  ## 输出要求
  - Markdown 纯文本。不要 JSON。不要结构化字段。
  - 绝不要输出你的评分、排名、买卖建议、仓位百分比
  - 不要创建分类标签（如"当前处于 XX 阶段"）
  - 反事实检查至少出现一次：思考"如果当前主要买方消失，结构是否改变"
  - 用自然语言描述博弈结构——"谁在买、谁在卖、谁被套、谁的交易在主导价格"
  """
  ```
  - 确认 TOML 语法正确——`system` 和 `template` 的多行字符串使用 `"""` 包裹且在 config 中的缩进正确
  - 保留文件顶部的 ADR-106 边界注释：`# ADR-106 boundary: personas carry PERSPECTIVE MANDATES ONLY.`

  **Must NOT do**:
  - 不在 system/template 中包含任何阈值（如 "> 50%"、"超过 X"）
  - 不包含 if/then 决策逻辑
  - 不包含 BUY/SELL/HOLD 建议
  - 不包含仓位百分比
  - 不包含 JSON schema 或结构化输出格式要求
  - 不修改 `crates/research-skills/src/action.rs` 的 `builtin_persona()` 函数

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 纯文本内容创作（TOML 配置文件），需要严格遵循 ADR-106 合规约束和格式规范
  - **Skills**: None required

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 Tasks 1, 3, 4 并行）
  - **Blocks**: Task 5（CLI QA 依赖此变更）
  - **Blocked By**: None

  **References** (CRITICAL):
  - `config/prompts.toml:1-12` — ADR-106 边界注释格式，"custom personas MUST provide both system and template"
  - `config/prompts.toml:22-43` — `short_term_trader` 定义，作为 system/template 结构参考
  - `crates/app-service/src/prompts.rs:18-25` — `PersonaDefinition` 结构，确认 `system` 和 `template` 均为 `Option<String>`
  - `crates/app-service/src/prompts.rs:52-76` — `resolve_persona()` 逻辑，确认自定义 persona 必须同时提供 `system` 和 `template`

  **Acceptance Criteria**:
  - `cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global` 不报 "unknown action" 错误
  - 不报 TOML parse error
  - 不报 "custom persona must define both system and template" 错误
  - JSON 输出中的 `persona_label` 字段为 "市场博弈视角"

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Persona 解析成功并返回有效结果（需配置 API key）
    Tool: Bash (cargo run)
    Preconditions: LLM API key configured
    Steps:
      1. Run: cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global 2>&1
      2. Check exit code = 0
      3. Parse JSON output, verify: .action == "market_adversarial_lens", .persona == "市场博弈视角", .markdown is a non-empty string
    Expected Result: JSON contains valid markdown output, persona_label is "市场博弈视角"
    Failure Indicators: "unknown action", TOML parse error, missing persona_label
    Evidence: .omo/evidence/task-2-persona-resolution.json

  Scenario: 无 API key 时返回 placeholder（降级验证）
    Tool: Bash (cargo run)
    Preconditions: LLM API key NOT configured (or use a scope with no data)
    Steps:
      1. Run: cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global 2>&1
      2. Parse JSON output, verify: .placeholder == true, .markdown contains "占位符" or "未配置"
    Expected Result: Placeholder flag is true, no panic
    Failure Indicators: Panic, crash, non-JSON output
    Evidence: .omo/evidence/task-2-placeholder-fallback.json
  ```

  **Evidence to Capture**:
  - [ ] `.omo/evidence/task-2-persona-resolution.json` — API key 配置时 LLM 响应
  - [ ] `.omo/evidence/task-2-placeholder-fallback.json` — 无 API key 时占位符输出

  **Commit**: `feat(prompts): add market_adversarial_lens persona for market microstructure analysis`
  - Files: `config/prompts.toml`

- [ ] 3. 前端按钮 + i18n 键

  **What to do**:
  - 在 `apps/desktop/frontend/src/components/LlmAnalysisPanel.vue` 的 `actions` 数组中（当前约 line 20-27，包含 6 个内置 action），新增第 7 个条目：
    ```javascript
    { key: 'market_adversarial_lens', label: t('research.marketAdversarialLens'), icon: '🔍' }
    ```
    （放置在 `portfolio_review` 之后）
  - 在 `apps/desktop/frontend/src/locales/zh.json` 的 `research` 嵌套对象中（约 line 593-598），新增：
    ```json
    "marketAdversarialLens": "市场博弈视角"
    ```
  - 在 `apps/desktop/frontend/src/locales/en.json` 的 `research` 嵌套对象中（约 line 593-598），新增：
    ```json
    "marketAdversarialLens": "Market Adversarial View"
    ```
  - 确认两个 JSON 文件语法正确（用 `Get-Content | ConvertFrom-Json` 或等价的 JSON linter 验证）

  **Must NOT do**:
  - 不引入下拉菜单/分组/可展开面板
  - 不修改按钮布局（grid/flex）——直接增加按钮即可，Vue 的 `v-for` 会自动渲染
  - 不在 `zh.json` 或 `en.json` 的 `analysis.*` 段（前端 UI 标签段，约 line 510-560）中添加——仅添加在 `research.*` 段（LLM action 标签段）
  - 不修改 `store.js`

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Vue 3 前端组件修改 + i18n JSON 编辑，涉及 UI 布局和技术文案
  - **Skills**: None required

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 Tasks 1, 2, 4 并行）
  - **Blocks**: Task 6（前端 QA 依赖此变更）
  - **Blocked By**: None

  **References** (CRITICAL):
  - `apps/desktop/frontend/src/components/LlmAnalysisPanel.vue:20-27` — `actions` 数组定义，确认现有按钮的 key/label/icon 格式
  - `apps/desktop/frontend/src/locales/zh.json:593-598` — `research.*` 段，确认现有 6 个 action 的 i18n key 命名模式（如 `research.marketStory`、`research.riskView`）
  - `apps/desktop/frontend/src/locales/en.json:593-598` — 对应的英文翻译

  **Acceptance Criteria**:
  - 两个 JSON 文件语法有效（无 trailing comma、无 broken nesting）
  - `research.marketAdversarialLens` 在 zh.json 和 en.json 中均存在且值非空
  - actions 数组包含 7 个条目（原 6 + 1）

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: 按钮在中英文环境下正确渲染
    Tool: Playwright
    Preconditions: Frontend built and desktop running
    Steps:
      1. Navigate to LLM analysis panel
      2. Verify a 7th button exists with text matching the locale
      3. Switch locale (zh ↔ en) via i18n toggle
      4. Verify button label changes accordingly
    Expected Result: 中文环境显示"市场博弈视角"，英文环境显示"Market Adversarial View"
    Failure Indicators: Button missing, label showing raw key "research.marketAdversarialLens", locale switch doesn't update label
    Evidence: .omo/evidence/task-3-button-zh.png, .omo/evidence/task-3-button-en.png
  ```

  **Evidence to Capture**:
  - [ ] `.omo/evidence/task-3-button-zh.png` — 中文环境按钮截图
  - [ ] `.omo/evidence/task-3-button-en.png` — 英文环境按钮截图

  **Commit**: `feat(desktop): add market adversarial lens button and i18n keys`
  - Files: `apps/desktop/frontend/src/components/LlmAnalysisPanel.vue`, `apps/desktop/frontend/src/locales/zh.json`, `apps/desktop/frontend/src/locales/en.json`

- [ ] 4. 文档更新

  **What to do**:
  - 更新 `crates/research-skills/AGENTS.md` 的 "5 个研究动作" 表格（约 line 23-30），改为 "6 个内置研究动作 + 文件 persona（自定义）"，新增一行：
    ```
    | `market_adversarial_lens` | 市场博弈视角 | 博弈分析师 | 从资金角色、流动性、筹码分布、预期差角度分析当前市场博弈结构（文件 persona） |
    ```
  - 更新表格上方标题行，体现文件 persona 的存在
  - 更新 AGENTS.md 中的 "哪里查找" 行：
    ```
    | 修改/新增文件 persona | `config/prompts.toml` | 编辑或新增 `[prompts.*]` section |
    ```
  - 更新根目录 `AGENTS.md` 中 LLM 分析部分的 action 列表（约 line 150），在 `--action` 选项列表中追加 `market_adversarial_lens`

  **Must NOT do**:
  - 不修改 `docs/` 下的操作手册（不属于本次 scope）
  - 不修改 `memory/decisions.md`（无 ADR 变更）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 纯 Markdown 文档更新，3 个文件的小范围编辑
  - **Skills**: None required

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 Tasks 1, 2, 3 并行）
  - **Blocks**: None（无下游依赖）
  - **Blocked By**: None

  **References** (CRITICAL):
  - `crates/research-skills/AGENTS.md:23-30` — 当前 action 表格
  - `crates/research-skills/AGENTS.md:12` — "6 个内置 action + 自定义 persona"的当前文档结构
  - 项目根 `AGENTS.md` — LLM 分析 CLI 命令部分，`--action` 参数的可选值列表

  **Acceptance Criteria**:
  - `research-skills/AGENTS.md` 的 action 表格包含 `market_adversarial_lens` 行
  - 根 `AGENTS.md` 的 `--action` 参数文档包含 `market_adversarial_lens`

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: 文档准确反映 persona 配置
    Tool: Bash (grep)
    Preconditions: Task 2 (persona prompt) completed
    Steps:
      1. grep "market_adversarial_lens" crates/research-skills/AGENTS.md → should match
      2. grep "market_adversarial_lens" AGENTS.md → should match (in --action list)
    Expected Result: Both AGENTS.md files reference the new persona
    Failure Indicators: No match found in either file
    Evidence: .omo/evidence/task-4-docs-grep.txt
  ```

  **Evidence to Capture**:
  - [ ] `.omo/evidence/task-4-docs-grep.txt` — grep 输出

  **Commit**: `docs(research-skills): document market_adversarial_lens persona`
  - Files: `crates/research-skills/AGENTS.md`, `AGENTS.md`

- [ ] 5. CLI QA —— persona 解析与回归验证

  **What to do**:
  - 运行 4 个 CLI 命令验证变更：
    1. `cargo check` — 确认零新增 warning
    2. `cargo test -p report-engine -p research-skills -p app-service` — 确认全绿
    3. `cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global` — 确认 persona 解析成功
    4. `cargo run -p quant-cli -- llm-analyze --action market_story --scope global` — 确认内置 persona 不受影响
    5. `cargo run -p quant-cli -- llm-analyze --action short_term_trader --scope global` — 确认文件 persona 不受影响
  - 记录每个命令的 exit code 和关键输出

  **Must NOT do**:
  - 不依赖真实 API key（测试 persona 解析和 TOML 加载即可；实际 LLM 调用需要 API key 时允许返回占位符）
  - 不修改任何源文件

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 纯 CLI 命令执行 + 输出验证，无代码变更
  - **Skills**: None required

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 Task 6 并行）
  - **Parallel Group**: Wave 2（与 Task 6 并行）
  - **Blocks**: None
  - **Blocked By**: Tasks 1, 2

  **References** (CRITICAL):
  - 项目根 `AGENTS.md` — `cargo check`、`cargo test`、`llm-analyze` 的可用 flags

  **Acceptance Criteria**:
  - `cargo check` exit code 0，新增 warning 数为 0
  - `cargo test` 三个 crate 全部 PASS
  - `llm-analyze --action market_adversarial_lens` 返回有效 JSON（含 `action`、`persona_label` 字段）
  - `llm-analyze --action market_story` 成功（回归验证）
  - `llm-analyze --action short_term_trader` 成功（回归验证）

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: 完整 CLI 验证链
    Tool: Bash (cargo)
    Preconditions: Tasks 1-4 completed, Rust toolchain available
    Steps:
      1. cargo check 2>&1 → verify exit code 0, no new warnings
      2. cargo test -p report-engine -p research-skills -p app-service 2>&1 → verify "test result: ok"
      3. cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global 2>&1 → verify exit code 0, JSON output .action == "market_adversarial_lens"
      4. cargo run -p quant-cli -- llm-analyze --action market_story --scope global 2>&1 → verify exit code 0
      5. cargo run -p quant-cli -- llm-analyze --action short_term_trader --scope global 2>&1 → verify exit code 0
    Expected Result: All 5 commands succeed
    Failure Indicators: Any command returns non-zero exit code, cargo check shows new warnings, unknown action error
    Evidence: .omo/evidence/task-5-cli-qa.txt (full output log)
  ```

  **Evidence to Capture**:
  - [ ] `.omo/evidence/task-5-cli-qa.txt` — 全部 5 个命令的输出

  **Commit**: NO（QA 验证，不产生代码变更）

- [ ] 6. 前端 QA —— 按钮渲染与交互

  **What to do**:
  - 构建前端并启动桌面端（或 Vite dev server + Tauri）：
    ```bash
    cd apps/desktop/frontend
    npm run build
    # 或 npm run dev
    ```
  - 使用 Playwright 验证：
    1. 导航至 LLM 分析面板
    2. 确认"市场博弈视角"按钮存在于面板中（第 7 个按钮）
    3. 确认按钮文本在中文环境下显示"市场博弈视角"
    4. 切换到英文环境，确认按钮文本变为 "Market Adversarial View"
    5. 点击按钮，确认 `loading` 状态出现（按钮变灰/显示 spinner）
    6. 验证 `llm-api` 调用携带 `action: "market_adversarial_lens"`
  - 如果 LLM API key 未配置，验证占位符文本正确渲染

  **Must NOT do**:
  - 不修改任何源文件
  - 不对按钮布局做视觉精细调整

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 前端 Playwright QA，截图验证 + 交互测试
  - **Skills**: [`playwright`]
    - `playwright`: 浏览器自动化，按钮渲染截图 + 交互验证

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 Task 5 并行）
  - **Parallel Group**: Wave 2（与 Task 5 并行）
  - **Blocks**: None
  - **Blocked By**: Task 3

  **References** (CRITICAL):
  - `apps/desktop/frontend/src/components/LlmAnalysisPanel.vue:20-27` — 修改后的 actions 数组
  - `apps/desktop/frontend/src/locales/zh.json` — 修改后的 i18n 键
  - `apps/desktop/frontend/src/store.js:82-95` — `llmLoading`、`llmError`、`llmAnalysis` 状态管理

  **Acceptance Criteria**:
  - 面板中有 7 个按钮（原 6 + 1）
  - 中文按钮文本为"市场博弈视角"
  - 英文按钮文本为 "Market Adversarial View"
  - 点击按钮触发 loading 状态

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: 按钮渲染与 locale 切换
    Tool: Playwright
    Preconditions: Frontend built and running (desktop or Vite dev)
    Steps:
      1. Navigate to the LLM analysis panel area
      2. Wait for .llm-actions selector to be visible
      3. Count buttons → expect 7
      4. Check the 7th button text → expect "市场博弈视角" (zh locale)
      5. Click i18n toggle to switch to English
      6. Check the 7th button text → expect "Market Adversarial View" (en locale)
    Expected Result: Button count = 7, labels match locale
    Failure Indicators: Button count ≠ 7, label shows raw key "research.marketAdversarialLens", locale switch doesn't work
    Evidence: .omo/evidence/task-6-button-render-zh.png, .omo/evidence/task-6-button-render-en.png

  Scenario: 按钮点击触发 loading 状态
    Tool: Playwright
    Preconditions: Frontend built and running
    Steps:
      1. Click the "市场博弈视角" button
      2. Verify button enters disabled/loading state (e.g., has .loading class, or button text changes to "分析中...")
      3. Wait for response (or timeout)
      4. Verify button returns to enabled state
    Expected Result: Loading state appears on click, clears on completion
    Failure Indicators: Button stays enabled during analysis, no visual feedback
    Evidence: .omo/evidence/task-6-button-loading.png
  ```

  **Evidence to Capture**:
  - [ ] `.omo/evidence/task-6-button-render-zh.png` — 中文环境按钮列表
  - [ ] `.omo/evidence/task-6-button-render-en.png` — 英文环境按钮列表
  - [ ] `.omo/evidence/task-6-button-loading.png` — 点击后的 loading 状态

  **Commit**: NO（QA 验证，不产生代码变更）

---

## Final Verification Wave

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns. Check evidence files exist.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo check` + `cargo test -p report-engine -p research-skills -p app-service`. Review all changed files for: `as any`/`@ts-ignore`, empty catches, console.log, commented-out code. Check AI slop: excessive comments, over-abstraction. Validate JSON files (zh.json, en.json) are valid JSON.
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from Tasks 5 and 6. Test: CLI invocation with valid action → valid JSON. CLI invocation with invalid action → error. Frontend button renders, dispatches, shows loading, renders markdown. Save evidence to `.omo/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **1**: `feat(research-skills): inject 6 computed fields into build_snapshot_context for LLM adversarial analysis` — `crates/research-skills/src/action.rs`
- **2**: `feat(prompts): add market_adversarial_lens persona for market microstructure analysis` — `config/prompts.toml`
- **3**: `feat(desktop): add market adversarial lens button and i18n keys` — `apps/desktop/frontend/src/components/LlmAnalysisPanel.vue`, `apps/desktop/frontend/src/locales/zh.json`, `apps/desktop/frontend/src/locales/en.json`
- **4**: `docs(research-skills): document market_adversarial_lens persona` — `crates/research-skills/AGENTS.md`

---

## Success Criteria

### Verification Commands
```bash
# Persona 解析
cargo run -p quant-cli -- llm-analyze --action market_adversarial_lens --scope global
# Expected: JSON output with "persona_label" field, no "unknown action" error

# 回归：内置 persona
cargo run -p quant-cli -- llm-analyze --action market_story --scope global
# Expected: succeeds, output format unchanged

# 回归：文件 persona
cargo run -p quant-cli -- llm-analyze --action short_term_trader --scope global
# Expected: succeeds, output format unchanged

# 构建检查
cargo check
# Expected: zero NEW warnings (pre-existing warnings acceptable)

# 测试
cargo test -p report-engine -p research-skills -p app-service
# Expected: all tests pass
```

### Final Checklist
- [ ] 所有 "Must Have" 已实现
- [ ] 所有 "Must NOT Have" 已验证合规
- [ ] `cargo check` 零新增 warning
- [ ] `cargo test` 全绿
- [ ] CLI QA 通过（persona 解析 + 回归验证）
- [ ] 前端 QA 通过（按钮渲染 + 交互）
- [ ] ADR-106 审核通过（无阈值/if-then/BUY-SELL-HOLD/仓位）
