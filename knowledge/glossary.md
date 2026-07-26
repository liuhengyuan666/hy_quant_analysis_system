# Project Ubiquitous Language (统一术语表)

> 防歧义护栏：遇到任何项目特有的缩写、业务黑话、代称，必须第一时间在此追加事实定义。

| 术语 (Term) | 英文全称/别名 | 核心定义与业务内涵 | 归属限界上下文 |
| :--- | :--- | :--- | :--- |
| Regime | Market Regime | 市场状态分类（RiskOn/RiskOff/Neutral），基于宏观因子评分 | 宏观因子域 |
| Scope | Analysis Scope | 分析范围（GLOBAL/CN/HK），不同scope使用不同的regime逻辑 | 信号生成域 |
| Rotation | Rotation Ranking | 轮动排名，基于相对强弱指标（RS）的标的排序 | 轮动域 |
| Signal | Trading Signal | 交易信号（买入/防御），综合regime和rotation生成 | 信号生成域 |
| Backtest | Strategy Backtest | 策略回测，基于历史信号模拟交易表现 | 回测域 |
| Universe | Symbol Universe | 标的池，配置在config/universe.json中 | 数据获取域 |
| GT | Ground Truth | 真实市场状态标签，用于验证regime预测准确性 | 研究验证域 |
| ADR | Architecture Decision Record | 架构决策记录，存储在memory/decisions.md中 | 项目管理域 |
| HSAHP | AH Share Premium Index | AH股溢价指数，当前数据源不可用已禁用 | 数据获取域 |
| HSCEI | Hang Seng China Enterprises Index | 恒生中国企业指数，HK市场anchor标的 | 数据获取域 |
| qfq | 前复权 | 统一日线口径，Eastmoney fqt=1/Tencent qfq | 数据获取域 |
| Macro F1 | Macro Factor F1 Score | 宏观因子预测准确性的F1评分，当前目标>0.65 | 研究验证域 |
| Sharpe | Sharpe Ratio | 夏普比率，衡量风险调整后收益 | 回测域 |
| CAGR | Compound Annual Growth Rate | 复合年增长率 | 回测域 |
| DD20 | 20-Day Drawdown | 20日最大回撤 | 回测域 |
| LLM | Large Language Model | 大语言模型，用于智能报告分析 | LLM集成域 |
| Tauri | Tauri Framework | Rust编写的桌面应用框架 | 桌面端域 |
| ClickHouse | ClickHouse DB | 分析型时序数据库 | 数据存储域 |
| SQLite | SQLite DB | 本地轻状态数据库 | 数据存储域 |
| FRED API | FRED API | St. Louis Fed API (`api.stlouisfed.org/fred`)，用于获取宏观因子数据 | 数据获取域 |
| FRED Config | FRED Configuration | `config/fred.toml` 中的 FRED 获取配置，包含 `enabled` 开关和 `api_key` | 配置域 |
| TOML Config | TOML Configuration | 系统配置管理模式（LLM/FRED 等），文件位于 `config/*.toml`，支持环境变量插值 | 配置域 |
| VIX | CBOE Volatility Index | 波动率指数，宏观因子之一 | 宏观因子域 |
| NFCI | National Financial Conditions Index | 国家金融状况指数 | 宏观因子域 |
| confirmation_days | Persistence Filter Days | Regime状态确认天数，当前production=1 | 宏观因子域 |
| Wave | Research Wave | 研究波浪/阶段（如Wave 7, Wave 9） | 研究验证域 |
| Shadow Production | Shadow Production | 影子生产环境，用于验证新功能 | 部署域 |
| Pipeline | Data Pipeline | 数据管线（ingest→indicators→macro→rotation→strategy→signals） | 数据流域 |
| Gate | Pipeline Gate | 管线关卡，检查各阶段日期是否推进 | 数据流域 |
| Trust Summary | Data Trust Summary | 数据可信度摘要（freshness/completeness/provenance） | 数据质量域 |
| Breadth | Market Breadth | 市场广度指标 | 指标计算域 |
| Environment | Market Environment | 市场环境层（per-scope） | 宏观因子域 |
| Strategy Preference | Strategy Preference Score | 策略偏好评分（四类策略） | 策略域 |
| Episode | Regime Episode | Regime状态持续时间段 | 宏观因子域 |
| Persistence | Regime Persistence | Regime状态持续性过滤 | 宏观因子域 |
| State Layer | State Classification Layer | 状态分类层（描述性） | 架构域 |
| Economic Layer | Economic Prediction Layer | 经济预测层（预测性） | 架构域 |
| Allocation Layer | Portfolio Allocation Layer | 组合配置层（决策性） | 架构域 |
| 3-State | Three-State Taxonomy | 三状态分类（Favorable/Neutral/Unfavorable） | 研究验证域 |
| Z-Score | Standard Score | 标准分数，用于宏观因子标准化 | 宏观因子域 |
| Orthogonality | Factor Orthogonality | 因子正交性，衡量因子独立性 | 研究验证域 |
| MI | Mutual Information | 互信息，衡量因子与目标变量的相关性 | 研究验证域 |
| K-Means | K-Means Clustering | K均值聚类，用于regime分类 | 研究验证域 |
| PlaceholderProvider | Placeholder LLM Provider | 占位LLM提供者（无真实API时返回模拟数据） | LLM集成域 |
| Agent Profile | LLM Agent Profile | LLM代理配置（技能、模型、参数） | LLM集成域 |
| Skill Registry | LLM Skill Registry | LLM技能注册表 | LLM集成域 |
| i18n | Internationalization | 国际化（当前支持zh/en） | 前端域 |
| Reactive Store | Vue Reactive Store | Vue 3响应式状态存储 | 前端域 |
| Event Bridge | Tauri Event Bridge | Tauri前后端事件桥接 | 前端域 |
| CSS Bridge | CSS Variable Bridge | CSS变量桥接（Plain JS与Vue共享样式） | 前端域 |
| Dashboard Bundle | Dashboard Bundle | 启动和scope reloads使用的数据聚合路径 | 报告与展示域 |
| Dashboard Snapshot | Dashboard Snapshot | 历史日期变化使用的数据快照路径 | 报告与展示域 |
| Freshness | Data Freshness | 数据新鲜度，衡量数据是否及时更新 | 数据质量域 |
| Completeness | Data Completeness | 数据完整性，衡量最新日样本是否全量 | 数据质量域 |
| Provenance | Data Provenance | 数据来源溯源，记录数据的生成路径和依赖 | 数据质量域 |
| Repair Window | Repair Window | 刷新管线中的自动修复窗口，用于修复被gate卡住的较早日期 | 数据流域 |
| Vite | Vite Build Tool | 前端构建工具，用于Vue 3前端打包 | 前端域 |
| Superseded | Superseded ADR/Task | 被取代的ADR/任务（不可恢复终端状态） | 项目管理域 |
| Frozen | Frozen Task | 冻结任务（等待前置条件） | 项目管理域 |
| Gated | Gated Task | 门控任务（依赖其他任务完成） | 项目管理域 |
| P0/P1/P2 | Priority Level | 优先级（P0最高，P2最低） | 项目管理域 |
| MVP | Minimum Viable Product | 最小可行产品 | 项目管理域 |
| **Execution Layer** | Execution Layer | 执行层（V5 新增），基于 Pattern Library 的收盘前执行过滤器，只决定执行时机，不创建投资想法 | 执行域 |
| **Pattern Library** | Pattern Library | 经验现象库，将市场观察现象（price action, volume, position）映射到执行指引 | 执行域 |
| **ExecutionState** | Execution State | 执行状态（BuyNow/Wait/NoChase/Reduce/Skip） | 执行域 |
| **ReasonTag** | Reason Tag | 执行决策原因标签（如 GapUpOverextended, VolumeSpike, StrongClose） | 执行域 |
| **SkipReason** | Skip Reason | 跳过原因（NoCandidate/StateGate/DataUnavailable），内部使用 | 执行域 |
| **IntradaySnapshot** | Intraday Snapshot | 实时市场快照（today_return, close_position, volume_ratio, distance_ma5 等） | 执行域 |
| **Explainability Layer** | Explainability Layer | 可解释性层，用于展示系统决策的归因拆解（TASK-092） | 可解释性域 |
| **Research Surface** | Research Surface | 研究表面，Shadow Production 期间允许新增的观测/诊断工具集合（如 symbol-diagnostics, rotation-ranking） | 治理域 |
| **Production Surface** | Production Surface | 生产表面，Shadow Production 期间冻结的核心观察链路（DashboardSnapshot, ResearchContext, Markdown Report） | 治理域 |
| **Attribution Breakdown** | Attribution Breakdown | 归因拆解，信号得分的四段贡献：Strategy (45%) + Alignment (15%) + Regime (20%) + Rotation (20%) | 可解释性域 |
| **Divergence Sample Library** | Divergence Sample Library | 分歧样本库，用于追踪 StrongBuy+DE_RISK 等模式的 T+20/T+60/T+120 收益样本 | 研究验证域 |
| **Symbol Diagnostics** | Symbol Diagnostics | 标的诊断，单标的深度归因拆解 CLI 工具（TASK-092 P0） | 可解释性域 |
| **Symbol Scoreboard** | Symbol Scoreboard | 标的记分板，全标的统一视图横向对比 CLI 工具（TASK-092 P1） | 可解释性域 |
| **TRAP** | Trap Record | 陷阱记录，记录已修复的根因和预防措施（如 TRAP-004, TRAP-005） | 项目管理域 |
| **max_partitions_per_insert_block** | ClickHouse Partition Limit | ClickHouse 单次插入分区数限制，已设置为 10000 以支持长历史数据 | 数据存储域 |
| **ResearchContext** | Research Context | V6 Reporting Platform 的 canonical semantic contract，跨消费者共享的语义模型，不承载 consumer-specific 字段 | 报告与展示域 |
| **ResearchDataset** | Research Dataset | app-service 内部的 transient raw query result，ephemeral 数据容器，不暴露到 app-service 边界之外 | 报告与展示域 |
| **ResearchSnapshot** | Research Snapshot | app-service 内部的 computation workspace，承载 SRD/Stretch/Rotation/Breadth/Analytics 结果，不直接查询数据源 | 报告与展示域 |
| **ReportInput** | Report Input | 文档生成流程独占的 document-specific payload，只承载 document payload，不重复 metadata | 报告与展示域 |
| **ReportingSnapshot** | Reporting Snapshot | 展示层 metadata + research context 的聚合快照，承载 scope/date/generated_at 等元数据 | 报告与展示域 |
| **ReportBuilder** | Report Builder Trait | 文档组装接口（Research / Audit / Review），当前无实现者，状态为 Pending Evaluation | 报告与展示域 |
| **Architecture Invariants** | Architecture Invariants | V6 Reporting Platform 的 10 条分层架构不可违反规则（ADR-069） | 架构域 |
| **Shadow Production Playbook** | Shadow Production Playbook | Shadow Production 阶段操作细则，见 `docs/shadow-production-playbook.md` | 治理域 |
| **Research Platform 1.0** | Research Platform 1.0 | 由 Observation (V6) + Market Evolution (V7.1) + Historical Evidence (V7.2) + Research Synthesis (V7.3) 四层组成的研究平台；ADR-077 起正式冻结 | 研究域 |
| **Research Content Evolution** | Research Content Evolution | 在 Research Platform 1.0 冻结后新增的市场观测、指标、因子、内容；只能通过现有冻结层消费，不修改 Semantic Architecture | 研究域 |
| **Observation Layer** | Observation Layer | V6 研究层，描述市场当前状态（Environment / Signal / Stretch / Rotation） | 研究域 |
| **Market Evolution Layer** | Market Evolution Layer | V7.1 研究层，描述市场演化（Confirmation / Recovery） | 研究域 |
| **Historical Evidence Layer** | Historical Evidence Layer | V7.2 研究层，基于 Market Fingerprint 检索历史相似样本（Normalizer / DistanceMetric / SimilarityMatcher / OutcomeProfiler / Calibration） | 研究域 |
| **Research Synthesis Layer** | Research Synthesis Layer | V7.3 研究层，基于 Evidence Aggregation 生成 Consensus（Bias / Confidence / Supporting Evidence / Contradicting Evidence） | 研究域 |
| **Confirmation** | Market Confirmation | 市场演化维度：确认当前趋势是否被广度、参与度和风险维度验证 | 研究域 |
| **Recovery** | Market Recovery | 市场演化维度：衡量市场从压力中恢复的程度 | 研究域 |
| **Market Fingerprint** | Market Fingerprint | 描述市场状态的指纹向量，由 ObservationVector 和 EvolutionVector 组成，用于历史相似匹配 | 研究域 |
| **Fingerprint** | Fingerprint | 一组可比较的市场特征向量，用于 Historical Analogues 相似性计算 | 研究域 |
| **Historical Analogues** | Historical Analogues | 通过 Market Fingerprint 在当前市场状态与历史状态之间寻找最相似样本，并给出其历史 Outcome Profile | 研究域 |
| **Outcome Profile** | Outcome Profile | 历史相似样本的前向收益与风险统计（median / mean / best / worst / positive ratio / median max drawdown） | 研究域 |
| **Distance Metric** | Distance Metric | 衡量两个 Fingerprint 之间相似度的算法（当前为 CosineDistance），属于 Historical Evidence 层内部实现 | 研究域 |
| **Similarity Matcher** | Similarity Matcher | 基于 Distance Metric 找出 Top-N 历史相似样本的组件 | 研究域 |
| **Normalizer** | Feature Normalizer | 对 Fingerprint 特征进行归一化，使不同量纲特征可比较 | 研究域 |
| **Calibration** | Research Calibration | V7.2C 校准框架：连续运行 Confirmation / Recovery / Analogues，记录等级、驱动、匹配日期和前向收益统计，输出 Markdown 校准报告 | 研究域 |
| **Calibration Baseline Version** | Calibration Baseline Version | 校准报告基线版本（从 1 开始），仅在 Evidence 语义变化时递增；实现优化不触发递增 | 研究域 |
| **Consensus** | Research Consensus | 研究综合：基于 Evidence Aggregation 对 Observation / Evolution / Historical Evidence 给出研究解释（Bias / Confidence / Evidence） | 研究域 |
| **Evidence Aggregation** | Evidence Aggregation | Consensus 的核心机制：每项证据贡献 `(source, weight)`，加权求和后通过阈值映射输出 Bias 和 Confidence；不使用硬编码 if-else 规则树 | 研究域 |
| **ConsensusConfig** | Consensus Configuration | 聚合 `EvidenceWeights`、`BiasThresholds`、`ConfidenceThresholds`、`ConflictedThresholds` 的可配置结构；默认值保持 V7.3 行为 | 研究域 |
| **ConsensusBias** | Consensus Bias | Consensus 的研究语言倾向：`Constructive` / `Neutral` / `Conflicted` / `Fragile` / `Cautious` | 研究域 |
| **Confidence** | Consensus Confidence | Consensus 的置信度：`Low` / `Medium` / `High` | 研究域 |
| **Supporting Evidence** | Supporting Evidence | 支持当前 Bias 的证据列表（WeightedEvidence） | 研究域 |
| **Contradicting Evidence** | Contradicting Evidence | 与当前 Bias 矛盾的证据列表（WeightedEvidence） | 研究域 |
| **Semantic Architecture** | Semantic Architecture | 研究平台的分层语义结构（Observation / Evolution / Evidence / Consensus）；ADR-077 后冻结，修改需新 ADR 和显式解冻 | 架构域 |
| **Candidate Evidence** | Candidate Evidence | Shadow Production 中的候选失效证据，单一窗口可能出现，但不足以触发 Kill Criterion，需要跨多个窗口重复验证 | 治理域 |
| **Regime Dependency** | Regime Dependency | 量化模型或信号的收益表现依赖于特定市场状态（Regime），在不同 Regime 下可能有效或失效 | 研究域 |
| **Model Bias** | Model Bias | 模型对特定市场结构、主题或 Regime 的系统性偏好或压制，不是随机误差，而是可识别的结构性偏差 | 研究域 |
| **Systematic Bias** | Systematic Bias | 模型在特定条件下对多个标的或整个主题产生的系统性错误，例如 State Layer 在高动量/低波动/成长扩散环境下全面压制 Growth 主题 | 研究域 |
| **Failure Attribution** | Failure Attribution | 研究层能力：不仅判断模型何时失效，还要解释失效时的市场环境（Breadth / Liquidity / Volatility / Macro / Economic / Crowding） | 研究域 |
| **Research Asset** | Research Asset | V8 持久化研究产物（Evidence / Snapshot，未来扩展 Knowledge / Validation / Hypothesis），可复现、可审计、可版本化，存放于本地 `workspace/` | 研究资产域 |
| **RA-XXXXXX** | Research Asset Identity | V8 统一资产身份：6 位大写数字字母 ID，metadata 中的 `AssetKind` 区分类型（ADR-081） | 研究资产域 |
| **Asset Lifecycle** | Research Asset Lifecycle | V8 统一生命周期状态机：`Draft → Verified → Published → Superseded → Archived`（ADR-080） | 研究资产域 |
| **EvidenceRef** | Evidence Reference | Snapshot 对 Evidence 的引用结构 `{ id, version }`，引用而非嵌入（ADR-079） | 研究资产域 |
| **Workspace** | Research Asset Workspace | V8 本地研究资产目录（gitignored），含 `evidence/`、`snapshots/`、`registry/*-index.json`，由 `app-service::workspace` 管理 | 研究资产域 |
| **Shadow Validation** | Shadow Validation | V8 Execution Platform Phase 2C：在真实市场环境验证 Evidence → Risk State → Shadow Assessment 链路稳定性的只读工作流 | 执行域 |
| **ShadowRiskAssessment** | Shadow Risk Assessment | Shadow Validation 每日产出的风险评估结果；只读观察，禁止 DecisionEngine 消费 | 执行域 |
| **Context Integrity Gate** | Execution Context Integrity Gate | 执行上下文完整性关卡，校验历史/当日数据上下文是否满足 Shadow Assessment 前置条件（`execution-context-integrity-gate`） | 执行域 |
| **Evidence Registry** | Evidence Registry | V8 Execution Platform 的证据登记表（`evidence-registry`），登记 execution-replay 计算的各类证据及其依赖 | 执行域 |
| **Execution Replay** | Execution Replay | V8 证据重放能力（`crates/execution-replay`），基于 ExecutionEvent 重放计算 Holding Risk / Risk Lifecycle / Decision Gate 等证据 | 执行域 |
| **Holding Risk** | Holding Risk Bundle | 持仓风险证据族（Bundle / Calibration / Persistence），execution-replay 的核心证据之一 | 执行域 |
| **Risk Lifecycle** | Risk Lifecycle | 风险状态生命周期状态机分析（`risk-lifecycle`），观察 HIGH_RISK 等状态的持续与切换 | 执行域 |
| **Persona** | LLM Persona | LLM 分析视角人格（6 个内置 action + `config/prompts.toml` 自定义）；只承载视角指令，禁含阈值规则（ADR-106） | LLM集成域 |
| **Previous Interpretation** | 前次解读 | 上一次 LLM 分析结果，自动注入下次 prompt；标注为非证据背景，永不作为证据输入（ADR-106） | LLM集成域 |
| **LLM History** | LLM Analysis History | `workspace/llm-history/{scope}/{action}/{date}.json` 下的 LLM 分析对话存档，支撑前次解读回环 | LLM集成域 |
| **portfolio_review** | Portfolio Review Action | LLM action：解读确定性引擎产出的组合姿态（Increase/Maintain/Reduce/Avoid），LLM 只解释不决策（ADR-106） | LLM集成域 |
| **market_adversarial_lens** | Market Adversarial Lens | 市场博弈视角 persona（ADR-109）：从资金角色冲突/强制卖盘/被套资金/预期差/信号生命周期 5 维解读市场博弈结构；文件 persona，含 web search 引导词（ADR-111） | LLM集成域 |
| **Shared Adversarial Context** | 共享博弈假设背景 | ADR-112：每 scope 每日一次的博弈分析落盘 `workspace/llm-history/{scope}/adversarial/`，按 persona 分级注入各 prompt；语义为"供验证或反驳的假设"，非结论 | LLM集成域 |
| **InjectLevel** | Inject Level (full/standard/compact/none) | ADR-113/114 注入级别：full=完整 analysis_text / standard=analysis_text 默认正文（等价 full，截断待 TASK-215）/ compact=record.summary 结构化摘要~400字符 / none=不注入；配置于 `[llm.adversarial.inject]`，CLI `--adversarial` 可单次覆盖 | LLM集成域 |
