# P2: Shared Adversarial Context Layer — 前置共享博弈分析层

## TL;DR

> **Quick Summary**: 将 P1 交付的 `market_adversarial_lens` 从"独立 persona"升级为"前置共享分析层"——默认开启，同一 scope 同一分析日期只计算一次博弈分析并落盘，随后按 **persona 职责分级**（full / summary / none）将"市场博弈**假设背景**"注入各 persona 的 prompt。注入语义是"供验证或反驳的假设"，不是"供接受的结论"。
>
> **核心语义**（用户裁定）：所有人默认**获得** adversarial lens 提供的市场竞争假设背景，而非自动**接受** adversarial 结论。下游 persona 的职责是结合系统数据验证或反驳其中的假设。
>
> **成本模型**：Daily Shared Context Pattern —— 每 scope 每日首次调用付一次前置 LLM 成本，后续全部复用落盘记录。成本 = 1 次/日/scope，而非 8 次/日。
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 4 waves
> **Critical Path**: T1/T2 → T3 → T4/T5 → T6

---

## Context

### 来自 P1 的已验证基础
- `market_adversarial_lens` persona 已上线并产出高质量五维博弈分析（ADR-109）
- `build_snapshot_context()` 已注入 6 个微观结构字段（ADR-110）
- `llm_history.rs` 现有 `save_record` / `latest_record` 机制——`workspace/llm-history/{scope}/{action}/{date}.json`
- P1 实测：LLM 主动引用注入字段且遵守 ADR-106 标注

### 启用决策（用户裁定：方案 A 修改版）
- **默认开启**：`auto_inject` 默认 `true`。定位是 Research Operating System 的基础设施能力（类比 primary key / Provenance / Evidence），不是可选插件
- **语义修正**：注入物是"市场竞争**假设背景**（hypothesis background）"。注入段头部明确：这是假设性解读，供下游 persona **验证或反驳**，不是事实、不是结论
- **分级注入**：不同 persona 职责不同，注入强度不同——市场叙事类 full（全文）、机制解释类 summary（弱注入）、数据检查类可 none、adversarial 自身 none（递归防护）

### 默认分级映射（写在 config 默认值中，可在 llm.toml 覆盖）
| Persona | 级别 | 理由 |
|---|---|---|
| `market_story` | full | 叙事需要完整博弈结构 |
| `portfolio_review` | full | 组合张力解读是核心受益场景 |
| `risk_view` | full | 风控视角与博弈分析天然互补 |
| `devils_advocate` | full | 质疑者需要假设靶子 |
| `short_term_trader` / `long_term_allocator` | full | 市场解读类 |
| `explain_decision` | summary | 解释引擎决策，弱注入防观点污染 |
| `preclose_review` | summary | 执行层解读，弱注入 |
| `market_adversarial_lens` | none | 递归防护（代码硬编码，不依赖配置） |
| 未知/自定义 persona | full | 市场解读类为默认假设，可在 llm.toml 覆盖 |

### 关键架构决策
| 决策 | 选择 | 理由 |
|---|---|---|
| 存储 | 复用 `llm_history`，action="adversarial" | 零新模块 |
| 新鲜度 | `record.report_date == snapshot.report_date` 才复用 | 防跨日污染，1 次/日成本 |
| 递归防护 | 代码硬编码 `action == "market_adversarial_lens"` → none | 不依赖配置正确性 |
| 失败降级 | 前置失败 → 静默跳过，主调用永不受影响 | 共享层是增强不是依赖 |
| 快照一致性 | 前置与主调用共享同一 `DashboardSnapshot` | 消除数据时间差 |
| CLI 覆盖 | `--adversarial <full\|summary\|none>` 单次覆盖配置 | 双向覆盖（可强制 full 也可临时关闭） |

### 未来接缝（不在本 P2 范围）
用户提出的三元组：Market Phase Engine（市场处于什么阶段）+ Contradiction Layer（系统内部冲突）+ Adversarial Lens（反方如何攻击）。三者是一组，前两者为后续独立规划候选。

---

## Work Objectives

### Core Objective
让博弈分析成为所有 LLM persona 默认共享的前置假设背景层：每 scope 每日计算一次，按 persona 职责分级注入，语义为"供验证或反驳的假设"，全程可配置、可覆盖、可降级、无递归。

### Concrete Deliverables
1. `llm_history.rs`：`adversarial_context_section(record, level)` 分级段落构建器 + 单元测试
2. `core-domain/src/lib.rs`：`AdversarialSection` + `InjectLevel` 枚举（serde default，Schema Evolution 合规）
3. `config_loader.rs`：`ResolvedLlmConfig` 新增 adversarial 配置解析
4. `config/llm.toml.example`：`[llm.adversarial]` + `[llm.adversarial.inject]` schema 文档
5. `lib.rs`：`analyze_with_action` 签名变更 + `ensure_adversarial_context()` + 分级注入 + 递归防护 + 降级
6. CLI：`--adversarial <full|summary|none>` flag
7. Tauri + 前端：可选参数 + 注入级别选择器
8. 文档：README + research-skills/AGENTS.md + app-service/AGENTS.md

