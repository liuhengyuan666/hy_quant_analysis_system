# Project Coding Idioms & Patterns (编码范式与设计方言)

> 认知护栏：记录本项目中被团队高度达成共识的“代码写法习惯”，新进 Agent 必须严格继承此编码风格。

## 1. 错误处理范式 (Error Handling)

- 使用 `anyhow` 进行错误传播，顶层函数返回 `Result<T>`
- 自定义错误使用 `thiserror` 派生
- 禁止裸 `unwrap()`，必须使用 `?` 或显式 `match`
- 数据获取失败进入 `failed_items`，不再静默产出空结果

## 2. 异步与并发范式 (Concurrency/Async)

- 使用 `tokio` 作为异步运行时
- 宏引擎和指标计算使用并行迭代（`par_iter`）
- Tauri 命令使用异步函数
- 数据刷新管线按顺序执行，不能倒序或跳过

## 3. 状态管理与数据转换 (Data Mapping)

- 核心领域模型定义在 `core-domain` crate
- 所有 DTO 使用 `serde` 序列化/反序列化
- ClickHouse JSON 反序列化字段必须携带 `#[serde(default)]`（Schema Evolution 政策）
- 状态转换使用显式状态机（Task Lifecycle、ADR Lifecycle）
- **枚举重命名兼容**：领域枚举重命名时保留 `#[serde(alias = "OLD_NAME")]` 兼容存量数据反序列化（如 ExecutionState: BUY_NOW→Increase）；注意 alias 只管输入，序列化输出永远是新名——所有 consumer（前端分组/i18n/统计）必须同步切换

## 4. 前端架构范式 (Frontend)

- Vue 3 + Composition API，使用 `reactive()` 共享状态
- 组件从 `store.js` 读取状态，通过 `event bridge` 回调 main.js
- CSS 变量桥接：全局 CSS 定义设计 token，Vue 组件消费 CSS 变量
- i18n 使用 `vue-i18n@11`，默认中文，嵌套 JSON key 结构
- **HTML/CSS 混合布局**：复杂数据可视化（如 RotationPanel）优先使用 HTML/CSS 原生布局替代纯 ECharts，提升信息密度、响应式对齐和无障碍访问；ECharts 仅用于纯图表场景
- **CSS 自定义 Tooltip**：禁用原生 `title` 和 ECharts 默认 Tooltip，使用绝对定位的 CSS 自定义 Tooltip（`position: absolute` + `visibility/opacity` 过渡），确保暗黑主题样式一致、无割裂感
- **Sticky 布局约束**：`position: sticky` 的侧边栏必须满足 (1) 顶部参考元素（如 header）固定高度，消除 `top` 旷量；(2) sticky 元素高度精确计算，使其 margin-box 不溢出 container 的 padding-box（通常 `height: calc(100vh - headerHeight - padding * 2 - 2px)` 留 2px 安全边距），避免滚动到底部时 push-out 抖动
- **CSS Hover 内嵌详情卡片**：复杂数据详情（如信号归因 breakdown）优先使用纯 CSS hover 触发内嵌卡片（`position: absolute` + `visibility/opacity` 过渡），替代独立弹窗/抽屉/滑出面板；卡片内使用 `grid` 布局（如 `grid-template-columns: 1fr 1fr`）实现多列信息密度最大化
- **强制 nowrap 文本策略**：悬浮卡片、内嵌详情面板等空间有限场景，所有文本元素必须声明 `white-space: nowrap` + `flex-shrink: 0`，防止 flex/grid 容器内因换行导致布局崩坏；配合 `overflow: hidden` 或容器扩宽（如 `width: 46rem`）保证可读性
- **前端投影边界（ADR-107/108）**：UI 不拥有投资语义——Frontend: Render / Backend: Interpret / Decision: Execution Engine。禁止前端出现任何分数到买卖语义的映射判断（如 `if (score > 70) show buy`）；只渲染后端 contract 输出的事实字段（best_strategy 标记、alignment、场景分等后端给定的值）；后端不为前端打包"超级 Dashboard API"，新认知层用独立展示入口而非堆进 Dashboard 首页

## 5. 数据管线范式 (Data Pipeline)

- 管线顺序：ingest → indicators → macro → rotation → strategy → signals → backtest
- 各阶段日期检查通过 `pipeline-dates` 和 `explain-latest-gate`
- 数据健康检查：`check-data-health` 检查 provider 可达性、缺口、异常波动
- 默认 `export-report` 在 latest gate 落后时直接失败（fail-loud）