### Definition of Done
- [ ] 默认（无 flag 无配置）调用 `market_story`：首次自动产出 adversarial 落盘，输出体现假设背景被消费
- [ ] 同日同 scope 第二次调用（任意 persona）：零额外前置调用，直接复用
- [ ] `explain_decision` 默认只收到 summary 级注入（~400 字符摘要）
- [ ] `--adversarial none` 单次关闭；`--adversarial full` 单次升级
- [ ] `--action market_adversarial_lens`：任何情况下无自我注入
- [ ] 无 API key：主调用正常 placeholder，前置静默跳过
- [ ] 注入段头部含"假设背景"+"验证或反驳"语义，非"结论"语义
- [ ] `cargo check` 零新增 warning；`cargo test` 全绿

### Must Have
- 注入段头部语义为"假设背景，供验证或反驳"，禁止"结论"语义
- 分级注入：full=全文 / summary=摘要 / none=跳过
- 递归防护硬编码（不依赖配置）
- 前置失败静默降级，绝不阻塞主调用
- 前置与主调用共享同一 snapshot
- 默认开启，但每一级都可在 llm.toml 覆盖，CLI 可单次覆盖

### Must NOT Have (Guardrails)
- 不新建存储模块/目录结构（复用 `llm_history`）
- 不修改 6 个内置 persona 的 prompt 常量
- 不改变返回 JSON 的现有字段（可新增，不删改）
- 不做跨日复用
- 不引入新的 Rust 依赖
- 不把 InjectLevel 的 none 默认值赋给除 adversarial 自身外的任何 persona（配置文件中用户可自行设置，但代码默认值不给）

---

## Execution Strategy

```
Wave 1 (并行，零依赖):
├── T1: llm_history.rs 分级段落构建器 + 测试              [quick]
└── T2: InjectLevel + config schema + 解析                [quick]

Wave 2 (依赖 Wave 1):
└── T3: lib.rs 集成 — ensure_adversarial_context + 分级注入 + 递归防护 + 签名变更 [unspecified-medium]

Wave 3 (依赖 T3，并行):
├── T4: CLI --adversarial flag                            [quick]
└── T5: Tauri 参数 + 前端注入级别选择器                    [visual-engineering]

Wave 4 (依赖 Wave 3，并行):
├── T6: CLI QA — 五场景验证                              [quick]
└── T7: 文档更新                                          [quick]

Final Wave:
└── F1: 合规审查                                          [oracle]
```

---

## TODOs

- [ ] 1. `llm_history.rs` — 分级 adversarial 段落构建器
  - 新增 `pub fn adversarial_context_section(record: &LlmAnalysisRecord, level: &str) -> String`
  - `full` → 注入完整 `analysis_text`；`summary` → 注入 `record.summary`；其他值（含 `none`，调用方应已过滤）→ 退化为 summary
  - 段落头部固定为：
    ```
    ## 市场博弈假设背景（{report_date}）
    
    > 注意：以下内容为前置博弈分析产生的**假设性背景**，描述市场可能的博弈结构。
    > 它不是事实证据，不是结论，不得作为你本次判断的依据来源。
    > 你的职责是结合系统数据验证或反驳其中的假设。
    ```
  - 单元测试：头部含"假设性背景"+"验证或反驳"；full 含全文、summary 含摘要
  - **MUST NOT**：不复用 `previous_interpretation_section`（语义不同）

- [ ] 2. `InjectLevel` + config schema + 解析
  - `core-domain/src/lib.rs`：新增 `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] #[serde(rename_all = "lowercase")] pub enum InjectLevel { Full, Summary, None }`（带 `Default` = Full）
  - `core-domain` `LlmSection` 新增 `#[serde(default)] pub adversarial: Option<AdversarialSection>`；`AdversarialSection { #[serde(default = "default_true")] pub auto_inject: bool, #[serde(default)] pub inject: HashMap<String, InjectLevel> }`
  - `config_loader.rs` `ResolvedLlmConfig` 新增 `adversarial_auto_inject: bool`（默认 true）+ `adversarial_inject: HashMap<String, InjectLevel>`（内置默认映射见 Context 节表格）；`resolve()` 从 TOML 合并，文件缺失键用代码默认
  - `config/llm.toml.example` 新增带注释的 `[llm.adversarial]` 段
  - **MUST NOT**：不改 legacy `LlmConfig`（SQLite 结构）；不把默认 auto_inject 写成 false

- [ ] 3. `lib.rs` — `analyze_with_action` 集成
  - 签名：`pub async fn analyze_with_action(&self, action: &str, scope: ReportScope, adversarial: Option<InjectLevel>) -> Result<Value>`；`None` → 查 `resolved.adversarial_inject.get(action)` → 缺省 Full；`auto_inject == false` 且 CLI 未显式指定 → 等同 None 级别
  - `ensure_adversarial_context(scope, snapshot, llm_config, api_key) -> Option<LlmAnalysisRecord>`：新鲜命中直接返回；否则用 `market_adversarial_lens` persona + 同一 snapshot 调 `call_llm_api`，成功落盘（action="adversarial"），任何失败 `eprintln!` + 返回 `None`
  - 注入点：在 previous_interpretation 之前；`action != "market_adversarial_lens"`（硬编码递归防护）且 level != None 且 record 存在时 push
  - 返回 JSON 新增 `"adversarial": {"injected": bool, "level": "full|summary|none", "fresh": bool}` 诊断字段
  - **MUST NOT**：前置失败不传播 Err；placeholder（无 key）不落盘；前置调用不递归进 `analyze_with_action`（直接走 build_prompt_with_persona + call_llm_api）

- [ ] 4. CLI — `--adversarial <full|summary|none>` flag
  - `main.rs` `LlmAnalyze` / `AnalyzeWithLlm` 变体新增 `adversarial: Option<String>`（clap `#[arg(long, value_parser = ["full","summary","none"])]`）
  - `commands/llm.rs` 两个 handler 透传，字符串映射到 `InjectLevel`

- [ ] 5. Tauri + 前端
  - `src-tauri/src/lib.rs` `analyze_with_llm(scope, action, adversarial: Option<String>)` 透传
  - `frontend/src/api/tauri.js` `analyzeWithLlm(scope, action, adversarial)` 第三参
  - `LlmAnalysisPanel.vue` 按钮行下方加注入级别选择（full/summary/none 三态，默认 full），i18n 双语言
  - **MUST NOT**：不改现有按钮行布局；选择器默认 full

- [ ] 6. CLI QA — 五场景验证
  - S1 默认开启+首次注入：`llm-analyze --action market_story` → adversarial 落盘 + 输出消费假设背景
  - S2 复用：紧接 `llm-analyze --action risk_view` → 无第二次前置调用
  - S3 分级：`llm-analyze --action explain_decision` → 注入为 summary 级
  - S4 CLI 覆盖：`--adversarial none` → 无注入；`--adversarial full` 于 explain_decision → 升级全文
  - S5 递归防护：`--action market_adversarial_lens --adversarial full` → 无自我注入
  - S6 降级（可选）：无 key → placeholder 正常，无报错
  - 证据落盘 `.omo/evidence/p2-*.txt|json`

- [ ] 7. 文档更新
  - README LLM 段补共享层说明 + `--adversarial` 用法
  - `research-skills/AGENTS.md` CONVENTIONS 增加共享层条目
  - `app-service/AGENTS.md` WHERE TO LOOK 增加 `ensure_adversarial_context` 行

---

## Final Verification Wave

- [ ] F1. 合规审查 — `oracle`
  - 验证：注入段为"假设背景"语义；分级正确；递归防护硬编码；失败降级；默认开启但可覆盖；无新增依赖；`builtin_persona` 未动
  - Output: `Must Have [N/N] | Must NOT Have [N/N] | VERDICT: APPROVE/REJECT`

---

## Commit Strategy
1. `feat(llm): add tiered adversarial context section builder` — llm_history.rs
2. `feat(config): adversarial auto_inject + per-persona inject levels` — core-domain, config_loader, llm.toml.example
3. `feat(app-service): default-on shared adversarial hypothesis injection` — lib.rs
4. `feat(cli): add --adversarial level flag to llm-analyze` — main.rs, commands/llm.rs
5. `feat(desktop): adversarial inject level selector` — src-tauri, tauri.js, LlmAnalysisPanel.vue, locales
6. `docs: shared adversarial context layer (default-on, tiered)` — README, AGENTS.md×2

## Success Criteria
```bash
cargo run -p quant-cli -- --quiet llm-analyze --action market_story --scope global        # S1
cargo run -p quant-cli -- --quiet llm-analyze --action risk_view --scope global           # S2
cargo run -p quant-cli -- --quiet llm-analyze --action explain_decision --scope global    # S3
cargo run -p quant-cli -- --quiet llm-analyze --action market_story --adversarial none    # S4a
cargo run -p quant-cli -- --quiet llm-analyze --action explain_decision --adversarial full # S4b
cargo run -p quant-cli -- --quiet llm-analyze --action market_adversarial_lens --adversarial full # S5
cargo check && cargo test -p app-service
```