## 6. 配置管理范式 (Configuration)

- **TOML 配置模式**：LLM 和 FRED 配置均使用 TOML 文件 + 环境变量插值（`config/llm.toml`、`config/fred.toml`）
- **API Key 安全存储**：TOML 文件 gitignored，支持 `${ENV_VAR}` 引用，避免硬编码
- **可配置开关**：FRED 获取支持 `enabled` 开关，禁用后使用 ClickHouse 已存数据（fail-safe）
- **配置加载优先级**：CLI 参数 > TOML 文件（含环境变量插值） > 默认值
- **LLM API Key 三级回退**：TOML → Keyring → SQLite
- **FRED API Key 单级回退**：TOML 文件（低敏感度，免费政府数据 API）
- Universe 配置使用 JSON（`config/universe.json`）
- 交易日历使用静态 JSON（`config/calendars/`）

## 7. 研究验证范式 (Research Validation)

- Ground Truth 与 Predictor 必须使用完全独立的数据路径
- Regime 评估使用三层独立框架：State Layer（描述性）、Economic Layer（预测性）、Allocation Layer（决策性）
- 所有实验结论需在 `confirmation_days=1` 下重新验证
- Wave 研究阶段化：Wave 7（GT验证）、Wave 8（Insight Composer）、Wave 9（Daily Report）

## 8. V6 Reporting Platform 范式 (Reporting Architecture)

- **数据所有权**：`ResearchDataset` 是 app-service 内部 ephemeral 查询结果容器，永不暴露到边界之外
- **语义所有权**：`ResearchContext` 是 canonical semantic model，跨消费者共享，不承载 consumer-specific 字段
- **计算工作区**：`ResearchSnapshot` 是 app-service 内部 computation workspace，由 `ResearchDataset` 构建，不再直接查询数据源
- **展示所有权**：`ReportingSnapshot` 承载 metadata（scope/date/generated_at）+ `ResearchContext`
- **文档输入**：`ReportInput` 仅承载 document-specific payload，不重复 metadata
- **渲染所有权**：Formatter（Markdown / Text / JSON）只负责渲染，无业务计算
- **领域计算归属**：所有可复用的研究计算位于 `core-domain::research`
- **Builder 状态**：`ReportBuilder` trait 当前为 Pending Evaluation，无实现者，不得人工添加实现
- **Schema Evolution**：ClickHouse JSON DTO 的新字段必须携带 `#[serde(default)]` 或手动 remapping

## 9. 外部数据源集成范式 (External Data Provider)

- **编码检测**：HTTP 响应必须验证编码格式，不要假设所有 API 返回 UTF-8；Tencent API 返回 GB18030，需使用 `encoding_rs` 解码
- **Symbol 映射**：每个 provider 有独立的 symbol 前缀规则；上海指数 `000xxx` 必须用 `sh` 前缀，深圳指数 `399xxx` 用 `sz` 前缀
- **字段索引验证**：解析分隔符格式时，必须确认字段索引对应关系；实际验证前不要假设字段顺序（如 `parts[1]` vs `parts[2]`）
- **降级策略**：实时数据获取失败时，必须降级为 `Skip` 状态，不能 panic 或影响主系统运行

- **Shadow Production 冻结**：State Layer、Economic Layer、Signal Engine、weights、thresholds、allocation、backtest 语义全部冻结
- **Production Surface 冻结**：DashboardSnapshot、ResearchContext、Markdown Report 在观察期间不修改展示逻辑
- **Research Surface 允许新增**：观测/诊断工具（symbol-diagnostics, symbol-scoreboard, rotation-ranking）作为独立 CLI 工具，不进入主观察链路
- **Explainability Layer 约束**：可以解释现有决策，但不得生成新评分、排名或决策信号
- **ADR 优先级**：User instruction > Active ADRs > Traps > Search results > Model knowledge

## 10. V7 Research Platform 1.0 范式 (Research Architecture)

- **研究分层不可变**：Observation (V6) → Market Evolution (V7.1) → Historical Evidence (V7.2) → Research Synthesis (V7.3)；ADR-077 后四层语义架构冻结，新增内容只能作为 Research Content Evolution 输入现有层
- **领域计算归属**：所有可复用的研究计算位于 `core-domain::research`；`app-service` 只编排，不存放量化逻辑
- **Evidence Aggregation**：Consensus 采用加权证据聚合 `(source, weight)`，不使用硬编码 if-else 规则树；新增证据类型时加入聚合层并重新跑 Calibration
- **研究语言输出**：Consensus 只输出研究语言（Bias / Confidence / Supporting Evidence / Contradicting Evidence / Summary），不输出买卖建议、仓位、目标价或止损
- **版本化漂移检测**：`ConsensusSummary` 和 `Calibration Baseline Version` 必须携带版本号，用于长期行为漂移检测
- **可配置化权重**：`ConsensusConfig` 将证据权重和阈值集中配置，默认值保持 V7.3 行为；权重/阈值变更需经 Calibration 验证并可能触发 Baseline Version 递增
- **Calibration 语义不变量**：`CURRENT_CALIBRATION_BASELINE_VERSION` 仅在 Evidence 语义变化时递增（距离度量、归一化、特征权重、阈值、报告统计语义），实现优化不触发递增
- **Historical Analogues 不暴露原始相似度**：对外使用 rank 或定性等级（Very High / High / Moderate / Low），避免用户误读原始距离值

## 12. LLM 集成范式 (LLM Integration)

- **LLM 边界冻结（ADR-106）**：数据流永远为 确定性引擎 → 决策事实 → LLM 解释（LLM 永远在最右端）；LLM 只解释，不创建信号、不评分、不排名、不覆盖决策、不输出仓位/目标价
- **Prompt 双源解析**：内置 persona 常量（`research-skills/src/action.rs`）+ `config/prompts.toml` 自定义 persona；TOML 只承载视角指令，禁含阈值规则与 if/then 逻辑（ADR-106）
- **上下文组装**：`app-service::analyze_with_action` 注入策略评分矩阵、数据完整性状态、组合姿态（仅 portfolio_review）与前次解读；`build_snapshot_context()` 刻意排除信号分数/RS 分数/回测指标，只传标签与排名
- **LLM 历史回环**：每次分析落盘 `workspace/llm-history/{scope}/{action}/{date}.json`，下次分析自动注入"前次解读"段并标注为非证据背景
- **输出不入户**：LLM 结果不作为分析数据写入 ClickHouse；仅导出 markdown 到 `reports/` 并在 report_snapshot 登记文件路径
- **未配置降级**：无 API key 时返回 placeholder 文本（`placeholder: true`），不 panic、不阻塞主链路
- **共享博弈假设背景（ADR-112~114）**：Daily Shared Context Pattern——每 scope 每日一次前置博弈分析落盘复用，按 persona 职责分级注入；注入段头部必须声明"假设背景，供验证或反驳"语义；递归防护硬编码（不依赖配置）；任何失败静默降级，绝不阻塞主调用；Level（粒度）与 ContentPolicy（尺寸保护）解耦为独立旋钮
- **异步预生成范式**：market-refresh 成功后的 LLM 预生成使用 detached `std::thread` + 独立 tokio runtime（不用 `tokio::spawn`——CLI 进程退出会杀死 runtime task）；fire-and-forget（不 join、不 await、不传播错误），`auto_inject` 单一总开关
- **LLM HTTP client 复用**：同一 `analyze_with_action` 内的前置调用与主调用共享一个 `async_openai::Client`（`build_llm_client` + `call_llm_api_with_client`），消除冷路径第二次 TLS 握手；跨线程（prewarm）各自建 client

## 11. V8 Research Asset & Execution Platform 范式

- **引用而非嵌入**：Snapshot 通过 `EvidenceRef { id, version }` 引用 Evidence，禁止把 Evidence 数据复制进 Snapshot
- **统一身份与生命周期**：所有 Research Asset 使用 `RA-XXXXXX` 身份与 `Draft → Verified → Published → Superseded → Archived` 状态机；新增资产类型必须复用该身份/生命周期，不得另起体系
- **可复现性**：每个 Asset 携带 `dataset_hash` 和 `config_hash`，同一输入重算应得到可对比的 hash
- **门控式演进（P3 Gate）**：Evidence Score/Weight 等数值化权重必须等真实资产积累达标（1000+ 资产、30 天 Replay 稳定、2 周期 Calibration 稳定）后才设计；权重基于真实资产分布而非假设
- **计算+格式化成对模块**：`execution-replay` 内每个证据域由 `<domain>.rs`（计算）+ `<domain>_formatter.rs`（输出）成对组成，新证据沿用该结构
- **Shadow 只读约束**：Shadow Validation 产物（ShadowRiskAssessment 等）只读观察，不进交易链路；禁止 DecisionEngine 消费、禁止修改 ExecutionPolicy、禁止自动交易
